import { describe, expect, it } from 'vitest';
import { PeleError, detectType, parse, render, supports } from '../../src/index.js';
import { parseGenericTypes as genericTypes } from '../../src/diagrams/common/generics.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus } from '../support/corpus.js';
import { assertWellFormed, elements } from '../support/xml.js';

const corpus = loadCorpus('er', /erDiagram/);
const options = { measurer: metricsMeasurer };

function tryRender(src: string): string | undefined {
  try {
    return render(src, options).svg;
  } catch (error) {
    if (error instanceof PeleError) return undefined;
    throw error;
  }
}

const svgOf = (src: string): string => render(src, options).svg;
const count = (svg: string, needle: string): number => svg.split(needle).length - 1;

// The first entity group with the given name: its translate offset and the size of its box.
function box(svg: string, name: string): { x: number; y: number; w: number; h: number } {
  const m = new RegExp(
    `data-id="${name}" transform="translate\\(([-\\d.]+),([-\\d.]+)\\)"><rect x="[-\\d.]+" y="[-\\d.]+" width="([\\d.]+)" height="([\\d.]+)"`
  ).exec(svg);
  if (!m) throw new Error(`no entity ${name}`);
  return { x: Number(m[1]), y: Number(m[2]), w: Number(m[3]), h: Number(m[4]) };
}

