import {
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const root = fileURLToPath(new URL("../", import.meta.url));
export const peleDir = resolve(root, process.env.PELE_DIR ?? "../pele");
export const fixtures =
  process.env.PELERS_FUZZ_OUT ?? join(root, "tests/fixtures");

const upstream = "pele";
const upstreamName = /pele(?!rs)/g;
const upstreamTitle = /Pele(?!rs)/g;

export const version = /^version = "([^"]+)"/m.exec(
  readFileSync(join(root, "Cargo.toml"), "utf8"),
)[1];

export function peleSrc(path) {
  if (!existsSync(join(peleDir, "src/index.ts")))
    throw new Error(
      `pele checkout not found at ${peleDir}; clone obsidianmd/pele at 1.13.0 there or set PELE_DIR`,
    );
  return import(join(peleDir, "src", path));
}

export const toPelers = (text) =>
  text.replace(upstreamName, "pelers").replace(upstreamTitle, "Pelers");

export const toUpstream = (text) =>
  text.replace(/--pelers/g, `--${upstream}`);

function contentId(text) {
  let a = 0x811c9dc5;
  let b = 5381;
  for (let i = 0; i < text.length; i++) {
    const c = text.charCodeAt(i);
    a = Math.imul(a ^ c, 0x01000193) >>> 0;
    b = (Math.imul(b, 33) + c) >>> 0;
  }
  return a.toString(36) + b.toString(36);
}

const sankeyGradient =
  /<linearGradient id="([^"]*-sankey-)([0-9a-z]+)" gradientUnits="userSpaceOnUse" ([^]*?)<\/linearGradient>/g;

function rehashSankey(svg) {
  const ids = new Map();
  for (const [, prefix, hash, stops] of svg.matchAll(sankeyGradient))
    ids.set(`${prefix}${hash}"`, `${prefix}${contentId(stops)}"`);
  for (const [before, after] of ids) svg = svg.replaceAll(before, after);
  for (const [before, after] of ids)
    svg = svg.replaceAll(
      `#${before.slice(0, -1)})`,
      `#${after.slice(0, -1)})`,
    );
  return svg;
}

export const normalize = (svg) => rehashSankey(toPelers(svg));

export const writeFixture = (path, text) =>
  writeFileSync(path, toPelers(text));

export async function peleCore() {
  const core = await peleSrc("index.ts");
  const { info } = await peleSrc("diagrams/info/index.ts");
  const { labelSvg } = await peleSrc("svg/builder.ts");
  const { svgDocument } = await peleSrc("svg/root.ts");
  const { layoutLabel } = await peleSrc("text/label.ts");
  const { Style, defaultMeasurer } = await peleSrc("text/measurer.ts");
  const renderInfo = (model, _config, options) => {
    const size = options.fontSize ?? 16;
    const measurer =
      options.measurer ??
      defaultMeasurer(options.fontFamily, options.fontFamilyMono);
    const pad = options.padding ?? 8;
    const title = layoutLabel(
      model.title,
      false,
      measurer,
      size,
      4000,
      Style.Bold,
    );
    const titleHeight = title.height > 0 ? title.height + 8 : 0;
    const text = layoutLabel(`Pelers v${version}`, false, measurer, size, 4000);
    const width = Math.ceil(Math.max(title.width, text.width) + 2 * pad);
    const height = Math.ceil(titleHeight + text.height + 2 * pad);
    const content =
      labelSvg(
        title,
        width / 2,
        pad + title.height / 2,
        ' class="pelers-title" font-weight="var(--_tw)"',
      ) +
      labelSvg(
        text,
        width / 2,
        pad + titleHeight + text.height / 2,
        ' class="pelers-version" fill="var(--_m)"',
      );
    return {
      svg: svgDocument("info", width, height, size, options, model, content),
      width,
      height,
      links: [],
    };
  };
  core.register({ ...info, render: renderInfo });
  return {
    ...core,
    render(source, options = {}) {
      const rendered = core.render(toUpstream(source), {
        ...(core.detectType(source) === "cynefin" ? { idPrefix: "pelers" } : {}),
        ...options,
        outputLimit: Infinity,
      });
      const svg = normalize(rendered.svg);
      const outputLimit = options.outputLimit ?? 4_000_000;
      if (svg.length > outputLimit)
        throw new core.PeleError(
          `Diagram output is longer than the limit of ${outputLimit} characters.`,
          "limit",
          { type: rendered.type, mermaidType: rendered.mermaidType },
        );
      return { ...rendered, svg };
    },
  };
}

export async function peleInternals(path, names) {
  let text = readFileSync(join(peleDir, "src", path), "utf8");
  for (const name of names)
    text = text.replace(
      new RegExp(`^(const|function) ${name}\\b`, "m"),
      `export $1 ${name}`,
    );
  text = text.replace(
    /from '\.\/([^']+)'/g,
    (_, file) => `from '${join(peleDir, "src", file)}'`,
  );
  const dir = mkdtempSync(join(tmpdir(), "pelers-oracle-"));
  const file = join(dir, path.replace(/\//g, "_"));
  writeFileSync(file, text);
  try {
    return await import(file);
  } finally {
    rmSync(dir, { recursive: true });
  }
}
