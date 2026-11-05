import { describe, expect, it } from 'vitest';
import { PeleError, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus } from '../support/corpus.js';
import { assertWellFormed } from '../support/xml.js';

const options = { measurer: metricsMeasurer };
const corpus = loadCorpus('requirement', /requirementDiagram/i);

const BASIC = `requirementDiagram

requirement test_req {
id: 1
text: the test text.
risk: high
verifymethod: test
}

element test_entity {
type: simulation
docRef: reqs/test_entity
}

test_entity - satisfies -> test_req
`;

function count(svg: string, pattern: RegExp): number {
  return svg.match(pattern)?.length ?? 0;
}

describe('requirement rendering', () => {
  it('renders every documentation example as well-formed SVG', () => {
    let rendered = 0;
    for (const src of corpus) {
      let svg: string;
      try {
        svg = render(src, options).svg;
      } catch (error) {
        expect(error).toBeInstanceOf(PeleError);
        continue;
      }
      rendered++;
      const where = JSON.stringify(src).slice(0, 120);
      expect(() => assertWellFormed(svg), where).not.toThrow();
      expect(svg, where).not.toContain('NaN');
      expect(svg, where).not.toContain('undefined');
      expect(render(src, options).svg, where).toBe(svg);
    }
    expect(rendered).toBeGreaterThan(9);
  });

  it('draws a box per requirement and element, with a header and fields', () => {
    const { svg, type } = render(BASIC, options);
    expect(type).toBe('requirement');
    expect(supports('requirementDiagram\n')).toBe(true);
    expect(svg).toMatch(/^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" class="pele pele-requirement"/);
    expect(count(svg, /class="pele-node pele-requirement"/g)).toBe(1);
    expect(count(svg, /class="pele-node pele-element"/g)).toBe(1);
    expect(svg).toContain('data-id="test_req"');
    expect(svg).toContain('>«Requirement»<');
    expect(svg).toContain('>«Element»<');
    for (const text of [
      'ID',
      'Text',
      'Risk',
      'Verification',
      'Type',
      'Doc Ref',
      '1',
      'the test text.',
      'High',
      'Test',
      'simulation',
      'reqs/test_entity',
    ]) {
      expect(svg, text).toContain(`>${text}<`);
    }
    expect(count(svg, /class="pele-divider"/g)).toBe(2);
  });

  it('names each requirement type', () => {
    const types: [string, string][] = [
      ['requirement', 'Requirement'],
      ['functionalRequirement', 'Functional Requirement'],
      ['interfaceRequirement', 'Interface Requirement'],
      ['performanceRequirement', 'Performance Requirement'],
      ['physicalRequirement', 'Physical Requirement'],
      ['designConstraint', 'Design Constraint'],
    ];
    for (const [keyword, name] of types) {
      expect(render(`requirementDiagram\n${keyword} r {\n}\n`, options).svg).toContain(`>«${name}»<`);
    }
  });

  it('leaves out empty fields and the divider of an empty body', () => {
    const { svg } = render('requirementDiagram\nrequirement r {\nid: 7\n}\nelement e {\n}\n', options);
    expect(svg).toContain('>ID<');
    expect(svg).not.toContain('>Text<');
    expect(svg).not.toContain('>Type<');
    expect(count(svg, /class="pele-divider"/g)).toBe(1);
  });

  it('draws contains with a circled plus at its source and the rest as dashed arrows', () => {
    const contains = render(
      'requirementDiagram\nrequirement a {\n}\nrequirement b {\n}\na - contains -> b\n',
      options
    ).svg;
    expect(contains).toContain('class="pele-edge pele-relationship-contains" data-id="a-b-0"');
    expect(count(contains, /<circle class="pele-marker"/g)).toBe(1);
    expect(contains).not.toContain('stroke-dasharray');
    expect(contains).toContain('>«contains»<');
    // The mark sits under the source, which is the upper node.
    const cy = Number(/<circle class="pele-marker" cx="[\d.]+" cy="([\d.]+)"/.exec(contains)![1]);
    const [ay, by] = ['a', 'b'].map((id) =>
      Number(new RegExp(`data-id="${id}" transform="translate\\([\\d.]+,([\\d.]+)\\)"`).exec(contains)![1])
    );
    expect(ay).toBeLessThan(by);
    expect(Math.abs(cy - ay)).toBeLessThan(Math.abs(cy - by));

    for (const kind of ['copies', 'derives', 'satisfies', 'verifies', 'refines', 'traces']) {
      const svg = render(`requirementDiagram\nrequirement a {\n}\nrequirement b {\n}\na - ${kind} -> b\n`, options).svg;
      expect(svg, kind).toContain('class="pele-edge pele-relationship-arrow"');
      expect(svg, kind).toContain('stroke-dasharray="5 4"');
      expect(svg, kind).toContain(`>«${kind}»<`);
      expect(svg, kind).not.toContain('<circle');
    }
  });

  it('reads a left arrow as the same relationship written the other way round', () => {
    const forward = render('requirementDiagram\nrequirement a {\n}\nelement b {\n}\nb - satisfies -> a\n', options).svg;
    const backward = render(
      'requirementDiagram\nrequirement a {\n}\nelement b {\n}\na <- satisfies - b\n',
      options
    ).svg;
    expect(backward).toBe(forward);
  });

  it('lays out along the declared direction', () => {
    const body = 'requirement a {\n}\nrequirement b {\n}\nrequirement c {\n}\na - contains -> b\nb - contains -> c\n';
    const tall = render(`requirementDiagram\n${body}`, options);
    const wide = render(`requirementDiagram\ndirection LR\n${body}`, options);
    expect(tall.height).toBeGreaterThan(tall.width);
    expect(wide.width).toBeGreaterThan(wide.height);
    expect(render(`requirementDiagram\ndirection BT\n${body}`, options).svg).not.toBe(tall.svg);
    expect(render(`requirementDiagram\ndirection RL\n${body}`, options).svg).not.toBe(wide.svg);
  });

  it('draws self relationships and parallel relationships', () => {
    const { svg } = render(
      'requirementDiagram\nrequirement a {\n}\nrequirement b {\n}\na - contains -> a\na - traces -> b\na - refines -> b\n',
      options
    );
    expect(count(svg, /class="pele-edge /g)).toBe(3);
    expect(count(svg, /class="pele-edge-label"/g)).toBe(3);
    expect(svg).toContain('data-id="a-a-0"');
  });

  it('keeps the ends of edges that share a side apart', () => {
    const { svg } = render(
      'requirementDiagram\nrequirement a {\n}\nrequirement b {\n}\nrequirement c {\n}\na - contains -> b\na - contains -> c\n',
      options
    );
    const xs = [...svg.matchAll(/<circle class="pele-marker" cx="([\d.]+)"/g)].map((m) => Number(m[1]));
    expect(xs.length).toBe(2);
    expect(Math.abs(xs[0] - xs[1])).toBeGreaterThan(12);
  });

  it('does not draw a relationship whose end is not defined', () => {
    const { svg } = render('requirementDiagram\nrequirement a {\n}\na - contains -> missing\n', options);
    expect(svg).not.toContain('pele-edge');
    expect(svg).not.toContain('missing');
  });

  it('applies style, classDef, class and the ::: shorthand', () => {
    const { svg } = render(
      `requirementDiagram
requirement a:::hot {
id: 1
}
element b {
type: t
}
element c {
}
classDef hot fill:#f96,stroke:#333,stroke-width:4px
classDef cold color:blue
class b cold
c:::hot,cold
style c stroke:#0a0
`,
      options
    );
    expect(svg).toMatch(
      /class="pele-node pele-requirement hot" data-id="a"[^>]*><rect[^>]*style="rx:var\(--_r\);fill:#f96;stroke:#333;stroke-width:4px;"/
    );
    expect(svg).toMatch(/class="pele-node pele-element cold" data-id="b"/);
    expect(svg).toMatch(
      /class="pele-node pele-element hot cold" data-id="c"[^>]*><rect[^>]*style="rx:var\(--_r\);fill:#f96;stroke:#0a0;stroke-width:4px;"/
    );
    expect(svg).toContain('style="fill:blue;"');
    // The divider takes the stroke but not the fill.
    expect(svg).toMatch(/class="pele-divider"[^>]*style="stroke:#333;stroke-width:4px;"/);
  });

  it('picks a contrasting text color on fills a style sets', () => {
    const { svg } = render(
      'requirementDiagram\nrequirement a {\nid: 1\ntext: words\n}\nelement b {\ntype: t\n}\nelement c {\n}\nstyle a fill:#1f3a5f\nstyle b fill:#1f3a5f,color:#eee\n',
      options
    );
    const node = (id: string): string => new RegExp(`data-id="${id}"[^>]*>(.*?)</g></g>`).exec(svg)![1];
    for (const part of ['pele-stereotype', 'pele-label', 'pele-field', 'pele-value']) {
      expect(node('a')).toMatch(new RegExp(`<text class="${part}"[^>]* fill="var\\(--pele-fg-on-dark,#fff\\)"`));
    }
    expect(node('b')).toMatch(/<text class="pele-label"[^>]* style="fill:#eee;"/);
    expect(node('b')).not.toContain('--pele-fg-on');
    expect(node('c')).toMatch(/<text class="pele-stereotype"[^>]* fill="var\(--_m\)"/);
  });

  it('reads markdown emphasis in names and text', () => {
    const { svg } = render(
      'requirementDiagram\nrequirement "__name__" {\ntext: "*slanted* and **heavy** words"\n}\n',
      options
    );
    expect(svg).toContain('data-id="__name__"');
    expect(svg).toMatch(/<text[^>]* font-weight="var\(--_hw\)"[^>]*><tspan[^>]*>name<\/tspan>/);
    expect(svg).toMatch(/<tspan[^>]*font-style="italic">slanted<\/tspan>/);
    expect(svg).toMatch(/<tspan[^>]*font-weight="var\(--_w\)">heavy<\/tspan>/);
    expect(svg).not.toContain('*');
    expect(svg).not.toContain('__name__<');
  });

  it('wraps long text and grows the box to hold it', () => {
    const short = render('requirementDiagram\nrequirement a {\ntext: short\n}\n', options);
    const long = render(
      `requirementDiagram\nrequirement a {\ntext: "${'many words in a row '.repeat(12)}"\n}\n`,
      options
    );
    expect(long.height).toBeGreaterThan(short.height + 60);
    expect(long.width).toBeLessThan(420);
  });

  it('draws the front matter title and names the diagram for assistive technology', () => {
    const { svg } = render(
      '---\ntitle: Checkout\n---\nrequirementDiagram\naccTitle: Short name\naccDescr: Longer text\nrequirement a {\n}\n',
      options
    );
    expect(svg).toContain('class="pele-title"');
    expect(svg).toContain('>Checkout<');
    expect(svg).toContain('<title id="pele-title">Short name</title>');
    expect(svg).toContain('<desc id="pele-desc">Longer text</desc>');
  });

  it('exposes the model', () => {
    const model = parse(BASIC);
    expect(model.type).toBe('requirement');
    if (model.type === 'requirement') {
      expect(model.requirements.get('test_req')).toMatchObject({
        type: 'Requirement',
        requirementId: '1',
        risk: 'High',
        verifyMethod: 'Test',
      });
      expect(model.elements.get('test_entity')).toMatchObject({ type: 'simulation', docRef: 'reqs/test_entity' });
      expect(model.relations).toEqual([{ type: 'satisfies', src: 'test_entity', dst: 'test_req' }]);
      expect(model.direction).toBe('TB');
    }
  });

  it('keeps Mermaid quirks: the first of two repeated fields wins, and keywords ignore case', () => {
    const model = parse('requirementDiagram\nRequirement a {\nID: first\nid: second\nRISK: HIGH\n}\n');
    if (model.type === 'requirement') {
      expect(model.requirements.get('a')).toMatchObject({ requirementId: 'first', risk: 'High' });
    }
  });

  it('rejects what Mermaid rejects', () => {
    for (const src of [
      'requirementDiagram\nrequirement a {\nid: REQ-1\n}\n',
      'requirementDiagram\nrequirement a { \n}\n',
      'requirementDiagram\nrequirement a {\ntext: test the thing\n}\n',
      'requirementDiagram\na - contains - b\n',
      'requirementDiagram\nstyle a stroke-width:1.5px\n',
      'requirementDiagram\ntitle My diagram\n',
      'requirement a {\n}\n',
    ]) {
      expect(() => render(src, options), src).toThrow(PeleError);
    }
    expect(() => render('requirementDiagram\nrequirement a {\nid: REQ-1\n}\n', options)).toThrow(
      /Expecting 'NEWLINE', got 'LINE'/
    );
  });

  it('refuses a diagram that copies class styles without bound', () => {
    const src =
      'requirementDiagram\nrequirement a {\n}\nclassDef c ' + 'a:b,'.repeat(2000) + 'a:b\n' + 'class a c\n'.repeat(600);
    expect(() => render(src, options)).toThrow(/too many style declarations/);
  });

  it('renders an empty diagram', () => {
    expect(render('requirementDiagram\n', options).svg).toContain('<svg');
  });
});
