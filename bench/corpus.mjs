import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const root = join(dirname(fileURLToPath(import.meta.url)), "..");

export const referenceOptions = (type) =>
  type === "cynefin" ? { idPrefix: "pelers" } : undefined;

export const toPelers = (svg) =>
  svg.replace(/pele(?!rs)/g, "pelers").replace(/Pele(?!rs)/g, "Pelers");

export const sha256 = (text) =>
  createHash("sha256").update(text, "utf8").digest("hex");

function treeFlow(n) {
  let s = "flowchart TD\n";
  for (let i = 1; i < n; i++)
    s += `  n${Math.floor((i - 1) / 3)}[Node ${Math.floor((i - 1) / 3)}] --> n${i}[Node ${i}]\n`;
  return s;
}

export function buildCorpus() {
  const items = JSON.parse(
    readFileSync(join(root, "bench", "corpus.json"), "utf8"),
  );
  const subsetPath = join(root, "bench", "mermaid-subset.json");
  const subsets = new Map();
  if (existsSync(subsetPath)) {
    for (const entry of JSON.parse(readFileSync(subsetPath, "utf8"))) {
      if (entry.ok) subsets.set(entry.index, entry.source);
    }
  }
  const corpus = items.map((item, index) => ({
    type: item.type,
    source: subsets.get(index) ?? item.source,
    substituted: subsets.has(index),
  }));
  for (const n of [10, 100])
    corpus.push({
      type: `flowchart-${n}`,
      source: treeFlow(n),
      substituted: false,
    });
  return corpus;
}
