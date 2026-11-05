import { describe, expect, it } from 'vitest';
import { PeleError } from '../../src/errors.js';
import { SankeyDb } from '../../src/diagrams/sankey/db.js';
import { layoutSankey, type Alignment, type LayoutOptions } from '../../src/diagrams/sankey/layout.js';

function build(rows: [string, string, number][]): SankeyDb {
  const db = new SankeyDb();
  for (const [source, target, value] of rows)
    db.addLink(db.findOrCreateNode(source), db.findOrCreateNode(target), value);
  return db;
}

const options = (alignment: Alignment = 'justify', extra: Partial<LayoutOptions> = {}): LayoutOptions => ({
  width: 600,
  height: 400,
  nodeWidth: 10,
  nodePadding: 12,
  labelHeight: 0,
  alignment,
  ...extra,
});

function layers(db: SankeyDb, alignment: Alignment): Record<string, number> {
  const { nodes } = layoutSankey(db, options(alignment));
  return Object.fromEntries(db.nodes.map((node, i) => [node.id, nodes[i].layer]));
}

// a -> b -> c -> d, with a late source x -> d and an early sink a -> y.
const SHAPE: [string, string, number][] = [
  ['a', 'b', 4],
  ['b', 'c', 3],
  ['c', 'd', 3],
  ['x', 'd', 2],
  ['a', 'y', 1],
];

