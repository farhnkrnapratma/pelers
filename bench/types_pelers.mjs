import { readFileSync } from 'node:fs';
import { render } from 'pele';
const corpus = JSON.parse(readFileSync('corpus.json', 'utf8'));
const per = new Map();
for (const item of corpus) {
  const run = () => render(item.source);
  for (let i = 0; i < 3; i++) run();
  let best = Infinity;
  for (let k = 0; k < 3; k++) {
    const start = performance.now();
    let iters = 0;
    do { run(); iters++; } while (performance.now() - start < 10);
    best = Math.min(best, (performance.now() - start) / iters);
  }
  const e = per.get(item.type) ?? { ms: 0, n: 0 };
  e.ms += best; e.n++;
  per.set(item.type, e);
}
for (const [t, e] of [...per].sort()) console.log(`TYPE ${t} ${(e.ms / e.n).toFixed(5)}`);
