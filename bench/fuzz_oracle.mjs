import { toPelers } from "./corpus.mjs";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";

const [casesPath, outPath] = process.argv.slice(2);
const oracle = await import(
  createRequire(import.meta.url).resolve("pele")
).then((m) => m);
const cases = JSON.parse(readFileSync(casesPath, "utf8"));
const referenceOptions = (type) =>
  type === "cynefin" ? { idPrefix: "pelers" } : undefined;

const results = [];
for (const { source, type } of cases) {
  try {
    const out = await oracle.render(source, referenceOptions(type));
    const svg = typeof out === "string" ? out : out.svg;
    results.push({ ok: true, svg: toPelers(svg) });
  } catch (error) {
    results.push({
      ok: false,
      message: String(error?.message ?? error).split("\n")[0],
    });
  }
}
writeFileSync(outPath, JSON.stringify(results));
