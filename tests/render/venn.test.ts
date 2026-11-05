import { describe, expect, it } from 'vitest';
import { PeleError, detectType, parse, render, supports } from '../../src/index.js';
import {
  distanceForOverlap,
  findSpot,
  layoutCircles,
  lensArea,
  regionPath,
  type Circle,
} from '../../src/diagrams/venn/layout.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus } from '../support/corpus.js';
import { assertWellFormed } from '../support/xml.js';

const corpus = loadCorpus('venn', /venn-beta/);
const options = { measurer: metricsMeasurer };

function tryRender(src: string): string | undefined {
  try {
    return render(src, options).svg;
  } catch (error) {
    if (error instanceof PeleError) return undefined;
    throw error;
  }
}

const svgOf = (src: string): string => render('venn-beta\n' + src, options).svg;
const count = (svg: string, needle: string): number => svg.split(needle).length - 1;

function circlesOf(svg: string): Map<string, Circle> {
  const out = new Map<string, Circle>();
  for (const m of svg.matchAll(
    /<circle class="pele-venn-set" data-id="([^"]*)" cx="([-\d.]+)" cy="([-\d.]+)" r="([\d.]+)"/g
  )) {
    out.set(m[1], { id: m[1], x: Number(m[2]), y: Number(m[3]), r: Number(m[4]) });
  }
  return out;
}

// Where the first line of a region is written.
function labelAt(svg: string, id: string): { x: number; y: number; text: string } {
  const m = new RegExp(
    `<g class="pele-venn-region" data-id="${id.replace(/\|/g, '\\|')}"><text[^>]* x="([-\\d.]+)" y="([-\\d.]+)"[^>]*>([^<]*)<`
  ).exec(svg);
  if (!m) throw new Error(`no region ${id}`);
  return { x: Number(m[1]), y: Number(m[2]), text: m[3] };
}

const distance = (a: { x: number; y: number }, b: { x: number; y: number }): number => Math.hypot(a.x - b.x, a.y - b.y);
const area = (c: Circle): number => Math.PI * c.r * c.r;

