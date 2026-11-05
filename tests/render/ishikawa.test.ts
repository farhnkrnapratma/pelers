import { describe, expect, it } from 'vitest';
import { PeleError, detectType, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus } from '../support/corpus.js';
import { assertWellFormed } from '../support/xml.js';

const corpus = loadCorpus('ishikawa', /ishikawa/i).filter((src) => /^\s*ishikawa/i.test(src));
const options = { measurer: metricsMeasurer };
const DOC = corpus.find((src) => src.includes('Beautification'))!;

const svgOf = (src: string): string => render('ishikawa-beta\n' + src, options).svg;
const count = (svg: string, needle: string): number => svg.split(needle).length - 1;

interface Box {
  left: number;
  right: number;
  top: number;
  bottom: number;
  text: string;
}

// The box of every one-line text in the picture, from its anchor, its size and the measurer.
function textBoxes(svg: string): Box[] {
  const out: Box[] = [];
  for (const group of svg.matchAll(/<g class="pele-ishikawa-causes" font-size="(\d+)">(.*?)<\/g>/g)) {
    const size = Number(group[1]);
    for (const m of group[2].matchAll(/<text[^>]* x="([-\d.]+)" y="([-\d.]+)" text-anchor="middle">([^<]*)<\/text>/g)) {
      const text = m[3].replace(/&amp;/g, '&');
      const half = metricsMeasurer.width(text, size, 0) / 2;
      const y = Number(m[2]) - size * 0.35;
      out.push({
        left: Number(m[1]) - half,
        right: Number(m[1]) + half,
        top: y - size / 2,
        bottom: y + size / 2,
        text,
      });
    }
  }
  return out;
}

function categories(
  svg: string
): { id: string; body: string; bones: string; box: { x: number; y: number; w: number; h: number } }[] {
  return [
    ...svg.matchAll(
      /<g class="pele-ishikawa-category" data-id="([^"]*)"><path class="pele-ishikawa-bones" d="([^"]+)"[^>]*\/>(.*?)<\/g>(?=<g class="pele-ishikawa-(?:category|head)")/g
    ),
  ].map((m) => {
    const rect = /<rect x="([-\d.]+)" y="([-\d.]+)" width="([\d.]+)" height="([\d.]+)"/.exec(m[3])!;
    return {
      id: m[1],
      bones: m[2],
      body: m[3],
      box: { x: Number(rect[1]), y: Number(rect[2]), w: Number(rect[3]), h: Number(rect[4]) },
    };
  });
}

function spine(svg: string): { from: number; to: number; y: number } {
  const m = /<path class="pele-ishikawa-spine" d="M([-\d.]+),([-\d.]+)H([-\d.]+)"/.exec(svg)!;
  return { from: Number(m[1]), y: Number(m[2]), to: Number(m[3]) };
}