function edgePaths(svg: string): string[] {
  return [...svg.matchAll(/<g class="pele-edge [^>]*><path d="([^"]+)"/g)].map((m) => m[1]);
}

function ends(d: string): [number, number, number, number] {
  const nums = d.match(/-?\d+(?:\.\d+)?/g)!.map(Number);
  return [nums[0], nums[1], nums[nums.length - 2], nums[nums.length - 1]];
}

describe('ER diagram rendering', () => {
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
      expect(tryRender(src), where).toBe(svg);
    }
    expect(rendered).toBeGreaterThan(90);
  });

  it('is detected, supported and typed', () => {
    const src = 'erDiagram\n  A ||--o{ B : has';
    expect(detectType(src)).toBe('er');
    expect(supports(src)).toBe(true);
    const result = render(src, options);
    expect(result.type).toBe('er');
    expect(result.svg).toMatch(/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" class="pele pele-er"/);
    expect(result.svg).toContain(`viewBox="0 0 ${result.width} ${result.height}"`);
    const model = parse(src);
    expect(model.type).toBe('er');
    if (model.type === 'er') expect([...model.entities.keys()]).toEqual(['A', 'B']);
  });

  it('lays out along the declared direction', () => {
    const tall = render('erDiagram\n  A ||--o{ B : x\n  B ||--o{ C : y', options);
    const wide = render('erDiagram\n  direction LR\n  A ||--o{ B : x\n  B ||--o{ C : y', options);
    expect(tall.height).toBeGreaterThan(tall.width);
    expect(wide.width).toBeGreaterThan(wide.height);
    const up = svgOf('erDiagram\n  direction BT\n  A ||--o{ B : x');
    expect(box(up, 'A').y).toBeGreaterThan(box(up, 'B').y);
    const back = svgOf('erDiagram\n  direction RL\n  A ||--o{ B : x');
    expect(box(back, 'A').x).toBeGreaterThan(box(back, 'B').x);
  });

  it('draws an entity without attributes as a plain box', () => {
    const svg = svgOf('erDiagram\n  ISLAND');
    expect(count(svg, '<rect')).toBe(1);
    expect(svg).toContain('<g class="pele-node pele-entity" data-id="ISLAND"');
    expect(svg).toContain('>ISLAND</text>');
    expect(svg).not.toContain('pele-er-attributes');
  });

  it('draws the attribute table with aligned columns', () => {
    const svg = svgOf(
      'erDiagram\n  CAR {\n    string registrationNumber PK "The plate"\n    int wheels\n    string(99) make FK, UK\n    float? price "In euros"\n  }'
    );
    const column = (cls: string): string[] =>
      [...svg.matchAll(new RegExp(`<text class="${cls}"[^>]* x="([-\\d.]+)"`, 'g'))].map((m) => m[1]);
    for (const cls of ['pele-er-type', 'pele-er-name', 'pele-er-keys', 'pele-er-comment']) {
      const xs = column(cls);
      expect(xs.length, cls).toBeGreaterThan(1);
      expect(new Set(xs).size, cls).toBe(1);
    }
    expect(column('pele-er-type')).toHaveLength(4);
    expect(Number(column('pele-er-name')[0])).toBeGreaterThan(Number(column('pele-er-type')[0]));
    expect(Number(column('pele-er-keys')[0])).toBeGreaterThan(Number(column('pele-er-name')[0]));
    expect(Number(column('pele-er-comment')[0])).toBeGreaterThan(Number(column('pele-er-keys')[0]));
    for (const text of [
      '>string</text>',
      '>registrationNumber</text>',
      '>PK</text>',
      '>FK, UK</text>',
      '>The plate</text>',
      '>float?</text>',
      '>string(99)</text>',
    ]) {
      expect(svg).toContain(text);
    }
  });

  it('measures attribute types, names and keys in the monospace font when monospace lists attributes', () => {
    const src =
      'erDiagram\n  CAR {\n    string registrationNumber PK "The plate number, as issued by the registration office of the country"\n    int wheels FK, UK\n  }';
    const plain = svgOf(src);
    const mono = render(src, { ...options, monospace: ['attributes'] }).svg;
    const comment = (svg: string): string => /<text class="pele-er-comment".*?<\/text>/.exec(svg)![0];
    for (const cls of ['pele-er-type', 'pele-er-name', 'pele-er-keys']) {
      expect(mono).toMatch(new RegExp(`<text class="${cls}"[^>]* font-family="var\\(--_fm\\)"`));
    }
    expect(plain).not.toContain('font-family="var(--_fm)"');
    expect(comment(mono)).not.toContain('font-family');
    expect(comment(mono).replace(/ x="[-\d.]+"/g, '')).toBe(comment(plain).replace(/ x="[-\d.]+"/g, ''));
    expect(count(comment(mono), '<tspan')).toBe(2);
    expect(box(mono, 'CAR').w).toBeGreaterThan(box(plain, 'CAR').w);
    for (const name of ['`a  b`', '`<code>a  b</code>`']) {
      const spaced = render(`erDiagram\n  A {\n    string ${name}\n  }`, { ...options, monospace: ['attributes'] }).svg;
      expect(spaced).toMatch(
        /<text class="pele-er-name" font-family="var\(--_fm\)" xml:space="preserve"><tspan[^>]*>a {2}b<\/tspan>/
      );
    }
  });

  it('ignores monospace values that do not list attributes', () => {
    const src = 'erDiagram\n  CAR {\n    string registrationNumber PK "The plate"\n  }';
    const plain = svgOf(src);
    for (const monospace of [[], ['attribute'], ['members'], 'attributes', { attributes: true }, true, null]) {
      expect(render(src, { ...options, monospace } as object).svg).toBe(plain);
    }
  });

  it("keeps an entity's own font-family on its attributes when monospace lists them", () => {
    for (const rule of [
      'style A color:black,font-family:serif',
      'classDef serif color:black,font-family:serif\n  class A serif',
    ]) {
      const src = `erDiagram\n  A {\n    string name PK\n  }\n  B {\n    string name PK\n  }\n  ${rule}`;
      const attributes = (svg: string): string => /data-id="A".*?(<g class="pele-er-attributes".*?<\/g>)/.exec(svg)![1];
      const svg = render(src, { ...options, monospace: ['attributes'] }).svg;
      expect(attributes(svg)).toBe(attributes(svgOf(src)));
      expect(/data-id="B".*$/.exec(svg)![0]).toContain('font-family="var(--_fm)"');
    }
  });

  it('shades alternate rows and rounds the last one with the box', () => {
    const three = svgOf('erDiagram\n  A {\n int a\n int b\n int c\n }');
    const stripes = /<g class="pele-er-stripes" fill="var\(--_bg\)">(.*?)<\/g>/.exec(three)![1];
    // Rows one and three; the third is the last, so it is drawn as a rounded part and a square part.
    expect(count(stripes, '<rect')).toBe(3);
    expect(count(stripes, 'rx:var(--_r)')).toBe(1);
    const two = svgOf('erDiagram\n  A {\n int a\n int b\n }');
    expect(count(/<g class="pele-er-stripes"[^>]*>(.*?)<\/g>/.exec(two)![1], '<rect')).toBe(1);
  });

  it('keeps every row inside the box and every row the same height', () => {
    const svg = svgOf('erDiagram\n  A {\n int a\n int b\n int c\n int d\n }');
    const { h } = box(svg, 'A');
    const ys = [...svg.matchAll(/<text class="pele-er-type"[^>]* y="([-\d.]+)"/g)].map((m) => Number(m[1]));
    expect(ys).toHaveLength(4);
    const gaps = ys.slice(1).map((y, i) => y - ys[i]);
    expect(new Set(gaps.map((g) => g.toFixed(2))).size).toBe(1);
    expect(ys[0]).toBeGreaterThan(-h / 2);
    expect(ys[3]).toBeLessThan(h / 2);
  });

  it('shows the alias in place of the name', () => {
    const svg = svgOf('erDiagram\n  p[Person] {\n string name\n }\n  a["Customer Account"]\n  p ||--o| a : has');
    expect(svg).toContain('>Person</text>');
    expect(svg).toContain('>Customer Account</text>');
    expect(svg).not.toContain('>p</text>');
    expect(svg).toContain('data-id="p"');
  });

  it('draws a marker for each cardinality at the right end', () => {
    const shapes = (src: string): string[] => {
      const svg = svgOf(src);
      const group = /<g class="pele-edge [^>]*>(.*?)<\/g>/.exec(svg)![1];
      return [...group.matchAll(/<(path|circle) class="pele-marker"/g)].map((m) => m[1]);
    };
    expect(shapes('erDiagram\n A ||--|| B : x')).toEqual(['path', 'path']);
    expect(shapes('erDiagram\n A |o--o| B : x')).toEqual(['path', 'circle', 'path', 'circle']);
    expect(shapes('erDiagram\n A }o--o{ B : x')).toEqual(['path', 'circle', 'path', 'circle']);
    expect(shapes('erDiagram\n A }|--|{ B : x')).toEqual(['path', 'path']);
    expect(shapes('erDiagram\n A ||--o{ B : x')).toEqual(['path', 'path', 'circle']);
    expect(shapes('erDiagram\n A u--|| B : x')).toEqual(['path', 'path']);
    // The ring of "zero or more" belongs to B, the far end of the line from A.
    const svg = svgOf('erDiagram\n A ||--o{ B : x');
    const ring = /<circle class="pele-marker" cx="([-\d.]+)" cy="([-\d.]+)"/.exec(svg)!;
    const a = box(svg, 'A');
    const b = box(svg, 'B');
    expect(Math.abs(Number(ring[2]) - b.y)).toBeLessThan(Math.abs(Number(ring[2]) - a.y));
  });

  it('gives the word aliases the same markers as the symbols', () => {
    const markers = (src: string): string =>
      svgOf(src)
        .match(/<g class="pele-edges".*?<\/g><\/g>/)![0]
        .replace(/data-id="[^"]*"/g, '');
    expect(markers('erDiagram\n A one or zero to one or more B : x')).toBe(markers('erDiagram\n A |o--|{ B : x'));
    expect(markers('erDiagram\n A only one to zero or many B : x')).toBe(markers('erDiagram\n A ||--o{ B : x'));
    expect(markers('erDiagram\n A 1 optionally to many(1) B : x')).toBe(markers('erDiagram\n A ||..|{ B : x'));
    expect(markers('erDiagram\n A zero or one to 0+ B : x')).toBe(markers('erDiagram\n A |o--o{ B : x'));
    expect(markers('erDiagram\n A 1+ to many(0) B : x')).toBe(markers('erDiagram\n A }|--o{ B : x'));
  });

  it('draws identifying lines solid and non-identifying lines dashed', () => {
    const solid = svgOf('erDiagram\n A ||--|| B : x');
    const dashed = svgOf('erDiagram\n A ||..|| B : x');
    expect(solid).toContain('class="pele-edge pele-er-identifying"');
    expect(solid).not.toContain('stroke-dasharray');
    expect(dashed).toContain('class="pele-edge pele-er-non-identifying"');
    expect(count(dashed, 'stroke-dasharray')).toBe(1);
  });

  it('runs each line all the way to both boxes', () => {
    for (const dir of ['TB', 'BT', 'LR', 'RL']) {
      const svg = svgOf(`erDiagram\n direction ${dir}\n A ||--o{ B : x`);
      const [sx, sy, ex, ey] = ends(edgePaths(svg)[0]);
      const a = box(svg, 'A');
      const b = box(svg, 'B');
      const onBorder = (x: number, y: number, e: typeof a): boolean =>
        (Math.abs(Math.abs(x - e.x) - e.w / 2) < 0.02 && Math.abs(y - e.y) <= e.h / 2) ||
        (Math.abs(Math.abs(y - e.y) - e.h / 2) < 0.02 && Math.abs(x - e.x) <= e.w / 2);
      expect(onBorder(sx, sy, a), `${dir} start`).toBe(true);
      expect(onBorder(ex, ey, b), `${dir} end`).toBe(true);
    }
  });

  it('spaces out the lines that meet one side of an entity', () => {
    const svg = svgOf('erDiagram\n A ||--o{ B : x\n A ||--o{ C : y\n A ||--o{ D : z');
    const starts = edgePaths(svg).map((d) => ends(d)[0]);
    expect(new Set(starts).size).toBe(3);
    const sorted = [...starts].sort((p, q) => p - q);
    expect(sorted[1] - sorted[0]).toBeGreaterThanOrEqual(12);
    const a = box(svg, 'A');
    for (const x of starts) expect(Math.abs(x - a.x)).toBeLessThan(a.w / 2);
  });

  it('labels relationships and leaves an empty label out', () => {
    const svg = svgOf('erDiagram\n A ||--o{ B : "places an order"\n B ||--o{ C : ""');
    expect(count(svg, '<g class="pele-edge-label"')).toBe(1);
    expect(svg).toContain('>places an order</text>');
    expect(count(svg, '<g class="pele-edge ')).toBe(2);
  });

  it('draws a relationship from an entity to itself as a loop with both markers', () => {
    const svg = svgOf('erDiagram\n EMPLOYEE ||--o{ EMPLOYEE : manages');
    const m = /^M([-\d.]+),([-\d.]+)H.*,([-\d.]+)H([-\d.]+)$/.exec(edgePaths(svg)[0])!;
    const e = box(svg, 'EMPLOYEE');
    expect(Number(m[1])).toBeCloseTo(e.x + e.w / 2, 1);
    expect(Number(m[4])).toBeCloseTo(e.x + e.w / 2, 1);
    expect(Number(m[2])).toBeLessThan(e.y);
    expect(Number(m[3])).toBeGreaterThan(e.y);
    expect(Number(m[3])).toBeLessThan(e.y + e.h / 2);
    expect(count(svg, 'class="pele-marker"')).toBe(3);
    expect(svg).toContain('>manages</text>');
  });

  it('loops out of the bottom when ranks run across, and stacks several loops', () => {
    const svg = svgOf('erDiagram\n direction LR\n A ||--o{ A : self\n A ||--o{ B : x');
    const loop = edgePaths(svg).find((d) => d.includes('V'))!;
    const m = /^M([-\d.]+),([-\d.]+)V.*V([-\d.]+)$/.exec(loop)!;
    const a = box(svg, 'A');
    expect(Number(m[2])).toBeCloseTo(a.y + a.h / 2, 1);
    expect(Number(m[3])).toBeCloseTo(a.y + a.h / 2, 1);
    expect(Math.abs(Number(m[1]) - a.x)).toBeLessThan(a.w / 2);

    const two = svgOf('erDiagram\n A ||--o{ A : first\n A |o--|| A : second');
    const ys = edgePaths(two).map((d) => Number(/^M[-\d.]+,([-\d.]+)H/.exec(d)![1]));
    const e = box(two, 'A');
    expect(ys).toHaveLength(2);
    expect(ys[1] - ys[0]).toBeGreaterThanOrEqual(30);
    for (const y of ys) expect(Math.abs(y - e.y)).toBeLessThan(e.h / 2);
    expect(two).toContain('>first</text>');
    expect(two).toContain('>second</text>');
  });

  it('applies style, classDef, class and the ::: shorthand', () => {
    const svg = svgOf(
      'erDiagram\n A:::hot { int id }\n B\n C\n class B hot\n classDef hot fill:#f96,stroke:#333\n style C fill:#bbf,color:#fff\n classDef default stroke-width:2px'
    );
    expect(svg).toContain('<g class="pele-node pele-entity hot" data-id="A"');
    expect(svg).toContain('<g class="pele-node pele-entity hot" data-id="B"');
    expect(svg).toContain('style="rx:var(--_r);stroke-width:2px;fill:#f96;stroke:#333;"');
    expect(svg).toMatch(/data-id="C"[^>]*><rect[^>]* style="rx:var\(--_r\);stroke-width:2px;fill:#bbf;"/);
    expect(svg).toMatch(/<text class="pele-label" style="fill:#fff;"[^>]*>C<\/text>/);
    // Shading over a chosen fill is a tint of it rather than the page background.
    expect(svg).toContain('<g class="pele-er-stripes" fill="var(--pele-fg-on-dark,#fff)" opacity="0.6">');
  });

  it('picks a contrasting text color on fills a style sets', () => {
    const svg = svgOf(
      'erDiagram\n A { int id PK "key" }\n B { int id }\n C { int id }\n D { int id }\n subgraph G\n  E\n end\n style A fill:navy\n style B fill:#BBDEFB\n style C fill:#BBDEFB,color:#333\n style G fill:#FFF9C4'
    );
    const entity = (id: string): string => new RegExp(`data-id="${id}"[^>]*>(.*?)</g></g>`).exec(svg)![1];
    expect(entity('A')).toContain('<g class="pele-er-stripes" fill="var(--pele-fg-on-light,#1f1f1f)" opacity="0.6">');
    for (const cell of ['pele-label', 'pele-er-type', 'pele-er-name', 'pele-er-keys', 'pele-er-comment']) {
      expect(entity('A')).toMatch(new RegExp(`<text class="${cell}"[^>]* fill="var\\(--pele-fg-on-dark,#fff\\)"`));
    }
    expect(entity('B')).toContain('<text class="pele-label" fill="var(--pele-fg-on-light,#1f1f1f)"');
    expect(entity('B')).toContain('<g class="pele-er-stripes" fill="var(--pele-fg-on-dark,#fff)" opacity="0.6">');
    expect(entity('C')).not.toContain('--pele-fg-on-light');
    expect(entity('C')).toContain('<text class="pele-er-type" fill="var(--_m)" style="fill:#333;"');
    expect(entity('D')).toContain('<g class="pele-er-stripes" fill="var(--_bg)">');
    expect(entity('D')).not.toContain('--pele-fg-on');
    expect(svg).toMatch(/data-id="G">.*?<text class="pele-cluster-label" fill="var\(--pele-fg-on-light,#1f1f1f\)"/);
  });

  it('formats markdown and keeps unicode in names, attributes and labels', () => {
    const svg = svgOf(
      'erDiagram\n "This **is** _Markdown_" ||--o{ "Ünïcödé ❤ 日本語" : "**bold** label"\n T {\n string x "a *b* c"\n }'
    );
    expect(svg).toContain('<tspan font-weight="var(--_w)">is</tspan>');
    expect(svg).toContain('<tspan font-style="italic">Markdown</tspan>');
    expect(svg).toContain('Ünïcödé ❤ 日本語');
    expect(svg).toMatch(/<tspan[^>]* font-weight="var\(--_w\)">bold<\/tspan><tspan> label<\/tspan>/);
    expect(svg).toContain('<tspan font-style="italic">b</tspan>');
  });

  it('reads tildes in attribute text as generic brackets', () => {
    expect(genericTypes('List~int~')).toBe('List<int>');
    expect(genericTypes('Map~K,V~')).toBe('Map<K,V>');
    expect(genericTypes('Array~Array~string~~')).toBe('Array<Array<string>>');
    expect(genericTypes('a~b')).toBe('a~b');
    expect(genericTypes('plain')).toBe('plain');
    const svg = svgOf('erDiagram\n A {\n List~string~ tags\n }');
    expect(svg).toContain('>List&lt;string&gt;</text>');
  });

  it('draws subgraphs as clusters, nested, with relationships to them', () => {
    const svg = svgOf(
      'erDiagram\n subgraph outer [Outer title]\n  A\n  subgraph inner\n   B { int id }\n  end\n end\n subgraph "Empty One"\n end\n outer ||--o{ C : links\n C ||--|| inner : x'
    );
    expect(count(svg, '<g class="pele-cluster"')).toBe(3);
    expect(svg).toContain('data-id="outer"');
    expect(svg).toContain('>Outer title</text>');
    expect(svg).toContain('>inner</text>');
    expect(svg).toContain('>Empty One</text>');
    expect(count(svg, '<g class="pele-edge ')).toBe(2);
    const rects = [
      ...svg.matchAll(
        /<g class="pele-cluster" data-id="(\w+)"><rect x="([-\d.]+)" y="([-\d.]+)" width="([\d.]+)" height="([\d.]+)"/g
      ),
    ];
    const outer = rects
      .find((m) => m[1] === 'outer')!
      .slice(2)
      .map(Number);
    const inner = rects
      .find((m) => m[1] === 'inner')!
      .slice(2)
      .map(Number);
    expect(inner[0]).toBeGreaterThan(outer[0]);
    expect(inner[1]).toBeGreaterThan(outer[1]);
    expect(inner[0] + inner[2]).toBeLessThan(outer[0] + outer[2]);
    expect(inner[1] + inner[3]).toBeLessThan(outer[1] + outer[3]);
    const b = box(svg, 'B');
    expect(b.x).toBeGreaterThan(inner[0]);
    expect(b.x).toBeLessThan(inner[0] + inner[2]);
  });

  it('draws a subgraph fill the author sets at full opacity', () => {
    const { svg } = render('erDiagram\n subgraph g\n  A\n end\n subgraph h\n  B\n end\n style g fill:#fee', options);
    expect(svg).toMatch(/data-id="g"><rect [^>]*fill="var\(--_a\)" stroke=/);
    expect(svg).toMatch(/data-id="h"><rect [^>]*fill-opacity="0.5"/);
  });

  it('cases relationships that cross a subgraph fill a style sets and inks those inside it', () => {
    const src =
      'erDiagram\n subgraph g\n  A ||--o{ B : has\n end\n B ||--|| C : is\n C ||--|| D : of\n C ||--o{ C : self\n style g fill:#BBDEFB';
    const { svg } = render(src, options);
    const casings = /<g class="pele-casings"[^>]*>(.*?)<\/g>/.exec(svg)![1];
    const edges = [...svg.matchAll(/class="pele-edge [^"]*" data-id="[^"]+"[^>]*><path d="([^"]+)"/g)].map((m) => m[1]);
    expect(edges.map((d) => casings.includes(`<path d="${d}"/>`))).toEqual([false, true, false, false]);
    // A --> B lies inside the subgraph, so it takes the fill's ink instead.
    expect(svg).toMatch(
      /class="pele-edge [^"]*" data-id="[^"]+" stroke="var\(--pele-fg-on-light,#1f1f1f\)"><path d="[^"]+"/
    );
    expect(svg.match(/stroke="var\(--pele-fg-on/g)).toHaveLength(1);
    // A hollow ring keeps the fill's color inside it.
    const ring = svgOf('erDiagram\n subgraph S\n  A ||--o{ B : has\n end\n style S fill:black');
    expect(ring).toMatch(/<circle class="pele-marker"[^>]* fill="black"\/>/);
    expect(casings).not.toContain('pele-marker');
    expect(render(src.replace('\n style g fill:#BBDEFB', ''), options).svg).not.toMatch(
      /pele-casings|stroke="var\(--pele-fg-on/
    );
  });

  it('connects a relationship written before its subgraph to that subgraph', () => {
    const svg = svgOf('erDiagram\n g ||--o{ X : early\n subgraph g\n  A\n end');
    expect(count(svg, '<g class="pele-edge ')).toBe(1);
    expect(svg).not.toContain('data-id="g" transform');
  });

  it('styles a subgraph and lets it set its own direction', () => {
    const svg = svgOf(
      'erDiagram\n subgraph g\n  direction LR\n  A ||--|| B : x\n end\n style g fill:#fee\n classDef k stroke:#c33\n class g k'
    );
    expect(svg).toMatch(
      /<g class="pele-cluster k" data-id="g"><rect[^>]* style="rx:var\(--_r\);stroke:#c33;fill:#fee;"/
    );
    expect(box(svg, 'B').x).toBeGreaterThan(box(svg, 'A').x);
    expect(box(svg, 'B').y).toBeCloseTo(box(svg, 'A').y, 1);
  });

  it('writes the title and the accessible title and description', () => {
    const svg = svgOf(
      '---\ntitle: Order example\n---\nerDiagram\n accTitle: Orders\n accDescr: How orders relate\n A ||--o{ B : x'
    );
    expect(svg).toContain('<text class="pele-title" font-weight="var(--_tw)"');
    expect(svg).toContain('>Order example</tspan></text>');
    expect(svg).toContain('<title id="pele-title">Orders</title>');
    expect(svg).toContain('<desc id="pele-desc">How orders relate</desc>');
  });

  it('links an entity with the internal-link class', () => {
    const result = render('erDiagram\n Note:::internal-link { string body }\n p["My page"]:::internal-link', options);
    expect(result.svg).toContain('<a class="internal-link" href="Note" rel="noopener" data-href="Note">');
    expect(result.links).toEqual([
      { id: 'Note', href: 'Note', internal: true },
      { id: 'p', href: 'My page', internal: true },
    ]);
  });

  it('uses only theme aliases for its own colors', () => {
    const svg = svgOf('erDiagram\n A ||--o{ B : x\n A { int id PK "c" }\n subgraph g\n C\n end');
    for (const el of elements(svg)) {
      for (const [name, value] of el.attrs) {
        if (name === 'fill' || name === 'stroke')
          expect(value, `${name} on <${el.name}>`).toMatch(/^(?:none|var\(--_\w+\)|currentColor)$/);
      }
    }
  });

  it('honors spacing set in the configuration', () => {
    const tight = render('erDiagram\n A ||--o{ B : x', options);
    const loose = render('---\nconfig:\n  er:\n    rankSpacing: 200\n---\nerDiagram\n A ||--o{ B : x', options);
    expect(loose.height).toBeGreaterThan(tight.height + 100);
  });

  it('rejects what Mermaid rejects', () => {
    expect(() => render('erDiagram\n A ||--o{ B', options)).toThrow(PeleError);
    expect(() => render('erDiagram\n A { int }', options)).toThrow(/Expecting 'ATTRIBUTE_WORD'/);
    expect(() => render('erDiagram\n style A fill:rgb(1,2,3)\n', options)).toThrow(/Lexical error/);
    expect(() => render('erDiagram\n subgraph g\n A', options)).toThrow(PeleError);
  });
});