describe('venn diagram rendering', () => {
  it('renders every corpus input that parses, as well-formed SVG', () => {
    let rendered = 0;
    for (const src of corpus) {
      const svg = tryRender(src);
      if (svg === undefined) continue;
      rendered++;
      const where = JSON.stringify(src).slice(0, 120);
      expect(() => assertWellFormed(svg), where).not.toThrow();
      expect(svg, where).not.toMatch(/<script|<style|<foreignObject|<defs|<marker|\son\w+=|javascript:/i);
      expect(svg, where).not.toContain('NaN');
      expect(svg, where).not.toContain('undefined');
      expect(svg, where).not.toContain('Infinity');
      expect(tryRender(src), where).toBe(svg);
    }
    expect(rendered).toBeGreaterThan(20);
  });

  it('is detected, supported and typed', () => {
    const src = 'venn-beta\n  set A\n  set B\n  union A,B';
    expect(detectType(src)).toBe('venn');
    expect(supports(src)).toBe(true);
    const result = render(src, options);
    expect(result.type).toBe('venn');
    expect(result.svg).toMatch(/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" class="pele pele-venn"/);
    expect(result.svg).toContain(`viewBox="0 0 ${result.width} ${result.height}"`);
    const model = parse(src);
    expect(model.type).toBe('venn');
    if (model.type === 'venn') {
      expect(model.subsets).toEqual([
        { sets: ['A'], size: 10, label: undefined },
        { sets: ['B'], size: 10, label: undefined },
        { sets: ['A', 'B'], size: 2.5, label: undefined },
      ]);
    }
  });

  it('draws one set as one circle with its name in the middle', () => {
    const svg = svgOf('set Only');
    const circles = circlesOf(svg);
    expect(circles.size).toBe(1);
    const c = circles.get('Only')!;
    const label = labelAt(svg, 'Only');
    expect(label.text).toBe('Only');
    expect(label.x).toBeCloseTo(c.x, 0);
    expect(Math.abs(label.y - c.y)).toBeLessThan(8);
    expect(svg).toContain(
      'fill="var(--pele-series-1,#4c78a8)" fill-opacity="0.2" stroke="var(--pele-series-1,#4c78a8)"'
    );
  });

  it('gives each set the next series color', () => {
    const svg = svgOf('set A\nset B\nset C');
    expect(svg).toContain('data-id="A" cx');
    for (const k of [1, 2, 3]) expect(count(svg, `fill="var(--pele-series-${k},`)).toBe(1);
  });

  it('keeps sets that share nothing apart', () => {
    const { A, B } = Object.fromEntries(circlesOf(svgOf('set A\nset B')));
    expect(distance(A, B)).toBeGreaterThan(A.r + B.r);
    expect(A.y).toBeCloseTo(B.y, 1);
  });

  it('sizes two circles by their sets and their overlap by the union', () => {
    for (const [a, b, shared] of [
      [20, 12, 3],
      [10, 10, 2.5],
      [30, 5, 1],
      [8, 40, 6],
    ]) {
      const { A, B } = Object.fromEntries(circlesOf(svgOf(`set A:${a}\nset B:${b}\nunion A,B:${shared}`)));
      expect(area(A) / area(B)).toBeCloseTo(a / b, 1);
      const overlap = lensArea(A.r, B.r, distance(A, B));
      expect(overlap / area(A)).toBeCloseTo(shared / a, 2);
    }
  });

  it('puts one circle inside the other when the overlap is the whole smaller set', () => {
    const { A, B } = Object.fromEntries(circlesOf(svgOf('set A:30\nset B:5\nunion A,B:5')));
    expect(distance(A, B) + B.r).toBeLessThanOrEqual(A.r + 0.02);
    const tooMuch = Object.fromEntries(circlesOf(svgOf('set A:30\nset B:5\nunion A,B:500')));
    expect(distance(tooMuch.A, tooMuch.B) + tooMuch.B.r).toBeLessThanOrEqual(tooMuch.A.r + 0.02);
  });

  it('lets an overlap of nothing only touch', () => {
    const { A, B } = Object.fromEntries(circlesOf(svgOf('set A\nset B\nunion A,B:0')));
    expect(distance(A, B)).toBeCloseTo(A.r + B.r, 0);
  });

  it('writes each name in the part of its circle the other does not cover', () => {
    const svg = svgOf('set A["Alpha"]\nset B["Beta"]\nunion A,B["Both"]');
    const { A, B } = Object.fromEntries(circlesOf(svg));
    const alpha = labelAt(svg, 'A');
    const beta = labelAt(svg, 'B');
    const both = labelAt(svg, 'A|B');
    expect([alpha.text, beta.text, both.text]).toEqual(['Alpha', 'Beta', 'Both']);
    expect(distance(alpha, A)).toBeLessThan(A.r);
    expect(distance(alpha, B)).toBeGreaterThan(B.r);
    expect(distance(beta, B)).toBeLessThan(B.r);
    expect(distance(beta, A)).toBeGreaterThan(A.r);
    expect(distance(both, A)).toBeLessThan(A.r);
    expect(distance(both, B)).toBeLessThan(B.r);
    // The arrangement is symmetric, and so are the labels.
    expect(alpha.y).toBeCloseTo(beta.y, 1);
    expect((alpha.x + beta.x) / 2).toBeCloseTo(both.x, 0);
  });

  it('arranges three sets in a triangle with each pair as far apart as its union asks', () => {
    const svg = svgOf('set A:20\nset B:14\nset C:9\nunion A,B:4\nunion B,C:2\nunion A,C:3');
    const { A, B, C } = Object.fromEntries(circlesOf(svg));
    expect(lensArea(A.r, B.r, distance(A, B)) / area(A)).toBeCloseTo(4 / 20, 2);
    expect(lensArea(B.r, C.r, distance(B, C)) / area(B)).toBeCloseTo(2 / 14, 2);
    expect(lensArea(A.r, C.r, distance(A, C)) / area(A)).toBeCloseTo(3 / 20, 2);
    // The first two sit side by side and the third below them.
    expect(A.y).toBeCloseTo(B.y, 1);
    expect(A.x).toBeLessThan(B.x);
    expect(C.y).toBeGreaterThan(A.y);
    expect(C.x).toBeGreaterThan(A.x);
    expect(C.x).toBeLessThan(B.x);
  });

  it('gives the middle of three sets a region of its own, and each pair one too', () => {
    const svg = svgOf('set X\nset Y\nset Z\nunion X,Y["xy"]\nunion Y,Z["yz"]\nunion X,Z["xz"]\nunion X,Y,Z["all"]');
    const { X, Y, Z } = Object.fromEntries(circlesOf(svg));
    const inside = (p: { x: number; y: number }, c: Circle): boolean => distance(p, c) < c.r;
    const all = labelAt(svg, 'X|Y|Z');
    expect(inside(all, X) && inside(all, Y) && inside(all, Z)).toBe(true);
    const xy = labelAt(svg, 'X|Y');
    expect(inside(xy, X) && inside(xy, Y) && !inside(xy, Z)).toBe(true);
    const yz = labelAt(svg, 'Y|Z');
    expect(inside(yz, Y) && inside(yz, Z) && !inside(yz, X)).toBe(true);
    const x = labelAt(svg, 'X');
    expect(inside(x, X) && !inside(x, Y) && !inside(x, Z)).toBe(true);
  });

  it('overlaps every pair of a three-way union that states no pairs', () => {
    const svg = svgOf('set A\nset B\nset C\nunion A,B,C["Innovation"]');
    const { A, B, C } = Object.fromEntries(circlesOf(svg));
    for (const [p, q] of [
      [A, B],
      [B, C],
      [A, C],
    ])
      expect(distance(p, q)).toBeLessThan(p.r + q.r);
    const middle = labelAt(svg, 'A|B|C');
    for (const c of [A, B, C]) expect(distance(middle, c)).toBeLessThan(c.r);
    expect(middle.text).toBe('Innovation');
  });

  it('places more than three sets so that only the stated pairs overlap', () => {
    const svg = svgOf('set A\nset B\nset C\nset D\nset E\nunion A,B\nunion C,D\nunion D,E');
    const c = Object.fromEntries(circlesOf(svg));
    const overlapping = (p: Circle, q: Circle): boolean => distance(p, q) < p.r + q.r - 0.5;
    expect(overlapping(c.A, c.B)).toBe(true);
    expect(overlapping(c.C, c.D)).toBe(true);
    expect(overlapping(c.D, c.E)).toBe(true);
    for (const [p, q] of [
      ['A', 'C'],
      ['A', 'D'],
      ['A', 'E'],
      ['B', 'C'],
      ['B', 'D'],
      ['B', 'E'],
      ['C', 'E'],
    ]) {
      expect(overlapping(c[p], c[q]), `${p} and ${q}`).toBe(false);
    }
  });

  it('writes texts under the label of their region, smaller', () => {
    const svg = svgOf(
      'set A["Frontend"]\n  text A1["React"]\n  text A2\nset B\nunion A,B["Shared"]\n  text AB1["OpenAPI"]'
    );
    const region = /<g class="pele-venn-region" data-id="A">(.*?)<\/g>/.exec(svg)![1];
    const ys = [...region.matchAll(/ y="([-\d.]+)"/g)].map((m) => Number(m[1]));
    expect(ys).toHaveLength(3);
    expect(ys[0]).toBeLessThan(ys[1]);
    expect(ys[1]).toBeLessThan(ys[2]);
    expect(region).toContain('<text class="pele-venn-text" data-id="A1" font-size="13" fill="var(--_m)"');
    expect(region).toContain('>React</text>');
    expect(region).toContain('>A2</text>');
    expect(/<g class="pele-venn-region" data-id="A\|B">(.*?)<\/g>/.exec(svg)![1]).toContain('>OpenAPI</text>');
  });

  it('leaves out a text whose sets were never stated together', () => {
    const svg = svgOf('set A\nset B\ntext A,B orphan\ntext A kept');
    expect(svg).not.toContain('orphan');
    expect(svg).toContain('>kept</text>');
  });

  it('wraps a label that is wider than its region and keeps it inside the picture', () => {
    const result = render(
      'venn-beta\nset A["A very long label for a set that goes on and on"]\nset B\nunion A,B["An intersection with a long label"]',
      options
    );
    const region = /<g class="pele-venn-region" data-id="A">(.*?)<\/g>/.exec(result.svg)![1];
    expect(count(region, '<tspan')).toBeGreaterThan(1);
    for (const m of result.svg.matchAll(/<tspan x="([-\d.]+)" y="([-\d.]+)"[^>]*>([^<]*)</g)) {
      const half = metricsMeasurer.width(m[3], 16, 0) / 2;
      expect(Number(m[1]) - half).toBeGreaterThanOrEqual(0);
      expect(Number(m[1]) + half).toBeLessThanOrEqual(result.width);
    }
  });

  it('applies styles to a set, to an overlap and to a text', () => {
    const svg = svgOf(
      'set A\n  text A1["React"]\nset B\nunion A,B["AB"]\nstyle A fill:#ff6b6b, stroke-width:3\nstyle A,B color:#333\nstyle A1 color:red\nstyle B fill-opacity:0.5\nstyle A stroke:black'
    );
    expect(svg).toMatch(
      /data-id="A" cx="[-\d.]+" cy="[-\d.]+" r="[\d.]+" fill="[^"]+" fill-opacity="0.2" stroke="[^"]+" style="fill:#ff6b6b;stroke-width:3;stroke:black;"\/>/
    );
    expect(svg).toMatch(/data-id="B" [^>]* style="fill-opacity:0.5;"\/>/);
    expect(svg).toContain('<text class="pele-label" font-size="14" style="fill:#333;"');
    expect(svg).toContain('data-id="A1" font-size="13" fill="var(--_m)" style="fill:red;"');
    // A text color on an overlap does not draw the overlap.
    expect(svg).not.toContain('pele-venn-overlap');
  });

  it('picks a contrasting text color on an overlap a style fills', () => {
    const svg = svgOf(
      'set A\nset B\nset C\nunion A,B["ab"]\n  text T["note"]\nunion B,C["bc"]\nstyle A,B fill:#1f3a5f\nstyle A fill:navy\nstyle B,C fill:#1f3a5f,color:#eee'
    );
    expect(svg).toMatch(/<text class="pele-label" font-size="\d+" fill="var\(--pele-fg-on-dark,#fff\)"[^>]*>ab</);
    expect(svg).toMatch(
      /<text class="pele-venn-text" data-id="T" font-size="\d+" fill="var\(--pele-fg-on-dark,#fff\)"[^>]*>note</
    );
    expect(svg).toMatch(/<text class="pele-label" font-size="\d+" style="fill:#eee;"[^>]*>bc</);
    // A set's own fill is drawn faint, so its label keeps the theme's color.
    expect(svg).toMatch(/<text class="pele-label" x=[^>]*>A</);
  });

  it('reads the fill under the text from what is drawn last there', () => {
    const covered = svgOf(
      'set A: 100\nset B: 100\nset C: 100\nunion A,B,C["triple"]: 50\nunion A,B["ab"]: 70\nunion A,C["ac"]: 70\nunion B,C["bc"]: 70\nstyle A,B,C fill:black\nstyle A,B fill:white'
    );
    expect(covered).toMatch(/fill="var\(--pele-fg-on-light,#1f1f1f\)"[^>]*>triple</);
    expect(svgOf('set A\nstyle A fill:black,fill-opacity:1')).toMatch(/fill="var\(--pele-fg-on-dark,#fff\)"[^>]*>A</);
    expect(svgOf('set A: 100\nset B: 100\nunion A,B["x"]: 5\nstyle A,B fill:black')).toMatch(
      /fill="var\(--pele-fg-on-dark,#fff\)"[^>]*>x</
    );
  });

  it('draws an overlap that is given a fill as the shape the circles share', () => {
    const svg = svgOf(
      'set A\nset B\nset C\nunion A,B["ab"]\nunion A,B,C["abc"]\nstyle A,B fill:gold\nstyle A,B,C fill:#333'
    );
    const circles = [...circlesOf(svg).values()];
    const paths = [
      ...svg.matchAll(/<path class="pele-venn-overlap" data-id="([^"]*)" d="([^"]+)" fill="none" style="([^"]*)"\/>/g),
    ];
    expect(paths.map((m) => [m[1], m[3]])).toEqual([
      ['A|B', 'fill:gold;'],
      ['A|B|C', 'fill:#333;'],
    ]);
    // A lens has two corners and two arcs; the middle of three circles has three of each.
    expect(count(paths[0][2], 'A')).toBe(2);
    expect(count(paths[1][2], 'A')).toBe(3);
    // Every corner lies on or inside every circle of its overlap.
    const corners = (d: string): { x: number; y: number }[] =>
      [...d.matchAll(/(?:M| 0 [01] 1 )([-\d.]+),([-\d.]+)/g)].map((m) => ({ x: Number(m[1]), y: Number(m[2]) }));
    for (const p of corners(paths[0][2]))
      for (const c of circles.slice(0, 2)) expect(distance(p, c)).toBeLessThan(c.r + 0.1);
    for (const p of corners(paths[1][2])) for (const c of circles) expect(distance(p, c)).toBeLessThan(c.r + 0.1);
  });

  it('draws the title above, without the quotes it may be written in', () => {
    const quoted = render('venn-beta\ntitle "Team overlap"\nset A', options).svg;
    expect(quoted).toContain('<text class="pele-title" font-weight="var(--_tw)"');
    expect(quoted).toContain('>Team overlap</tspan>');
    const front = render('---\ntitle: From the front matter\n---\nvenn-beta\nset A', options).svg;
    expect(front).toContain('>From the front matter</tspan>');
    const both = render('---\ntitle: Front\n---\nvenn-beta\ntitle Inside\nset A', options).svg;
    expect(both).toContain('>Inside</tspan>');
    expect(both).not.toContain('Front');
  });

  it('keeps the character before a comment', () => {
    const model = parse('venn-beta\nset A%% note\nset B:5%% note\nunion A,B%% note');
    if (model.type !== 'venn') throw new Error('wrong type');
    expect(model.subsets.map((s) => s.sets)).toEqual([['A'], ['B'], ['A', 'B']]);
    expect(model.subsets[1].size).toBe(5);
  });

  it('treats a bare title as no title', () => {
    const model = parse('venn-beta\ntitle\nset A\nset title\ntitle Kept\nset B');
    if (model.type !== 'venn') throw new Error('wrong type');
    expect(model.title).toBe('Kept');
    expect(model.subsets.map((s) => s.sets)).toEqual([['A'], ['title'], ['B']]);
  });

  it('reads a name that starts with a keyword and a hyphen', () => {
    const model = parse(
      'venn-beta\nset set-theory\nset union-x\nunion set-theory,union-x["both"]\n  text-5\ntext set-theory text-books\nstyle set-theory fill:red,x:style-y'
    );
    if (model.type !== 'venn') throw new Error('wrong type');
    expect(model.subsets.map((s) => s.sets)).toEqual([['set-theory'], ['union-x'], ['set-theory', 'union-x']]);
    expect(model.textNodes.map((t) => t.id)).toEqual(['-5', 'text-books']);
    expect(model.styles[0].styles.get('x')).toBe('style-y');
  });

  it('uses the last statement about a set or an overlap', () => {
    const svg = svgOf('set A["first"]:10\nset B:10\nset A["second"]:40');
    const circles = circlesOf(svg);
    expect(circles.size).toBe(2);
    expect(area(circles.get('A')!) / area(circles.get('B')!)).toBeCloseTo(4, 1);
    expect(svg).toContain('>second</text>');
    expect(svg).not.toContain('>first</text>');
  });

  it('draws sets with no size, or a negative one, small rather than not at all', () => {
    const svg = svgOf('set A:0\nset B:-5\nset C:100');
    const { A, B, C } = Object.fromEntries(circlesOf(svg));
    expect(A.r).toBeGreaterThan(20);
    expect(B.r).toBeGreaterThan(20);
    expect(C.r).toBeGreaterThan(A.r);
    expect(svg).not.toContain('NaN');
  });

  it('draws nothing but the frame when there are no sets', () => {
    const result = render('venn-beta\n', options);
    expect(result.svg).toContain('<g class="pele-venn-sets"></g>');
    expect(result.width).toBeGreaterThan(0);
    expect(result.height).toBeGreaterThan(0);
  });

  it('rejects what Mermaid rejects', () => {
    const message = (src: string): string => {
      try {
        render(src, options);
      } catch (error) {
        expect(error).toBeInstanceOf(PeleError);
        return (error as Error).message;
      }
      return 'no error';
    };
    expect(message('venn-beta\nunion A')).toBe('union requires multiple identifiers');
    expect(message('venn-beta\nset A\nunion A,B')).toBe('unknown set identifier: B');
    expect(message('venn-beta\nset A\nunion A,"B#35;x"')).toBe('unknown set identifier: B#35;x');
    expect(message('venn-beta\nset "B#x"\nset A\nunion A,"B#35;x"')).toBe('unknown set identifier: B#35;x');
    expect(message('venn-beta\nset A,B')).toMatch(
      /^Parse error on line 2:\nset A,B\n-----\^\nExpecting 'EOF', 'NEWLINE', 'TITLE', 'SET', 'UNION', 'TEXT', 'INDENT_TEXT', 'STYLE', got 'COMMA'$/
    );
    expect(message('venn-beta\nset A $')).toMatch(/^Lexical error on line 2\. Unrecognized text\./);
  });
});