describe('Ishikawa diagram rendering', () => {
  it('renders every corpus input, as well-formed SVG', () => {
    expect(corpus.length).toBeGreaterThan(0);
    for (const src of corpus) {
      const { svg } = render(src, options);
      expect(() => assertWellFormed(svg)).not.toThrow();
      expect(svg).not.toMatch(/<script|<style|<foreignObject|<defs|<marker|\son\w+=|javascript:/i);
      expect(svg).not.toMatch(/NaN|undefined|Infinity/);
      expect(render(src, options).svg).toBe(svg);
    }
  });

  it('is detected under both keywords, supported and typed', () => {
    for (const keyword of ['ishikawa-beta', 'ishikawa', 'Ishikawa-Beta']) {
      const src = `${keyword}\n  Effect\n    Cause\n`;
      expect(detectType(src)).toBe('ishikawa');
      expect(supports(src)).toBe(true);
      const result = render(src, options);
      expect(result.type).toBe('ishikawa');
      expect(result.svg).toMatch(/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" class="pele pele-ishikawa"/);
      expect(result.svg).toContain(`viewBox="0 0 ${result.width} ${result.height}"`);
    }
    const model = parse('ishikawa-beta\n  Effect\n    Cause\n      Sub\n');
    expect(model.type).toBe('ishikawa');
    if (model.type === 'ishikawa') {
      expect(model.root).toEqual({
        text: 'Effect',
        children: [{ text: 'Cause', children: [{ text: 'Sub', children: [] }] }],
      });
    }
  });

  it('puts the effect in a head at the right end of a level spine', () => {
    const svg = svgOf('  Late delivery\n    People\n    Process\n');
    const s = spine(svg);
    expect(s.to).toBeGreaterThan(s.from);
    expect(svg).toContain('<g class="pele-ishikawa-head" data-id="Late delivery">');
    expect(svg).toMatch(/<text class="pele-label" font-weight="var\(--_hw\)"[^>]*><tspan[^>]*>Late delivery<\/tspan>/);
    // The head starts where the spine's arrowhead ends, and every category is to its left.
    const head = /<g class="pele-ishikawa-head"[^>]*><path d="M([-\d.]+),/.exec(svg)!;
    expect(Number(head[1])).toBeCloseTo(s.to + 7, 1);
    for (const category of categories(svg)) expect(category.box.x + category.box.w).toBeLessThan(Number(head[1]) + 1);
    expect(count(svg, 'class="pele-marker"')).toBe(3);
  });

  it('draws only the head when the effect has no causes', () => {
    const svg = svgOf('  Just an effect\n');
    expect(count(svg, 'pele-ishikawa-category')).toBe(0);
    expect(svg).toContain('pele-ishikawa-head');
    expect(svg).toContain('pele-ishikawa-spine');
  });

  it('draws nothing but the frame when there is no effect', () => {
    const result = render('ishikawa-beta%% only a comment', options);
    expect(result.svg).not.toContain('pele-ishikawa-head');
    expect(result.width).toBeGreaterThan(0);
  });

  it('hangs the categories off the spine by turns above and below it, slanting away from the head', () => {
    const svg = svgOf('  E\n    One\n    Two\n    Three\n    Four\n    Five\n');
    const s = spine(svg);
    const all = categories(svg);
    expect(all.map((c) => c.id)).toEqual(['One', 'Two', 'Three', 'Four', 'Five']);
    all.forEach((category, i) => {
      const [x0, y0, x1, y1] = category.bones.match(/-?\d+(?:\.\d+)?/g)!.map(Number);
      // It starts at the spine and ends farther from it and farther from the head.
      expect(Math.abs(y0 - s.y)).toBeLessThan(8);
      expect(x1).toBeLessThan(x0);
      if (i % 2 === 0) expect(y1).toBeLessThan(s.y);
      else expect(y1).toBeGreaterThan(s.y);
      // Its name sits in a box at the far end.
      expect(category.box.x + category.box.w / 2).toBeCloseTo(x1, 1);
      if (i % 2 === 0) expect(category.box.y + category.box.h).toBeCloseTo(y1, 1);
      else expect(category.box.y).toBeCloseTo(y1, 1);
    });
    // A pair shares its place on the spine, and each pair is farther from the head than the last.
    const starts = all.map((c) => Number(c.bones.match(/-?\d+(?:\.\d+)?/)![0]));
    expect(starts[0]).toBeCloseTo(starts[1], 1);
    expect(starts[2]).toBeCloseTo(starts[3], 1);
    expect(starts[2]).toBeLessThan(starts[0] - 20);
    expect(starts[4]).toBeLessThan(starts[2] - 20);
  });

  it('makes the spine longer for more and for larger categories', () => {
    const length = (src: string): number => {
      const s = spine(svgOf(src));
      return s.to - s.from;
    };
    const two = length('  E\n    A\n    B\n');
    const six = length('  E\n    A\n    B\n    C\n    D\n    F\n    G\n');
    const wide = length('  E\n    A\n      A cause with a long description\n    B\n    C\n    D\n    F\n    G\n');
    expect(six).toBeGreaterThan(two + 60);
    expect(wide).toBeGreaterThan(six + 100);
  });

  it('draws causes as level bones off their category and sub-causes as slanted twigs off those', () => {
    const svg = svgOf('  E\n    Cat\n      Cause\n        Sub one\n        Sub two\n      Other\n');
    const [category] = categories(svg);
    const segments = category.bones.split('M').slice(1);
    // The category's bone, two level bones and two twigs.
    expect(segments).toHaveLength(5);
    expect(segments.filter((s) => s.includes('H'))).toHaveLength(2);
    expect(segments.filter((s) => s.includes('L'))).toHaveLength(3);
    expect(category.body).toContain('>Cause</text>');
    expect(category.body).toMatch(/<text fill="var\(--_m\)"[^>]*>Sub one<\/text>/);
    // A cause with sub-causes has a longer bone than one without, to hold their twigs.
    const lengths = segments
      .filter((s) => s.includes('H'))
      .map((s) => {
        const [x, , to] = s.match(/-?\d+(?:\.\d+)?/g)!.map(Number);
        return x - to;
      });
    expect(Math.max(...lengths)).toBeGreaterThan(Math.min(...lengths) + 20);
  });

  it('reads the causes of a category from the top down on both sides of the spine', () => {
    const svg = svgOf(
      '  E\n    Above\n      first\n      second\n      third\n    Below\n      first\n      second\n      third\n'
    );
    const boxes = textBoxes(svg);
    for (const side of [boxes.slice(0, 3), boxes.slice(3)]) {
      const byY = [...side].sort((a, b) => a.top - b.top).map((b) => b.text);
      expect(byY).toEqual(['first', 'second', 'third']);
    }
  });

  it('never lets two texts overlap, and keeps all of them off the category boxes and inside the picture', () => {
    const outlines = [
      DOC,
      'ishikawa-beta\n  E\n    A\n      a1\n        a11\n          a111\n            a1111\n          a112\n        a12\n      a2\n    B\n      b1\n      b2\n      b3\n    C\n    D\n      d1\n        d11\n        d12\n        d13\n      d2\n        d21\n    F\n      a rather long cause that goes on\n        and a long sub cause as well\n',
      'ishikawa-beta\n  E\n' +
        Array.from(
          { length: 9 },
          (_, c) =>
            `    Category ${c}\n` +
            Array.from(
              { length: 1 + (c % 4) },
              (_, k) => `      Cause ${c}.${k}\n        Sub ${c}.${k}.a\n        Sub ${c}.${k}.b\n`
            ).join('')
        ).join(''),
    ];
    for (const src of outlines) {
      const result = render(src, options);
      const shift = /<g transform="translate\(([-\d.]+),([-\d.]+)\)">/.exec(result.svg)!;
      const boxes = textBoxes(result.svg);
      expect(boxes.length).toBeGreaterThan(8);
      for (let i = 0; i < boxes.length; i++) {
        const a = boxes[i];
        expect(a.left + Number(shift[1])).toBeGreaterThanOrEqual(0);
        expect(a.bottom + Number(shift[2])).toBeLessThanOrEqual(result.height);
        expect(a.top + Number(shift[2])).toBeGreaterThanOrEqual(0);
        for (let j = i + 1; j < boxes.length; j++) {
          const b = boxes[j];
          const apart = a.right <= b.left || b.right <= a.left || a.bottom <= b.top || b.bottom <= a.top;
          expect(apart, `${a.text} and ${b.text}`).toBe(true);
        }
        for (const category of categories(result.svg)) {
          const box = category.box;
          const apart = a.right <= box.x || box.x + box.w <= a.left || a.bottom <= box.y || box.y + box.h <= a.top;
          expect(apart, `${a.text} and the box of ${category.id}`).toBe(true);
        }
      }
      const all = categories(result.svg);
      for (let i = 0; i < all.length; i++) {
        for (let j = i + 1; j < all.length; j++) {
          const a = all[i].box;
          const b = all[j].box;
          expect(
            a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y,
            `${all[i].id} and ${all[j].id}`
          ).toBe(true);
        }
      }
    }
  });

  it('wraps long text', () => {
    const svg = svgOf(
      '  An effect described at such length that it has to be wrapped\n    A category with a name that is long as well\n      A cause that takes a good many words to state in full\n'
    );
    expect(count(/<g class="pele-ishikawa-head".*$/.exec(svg)![0], '<tspan')).toBeGreaterThan(1);
    expect(count(/<g class="pele-ishikawa-causes"[^>]*>(.*?)<\/g>/.exec(svg)![1], '<tspan')).toBeGreaterThan(1);
  });

  it('nests by indentation relative to the first cause', () => {
    const model = parse('ishikawa-beta\n        Problem\n  A\n      A1\n    A2\n  B\n C\n');
    if (model.type !== 'ishikawa') throw new Error('wrong type');
    const outline = (node: { text: string; children: unknown[] }): unknown => [
      node.text,
      ...(node.children as { text: string; children: unknown[] }[]).map(outline),
    ];
    expect(outline(model.root!)).toEqual(['Problem', ['A', ['A1'], ['A2']], ['B'], ['C']]);
  });

  it('draws the front matter title above the diagram', () => {
    const { svg } = render('---\ntitle: Root causes\n---\nishikawa-beta\n  Effect\n    Cause\n', options);
    expect(svg).toContain('<text class="pele-title" font-weight="var(--_tw)"');
    expect(svg).toContain('>Root causes</tspan>');
  });

  it('rejects what Mermaid rejects', () => {
    // A blank line between the keyword and an indented effect is one, as is the keyword alone.
    for (const src of ['ishikawa-beta', 'ishikawa-beta\n\n  Effect\n']) {
      expect(() => render(src, options)).toThrow(PeleError);
    }
    expect(render('ishikawa-beta\n', options).svg).not.toContain('pele-ishikawa-head');
    expect(() => render('ishikawa-beta\n  Effect\n  ishikawa again\n', options)).toThrow(
      /Expecting 'SPACELINE', 'NL', 'EOF', 'TEXT', got 'ISHIKAWA'/
    );
  });
});