describe('sankey layout', () => {
  it('puts each node in the column of its longest distance from a source', () => {
    expect(layers(build(SHAPE), 'left')).toEqual({ a: 0, b: 1, c: 2, d: 3, x: 0, y: 1 });
    expect(
      layers(
        build([
          ['a', 'b', 1],
          ['b', 'c', 1],
          ['a', 'c', 1],
        ]),
        'left'
      )
    ).toEqual({ a: 0, b: 1, c: 2 });
  });

  it('honours the four alignments', () => {
    expect(layers(build(SHAPE), 'justify')).toEqual({ a: 0, b: 1, c: 2, d: 3, x: 0, y: 3 });
    expect(layers(build(SHAPE), 'right')).toEqual({ a: 0, b: 1, c: 2, d: 3, x: 2, y: 3 });
    expect(layers(build(SHAPE), 'center')).toEqual({ a: 0, b: 1, c: 2, d: 3, x: 2, y: 1 });
  });

  it('spreads the columns over the width', () => {
    const { nodes } = layoutSankey(build(SHAPE), options('left'));
    expect(nodes[0].x).toBe(0);
    expect(nodes[3].x).toBe(590);
    expect(nodes[1].x).toBeCloseTo(590 / 3, 6);
  });

  it('gives nodes and bands sizes in proportion to their flow', () => {
    const db = build([
      ['a', 'b', 30],
      ['a', 'c', 10],
      ['b', 'd', 30],
    ]);
    const { nodes, links } = layoutSankey(db, options());
    expect(nodes[0].value).toBe(40);
    // The second column holds two nodes and one gap, and sets the scale.
    expect(nodes[0].height).toBeCloseTo(388, 6);
    expect(nodes[1].height / nodes[2].height).toBeCloseTo(3, 6);
    expect(links[0].thickness / links[1].thickness).toBeCloseTo(3, 6);
    expect(links[0].thickness + links[1].thickness).toBeCloseTo(nodes[0].height, 6);
  });

  it('takes the larger of what flows in and what flows out as a node value', () => {
    const { nodes } = layoutSankey(
      build([
        ['a', 'b', 5],
        ['b', 'c', 2],
      ]),
      options()
    );
    expect(nodes[1].value).toBe(5);
    expect(nodes[1].height).toBeGreaterThan(nodes[2].height);
  });

  it('keeps nodes of a column apart and inside the height', () => {
    const rows: [string, string, number][] = [];
    for (let i = 0; i < 12; i++) rows.push([`s${i % 4}`, `t${i}`, 1 + (i % 5)]);
    const db = build(rows);
    const { nodes } = layoutSankey(db, options());
    const columns = new Map<number, typeof nodes>();
    for (const node of nodes) columns.set(node.layer, [...(columns.get(node.layer) ?? []), node]);
    for (const column of columns.values()) {
      column.sort((a, b) => a.y - b.y);
      expect(column[0].y).toBeGreaterThanOrEqual(-1e-6);
      for (let i = 1; i < column.length; i++) {
        expect(column[i].y - (column[i - 1].y + column[i - 1].height)).toBeGreaterThanOrEqual(12 - 1e-6);
      }
      const last = column[column.length - 1];
      expect(last.y + last.height).toBeLessThanOrEqual(400 + 1e-6);
    }
  });

  it('stacks bands on a node in the order of their other ends, inside the node', () => {
    const db = build([
      ['a', 'x', 1],
      ['a', 'y', 2],
      ['a', 'z', 3],
      ['b', 'z', 1],
    ]);
    const { nodes, links } = layoutSankey(db, options());
    const out = links.filter((link) => link.source === 0).sort((p, q) => p.sourceY - q.sourceY);
    const targets = out.map((link) => nodes[link.target].y);
    expect(targets).toEqual([...targets].sort((p, q) => p - q));
    for (const link of links) {
      const source = nodes[link.source];
      const target = nodes[link.target];
      expect(link.sourceY - link.thickness / 2).toBeGreaterThanOrEqual(source.y - 1e-6);
      expect(link.sourceY + link.thickness / 2).toBeLessThanOrEqual(source.y + source.height + 1e-6);
      expect(link.targetY - link.thickness / 2).toBeGreaterThanOrEqual(target.y - 1e-6);
      expect(link.targetY + link.thickness / 2).toBeLessThanOrEqual(target.y + target.height + 1e-6);
    }
  });

  it('reorders a column so that bands do not cross when they need not', () => {
    // Declared in an order that crosses: a feeds the lower target, b the upper one.
    const db = build([
      ['a', 'low', 1],
      ['b', 'high', 1],
      ['root', 'high', 0.1],
      ['root', 'low', 0.1],
    ]);
    const { nodes, links } = layoutSankey(db, options());
    const [first, second] = [links[0], links[1]];
    expect(Math.sign(nodes[first.source].y - nodes[second.source].y)).toBe(
      Math.sign(nodes[first.target].y - nodes[second.target].y)
    );
  });

  it('leaves room for the labels of thin nodes when the column can afford it', () => {
    const rows: [string, string, number][] = [['big', 't', 1000]];
    for (let i = 0; i < 8; i++) rows.push([`thin${i}`, 't', 1]);
    const tight = layoutSankey(build(rows), options('justify', { nodePadding: 2 }))
      .nodes.slice(2)
      .map((node) => node.y);
    const roomy = layoutSankey(build(rows), options('justify', { nodePadding: 2, labelHeight: 16 })).nodes;
    const ys = [roomy[0], ...roomy.slice(2)].map((node) => node.y + node.height / 2).sort((p, q) => p - q);
    for (let i = 1; i < ys.length; i++) expect(ys[i] - ys[i - 1]).toBeGreaterThanOrEqual(16 - 1e-6);
    expect(tight.length).toBe(8);
    // With more thin nodes than the height can hold labels for, the plain gaps are used.
    const many: [string, string, number][] = Array.from({ length: 100 }, (_, i) => [`n${i}`, 't', 1]);
    expect(
      layoutSankey(build(many), options('justify', { labelHeight: 16 })).nodes.every(
        (node) => node.y >= -1e-6 && node.y <= 400
      )
    ).toBe(true);
  });

  it('rejects circular links, naming a node on the cycle', () => {
    expect(() => layoutSankey(build([['a', 'a', 1]]), options())).toThrow(PeleError);
    expect(() =>
      layoutSankey(
        build([
          ['a', 'b', 1],
          ['b', 'a', 1],
        ]),
        options()
      )
    ).toThrow(/circular links/);
    expect(() =>
      layoutSankey(
        build([
          ['s', 'a', 1],
          ['a', 'b', 1],
          ['b', 'c', 1],
          ['c', 'a', 1],
          ['c', 'tail', 1],
        ]),
        options()
      )
    ).toThrow(/"(a|b|c)" is part of one/);
    try {
      layoutSankey(
        build([
          ['a', 'b', 1],
          ['b', 'a', 1],
        ]),
        options()
      );
    } catch (error) {
      expect((error as PeleError).code).toBe('semantic');
    }
  });

  it('treats values that are not positive numbers as no flow, and stays finite', () => {
    const db = build([
      ['a', 'b', NaN],
      ['a', 'c', -5],
      ['a', 'd', Infinity],
      ['b', 'e', 0],
      ['x', 'y', 1e308],
      ['x', 'z', 1e308],
    ]);
    const { nodes, links } = layoutSankey(db, options());
    for (const node of nodes) for (const v of [node.x, node.y, node.height]) expect(Number.isFinite(v)).toBe(true);
    for (const link of links)
      for (const v of [link.thickness, link.sourceY, link.targetY]) expect(Number.isFinite(v)).toBe(true);
    expect(nodes[0].value).toBe(0);
    expect(nodes[0].height).toBe(0);
    expect(layoutSankey(new SankeyDb(), options()).nodes).toEqual([]);
  });

  it('is deterministic', () => {
    expect(layoutSankey(build(SHAPE), options())).toEqual(layoutSankey(build(SHAPE), options()));
  });
});
