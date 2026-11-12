import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const pelers = await import(join(root, 'index.native.js'));
const reference = await import('pele');
const corpus = JSON.parse(readFileSync(join(root, 'bench', 'corpus.json'), 'utf8'));
const ROUNDS = 5;
const MIN_MS = 25;
const referenceOptions = (type) => (type === 'cynefin' ? { idPrefix: 'pelers' } : undefined);

function timeOnce(fn) {
  let count = 0;
  const start = performance.now();
  let elapsed = 0;
  do {
    fn();
    count++;
    elapsed = performance.now() - start;
  } while (elapsed < MIN_MS);
  return elapsed / count;
}

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

const rows = [];
for (const { type, source } of corpus) {
  const opts = referenceOptions(type);
  for (let i = 0; i < 3; i++) {
    pelers.render(source);
    reference.render(source, opts);
  }
  const pelersTimes = [];
  const referenceTimes = [];
  for (let round = 0; round < ROUNDS; round++) {
    if (round % 2 === 0) {
      pelersTimes.push(timeOnce(() => pelers.render(source)));
      referenceTimes.push(timeOnce(() => reference.render(source, opts)));
    } else {
      referenceTimes.push(timeOnce(() => reference.render(source, opts)));
      pelersTimes.push(timeOnce(() => pelers.render(source)));
    }
  }
  rows.push({ type, pelers: median(pelersTimes), reference: median(referenceTimes) });
}

const byType = {};
for (const row of rows) {
  const entry = (byType[row.type] ??= { n: 0, pelers: 0, reference: 0, ratios: [] });
  entry.n++;
  entry.pelers += row.pelers;
  entry.reference += row.reference;
  entry.ratios.push(row.reference / row.pelers);
}
const lines = ['| type | n | pelers ms | pele ms | speedup vs pele (sum) | min per-source |', '| --- | ---: | ---: | ---: | ---: | ---: |'];
const all = { n: 0, pelers: 0, reference: 0 };
for (const [type, e] of Object.entries(byType).sort()) {
  lines.push(`| ${type} | ${e.n} | ${(e.pelers / e.n).toFixed(4)} | ${(e.reference / e.n).toFixed(4)} | ${(e.reference / e.pelers).toFixed(2)}x | ${Math.min(...e.ratios).toFixed(2)}x |`);
  all.n += e.n;
  all.pelers += e.pelers;
  all.reference += e.reference;
}
lines.push(`| **all** | ${all.n} | ${(all.pelers / all.n).toFixed(4)} | ${(all.reference / all.n).toFixed(4)} | ${(all.reference / all.pelers).toFixed(2)}x | |`);
const out = lines.join('\n');
console.log(out);
writeFileSync(join(root, 'bench', 'napi_vs_pele.md'), `${out}\n`);
writeFileSync(join(root, 'bench', 'napi_vs_pele.json'), JSON.stringify(rows, null, 1));
