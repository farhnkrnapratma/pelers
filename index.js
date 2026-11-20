import {
  Parsed,
  detectType as nativeDetectType,
  parse as nativeParse,
  renderParsed,
  renderSource,
} from "./index.native.js";

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

const parseOptions = (options) => ({
  limit: options.limit,
  maxEdges: options.maxEdges,
  config: options.config === undefined ? undefined : JSON.stringify(options.config),
});

const renderOptions = (options) => ({
  fontFamily: options.fontFamily,
  fontFamilyMono: options.fontFamilyMono,
  monospace: options.monospace === undefined ? undefined : [...options.monospace],
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
  now: options.now instanceof Date ? options.now.getTime() : options.now,
});

const offsets = (ms) => -new Date(ms).getTimezoneOffset() * 60000;

const measure = (measurer) =>
  measurer && ((text, size, style) => measurer.width(text, size, style));

function settled(outcome) {
  if (outcome.error) throw thrown(outcome.error);
  const { title, ...result } = outcome.result;
  return title === undefined || title === null ? result : { ...result, title };
}

export function parse(text, options = {}) {
  const parsed = nativeParse(text, parseOptions(options), offsets);
  if (parsed instanceof Parsed) return parsed;
  throw thrown(parsed);
}

export function render(source, options = {}) {
  const callbacks = [options.icons, options.math, measure(options.measurer), offsets];
  if (typeof source === "string")
    return settled(
      renderSource(source, parseOptions(options), renderOptions(options), ...callbacks),
    );
  if (source instanceof Parsed)
    return settled(renderParsed(source, renderOptions(options), ...callbacks));
  throw new TypeError("render() takes diagram text or a model that parse() returned.");
}

export function detectType(text) {
  const found = nativeDetectType(text);
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

export function register() {}

export function registered(type) {
  return TYPES.has(type);
}

export function forgetTextWidths() {}
