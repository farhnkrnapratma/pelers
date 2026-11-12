import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import puppeteer from 'puppeteer-core';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const require = createRequire(import.meta.url);
const fixtures = join(root, 'tests', 'fixtures');
const PER_TYPE = Number(process.env.BENCH_PER_TYPE ?? 20);
const PASSES = 5;
const MIN_SAMPLE_MS = 25;
const CHROME = process.env.CHROME ?? '/usr/sbin/google-chrome';

const pelers = await import(join(root, 'index.native.js'));
const reference = await import('pele');

const corpus = [];
if (process.env.BENCH_CORPUS === 'fixtures') {
  for (const file of readdirSync(fixtures).filter((f) => f.startsWith('render-') && !f.startsWith('render-mut-'))) {
    const type = file.slice('render-'.length, -'.json'.length);
    const cases = JSON.parse(readFileSync(join(fixtures, file), 'utf8')).cases;
    let taken = 0;
    for (const c of cases) {
      if (taken >= PER_TYPE) break;
      if (c.error || !c.source || Object.keys(c.options ?? {}).length > 0) continue;
      corpus.push({ type, source: c.source });
      taken++;
    }
  }
} else {
  for (const item of JSON.parse(readFileSync(join(root, 'bench', 'corpus.json'), 'utf8'))) {
    corpus.push({ type: item.type, source: item.source });
  }
}
const referenceOptions = (type) => (type === 'cynefin' ? { idPrefix: 'pelers' } : undefined);
const toPelers = (svg) => svg.replace(/pele(?!rs)/g, 'pelers').replace(/Pele(?!rs)/g, 'Pelers');

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function timeSync(fn) {
  for (let i = 0; i < 3; i++) fn();
  const passes = [];
  for (let p = 0; p < PASSES; p++) {
    let count = 0;
    const start = performance.now();
    let elapsed = 0;
    do {
      fn();
      count++;
      elapsed = performance.now() - start;
    } while (elapsed < MIN_SAMPLE_MS);
    passes.push(elapsed / count);
  }
  return median(passes);
}

function treeFlow(n) {
  let s = 'flowchart TD\n';
  for (let i = 1; i < n; i++) s += `  n${Math.floor((i - 1) / 3)}[Node ${Math.floor((i - 1) / 3)}] --> n${i}[Node ${i}]\n`;
  return s;
}

const scaling = [];
for (const n of [10, 100]) {
  corpus.push({ type: `flowchart-${n}`, source: treeFlow(n) });
}

let browser;
let page;

async function launchBrowser() {
  browser = await puppeteer.launch({
    executablePath: CHROME,
    headless: true,
    args: ['--no-sandbox', '--disable-gpu'],
    protocolTimeout: 60000,
  });
  page = await browser.newPage();
  await page.setContent('<!doctype html><html><body></body></html>');
  await page.addScriptTag({ content: readFileSync(require.resolve('mermaid/dist/mermaid.min.js'), 'utf8') });
  await page.evaluate(() => {
    window.mermaid.initialize({ startOnLoad: false });
    window.__benchId = 0;
  });
}

await launchBrowser();
const mermaidFailures = [];

async function restartBrowser() {
  await browser.close().catch(() => {});
  await launchBrowser();
}

async function mermaidRender(source) {
  return page.evaluate(async (text) => {
    const id = `m${window.__benchId++}`;
    try {
      const { svg } = await window.mermaid.render(id, text);
      document.getElementById(id)?.remove();
      return svg;
    } catch {
      document.getElementById(id)?.remove();
      return null;
    }
  }, source);
}

async function mermaidTime(source) {
  return page.evaluate(async (text, passes, minMs) => {
    const once = async () => {
      const id = `m${window.__benchId++}`;
      await window.mermaid.render(id, text);
      document.getElementById(id)?.remove();
    };
    for (let i = 0; i < 3; i++) await once();
    const values = [];
    for (let p = 0; p < passes; p++) {
      let count = 0;
      const start = performance.now();
      let elapsed = 0;
      do {
        await once();
        count++;
        elapsed = performance.now() - start;
      } while (elapsed < minMs);
      values.push(elapsed / count);
    }
    values.sort((a, b) => a - b);
    return values[Math.floor(values.length / 2)];
  }, source, PASSES, MIN_SAMPLE_MS);
}

