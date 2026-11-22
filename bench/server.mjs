import { createRequire } from "node:module";
import { join } from "node:path";
import { buildCorpus, referenceOptions, root, toPelers } from "./corpus.mjs";

process.env.TZ = "UTC";
const PASSES = 5;
const MIN_SAMPLE_MS = 25;
const NOW = Date.UTC(2026, 0, 15, 10);
const require = createRequire(import.meta.url);

const pelers = await import(join(root, "index.js"));
const pele = await import(require.resolve("pele"));
const runtime =
  typeof Bun === "undefined" ? `node ${process.version}` : `bun ${Bun.version}`;

const median = (values) =>
  [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];

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

const attempt = (fn) => {
  try {
    return fn();
  } catch {
    return null;
  }
};

console.error(`server: pelers napi against pele npm under ${runtime}`);
const rows = buildCorpus().map(({ type, source }) => {
  const options = { ...referenceOptions(type), now: NOW };
  const ours = attempt(() => pelers.render(source, { now: NOW }).svg);
  const theirs = attempt(() => toPelers(pele.render(source, options).svg));
  return {
    type,
    same: ours !== null && ours === theirs,
    pelers:
      ours === null
        ? null
        : timeSync(() => pelers.render(source, { now: NOW })),
    pele: theirs === null ? null : timeSync(() => pele.render(source, options)),
  };
});
process.stdout.write(JSON.stringify({ runtime, rows }));
