import type { C4Attr, C4Boundary, C4Element, C4Model, C4Named, C4Rel, C4Shape } from './types.js';

const ELEMENT_FIELDS = new Set([
  'label',
  'type',
  'descr',
  'techn',
  'sprite',
  'tags',
  'link',
  'bgColor',
  'fontColor',
  'borderColor',
  'shadowing',
  'shape',
  'legendText',
  'legendSprite',
]);
const REL_FIELDS = new Set(['label', 'descr', 'techn', 'sprite', 'tags', 'link', 'textColor', 'lineColor']);

type Target = C4Element | C4Rel;
type Fields = Record<string, string | undefined>;

// Mermaid assigns a named argument to whatever property it names, structural ones included.
// Here it reaches only the fields above; any other name is kept aside in `extra`.
function named(target: Target, attr: C4Named): void {
  const { key, value } = attr;
  if ('from' in target && (key === 'offsetX' || key === 'offsetY')) target[key] = parseInt(value);
  else if (('from' in target ? REL_FIELDS : ELEMENT_FIELDS).has(key)) (target as unknown as Fields)[key] = value;
  else (target.extra ??= new Map()).set(key, value);
}

// A positional argument fills the field of its slot. A named one fills the field it names, whichever slot it is in.
function put(target: Target, field: string, attr: C4Attr | undefined): void {
  if (attr === undefined) return;
  if (typeof attr === 'string') (target as unknown as Fields)[field] = attr;
  else named(target, attr);
}

function text(target: Target, field: 'descr' | 'techn', attr: C4Attr | undefined): void {
  if (attr === undefined) target[field] ??= '';
  else put(target, field, attr);
}

function label(target: Target, attr: C4Attr | undefined): void {
  target.labelAttr = undefined;
  if (typeof attr === 'object') {
    target.label = '';
    target.labelAttr = attr;
    named(target, attr);
  } else {
    target.label = attr ?? '';
  }
}

export class C4Db implements C4Model {
  type = 'c4' as const;
  c4Type: string | undefined;
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  shapes: C4Shape[] = [];
  boundaries: C4Boundary[] = [{ alias: 'global', label: 'global', type: 'global', parent: undefined }];
  rels: C4Rel[] = [];
  shapeInRow = 4;
  boundaryInRow = 2;

  private shapeIndex = new Map<string, C4Shape>();
  // The top-level boundary is not listed, so a boundary a diagram names `global` is its own box.
  // In Mermaid it replaces the top level and nothing is drawn.
  private boundaryIndex = new Map<string, C4Boundary>();
  private relIndex = new Map<string, Map<string, C4Rel>>();
  private current: C4Boundary = this.boundaries[0];
  private stack: C4Boundary[] = [];

  setC4Type(type: string): void {
    this.c4Type = type;
  }

  setTitle(title: string): void {
    this.title = title;
  }

  setAccDescription(descr: string): void {
    this.accDescr = descr.replace(/\n\s+/g, '\n');
  }

  private element<E extends C4Element>(list: E[], index: Map<string, E>, alias: C4Attr, title: C4Attr | undefined): E {
    let el = typeof alias === 'string' ? index.get(alias) : undefined;
    if (el === undefined) {
      el = { alias: '', label: '', parent: undefined } as E;
      list.push(el);
      if (typeof alias === 'string') {
        el.alias = alias;
        index.set(alias, el);
      } else {
        el.aliasAttr = alias;
        named(el, alias);
      }
    }
    label(el, title);
    return el;
  }

  // alias, label, ?descr, ?sprite, ?tags, $link
  addPersonOrSystem(kind: string, attrs: C4Attr[]): void {
    const shape = this.element(this.shapes, this.shapeIndex, attrs[0], attrs[1]);
    text(shape, 'descr', attrs[2]);
    this.finishShape(shape, kind, attrs, 3);
  }

  // alias, label, ?techn, ?descr, ?sprite, ?tags, $link
  addContainer(kind: string, attrs: C4Attr[]): void {
    const shape = this.element(this.shapes, this.shapeIndex, attrs[0], attrs[1]);
    text(shape, 'techn', attrs[2]);
    text(shape, 'descr', attrs[3]);
    this.finishShape(shape, kind, attrs, 4);
  }

  addComponent(kind: string, attrs: C4Attr[]): void {
    this.addContainer(kind, attrs);
  }

  private finishShape(shape: C4Shape, kind: string, attrs: C4Attr[], at: number): void {
    put(shape, 'sprite', attrs[at]);
    put(shape, 'tags', attrs[at + 1]);
    put(shape, 'link', attrs[at + 2]);
    shape.kind = kind;
    shape.parent = this.current;
  }

  private boundary(attrs: C4Attr[], fallback: string): C4Boundary {
    const boundary = this.element(this.boundaries, this.boundaryIndex, attrs[0], attrs[1]);
    if (attrs[2] === undefined) boundary.type = fallback;
    else put(boundary, 'type', attrs[2]);
    return boundary;
  }

