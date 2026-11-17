import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import puppeteer from 'puppeteer-core';
const require = createRequire(import.meta.url);
const mermaidJs = readFileSync(require.resolve('mermaid/dist/mermaid.min.js'), 'utf8');
const corpus = JSON.parse(readFileSync('corpus.json', 'utf8'));
const indices = [21,22,23,24,25,26,27,36,37,41,42,43,44,45,46,47,49,54,58,87,95,98,177,194,195,196,265,305,306,360,384,387,416,417,420,447,448,449,450,452,453,454,455,456,460,467,487,525,535,550];
let browser;
let page;
async function launch() {
  browser = await puppeteer.launch({ executablePath: '/usr/sbin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-gpu'], protocolTimeout: 6000 });
  page = await browser.newPage();
  await page.setContent('<!doctype html><html><body></body></html>');
  await page.addScriptTag({ content: mermaidJs });
  await page.evaluate(() => window.mermaid.initialize({ startOnLoad: false }));
}
await launch();
let counter = 0;
async function renders(text) {
  const id = 's' + (counter++);
  try {
    return await page.evaluate(async (t, n) => { try { await window.mermaid.render(n, t); return true; } catch { return false; } }, text, id);
  } catch {
    await browser.close().catch(() => {});
    await launch();
    return false;
  }
}
const out = [];
for (const index of indices) {
  const { type, source } = corpus[index];
  if (!(await renders(source))) {
    let lines = source.split('\n');
    const header = lines[0];
    for (let i = lines.length - 1; i >= 1; i--) {
      const trial = lines.slice(0, i).concat(lines.slice(i + 1));
      if (await renders(trial.join('\n'))) lines = trial;
    }
    const subset = lines.join('\n');
    const ok = await renders(subset);
    out.push({ index, type, ok, lines: subset.split('\n').length, original: source.split('\n').length, source: subset });
    console.log(index, type, ok ? 'ok' : 'FAILED', subset.split('\n').length, '/', source.split('\n').length, JSON.stringify(header));
  } else {
    out.push({ index, type, ok: true, lines: source.split('\n').length, original: source.split('\n').length, source, unchanged: true });
    console.log(index, type, 'unchanged');
  }
}
writeFileSync('mermaid-subset.json', JSON.stringify(out, null, 2));
await browser.close().catch(() => {});