const rows = [];
let identical = 0;
for (const [index, { type, source }] of corpus.entries()) {
  if (index % 25 === 0) console.error(`progress ${index}/${corpus.length}`);
  let pelersSvg;
  let referenceSvg;
  try {
    pelersSvg = pelers.render(source);
  } catch {
    pelersSvg = null;
  }
  try {
    referenceSvg = toPelers(reference.render(source, referenceOptions(type)).svg);
  } catch {
    referenceSvg = null;
  }
  if (pelersSvg === null) continue;
  if (pelersSvg === referenceSvg) identical++;
  const row = {
    type,
    pelers: timeSync(() => pelers.render(source)),
    reference: referenceSvg === null ? null : timeSync(() => reference.render(source, referenceOptions(type))),
    mermaid: null,
  };
  try {
    if ((await mermaidRender(source)) !== null) row.mermaid = await mermaidTime(source);
  } catch (error) {
    mermaidFailures.push({ index: rows.length, type, message: String(error.message).split('\n')[0] });
    await restartBrowser();
  }
  rows.push(row);
}
await browser.close();

const summary = {};
for (const row of rows) {
  const s = (summary[row.type] ??= { n: 0, pelers: 0, reference: 0, referenceN: 0, mermaid: 0, mermaidN: 0 });
  s.n++;
  s.pelers += row.pelers;
  if (row.reference !== null) {
    s.reference += row.reference;
    s.referenceN++;
  }
  if (row.mermaid !== null) {
    s.mermaid += row.mermaid;
    s.mermaidN++;
  }
}

const mean = (sum, n) => (n === 0 ? null : sum / n);
const fmt = (v) => (v === null ? '-' : v.toFixed(3));
const ratio = (a, b) => (a === null || b === null ? '-' : `${(b / a).toFixed(1)}x`);
const lines = [
  '| type | n | pelers ms | pele ms | mermaid ms | vs pele | vs mermaid |',
  '| --- | ---: | ---: | ---: | ---: | ---: | ---: |',
];
const totals = { pelers: 0, reference: 0, referenceN: 0, mermaid: 0, mermaidN: 0, n: 0 };
for (const [type, s] of Object.entries(summary).sort()) {
  const pelersMs = mean(s.pelers, s.n);
  const referenceMs = mean(s.reference, s.referenceN);
  const mermaidMs = mean(s.mermaid, s.mermaidN);
  lines.push(`| ${type} | ${s.n} | ${fmt(pelersMs)} | ${fmt(referenceMs)} | ${fmt(mermaidMs)} | ${ratio(pelersMs, referenceMs)} | ${ratio(pelersMs, mermaidMs)} |`);
  totals.n += s.n;
  totals.pelers += s.pelers;
  totals.reference += s.reference;
  totals.referenceN += s.referenceN;
  totals.mermaid += s.mermaid;
  totals.mermaidN += s.mermaidN;
}
const allPeler = mean(totals.pelers, totals.n);
const allReference = mean(totals.reference, totals.referenceN);
const allMermaid = mean(totals.mermaid, totals.mermaidN);
lines.push(`| **all** | ${totals.n} | ${fmt(allPeler)} | ${fmt(allReference)} | ${fmt(allMermaid)} | ${ratio(allPeler, allReference)} | ${ratio(allPeler, allMermaid)} |`);
lines.push('');
lines.push(`pelers output byte-identical to pele npm (pele->pelers normalized, idPrefix pelers only for cynefin): ${identical} of ${corpus.length}`);
lines.push(`mermaid failures: ${mermaidFailures.length}`);
console.log(lines.join('\n'));
writeFileSync(join(root, 'bench', 'results.json'), JSON.stringify({ corpus: corpus.length, identical, rows, summary, mermaidFailures }, null, 2));