describe('venn geometry', () => {
  it('finds the distance that gives two circles a wanted overlap', () => {
    for (const [r1, r2, wanted] of [
      [1, 1, 0.5],
      [3, 1, 2],
      [2, 5, 0.01],
      [1, 1, 3.1],
    ]) {
      const d = distanceForOverlap(r1, r2, wanted);
      expect(lensArea(r1, r2, d)).toBeCloseTo(wanted, 6);
    }
    expect(distanceForOverlap(2, 1, 0)).toBe(3);
    expect(distanceForOverlap(2, 1, -1)).toBe(3);
    expect(distanceForOverlap(2, 1, Math.PI)).toBe(1);
    expect(distanceForOverlap(2, 1, 99)).toBe(1);
    expect(distanceForOverlap(2, 1, NaN)).toBe(3);
  });

  it('measures the lens of two circles', () => {
    expect(lensArea(1, 1, 2)).toBe(0);
    expect(lensArea(1, 1, 0)).toBeCloseTo(Math.PI, 9);
    expect(lensArea(3, 1, 1)).toBeCloseTo(Math.PI, 9);
    // Two unit circles a unit apart share 2π/3 − √3/2.
    expect(lensArea(1, 1, 1)).toBeCloseTo((2 * Math.PI) / 3 - Math.sqrt(3) / 2, 9);
  });

  it('finds the roomiest point of a region', () => {
    const circles: Circle[] = [
      { id: 'a', x: 0, y: 0, r: 10 },
      { id: 'b', x: 12, y: 0, r: 10 },
    ];
    const alone = findSpot(circles, [0], []);
    expect([alone.x, alone.y, alone.margin]).toEqual([0, 0, 10]);
    // Of a, outside b: midway between a's far side at -10 and b's near side at 2.
    const outside = findSpot(circles, [0], [1]);
    expect(outside.x).toBeCloseTo(-4, 1);
    expect(outside.y).toBeCloseTo(0, 1);
    expect(outside.margin).toBeCloseTo(6, 1);
    const lens = findSpot(circles, [0, 1], []);
    expect(lens.x).toBeCloseTo(6, 3);
    expect(lens.margin).toBeCloseTo(4, 3);
    expect(findSpot([circles[0], { id: 'c', x: 30, y: 0, r: 10 }], [0, 1], []).margin).toBeLessThan(0);
  });

  it('outlines what circles share', () => {
    const f = (v: number): string => String(Math.round(v * 100) / 100);
    const a = { id: 'a', x: 0, y: 0, r: 5 };
    expect(regionPath([a], f)).toBe('M-5,0A5,5 0 1 1 5,0A5,5 0 1 1 -5,0Z');
    expect(regionPath([a, { id: 'b', x: 6, y: 0, r: 5 }], f)).toBe('M3,-4A5,5 0 0 1 3,4A5,5 0 0 1 3,-4Z');
    expect(regionPath([a, { id: 'b', x: 20, y: 0, r: 5 }], f)).toBe('');
    // A circle inside another is the whole of what they share.
    expect(regionPath([a, { id: 'b', x: 1, y: 0, r: 2 }], f)).toBe('M-1,0A2,2 0 1 1 3,0A2,2 0 1 1 -1,0Z');
  });

  it('lays out the same circles every time', () => {
    const subsets = [
      { sets: ['A'], size: 10, label: undefined },
      { sets: ['B'], size: 20, label: undefined },
      { sets: ['C'], size: 5, label: undefined },
      { sets: ['D'], size: 5, label: undefined },
      { sets: ['A', 'B'], size: 3, label: undefined },
      { sets: ['C', 'D'], size: 1, label: undefined },
    ];
    expect(layoutCircles(subsets)).toEqual(layoutCircles(subsets));
    for (const c of layoutCircles(subsets)) expect(Number.isFinite(c.x) && Number.isFinite(c.y) && c.r > 0).toBe(true);
  });
});
