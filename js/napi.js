import * as engine from "../index.native.js";

const utf8 = new TextDecoder();

function result(out) {
  if (typeof out !== "string") {
    if (!(out instanceof Uint8Array)) return out;
    out = utf8.decode(out);
  }
  const line = out.indexOf("\n");
  const [width, height, type, mermaidType] = out.slice(0, line).split(" ");
  return {
    svg: out.slice(line + 1),
    width: Number(width),
    height: Number(height),
    links: [],
    type,
    mermaidType,
  };
}

export const napi = {
  ...engine,
  renderSource: (source, now, options, icons, math, measurer) =>
    result(engine.renderSource(source, now, options, icons, math, measurer)),
  renderParsed: (model, now, options, icons, math, measurer) =>
    result(engine.renderParsed(model, now, options, icons, math, measurer)),
};
