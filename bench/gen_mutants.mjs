import fs, { writeFileSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { createRequire } from "node:module";
import { join } from "node:path";
import {
  buildCorpus,
  referenceOptions,
  root,
  sha256,
  toPelers,
} from "./corpus.mjs";
import { mutantCases, rng } from "./mutate.mjs";

process.env.TZ = "UTC";

const require = createRequire(import.meta.url);
const pele = await import(require.resolve("pele"));
const perSource = Number(process.env.PER_SOURCE ?? 20);
const seed = Number(process.env.SEED ?? 20260513);

const OPTION_CHOICES = [
  (r) => ({ maxWidth: [120, 200, 320, 500, 900][Math.floor(r() * 5)] }),
  (r) => ({ fontSize: [10, 14, 20][Math.floor(r() * 3)] }),
  (r) => ({ padding: [0, 4, 20][Math.floor(r() * 3)] }),
  (r) => ({ responsive: r() < 0.5 }),
  () => ({ autoDirection: true }),
  (r) => ({ directionBreakpoint: [100, 400][Math.floor(r() * 2)] }),
  (r) => ({ maxEdges: [1, 5][Math.floor(r() * 2)] }),
  () => ({ links: true, linkSchemes: ["https"] }),
  () => ({ images: false, imageSchemes: ["https"] }),
  (r) => ({ fontFamily: ["Arial", "serif"][Math.floor(r() * 2)] }),
  (r) => ({ fontFamilyMono: ["Courier", "monospace"][Math.floor(r() * 2)] }),
  () => ({ idPrefix: "x" }),
  () => ({ linkRel: "noopener" }),
  (r) => ({ limit: [30, 500][Math.floor(r() * 2)] }),
];

function pickOptions(random) {
  if (random() < 0.4) return {};
  const count = 1 + Math.floor(random() * 3);
  const options = {};
  for (let k = 0; k < count; k++) {
    Object.assign(
      options,
      OPTION_CHOICES[Math.floor(random() * OPTION_CHOICES.length)](random),
    );
  }
  return options;
}

const seeds = JSON.parse(
  fs.readFileSync(join(root, "bench", "upstream-seeds.json"), "utf8"),
);
const seedCorpus = seeds
  .map(({ source }) => ({ type: pele.detectType(source) ?? "unknown", source }))
  .filter(({ type }) => type !== "info");
const cases = mutantCases([...buildCorpus(), ...seedCorpus], perSource, seed);
const optionRandom = rng(seed + 1);
const iconMock = (name) => (name.startsWith("fa:") ? '<path d="M1 1"/>' : null);
const mathMock = (tex, size) => {
  if (tex.includes("!")) return null;
  const width = tex.length * size * 0.5 + 2;
  const height = size * 1.2;
  if (tex.includes("?")) return { html: "<span>x</span>", width, height };
  return { svg: '<path d="M0 0"/>', width, height };
};
const RealDate = Date;
const clockAt = (ms) => {
  globalThis.Date = class extends RealDate {
    constructor(...args) {
      if (args.length === 0) super(ms);
      else super(...args);
    }
    static now() {
      return ms;
    }
  };
};
const CLOCKS = [
  Date.UTC(2026, 0, 15, 10, 0, 0),
  Date.UTC(2031, 6, 4, 18, 30, 0),
];
const out = cases.flatMap(({ type, source }) => {
  const callbacks = optionRandom() < 0.35;
  const options = pickOptions(optionRandom);
  const extra = callbacks ? { icons: iconMock, math: mathMock } : {};
  const renders = CLOCKS.map((ms) => {
    clockAt(ms);
    try {
      const rendered = pele.render(source, {
        ...referenceOptions(type),
        ...options,
        ...extra,
      });
      return sha256(
        toPelers(typeof rendered === "string" ? rendered : rendered.svg),
      );
    } catch {
      return null;
    }
  });
  globalThis.Date = RealDate;
  if (renders[0] !== renders[1]) return [];
  return [
    renders[0] === null
      ? { type, source, options, callbacks, ok: false }
      : { type, source, options, callbacks, ok: true, sha256: renders[0] },
  ];
});
const fixture = {
  reference:
    "pele@1.13.0 npm under bun (pele->pelers normalized, idPrefix pelers only for cynefin)",
  seed,
  perSource,
  cases: out,
};
writeFileSync(
  join(root, "tests", "fixtures", "mutants.json.gz"),
  gzipSync(`${JSON.stringify(fixture)}\n`, { level: 9 }),
);
console.log(
  `mutants ${out.length}, ok ${out.filter((c) => c.ok).length}, errors ${out.filter((c) => !c.ok).length}`,
);
