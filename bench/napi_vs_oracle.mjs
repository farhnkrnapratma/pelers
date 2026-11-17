import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildCorpus, root, sha256 } from "./corpus.mjs";

const pelers = await import(join(root, "index.native.js"));
const oracle = JSON.parse(
  readFileSync(join(root, "bench", "oracle-results.json"), "utf8"),
);
const corpus = buildCorpus();
const ROUNDS = 5;
const MIN_MS = 25;

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
let identical = 0;
for (const [index, { type, source }] of corpus.entries()) {
  for (let i = 0; i < 3; i++) pelers.render(source);
  const times = [];
  for (let round = 0; round < ROUNDS; round++)
    times.push(timeOnce(() => pelers.render(source)));
  const svg = pelers.render(source);
  if (sha256(svg) === oracle[index].sha) identical++;
  rows.push({ type, index, pelers: median(times), oracle: oracle[index].ms });
}

const byType = {};
for (const row of rows) {
  const entry = (byType[row.type] ??= {
    n: 0,
    pelers: 0,
    oracle: 0,
    ratios: [],
  });
  entry.n++;
  entry.pelers += row.pelers;
  entry.oracle += row.oracle;
  entry.ratios.push(row.oracle / row.pelers);
}
const lines = [
  "| type | n | pelers ms | bun oracle ms | speedup vs oracle (sum) | min per-source |",
  "| --- | ---: | ---: | ---: | ---: | ---: |",
];
const all = { n: 0, pelers: 0, oracle: 0 };
for (const [type, e] of Object.entries(byType).sort()) {
  lines.push(
    `| ${type} | ${e.n} | ${(e.pelers / e.n).toFixed(4)} | ${(e.oracle / e.n).toFixed(4)} | ${(e.oracle / e.pelers).toFixed(2)}x | ${Math.min(...e.ratios).toFixed(2)}x |`,
  );
  all.n += e.n;
  all.pelers += e.pelers;
  all.oracle += e.oracle;
}
lines.push(
  `| **all** | ${all.n} | ${(all.pelers / all.n).toFixed(4)} | ${(all.oracle / all.n).toFixed(4)} | ${(all.oracle / all.pelers).toFixed(2)}x | |`,
);
lines.push("");
lines.push(
  `pelers output byte-identical to bun oracle: ${identical} of ${corpus.length}`,
);
const out = lines.join("\n");
console.log(out);
writeFileSync(join(root, "bench", "napi_vs_oracle.md"), `${out}\n`);
writeFileSync(
  join(root, "bench", "napi_vs_oracle.json"),
  JSON.stringify(rows, null, 1),
);
