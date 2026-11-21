import { defaultMeasurer } from "./measure.js";
import { mountWith } from "./mount.js";
import { native } from "./native.js";

export { forgetTextWidths } from "./measure.js";
export { enableZoom } from "./zoom.js";

export class PelersError extends Error {
  constructor(message, code = "syntax", info = {}) {
    super(message);
    this.name = "PelersError";
    this.code = code;
    this.type = info.type ?? null;
    this.mermaidType = info.mermaidType ?? null;
    this.line = info.line ?? 0;
    this.column = info.column ?? 0;
    this.snippet = info.snippet ?? "";
  }
}

const TYPES = new Set([
  "flowchart",
  "sequence",
  "class",
  "state",
  "er",
  "pie",
  "gantt",
  "gitGraph",
  "mindmap",
  "timeline",
  "journey",
  "xychart",
  "quadrantChart",
  "requirement",
  "c4",
  "sankey",
  "block",
  "packet",
  "kanban",
  "architecture",
  "radar",
  "treemap",
  "info",
  "agentflow",
  "usecase",
  "railroad",
  "treeView",
  "venn",
  "cynefin",
  "wardley",
  "swimlane",
  "ishikawa",
  "eventmodeling",
]);

const thrown = (error) =>
  new PelersError(error.message, error.code, {
    type: error.type,
    mermaidType: error.mermaidType,
    line: error.line,
    column: error.column,
    snippet: error.snippet,
  });

const KEYS = [
  "limit",
  "maxEdges",
  "config",
  "fontFamily",
  "fontFamilyMono",
  "monospace",
  "fontSize",
  "idPrefix",
  "responsive",
  "maxWidth",
  "autoDirection",
  "directionBreakpoint",
  "padding",
  "outputLimit",
  "linkSchemes",
  "imageSchemes",
  "links",
  "images",
  "linkRel",
];

function nativeOptions(options) {
  if (KEYS.every((key) => options[key] === undefined)) return undefined;
  return {
    limit: options.limit,
    maxEdges: options.maxEdges,
    config:
      options.config === undefined ? undefined : JSON.stringify(options.config),
    fontFamily: options.fontFamily,
    fontFamilyMono: options.fontFamilyMono,
    monospace:
      options.monospace === undefined ? undefined : [...options.monospace],
    fontSize: options.fontSize,
    idPrefix: options.idPrefix,
    responsive: options.responsive,
    maxWidth: options.maxWidth,
    autoDirection: options.autoDirection,
    directionBreakpoint: options.directionBreakpoint,
    padding: options.padding,
    outputLimit: options.outputLimit,
    linkSchemes: options.linkSchemes,
    imageSchemes: options.imageSchemes,
    links: options.links,
    images: options.images,
    linkRel: options.linkRel,
  };
}

const now = (options) =>
  options.now instanceof Date
    ? options.now.getTime()
    : (options.now ?? Date.now());

function measurerFor(options) {
  const measurer =
    options.measurer ??
    defaultMeasurer(options.fontFamily, options.fontFamilyMono);
  return measurer && ((text, size, style) => measurer.width(text, size, style));
}

function settled(outcome) {
  if (outcome.code !== undefined) throw thrown(outcome);
  return outcome;
}

function loaded() {
  if (native === undefined)
    throw new PelersError(
      "pelers is not loaded yet; await load() from pelers/lazy first.",
      "unsupported-diagram",
    );
  return native;
}

export function parse(text, options = {}) {
  const engine = loaded();
  const parsed = engine.parse(text, nativeOptions(options));
  if (parsed instanceof engine.Parsed) return parsed;
  throw thrown(parsed);
}

export function render(source, options = {}) {
  const engine = loaded();
  const at = now(options);
  const given = nativeOptions(options);
  const measurer = measurerFor(options);
  if (typeof source === "string")
    return settled(
      engine.renderSource(
        source,
        at,
        given,
        options.icons,
        options.math,
        measurer,
      ),
    );
  if (source instanceof engine.Parsed)
    return settled(
      engine.renderParsed(
        source,
        at,
        given,
        options.icons,
        options.math,
        measurer,
      ),
    );
  throw new TypeError(
    "render() takes diagram text or a model that parse() returned.",
  );
}

export function detectType(text) {
  const found = loaded().detectType(text);
  if (found !== null && typeof found === "object") throw thrown(found);
  return found;
}

export function supports(text) {
  try {
    const type = detectType(text);
    return type !== null && TYPES.has(type);
  } catch {
    return false;
  }
}

export function mount(element, text, options = {}) {
  return mountWith({ parse, render }, element, text, options);
}

export function register() {}

export function registered(type) {
  return native !== undefined && TYPES.has(type);
}