  private open(boundary: C4Boundary): void {
    boundary.parent = this.current;
    this.stack.push(this.current);
    this.current = boundary;
  }

  // alias, label, ?type, ?tags, $link
  addPersonOrSystemBoundary(attrs: C4Attr[], fallback = 'system'): void {
    const boundary = this.boundary(attrs, fallback);
    put(boundary, 'tags', attrs[3]);
    put(boundary, 'link', attrs[4]);
    this.open(boundary);
  }

  addContainerBoundary(attrs: C4Attr[]): void {
    this.addPersonOrSystemBoundary(attrs, 'container');
  }

  // alias, label, ?type, ?descr, ?sprite, ?tags, $link
  addDeploymentNode(nodeType: string, attrs: C4Attr[]): void {
    const boundary = this.boundary(attrs, 'node');
    text(boundary, 'descr', attrs[3]);
    // Mermaid drops whatever is in the sprite slot, a named argument included.
    put(boundary, 'sprite', attrs[4]);
    put(boundary, 'tags', attrs[5]);
    put(boundary, 'link', attrs[6]);
    boundary.nodeType = nodeType;
    this.open(boundary);
  }

  popBoundaryParseStack(): void {
    this.current = this.stack.pop() ?? this.boundaries[0];
  }

  // from, to, label, ?techn, ?descr, ?sprite, ?tags, $link
  addRel(type: string, attrs: C4Attr[]): void {
    const from = attrs[0];
    const to = attrs[1];
    if (from === undefined || to === undefined || attrs[2] === undefined) return;
    const known = typeof from === 'string' && typeof to === 'string';
    let rel = known ? this.relIndex.get(from)?.get(to) : undefined;
    if (rel === undefined) {
      rel = { type, from: '', to: '', label: '' };
      this.rels.push(rel);
      if (known) {
        let targets = this.relIndex.get(from);
        if (targets === undefined) this.relIndex.set(from, (targets = new Map()));
        targets.set(to, rel);
      }
      if (typeof from === 'string') rel.from = from;
      else rel.fromAttr = from;
      if (typeof to === 'string') rel.to = to;
      else rel.toAttr = to;
    }
    rel.type = type;
    label(rel, attrs[2]);
    text(rel, 'techn', attrs[3]);
    text(rel, 'descr', attrs[4]);
    put(rel, 'sprite', attrs[5]);
    put(rel, 'tags', attrs[6]);
    put(rel, 'link', attrs[7]);
  }

  getC4Shape(alias: string): C4Shape | C4Boundary | undefined {
    return this.shapeIndex.get(alias) ?? this.boundaryIndex.get(alias);
  }

  // elementName, ?bgColor, ?fontColor, ?borderColor, ?shadowing, ?shape, ?sprite, ?techn, ?legendText, ?legendSprite
  updateElStyle(_type: string, attrs: C4Attr[]): void {
    const el = typeof attrs[0] === 'string' ? this.getC4Shape(attrs[0]) : undefined;
    if (el === undefined) return;
    put(el, 'bgColor', attrs[1]);
    put(el, 'fontColor', attrs[2]);
    put(el, 'borderColor', attrs[3]);
    put(el, 'shadowing', attrs[4]);
    put(el, 'shape', attrs[5]);
    put(el, 'sprite', attrs[6]);
    put(el, 'techn', attrs[7]);
    put(el, 'legendText', attrs[8]);
    put(el, 'legendSprite', attrs[9]);
  }

  // from, to, ?textColor, ?lineColor, ?offsetX, ?offsetY
  updateRelStyle(_type: string, attrs: C4Attr[]): void {
    const from = attrs[0];
    const to = attrs[1];
    const rel = typeof from === 'string' && typeof to === 'string' ? this.relIndex.get(from)?.get(to) : undefined;
    if (rel === undefined) return;
    put(rel, 'textColor', attrs[2]);
    put(rel, 'lineColor', attrs[3]);
    const x = attrs[4];
    const y = attrs[5];
    if (typeof x === 'string') rel.offsetX = parseInt(x);
    else if (x !== undefined) named(rel, x);
    if (typeof y === 'string') rel.offsetY = parseInt(y);
    else if (y !== undefined) named(rel, y);
  }

  // ?c4ShapeInRow, ?c4BoundaryInRow. As in Mermaid, the slot decides which one a value sets, not its name.
  updateLayoutConfig(_type: string, attrs: C4Attr[]): void {
    const count = (attr: C4Attr | undefined): number =>
      attr === undefined ? NaN : parseInt(typeof attr === 'string' ? attr : attr.value);
    const shapes = count(attrs[0]);
    const boundaries = count(attrs[1]);
    if (shapes >= 1) this.shapeInRow = shapes;
    if (boundaries >= 1) this.boundaryInRow = boundaries;
  }
}
