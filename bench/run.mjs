import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import puppeteer from "puppeteer-core";
import { buildCorpus, referenceOptions, root, toPelers } from "./corpus.mjs";

process.env.TZ = "UTC";
const PASSES = 5;
const MIN_SAMPLE_MS = 25;
const CHROME = process.env.CHROME ?? "/usr/sbin/google-chrome";
const NOW = Date.UTC(2026, 0, 15, 10);
const require = createRequire(import.meta.url);
const peleDist = dirname(require.resolve("pele"));

const corpus = buildCorpus();
const runtimes = [
  ["bun", "bun"],
  ["node", "node"],
].map(([label, command]) => {
  const out = execFileSync(command, [join(root, "bench/server.mjs")], {
    cwd: root,
    stdio: ["ignore", "pipe", "inherit"],
    maxBuffer: 1 << 26,
  });
  return { label, ...JSON.parse(out.toString()) };
});

const files = new Map([
  ["/", { body: "<!doctype html><html><body></body></html>", type: "text/html" }],
]);
const http = Bun.serve({
  port: 0,
  async fetch(request) {
    const path = new URL(request.url).pathname;
    if (files.has(path)) {
      const { body, type } = files.get(path);
      return new Response(body, { headers: { "content-type": type } });
    }
    const file = Bun.file(
      path.startsWith("/pele/") ? join(peleDist, path.slice(6)) : join(root, path),
    );
    return (await file.exists()) ? new Response(file) : new Response(null, { status: 404 });
  },
});

const browser = await puppeteer.launch({
  executablePath: CHROME,
  headless: true,
  args: ["--no-sandbox", "--disable-gpu"],
  protocolTimeout: 600000,
});
const chrome = await browser.version();
const page = await browser.newPage();
await page.goto(`http://localhost:${http.port}/`);
await page.addScriptTag({
  content: readFileSync(require.resolve("mermaid/dist/mermaid.min.js"), "utf8"),
});
await page.evaluate(async (entries) => {
  window.pelers = await import(entries.core);
  window.native = await import(entries.native);
  window.engines = {};
  for (const [name, path] of Object.entries(entries.engines)) {
    const engine = await import(path);
    await engine.default();
    window.engines[name] = engine;
  }
  window.pele = await import(entries.pele);
  window.mermaid.initialize({ startOnLoad: false });
  window.mermaidId = 0;
}, {
  core: "/js/core.js",
  native: "/js/native.js",
  engines: { fast: "/wasm/fast/pelers.js", small: "/wasm/small/pelers.js" },
  pele: "/pele/pele.js",
});

console.error("browser: pelers wasm against pele npm and mermaid in Chrome");
const browserRows = [];
for (const [index, { type, source }] of corpus.entries()) {
  if (index % 50 === 0) console.error(`  ${index}/${corpus.length}`);
  const row = await page.evaluate(
    async (text, options, passes, minMs) => {
      const median = (values) =>
        values.sort((a, b) => a - b)[Math.floor(values.length / 2)];
      const time = async (fn) => {
        for (let i = 0; i < 3; i++) await fn();
        const values = [];
        for (let p = 0; p < passes; p++) {
          let count = 0;
          const start = performance.now();
          let elapsed = 0;
          do {
            await fn();
            count++;
            elapsed = performance.now() - start;
          } while (elapsed < minMs);
          values.push(elapsed / count);
        }
        return median(values);
      };
      const safe = async (fn) => {
        try {
          return await fn();
        } catch {
          return null;
        }
      };
      const on = (name) => () => {
        window.native.bind(window.engines[name]);
        return window.pelers.render(text, { now: options.now });
      };
      const ours = await safe(() => on("fast")().svg);
      const theirs = await safe(() => window.pele.render(text, options).svg);
      const drawMermaid = async () => {
        const id = `m${window.mermaidId++}`;
        try {
          return (await window.mermaid.render(id, text)).svg;
        } finally {
          document.getElementById(id)?.remove();
        }
      };
      const mermaidOk = (await safe(drawMermaid)) !== null;
      return {
        ours,
        theirs,
        pelers: ours === null ? null : await time(on("fast")),
        small: ours === null ? null : await time(on("small")),
        pele: theirs === null ? null : await time(() => window.pele.render(text, options)),
        mermaid: mermaidOk ? await safe(() => time(drawMermaid)) : null,
      };
    },
    source,
    { ...referenceOptions(type), now: NOW },
    PASSES,
    MIN_SAMPLE_MS,
  );
  browserRows.push({
    type,
    same: row.ours !== null && row.theirs !== null && row.ours === toPelers(row.theirs),
    pelers: row.pelers,
    small: row.small,
    pele: row.pele,
    mermaid: row.mermaid,
  });
}
await browser.close();
http.stop();

