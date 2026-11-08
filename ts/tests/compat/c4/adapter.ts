import { C4Db } from '../../../src/diagrams/c4/db.js';
import { parseC4 } from '../../../src/diagrams/c4/parser.js';
import type { C4Boundary, C4Element, C4Named, C4Rel, C4Shape } from '../../../src/diagrams/c4/types.js';

export function setConfig(_config: unknown): void {}

let db = new C4Db();
let views = new WeakMap<object, unknown>();

type View = Record<string, unknown>;

// Mermaid wraps text fields as { text } and keeps a named argument found in the alias or
// label slot as a one-key object there. Pele stores plain strings, with that argument beside them.
const text = (value: string | undefined): { text: string } | undefined => (value === undefined ? undefined : { text: value });
const pair = (attr: C4Named): View => ({ [attr.key]: attr.value });
const slot = (value: string, attr: C4Named | undefined): string | View => (attr ? pair(attr) : value);

function cached<S extends object>(source: S, build: (source: S) => View): View {
  let view = views.get(source) as View | undefined;
  if (view === undefined) views.set(source, (view = build(source)));
  return view;
}

function elementView(el: C4Element): View {
  return {
    alias: slot(el.alias, el.aliasAttr),
    label: { text: slot(el.label, el.labelAttr) },
    type: text(el.type),
    descr: text(el.descr),
    techn: text(el.techn),
    sprite: el.sprite,
    tags: el.tags,
    link: el.link,
    bgColor: el.bgColor,
    fontColor: el.fontColor,
    borderColor: el.borderColor,
    shadowing: el.shadowing,
    shape: el.shape,
    legendText: el.legendText,
    legendSprite: el.legendSprite,
    parentBoundary: el.parent ? el.parent.alias : '',
    // Mermaid's wrap flag is false at parse time and set from config when drawing.
    wrap: el.parent ? false : undefined,
  };
}

const shapeView = (shape: C4Shape): View => cached(shape, (s) => ({ ...elementView(s), typeC4Shape: { text: s.kind } }));
const boundaryView = (boundary: C4Boundary): View => cached(boundary, (b) => ({ ...elementView(b), nodeType: b.nodeType }));
const relView = (rel: C4Rel): View =>
  cached(rel, (r) => ({
    type: r.type,
    from: slot(r.from, r.fromAttr),
    to: slot(r.to, r.toAttr),
    label: { text: slot(r.label, r.labelAttr) },
    techn: text(r.techn),
    descr: text(r.descr),
    sprite: r.sprite,
    tags: r.tags,
    link: r.link,
    textColor: r.textColor,
    lineColor: r.lineColor,
    offsetX: r.offsetX,
    offsetY: r.offsetY,
    wrap: false,
  }));

const inside = <E extends C4Element>(list: E[], parent: string | null | undefined): E[] =>
  parent === undefined || parent === null ? list : list.filter((el) => (el.parent ? el.parent.alias : '') === parent);

// Exposes Pele's C4 model through the names Mermaid's specs call on c4Db.
export const c4Db = {
  clear(): void {
    db = new C4Db();
    views = new WeakMap();
  },
  getC4ShapeArray: (parent?: string | null) => inside(db.shapes, parent).map(shapeView),
  getBoundaries: (parent?: string | null) => inside(db.boundaries, parent).map(boundaryView),
  getRels: () => db.rels.map(relView),
  getC4Shape(alias: string) {
    const el = db.getC4Shape(alias);
    return el === undefined ? undefined : 'kind' in el ? shapeView(el) : boundaryView(el);
  },
  getTitle: () => db.title ?? '',
  getC4Type: () => db.c4Type,
  getC4ShapeInRow: () => db.shapeInRow,
  getC4BoundaryInRow: () => db.boundaryInRow,
  getAccDescription: () => db.accDescr ?? '',
};

const parser = {
  yy: c4Db,
  parse(src: string): boolean {
    parseC4(src, db);
    return true;
  },
};

export default { parser };
