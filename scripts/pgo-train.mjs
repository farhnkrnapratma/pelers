import { readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { root } from "./oracle.mjs";

const engine = await import(pathToFileURL(resolve(process.argv[2])).href);
engine.setOffsets((ms) => -new Date(ms).getTimezoneOffset() * 60000);
const golden = new URL("tests/golden/", `file://${root}`);
const sources = [
  ...JSON.parse(readFileSync(new URL("bench/corpus.json", `file://${root}`), "utf8")).map(
    (item) => item.source,
  ),
  ...readdirSync(golden, { recursive: true })
    .filter((name) => name.endsWith(".mmd"))
    .map((name) => readFileSync(new URL(name, golden), "utf8")),
];
const measurer = (text, size, style) => text.length * size * (style & 1 ? 0.62 : 0.55);
const now = Date.UTC(2026, 0, 15, 10);
for (let round = 0; round < 20; round++)
  for (const source of sources) {
    engine.renderSource(source, now);
    if (round % 5 === 0) {
      engine.renderSource(source, now, { maxWidth: 320, idPrefix: "x" }, undefined, undefined, measurer);
      const parsed = engine.parse(source);
      if (parsed instanceof engine.Parsed) engine.renderParsed(parsed, now);
    }
  }
