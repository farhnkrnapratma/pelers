const BOLD = 1;
const ITALIC = 2;
const MONO = 4;
const SUB = 8;
const SUP = 16;
const SMALLER = 1.2;
function context() {
  const g = globalThis;
  try {
    if (g.OffscreenCanvas) return new g.OffscreenCanvas(1, 1).getContext("2d");
    if (g.document) return g.document.createElement("canvas").getContext("2d");
  } catch {
    return null;
  }
  return null;
}
const MAX_CACHED = 20000;
function canvasFonts() {
  const g = globalThis;
  return g.document?.fonts ?? g.fonts;
}
function hasFont(fonts, font, text) {
  try {
    return fonts?.check(font, text) ?? true;
  } catch {
    return true;
  }
}
function canvasMeasurer(fontFamily, monoFamily = "monospace") {
  const ctx = context();
  if (!ctx) return null;
  const caches = new Map();
  let currentFont = "";
  let faces = canvasFonts()?.size;
  return {
    width(text, size, style) {
      const fonts = canvasFonts();
      if (fonts?.size !== faces) {
        faces = fonts?.size;
        caches.clear();
      }
      const eff = style & (SUB | SUP) ? size / SMALLER : size;
      const font =
        (style & ITALIC ? "italic " : "") +
        (style & BOLD ? "bold " : "") +
        eff +
        "px " +
        (style & MONO ? monoFamily : fontFamily);
      let cache = caches.get(font);
      if (!cache) caches.set(font, (cache = new Map()));
      let w = cache.get(text);
      if (w === undefined) {
        if (font !== currentFont) ctx.font = currentFont = font;
        w = ctx.measureText(text).width;
        if (hasFont(fonts, font, text)) {
          if (cache.size >= MAX_CACHED) cache.clear();
          cache.set(text, w);
        }
      }
      return w;
    },
  };
}
const shared = new Map();
let last;
export function defaultMeasurer(
  fontFamily = "sans-serif",
  monoFamily = "monospace",
) {
  if (
    last !== undefined &&
    last.fontFamily === fontFamily &&
    last.monoFamily === monoFamily
  )
    return last.measurer;
  const key =
    fontFamily +
    `
` +
    monoFamily;
  let m = shared.get(key);
  if (m === undefined) {
    m = canvasMeasurer(fontFamily, monoFamily);
    shared.set(key, m);
  }
  last = { fontFamily, monoFamily, measurer: m };
  return m;
}
export function forgetTextWidths() {
  shared.clear();
  last = undefined;
}
