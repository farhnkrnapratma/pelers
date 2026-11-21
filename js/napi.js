import * as engine from "../index.native.js";

const utf8 = new TextDecoder();

function result(out) {
  if (typeof out !== "string") {
    if (!(out instanceof Uint8Array)) return out;
    out = utf8.decode(out);
  }
  const a = out.indexOf(" ");
  const b = out.indexOf(" ", a + 1);
  const c = out.indexOf(" ", b + 1);
  const line = out.indexOf("\n", c + 1);
  return {
    svg: out.slice(line + 1),
    width: Number(out.slice(0, a)),
    height: Number(out.slice(a + 1, b)),
    links: [],
    type: out.slice(b + 1, c),
    mermaidType: out.slice(c + 1, line),
  };
}

export const napi = {
  ...engine,
  renderSource: (source, now, options, icons, math, measurer) =>
    result(engine.renderSource(source, now, options, icons, math, measurer)),
  renderParsed: (model, now, options, icons, math, measurer) =>
    result(engine.renderParsed(model, now, options, icons, math, measurer)),
};
