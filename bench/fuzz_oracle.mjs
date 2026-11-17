import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const [casesPath, outPath] = process.argv.slice(2);
const oracle = await import(
  createRequire(import.meta.url).resolve("pele")
).then((m) => m);
const cases = JSON.parse(readFileSync(casesPath, "utf8"));
const toPelers = (svg) =>
  svg.replace(/pele(?!rs)/g, "pelers").replace(/Pele(?!rs)/g, "Pelers");
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
