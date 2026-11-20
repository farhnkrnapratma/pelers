import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import assert from "node:assert/strict";
import { peleCore, peleDir, toPelers } from "./oracle.mjs";

process.env.TZ = "UTC";

const root = new URL("../", import.meta.url);
const read = (name) => readFileSync(new URL(name, root), "utf8");

const pkg = JSON.parse(read("package.json"));
assert.equal(pkg.exports["."].types, "./index.d.ts");

const dts = read("index.d.ts");
for (const name of [
  "parse",
  "render",
  "detectType",
  "supports",
  "register",
  "registered",
  "forgetTextWidths",
])
  assert.match(dts, new RegExp(`function ${name}\\(`), `index.d.ts declares ${name}`);

if (!existsSync(new URL("index.native.js", root))) {
  console.log("index.native.js not built, skipping runtime checks");
  process.exit(0);
}

const pelers = await import(new URL("index.js", root));
const { PelersError, detectType, parse, render, supports } = pelers;

const flow = render("flowchart LR\n  A[Start] --> B[End]");
assert.match(flow.svg, /^<svg/);
assert.ok(flow.width > 0 && flow.height > 0);
assert.equal(flow.type, "flowchart");
assert.equal(flow.mermaidType, "flowchart-v2");
assert.deepEqual(flow.links, []);
assert.equal(detectType("graph TD\n  A-->B"), "flowchart");
assert.equal(detectType("---\ntitle: T\n---\npie\n  \"a\": 1"), "pie");
assert.equal(detectType("hello"), null);
assert.equal(supports("hello"), false);
assert.equal(supports("sequenceDiagram\n  A->>B: hi"), true);
assert.equal(pelers.registered("venn"), true);
const info = render("info showInfo");
assert.match(info.svg, />Pelers v[^<]+<\/text>/);
const model = parse("flowchart LR\n  A --> B", { maxEdges: 10 });
assert.equal(model.type, "flowchart");
assert.equal(model.mermaidType, "flowchart-v2");
assert.equal(render(model).svg, render("flowchart LR\n  A --> B").svg);
assert.throws(() => render(42), TypeError);
assert.throws(
  () => render("hello"),
  (error) => error instanceof PelersError && error.code === "unsupported-diagram",
);
assert.throws(
  () => render("flowchart LR\n  A --> B", { outputLimit: 10 }),
  (error) => error instanceof PelersError && error.code === "limit" && error.type === "flowchart",
);
const boom = () => {
  throw new Error("boom");
};
assert.throws(() => render("flowchart LR\n  A[fa:fa-car x]", { icons: boom }), /boom/);

if (!existsSync(join(peleDir, "src/index.ts"))) {
  console.log(`bindings ok (parity skipped: no pele checkout at ${peleDir})`);
  process.exit(0);
}

const pele = await peleCore();
const corpus = join(peleDir, "tests/corpus");
const sources = readdirSync(corpus)
  .filter((name) => name.endsWith("-docs.json"))
  .sort()
  .flatMap((name) => JSON.parse(readFileSync(join(corpus, name), "utf8")));

const widths = { width: (text, size, style) => text.length * size * (style & 1 ? 0.62 : 0.55) };
const OPTIONS = [
  {},
  { fontSize: 20, padding: 0 },
  { maxWidth: 320 },
  { maxWidth: 320, autoDirection: false },
  { directionBreakpoint: 2000, maxWidth: 900 },
  { responsive: false, idPrefix: "x" },
  { monospace: ["members", "attributes", "files", "messages"] },
  { fontFamily: "Inter", fontFamilyMono: "Fira Code" },
  { links: true, linkSchemes: ["https", "mailto"], linkRel: "" },
  { images: false, imageSchemes: ["https"] },
  { now: Date.UTC(2026, 0, 15, 10) },
  { now: new Date(Date.UTC(2031, 6, 4)) },
  { config: { theme: "dark", flowchart: { curve: "linear" }, themeVariables: { pie1: "#123456" } } },
  { limit: 60 },
  { maxEdges: 2 },
  { outputLimit: 3000 },
  { measurer: widths },
  {
    icons: (name) => (name.startsWith("fa:") ? `<path d="M1 1" data-n="${name.length}"/>` : null),
    math: (tex, size) =>
      tex.includes("?")
        ? { html: "<span>m</span>", width: tex.length * size * 0.5, height: size * 1.2 }
        : { svg: '<path d="M0 0"/>', width: tex.length * size * 0.5, height: size * 1.2 },
  },
];

const outcome = (run) => {
  try {
    const { svg, width, height, links, type, mermaidType } = run();
    return { svg: toPelers(svg), width, height, links, type, mermaidType };
  } catch (error) {
    if (error instanceof TypeError || error instanceof ReferenceError) throw error;
    const { code, type, mermaidType, line, column, snippet } = error;
    return { error: toPelers(error.message), code, type, mermaidType, line, column, snippet };
  }
};

let cases = 0;
const failures = [];
for (const [index, source] of sources.entries())
  for (const [set, given] of OPTIONS.entries()) {
    const options = { now: Date.UTC(2026, 10, 20, 9), ...given };
    const want = outcome(() => pele.render(source, options));
    const got = outcome(() => render(source, options));
    cases += 1;
    try {
      assert.deepEqual(got, want);
    } catch {
      failures.push(`source ${index} options ${set} (${got.type ?? got.code})`);
    }
  }
if (failures.length > 0) {
  const tally = {};
  for (const failure of failures) {
    const key = failure.replace(/^source \d+ /, "");
    tally[key] = (tally[key] ?? 0) + 1;
  }
  console.log(tally);
}
assert.deepEqual(failures.slice(0, 20), [], `${failures.length} of ${cases} differ`);
console.log(`bindings ok (${cases} option cases match pele)`);
