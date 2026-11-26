import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pelers = (await import(join(root, "..", "index.js"))) as {
  render: (source: string) => { svg: string };
  detectType: (source: string) => string | null;
};

const generated = join(root, "src/lib/generated");
mkdirSync(generated, { recursive: true });

const examples = JSON.parse(
  readFileSync(join(root, "src/lib/content/examples.json"), "utf8"),
) as Record<string, string>;
const exampleSvgs: Record<string, string> = {};
for (const [name, source] of Object.entries(examples)) {
  if (name.endsWith("Example")) {
    exampleSvgs[name] = pelers.render(source).svg;
  }
}
writeFileSync(join(generated, "examples.json"), JSON.stringify(exampleSvgs));

const markdownDirs = [
  join(root, "src/lib/content"),
  join(root, "src/lib/content/docs"),
];
const diagrams: Record<string, string> = {};
const fence = /```mermaid\n([\s\S]*?)```/g;
for (const dir of markdownDirs) {
  for (const file of readdirSync(dir).filter((name) => name.endsWith(".md"))) {
    const text = readFileSync(join(dir, file), "utf8");
    for (const match of text.matchAll(fence)) {
      const source = match[1];
      if (!(source in diagrams)) {
        diagrams[source] = pelers.render(source).svg;
      }
    }
  }
}
writeFileSync(join(generated, "diagrams.json"), JSON.stringify(diagrams));
const golden = join(root, "..", "tests/golden");
const samples: Record<string, string> = {};
for (const dir of readdirSync(golden, { withFileTypes: true })) {
  if (!dir.isDirectory()) continue;
  const source = readFileSync(join(golden, dir.name, "000.mmd"), "utf8");
  const type = pelers.detectType(source);
  if (type && !(type in samples)) samples[type] = source;
}
writeFileSync(
  join(generated, "samples.json"),
  JSON.stringify(
    Object.fromEntries(
      Object.entries(samples).sort(([a], [b]) => a.localeCompare(b)),
    ),
  ),
);
console.log(
  `rendered ${Object.keys(exampleSvgs).length} examples and ${Object.keys(diagrams).length} diagrams, ${Object.keys(samples).length} playground samples`,
);
