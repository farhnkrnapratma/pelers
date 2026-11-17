// Times Pelers on Mermaid's flowchart documentation examples and on generated graphs.
// Run `npm run build` first. Pass --compare to also time beautiful-mermaid on the same inputs.
import { readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { render } from '../dist/pelers.js';

const docs = JSON.parse(readFileSync('tests/corpus/flowchart-docs.json', 'utf8'));

function chain(n, fan) {
  let s = 'flowchart TD\n';
  for (let i = 1; i < n; i++)
    s += `  n${Math.floor((i - 1) / fan)}[Node ${Math.floor((i - 1) / fan)}] --> n${i}[Node ${i}]\n`;
  return s;
}

function mesh(n, extra) {
  let s = 'flowchart TD\n';
  let seed = 7;
  const rnd = () => (seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0) / 4294967296;
  for (let i = 1; i < n; i++) s += `  n${Math.floor(rnd() * i)} --> n${i}\n`;
  for (let i = 0; i < extra; i++) s += `  n${Math.floor(rnd() * n)} -->|label ${i}| n${Math.floor(rnd() * n)}\n`;
  return s;
}

const cases = [
  ['docs: all examples', docs],
  ['tree: 50 nodes', [chain(50, 3)]],
  ['mesh: 50 nodes, 20 extra labelled edges', [mesh(50, 20)]],
  ['tree: 500 nodes', [chain(500, 3)]],
  ['mesh: 500 nodes, 200 extra labelled edges', [mesh(500, 200)]],
];

function time(fn, inputs) {
  const run = () => {
    let done = 0;
    for (const src of inputs) {
      try {
        fn(src);
        done++;
      } catch {
        continue;
      }
    }
    return done;
  };
  const done = run();
  let budget = 300;
  const samples = [];
  while (budget > 0 || samples.length < 5) {
    const t = performance.now();
    run();
    const ms = performance.now() - t;
    samples.push(ms);
    budget -= ms;
    if (samples.length >= 2000) break;
  }
  samples.sort((a, b) => a - b);
  return { ms: samples[samples.length >> 1], done };
}

const engines = [['pelers', (src) => render(src)]];
if (process.argv.includes('--compare')) {
  const bm = await import('beautiful-mermaid');
  engines.push(['beautiful-mermaid', (src) => bm.renderMermaidSVG(src)]);
}

const rows = [];
for (const [name, inputs] of cases) {
  const row = { case: name, diagrams: inputs.length };
  for (const [engine, fn] of engines) {
    const { ms, done } = time(fn, inputs);
    row[engine] = ms < 1 ? ms.toFixed(3) + ' ms' : ms.toFixed(1) + ' ms';
    if (done !== inputs.length) row[engine] += ` (${done} rendered)`;
  }
  rows.push(row);
}
console.table(rows);

const code = readFileSync('dist/pelers.js');
console.log(
  `bundle: ${(code.length / 1024).toFixed(1)} kB minified, ${(gzipSync(code).length / 1024).toFixed(1)} kB gzipped`
);