function table(rows, columns) {
  const byType = new Map();
  for (const row of rows) {
    const entry = byType.get(row.type) ?? { type: row.type, n: 0, same: 0, rows: [] };
    entry.n++;
    if (row.same) entry.same++;
    entry.rows.push(row);
    byType.set(row.type, entry);
  }
  const mean = (list, key) => {
    const values = list.map((row) => row[key]).filter((value) => value !== null);
    return values.length === 0 ? null : values.reduce((a, b) => a + b, 0) / values.length;
  };
  const both = (list, key) => list.filter((row) => row.pelers !== null && row[key] !== null);
  const speedup = (list, key) => {
    const pairs = both(list, key);
    if (pairs.length === 0) return ["-", "-"];
    const total = pairs.reduce((a, row) => a + row[key], 0) / pairs.reduce((a, row) => a + row.pelers, 0);
    const worst = Math.min(...pairs.map((row) => row[key] / row.pelers));
    return [`${total.toFixed(2)}x`, `${worst.toFixed(2)}x`];
  };
  const fmt = (value) => (value === null ? "-" : value.toFixed(4));
  const head = ["type", "n", "identical", "pelers ms", ...columns.map((c) => `${c} ms`)];
  for (const column of columns) head.push(`vs ${column}`, `worst vs ${column}`);
  const lines = [`| ${head.join(" | ")} |`, `| ${head.map((_, i) => (i === 0 ? "---" : "---:")).join(" | ")} |`];
  const entries = [...byType.values()].sort((a, b) => a.type.localeCompare(b.type));
  entries.push({ type: "**all**", n: rows.length, same: rows.filter((row) => row.same).length, rows });
  for (const entry of entries) {
    const cells = [entry.type, entry.n, `${entry.same}/${entry.n}`, fmt(mean(entry.rows, "pelers")), ...columns.map((c) => fmt(mean(entry.rows, c)))];
    for (const column of columns) cells.push(...speedup(entry.rows, column));
    lines.push(`| ${cells.join(" | ")} |`);
  }
  return lines.join("\n");
}

const versions = `pele ${JSON.parse(readFileSync(join(peleDist, "../package.json"), "utf8")).version}, mermaid ${JSON.parse(readFileSync(join(dirname(require.resolve("mermaid/package.json")), "package.json"), "utf8")).version}`;
const markdown = `# Benchmarks

Corpus: ${corpus.length} sources (bench/corpus.json). Official packages: ${versions}. Times are medians of ${PASSES} passes of at least ${MIN_SAMPLE_MS} ms per source. "vs" is total time of the other renderer over pelers'; "worst" is the smallest per-source ratio. "identical" counts sources whose SVG equals Pele's after renaming to pelers names. The addon comes from \`bun run build:release\` (profile-guided, the build CI tests and stores) and the WebAssembly modules from \`bun run build:wasm\`.

${runtimes.map(({ runtime, rows }) => `## Server: native addon under ${runtime}\n\n${table(rows, ["pele"])}`).join("\n\n")}

## Browser: WebAssembly in ${chrome}, fast build (pelers)

${table(browserRows, ["pele", "mermaid"])}

## Browser: WebAssembly in ${chrome}, small build (pelers/small)

${table(browserRows.map((row) => ({ ...row, pelers: row.small })), ["pele", "mermaid"])}
`;
writeFileSync(join(root, "bench", "results.md"), markdown);
writeFileSync(
  join(root, "bench", "results.json"),
  `${JSON.stringify({ versions, corpus: corpus.length, server: Object.fromEntries(runtimes.map(({ label, runtime, rows }) => [label, { runtime, rows }])), browser: browserRows }, null, 1)}\n`,
);
console.log(markdown);
