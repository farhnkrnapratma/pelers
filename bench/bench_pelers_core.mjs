import { readFileSync } from 'node:fs';
import { render } from 'pele';
const corpus = JSON.parse(readFileSync('corpus.json', 'utf8'));
let total = 0;
for (const item of corpus) {
  for (let i = 0; i < 3; i++) render(item.source);
  let iters = 0;
  const start = performance.now();
  do {
    render(item.source);
    iters++;
  } while (performance.now() - start < 25);
  total += (performance.now() - start) / iters;
}
console.log(`pele core mean ms per render: ${(total / corpus.length).toFixed(4)} over ${corpus.length} sources`);
