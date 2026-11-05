import { describe, expect, it } from 'vitest';
import { PeleError, detectType, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { random } from '../support/corpus.js';
import { NOTATIONS, corpusFor, grammar } from '../support/railroad-oracle.js';
import { assertWellFormed, elements } from '../support/xml.js';

const options = { measurer: metricsMeasurer };
const ebnf = (rules: string, extra: object = {}) => render('railroad-ebnf-beta\n' + rules, { ...options, ...extra });

interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

function rects(svg: string): Rect[] {
  return elements(svg)
    .filter((el) => el.name === 'rect')
    .map((el) => ({
      x: Number(el.attrs.get('x')),
      y: Number(el.attrs.get('y')),
      w: Number(el.attrs.get('width')),
      h: Number(el.attrs.get('height')),
    }));
}

function count(svg: string, needle: string): number {
  return svg.split(needle).length - 1;
}

// Every box inside the drawing, no two boxes overlapping, and every absolute track coordinate inside the drawing.
function assertTidy(svg: string, width: number, height: number, where: string): void {
  assertWellFormed(svg);
  const boxes = rects(svg);
  for (const box of boxes) {
    expect(Number.isFinite(box.x + box.y + box.w + box.h), where).toBe(true);
    expect(box.x >= 0 && box.y >= 0 && box.x + box.w <= width && box.y + box.h <= height, `${where}: box inside`).toBe(
      true
    );
  }
  if (boxes.length <= 400) {
    for (let i = 0; i < boxes.length; i++) {
      for (let j = i + 1; j < boxes.length; j++) {
        const a = boxes[i];
        const b = boxes[j];
        const apart = a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
        expect(apart, `${where}: boxes ${i} and ${j} overlap`).toBe(true);
      }
    }
  }
  for (const el of elements(svg)) {
    if (el.name !== 'path') continue;
    const d = el.attrs.get('d') ?? '';
    expect(d, where).not.toMatch(/NaN|Infinity|undefined/);
    for (const m of d.matchAll(/M(-?[\d.]+),(-?[\d.]+)|H(-?[\d.]+)|V(-?[\d.]+)/g)) {
      const xs = [m[1], m[3]].filter((v) => v !== undefined).map(Number);
      const ys = [m[2], m[4]].filter((v) => v !== undefined).map(Number);
      for (const x of xs) expect(x >= 0 && x <= width, `${where}: x ${x}`).toBe(true);
      for (const y of ys) expect(y >= 0 && y <= height, `${where}: y ${y}`).toBe(true);
    }
  }
}

describe('railroad rendering', () => {
  it('accepts the four keywords as one diagram type', () => {
    const sources = [
      'railroad-beta\nr = terminal("a") ;',
      'railroad-ebnf-beta\nr = "a" ;',
      'railroad-abnf-beta\nr = "a" ;',
      'railroad-peg-beta\nr <- "a" ;',
    ];
    const notations = ['railroad', 'ebnf', 'abnf', 'peg'];
    sources.forEach((src, i) => {
      expect(detectType(src)).toBe('railroad');
      expect(supports(src)).toBe(true);
      const model = parse(src);
      expect(model.type).toBe('railroad');
      if (model.type === 'railroad') {
        expect(model.notation).toBe(notations[i]);
        expect(model.rules).toEqual([{ name: 'r', definition: { type: 'terminal', value: 'a' } }]);
      }
      const { svg, type } = render(src, options);
      expect(type).toBe('railroad');
      expect(svg).toContain('class="pele pele-railroad"');
      expect(count(svg, 'class="pele-node pele-terminal"')).toBe(1);
    });
    // The four drawings differ in nothing but the notation they were written in.
    expect(new Set(sources.map((src) => render(src, options).svg)).size).toBe(1);
  });

  it('detects a keyword in any case, as Mermaid does, and then rejects it', () => {
    expect(detectType('Railroad-EBNF-Beta\nr = "a" ;')).toBe('railroad');
    expect(() => render('Railroad-EBNF-Beta\nr = "a" ;', options)).toThrow(PeleError);
    expect(() => render('RAILROAD-BETA\nr = terminal("a") ;', options)).toThrow(/railroad-beta/);
  });

  it('draws one diagram per rule with a heading, markers, and a track', () => {
    const { svg } = ebnf('first = "a" b ;\nsecond = c ;');
    expect(count(svg, '<g class="pele-rule"')).toBe(2);
    expect(svg).toContain('<g class="pele-rule" data-id="first">');
    expect(svg).toMatch(/<text class="pele-rule-name"[^>]*font-weight="var\(--_hw\)"[^>]*>first<\/text>/);
    expect(count(svg, 'class="pele-marker pele-start"')).toBe(2);
    expect(count(svg, 'class="pele-marker pele-end"')).toBe(2);
    expect(count(svg, 'class="pele-edge pele-track"')).toBe(2);
  });

  it('draws terminals as pills and non-terminals as boxes that name their rule', () => {
    const { svg } = ebnf('r = "lit" name ;');
    const [terminal, nonterminal] = rects(svg);
    expect(svg).toMatch(/<g class="pele-node pele-terminal"><rect[^>]* rx="15" fill="var\(--_s\)"/);
    expect(svg).toMatch(
      /<g class="pele-node pele-nonterminal" data-id="name"><rect[^>]* fill="var\(--_bg\)"[^>]* style="rx:var\(--_r\)"/
    );
    expect(svg).toContain('font-family="var(--_fm)">lit</text>');
    expect(terminal.h).toBe(30);
    // Left to right on one baseline.
    expect(nonterminal.x).toBeGreaterThan(terminal.x + terminal.w);
    expect(nonterminal.y).toBe(terminal.y);
  });

  it('keeps the first alternative on the track and stacks the rest below it', () => {
    const one = ebnf('r = a ;');
    const { svg, height } = ebnf('r = a | bb | "c" ;');
    const [a, b, c] = rects(svg);
    expect(a.y).toBe(rects(one.svg)[0].y);
    expect(b.y).toBeGreaterThanOrEqual(a.y + a.h);
    expect(c.y).toBeGreaterThanOrEqual(b.y + b.h);
    expect(a.x + a.w / 2).toBe(b.x + b.w / 2);
    expect(height).toBeGreaterThan(one.height);
    // Two quarter turns of one radius switch a branch off the track and back on.
    expect(count(svg, 'a10,10 0 0 1 10,10')).toBe(1);
    expect(count(svg, 'a10,10 0 0 0 10,10')).toBe(2);
  });

  it('draws an optional element with a bypass over it', () => {
    const plain = ebnf('r = a ;');
    const optional = ebnf('r = [ a ] ;');
    expect(optional.svg).toBe(ebnf('r = a? ;').svg);
    expect(optional.width).toBe(plain.width + 40);
    expect(optional.height).toBeGreaterThan(plain.height);
    // The bypass leaves room above; the element stays on the track.
    expect(rects(optional.svg)[0].y).toBeGreaterThan(rects(plain.svg)[0].y);
    expect(optional.svg).not.toContain('class="pele-marker" d=');
  });

  it('draws repetition as a loop back under the element, with a bypass when it may be skipped', () => {
    const plain = ebnf('r = a ;');
    const more = ebnf('r = a+ ;');
    const any = ebnf('r = a* ;');
    expect(more.width).toBe(plain.width + 20);
    expect(rects(more.svg)[0].y).toBe(rects(plain.svg)[0].y);
    expect(more.height).toBeGreaterThan(plain.height);
    expect(count(more.svg, 'class="pele-marker" d="M')).toBe(1);
    expect(any.svg).toBe(ebnf('r = { a } ;').svg);
    expect(any.width).toBe(more.width + 40);
    expect(rects(any.svg)[0].y).toBeGreaterThan(rects(more.svg)[0].y);
  });

  it('writes ABNF repeat counts on the loop', () => {
    const abnf = (rule: string) => render('railroad-abnf-beta\n' + rule, options).svg;
    expect(abnf('r = 3a ;')).toContain('>3×</text>');
    expect(abnf('r = 2*4a ;')).toContain('>2–4×</text>');
    expect(abnf('r = 2*a ;')).toContain('>≥2×</text>');
    expect(abnf('r = *5a ;')).toContain('>≤5×</text>');
    expect(abnf('r = ' + '9'.repeat(400) + 'a ;')).toContain('>∞×</text>');
    expect(abnf('r = *a ;')).not.toContain('pele-count');
    expect(abnf('r = 1*a ;')).not.toContain('pele-count');
    expect(abnf('r = 1a ;')).toBe(abnf('r = a ;'));
    expect(abnf('r = 0*1a ;')).toBe(abnf('r = [ a ] ;'));
  });

  it('draws special sequences dashed, and exceptions and predicates the way Mermaid models them', () => {
    const special = ebnf('r = ? any character ? ;').svg;
    expect(special).toMatch(/<g class="pele-node pele-special"><rect[^>]* stroke-dasharray="4 3"/);
    expect(special).toContain('<tspan fill="var(--_m)">? </tspan>any character<tspan fill="var(--_m)"> ?</tspan>');
    const exception = ebnf('r = letter - "x" ;').svg;
    expect(count(exception, 'pele-terminal')).toBe(2);
    expect(exception).toContain('>-</text>');
    const peg = render('railroad-peg-beta\nR <- !Keyword &"k" . !( a b ) ;', options).svg;
    expect(count(peg, 'pele-special')).toBe(4);
    expect(peg).toContain('>!Keyword<');
    expect(peg).toContain('>&amp;&quot;k&quot;<');
    expect(peg).toContain('>.<');
    expect(peg).toContain('>!(...)<');
  });

  it('shows text as written', () => {
    const { svg } = render(
      'railroad-beta\nr = sequence(terminal("<br>"), terminal("*a*"), terminal("#35;"), terminal("&lt;"), terminal(" "), terminal("a  b"), terminal("\\n\\t"), nonterminal("fa:fa-user")) ;',
      options
    );
    expect(svg).toContain('>&lt;br&gt;</text>');
    expect(svg).toContain('>*a*</text>');
    expect(svg).toContain('>#</text>');
    expect(svg).toContain('>&amp;lt;</text>');
    expect(svg).toContain('>␣</text>');
    expect(svg).toContain(' xml:space="preserve" font-family="var(--_fm)">a  b</text>');
    expect(svg).toContain('>\\n\\t</text>');
    expect(svg).toContain('>fa:fa-user</text>');
  });

  it('reads rule names that start with title in each notation', () => {
    for (const src of [
      'railroad-beta\ntitles = nonterminal("title-x") ;\ntitle-x = terminal("a") ;',
      'railroad-ebnf-beta\ntitles = title-x ;\ntitle-x = "a" ;',
      'railroad-abnf-beta\ntitles = title-x ;\ntitle-x = "a" ;',
      'railroad-peg-beta\ntitles <- title-x ;\ntitle-x <- "a" ;',
    ]) {
      const model = parse(src);
      expect(model.type, src).toBe('railroad');
      if (model.type === 'railroad')
        expect(
          model.rules.map((r) => r.name),
          src
        ).toEqual(['titles', 'title-x']);
      expect(render(src, options).svg, src).toContain('data-id="title-x"');
    }
  });

  it('takes the title from the diagram, or else from front matter', () => {
    expect(ebnf('title "Quoted \\"title\\""\nr = a ;').svg).toContain('class="pele-title" font-weight="var(--_tw)"');
    expect(ebnf('title "Quoted \\"title\\""\nr = a ;').svg).toContain('>Quoted &quot;title&quot;<');
    expect(render('---\ntitle: From front matter\n---\nrailroad-peg-beta\nR <- a ;', options).svg).toContain(
      '>From front matter<'
    );
    const own = render('---\ntitle: From front matter\n---\nrailroad-abnf-beta\ntitle Own\nr = a ;', options).svg;
    expect(own).toContain('>Own<');
    expect(own).not.toContain('From front matter');
    expect(ebnf('r = a ;').svg).not.toContain('pele-title');
  });

  it('names the drawing for assistive technology', () => {
    const { svg } = ebnf('accTitle: Grammar\naccDescr {\n  Two\n  lines\n}\nr = a ;');
    expect(svg).toContain('<title id="pele-title">Grammar</title>');
    expect(svg).toContain('<desc id="pele-desc">Two\nlines</desc>');
  });

  it('honours the railroad config keys that have a meaning here', () => {
    const src = 'railroad-ebnf-beta\nr = a ( b | c )* [ d ] ;\ns = e ;';
    const withConfig = (config: string) => render(`---\nconfig:\n  railroad:\n${config}\n---\n${src}`, options);
    const plain = render(src, options);
    expect(withConfig('    arcRadius: 20').width).toBeGreaterThan(plain.width);
    expect(withConfig('    arcRadius: 20').svg).toContain('a20,20 0 0 1 20,20');
    expect(withConfig('    padding: 20').width).toBeGreaterThan(plain.width);
    expect(withConfig('    padding: 20').height).toBeGreaterThan(plain.height);
    expect(withConfig('    horizontalSeparation: 40').width).toBe(plain.width + 2 * 24);
    expect(withConfig('    horizontalSeparation: 40').height).toBe(plain.height);
    expect(withConfig('    verticalSeparation: 30').height).toBeGreaterThan(plain.height);
    expect(withConfig('    verticalSeparation: 30').width).toBe(plain.width);
    expect(withConfig('    fontSize: 20').svg).toContain(
      '<g class="pele-rules" transform="translate(0.5,0.5)" font-size="20"'
    );
    expect(withConfig('    fontSize: 20').width).toBeGreaterThan(plain.width);
    expect(withConfig('    strokeWidth: 2').svg).toContain('stroke="var(--_l)" stroke-width="2"');
    expect(plain.svg).not.toContain('stroke-width');
    expect(withConfig('    markerRadius: 8').svg).toContain('r="8"');
    expect(withConfig('    showMarkers: false').svg).not.toContain('<circle');
    expect(withConfig('    showMarkers: false').width).toBe(plain.width - 16);
    expect(withConfig('    arcRadius: wide\n    padding: -4\n    fontSize: "x"').svg).toBe(plain.svg);
    expect(render(src, { ...options, config: { railroad: { arcRadius: 20 } } }).svg).toBe(
      withConfig('    arcRadius: 20').svg
    );
  });

  it('draws a rule again when its name is defined again', () => {
    const { svg } = ebnf('r = a ;\nr = b ;');
    expect(count(svg, '<g class="pele-rule" data-id="r">')).toBe(2);
    const model = parse('railroad-ebnf-beta\nr = a ;\nr = b ;');
    if (model.type === 'railroad')
      expect(model.rules.map((rule) => rule.definition)).toEqual([
        { type: 'nonterminal', name: 'a' },
        { type: 'nonterminal', name: 'b' },
      ]);
  });

  it('draws a grammar with no rules', () => {
    const empty = render('railroad-beta', options);
    expect(empty.svg).toContain('<svg');
    expect(empty.svg).not.toContain('pele-rule');
    expect(render('railroad-ebnf-beta\ntitle Only a title', options).svg).toContain('>Only a title<');
  });

  it('scales with the font size and is the same every time', () => {
    const src = 'railroad-ebnf-beta\nr = a ( b | "c" )+ ;';
    expect(render(src, options).svg).toBe(render(src, options).svg);
    expect(render(src, { ...options, fontSize: 24 }).width).toBeGreaterThan(render(src, options).width);
    expect(render(src, options).svg).toContain('max-width:100%;height:auto;');
    expect(render(src, { ...options, responsive: false }).svg).not.toContain('max-width');
  });

  it('keeps every box apart and inside the drawing', () => {
    let drawn = 0;
    for (const notation of NOTATIONS) {
      const rnd = random(11);
      const sources = [...corpusFor(notation).corpus];
      for (let i = 0; i < 80; i++) sources.push(grammar(notation, rnd));
      for (const src of sources) {
        let result;
        try {
          result = render(src, options);
        } catch (error) {
          expect(error).toBeInstanceOf(PeleError);
          continue;
        }
        assertTidy(result.svg, result.width, result.height, JSON.stringify(src).slice(0, 200));
        drawn++;
      }
    }
    expect(drawn).toBeGreaterThan(300);
  }, 60_000);

  it('holds the same layout rules under unusual config', () => {
    const src = 'railroad-ebnf-beta\ntitle T\nr = a ( b | [ c d ] | { e | f } )+ g? ;\ns = ( ( a+ )+ | b* )* - "x" ;';
    for (const railroad of [
      { arcRadius: 0 },
      { arcRadius: 40, verticalSeparation: 0 },
      { padding: 0, horizontalSeparation: 0, verticalSeparation: 0 },
      { arcRadius: 7.5, padding: 3.3, verticalSeparation: 2.5, horizontalSeparation: 1.5, fontSize: 9.5 },
      { arcRadius: 1e9, padding: 1e9, fontSize: 1e9, markerRadius: 1e9, strokeWidth: 1e9 },
      { arcRadius: Infinity, padding: NaN, fontSize: -1, showMarkers: 'no', markerRadius: null },
    ]) {
      const result = render(src, { ...options, config: { railroad } as never });
      assertTidy(result.svg, result.width, result.height, JSON.stringify(railroad));
    }
  });
});
