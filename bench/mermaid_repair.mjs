import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import puppeteer from 'puppeteer-core';
import { root } from './corpus.mjs';

const require = createRequire(import.meta.url);
const mermaidJs = readFileSync(require.resolve('mermaid/dist/mermaid.min.js'), 'utf8');
const corpus = JSON.parse(readFileSync(join(root, 'bench', 'corpus.json'), 'utf8'));
const indices = [21, 22, 23, 24, 25, 26, 27, 44, 45, 95, 177, 194, 265, 416, 417, 452, 453, 460, 487, 525, 535];
const PROBE_TIMEOUT = 4000;

let browser;
let page;
let counter = 0;

async function open() {
    if (browser) await browser.close().catch(() => {});
    browser = await puppeteer.launch({ executablePath: '/usr/sbin/google-chrome', headless: true, args: ['--no-sandbox', '--disable-gpu'], protocolTimeout: PROBE_TIMEOUT });
    page = await browser.newPage();
    await page.setContent('<!doctype html><html><body></body></html>');
    await page.addScriptTag({ content: mermaidJs });
    await page.evaluate(() => window.mermaid.initialize({ startOnLoad: false }));
}

async function renders(text) {
    counter++;
    try {
        return await page.evaluate(async (t, id) => {
            try {
                await window.mermaid.render(id, t);
                return true;
            } catch {
                return false;
            }
        }, text, `r${counter}`);
    } catch {
        await open();
        return false;
    }
}

function frontMatterEnd(lines) {
    if (lines[0] !== '---') return 0;
    const close = lines.indexOf('---', 1);
    return close < 0 ? 0 : close + 1;
}

function stripAccDescr(lines) {
    const out = [];
    for (let i = 0; i < lines.length; i++) {
        if (/^accDescr \{/.test(lines[i])) {
            while (i < lines.length && lines[i] !== '}') i++;
            continue;
        }
        out.push(lines[i]);
    }
    return out;
}

async function repair(source) {
    let lines = stripAccDescr(source.split('\n'));
    if (await renders(lines.join('\n'))) return { lines, removed: [] };
    const fm = frontMatterEnd(lines);
    const first = Math.max(fm, 0);
    const body = () => lines.map((_, i) => i).filter((i) => i >= first && i + 1 < lines.length + 1);
    const headerIndex = first;
    const candidates = () => lines.map((_, i) => i).filter((i) => i > headerIndex);
    const without = (drop) => lines.filter((_, i) => !drop.includes(i));

    for (const i of candidates()) {
        if (await renders(without([i]).join('\n'))) return { lines: without([i]), removed: [lines[i]] };
    }
    for (const i of candidates()) {
        for (const j of candidates()) {
            if (j <= i) continue;
            if (await renders(without([i, j]).join('\n'))) return { lines: without([i, j]), removed: [lines[i], lines[j]] };
        }
    }
    void body;
    return null;
}

await open();
const results = [];
for (const index of indices) {
    const { type, source } = corpus[index];
    const found = await repair(source);
    if (found) {
        const text = found.lines.join('\n');
        console.log(`#${index} ${type} repaired, removed ${found.removed.length}: ${JSON.stringify(found.removed)}`);
        results.push({ index, type, ok: true, source: text, original: source, removed: found.removed });
    } else {
        console.log(`#${index} ${type} NO REPAIR FOUND`);
        results.push({ index, type, ok: false, source, original: source, removed: [] });
    }
}
writeFileSync(join(root, 'bench', 'mermaid-repairs.json'), JSON.stringify(results, null, 2));
await browser.close().catch(() => {});
