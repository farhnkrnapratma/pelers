import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const SCALE = Number(process.env.PELERS_FUZZ_SCALE ?? 1);
const OUT = process.env.PELERS_FUZZ_OUT ?? (join(root, 'tests/fixtures'));
const { FlowDb } = await import(join(root, 'ts/src/diagrams/flowchart/db.ts'));
const { parseFlowchart } = await import(join(root, 'ts/src/diagrams/flowchart/parser.ts'));
const { buildFlowGraph } = await import(join(root, 'ts/src/diagrams/flowchart/graph.ts'));

const sources = JSON.parse(readFileSync(join(root, 'tests/fixtures/flow-lexer.json'), 'utf8')).map(c => c.source);

const nn = v => (v === undefined ? null : v);
const dump = db => ({
  direction: nn(db.direction),
  nodes: [...db.nodes.values()].map(n => ({
    id: n.id,
    labelType: n.labelType,
    text: nn(n.text),
    type: nn(n.type),
    styles: n.styles,
    classes: n.classes,
    dir: nn(n.dir),
    props: n.props === undefined ? null : n.props,
    link: nn(n.link),
    linkTarget: nn(n.linkTarget),
    icon: nn(n.icon),
    form: nn(n.form),
    pos: nn(n.pos),
    img: nn(n.img),
    constraint: nn(n.constraint),
    assetWidth: nn(n.assetWidth),
    assetHeight: nn(n.assetHeight),
  })),
  edges: db.edges.map(e => ({
    start: e.start,
    end: e.end,
    type: nn(e.type),
    text: e.text,
    labelType: e.labelType,
    classes: e.classes,
    style: nn(e.style),
    stroke: nn(e.stroke),
    length: nn(e.length),
    id: nn(e.id),
    isUserDefinedId: e.isUserDefinedId,
    interpolate: nn(e.interpolate),
    animate: nn(e.animate),
    animation: nn(e.animation),
  })),
  subgraphs: db.subgraphs.map(s => ({
    id: s.id,
    nodes: s.nodes,
    title: s.title,
    classes: s.classes,
    dir: nn(s.dir),
    labelType: s.labelType,
    metadata: s.metadata ?? {},
  })),
  classes: [...db.classes.values()].map(c => ({ id: c.id, styles: c.styles, textStyles: c.textStyles })),
  tooltips: [...db.tooltips.entries()],
  defaultEdgeStyle: nn(db.defaultEdgeStyle),
  defaultInterpolate: nn(db.defaultInterpolate),
  accTitle: nn(db.accTitle),
  accDescr: nn(db.accDescr),
});

const dumpGraph = graph => ({
  nodes: graph.nodes.map(n => ({
    id: n.id,
    label: nn(n.label),
    labelType: n.labelType,
    shape: n.shape,
    isGroup: n.isGroup,
    parentId: nn(n.parentId),
    cssStyles: n.cssStyles,
    cssCompiledStyles: n.cssCompiledStyles,
    cssClasses: n.cssClasses,
    dir: nn(n.dir),
    link: nn(n.link),
    linkTarget: nn(n.linkTarget),
    tooltip: nn(n.tooltip),
    icon: nn(n.icon),
    pos: nn(n.pos),
    img: nn(n.img),
    assetWidth: nn(n.assetWidth),
    assetHeight: nn(n.assetHeight),
    constraint: nn(n.constraint),
    colorIndex: nn(n.colorIndex),
    metadata: nn(n.metadata),
  })),
  edges: graph.edges.map(e => ({
    id: e.id,
    isUserDefinedId: e.isUserDefinedId,
    start: e.start,
    end: e.end,
    type: e.type,
    label: e.label,
    labelType: e.labelType,
    thickness: nn(e.thickness),
    pattern: nn(e.pattern),
    minlen: nn(e.minlen),
    arrowTypeStart: e.arrowTypeStart,
    arrowTypeEnd: e.arrowTypeEnd,
    cssCompiledStyles: e.cssCompiledStyles,
    style: e.style,
    classes: e.classes,
    animate: nn(e.animate),
    animation: nn(e.animation),
    curve: nn(e.curve),
  })),
});

const out = sources.map(source => {
  const db = new FlowDb({ inheritDir: false });
  let error = null;
  try {
    parseFlowchart(source, db);
  } catch (e) {
    error = { message: e.message, code: e.code ?? null };
  }
  const graph = error ? null : dumpGraph(buildFlowGraph(db, undefined));
  return { source, error, db: dump(db), graph };
});
writeFileSync(join(OUT, 'flow-parse.json'), `${JSON.stringify(out)}\n`);
const failed = out.filter(c => c.error).length;
console.log(`flow parse: ${out.length} inputs, ${failed} with errors`);
