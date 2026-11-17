import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { createRequire } from "node:module";
import {
  buildCorpus,
  referenceOptions,
  root,
  sha256,
  toPelers,
} from "./corpus.mjs";

const oracle = await import(
  createRequire(import.meta.url).resolve("pele")
).then((m) => m);
const PASSES = 5;
const MIN_SAMPLE_MS = 25;

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

const corpus = buildCorpus();
const results = [];
for (const { type, source } of corpus) {
  const render = () => oracle.render(source, referenceOptions(type));
  let entry;
  try {
    const out = await render();
    const svg = toPelers(typeof out === "string" ? out : out.svg);
    entry = { type, sha: sha256(svg), ms: timeSync(() => render()) };
  } catch (error) {
    entry = {
      type,
      sha: null,
      error: String(error?.message ?? error).split("\n")[0],
      ms: null,
    };
  }
  results.push(entry);
}
writeFileSync(
  join(root, "bench", "oracle-results.json"),
  JSON.stringify(results),
);
console.log(
  `oracle rows ${results.length}, errors ${results.filter((r) => r.sha === null).length}`,
);
