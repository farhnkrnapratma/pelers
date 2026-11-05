import { describe, expect, it } from 'vitest';
import { PeleError, parse, render, supports } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const options = { measurer: metricsMeasurer };
const count = (svg: string, re: RegExp): number => svg.match(re)?.length ?? 0;
const relations = (src: string): string[] => {
  const model = parse(src);
  return model.type === 'eventmodeling' ? model.relations.map((r) => `${r.source.id}>${r.target.id}`) : [];
};

const BASIC = 'eventmodeling\ntf 01 ui CartUI\ntf 02 cmd AddItem\ntf 03 evt ItemAdded\n';

describe('eventmodeling rendering', () => {
  it('draws a lane per kind of entity, a box per frame, and the inferred flow', () => {
    const { svg, type } = render(BASIC, options);
    expect(type).toBe('eventmodeling');
    expect(supports(BASIC)).toBe(true);
    expect(count(svg, /class="pele-cluster pele-em-lane"/g)).toBe(3);
    for (const label of ['UI/Automation', 'Command/Read Model', 'Events', 'CartUI', 'AddItem', 'ItemAdded'])
      expect(svg).toContain(`>${label}<`);
    expect(svg).toContain('class="pele-node pele-em-ui" data-id="01"');
    expect(svg).toContain('class="pele-node pele-em-command" data-id="02"');
    expect(svg).toContain('class="pele-node pele-em-event" data-id="03"');
    expect(count(svg, /class="pele-edge"/g)).toBe(2);
    expect(count(svg, /class="pele-marker"/g)).toBe(2);
    expect(svg).toContain('data-id="01-02"');
    expect(svg).not.toContain('NaN');
  });

  it('colors each kind from its own series slot and leaves a UI plain', () => {
    const { svg } = render('eventmodeling\ntf 01 ui A\ntf 02 cmd B\ntf 03 evt C\ntf 04 rmo D\ntf 05 pcr E\n', options);
    for (const slot of [1, 2, 3, 7]) expect(svg).toContain(`var(--pele-series-${slot},`);
    expect(count(svg, /fill-opacity="0.22"/g)).toBe(4);
  });

  it('moves time to the right, and keeps frames of a lane from overlapping', () => {
    const { svg } = render('eventmodeling\ntf 01 cmd A\ntf 02 evt B\ntf 03 cmd C\ntf 04 cmd D\ntf 05 evt E\n', options);
    const box = (id: string): [number, number] => {
      const m = new RegExp(`data-id="${id}"><rect x="([\\d.]+)" y="[\\d.]+" width="([\\d.]+)"`).exec(svg)!;
      return [Number(m[1]), Number(m[1]) + Number(m[2])];
    };
    for (const [a, b] of [
      ['01', '02'],
      ['02', '03'],
      ['03', '04'],
      ['04', '05'],
    ])
      expect(box(b)[0]).toBeGreaterThan(box(a)[0]);
    expect(box('03')[0]).toBeGreaterThan(box('01')[1]);
    expect(box('04')[0]).toBeGreaterThan(box('03')[1]);
    expect(box('05')[0]).toBeGreaterThan(box('02')[1]);
  });

  it('infers each relation from the nearest earlier frame in another lane', () => {
    expect(relations(BASIC)).toEqual(['01>02', '02>03']);
    expect(relations('eventmodeling\ntf 01 evt A\ntf 02 evt B\ntf 03 rmo C\ntf 04 rmo D\n')).toEqual([
      '02>03',
      '02>04',
    ]);
  });

  it('stops the inferred flow at a reset frame and follows explicit sources', () => {
    expect(relations('eventmodeling\ntf 01 ui A\ntf 02 cmd B\nrf 03 evt C\ntf 04 pcr D\n')).toEqual(['01>02', '03>04']);
    expect(relations('eventmodeling\ntf 01 evt A\ntf 02 ui B\nrf 03 rmo C ->> 01\n')).toEqual(['01>02', '01>03']);
    expect(relations('eventmodeling\nrf 02 evt A\nrf 03 evt B\ntf 01 rmo C ->> 03 ->> 02 ->> 09\n')).toEqual([
      '02>01',
      '03>01',
    ]);
    expect(relations('eventmodeling\ntf 01 evt A ->> 02\ntf 02 cmd B\n')).toEqual(['01>02']);
  });

  it('gives each namespace its own lane within a group, shared by its frames', () => {
    const src =
      'eventmodeling\ntf 01 cmd Cart.Add\ntf 02 evt Cart.Added\ntf 03 cmd Cart.Remove\ntf 04 evt Other.Seen\ntf 05 evt Plain\ntf 06 evt a.b.c\n';
    const model = parse(src);
    expect(model.type).toBe('eventmodeling');
    if (model.type !== 'eventmodeling') return;
    expect(model.lanes.map((lane) => [lane.index, lane.label])).toEqual([
      [101, 'C/RM: Cart'],
      [200, 'Events'],
      [201, 'Stream: Cart'],
      [202, 'Stream: Other'],
    ]);
    expect(model.boxes.map((box) => [box.name, box.lane.index])).toEqual([
      ['Add', 101],
      ['Added', 201],
      ['Remove', 101],
      ['Seen', 202],
      ['Plain', 200],
      ['a.b.c', 200],
    ]);
    expect(count(render(src, options).svg, /class="pele-cluster pele-em-lane"/g)).toBe(4);
  });

  it('shows inline data and data blocks, a block winning over inline data', () => {
    const src =
      'eventmodeling\ntf 01 cmd A { productId: 7 }\ntf 02 evt B [[Payload]] { ignored: true }\ntf 03 evt C "{ "a": 1 }"\ntf 04 evt D [[Missing]]\ntf 05 evt E {x}\n' +
      'data Payload {\n  description: string\n    nested: true\n}\n';
    const model = parse(src);
    if (model.type !== 'eventmodeling') throw new Error('wrong type');
    expect(model.boxes.map((box) => box.data)).toEqual([
      'productId: 7',
      'description: string\n  nested: true',
      '"a": 1',
      undefined,
      'x',
    ]);
    const { svg } = render(src, options);
    expect(svg).toContain('>productId: 7</tspan>');
    expect(svg).toContain('>  nested: true</tspan>');
    expect(svg).not.toContain('ignored');
    expect(count(svg, /class="pele-em-data"/g)).toBe(4);
  });

  it('wraps long data lines without splitting emoji', () => {
    const { svg } = render(`eventmodeling\ntf 01 cmd A {${'a'.repeat(47)}😀Z}`, options);
    expect(svg).toContain(`>${'a'.repeat(47)}😀</tspan>`);
    expect(svg).toContain('>Z</tspan>');
  });

  it('keeps comment lines inside data blocks, as Mermaid does', () => {
    const { svg } = render(
      'eventmodeling\n%% a real comment\ntf 01 evt A [[D]]\ndata D {\n  %% kept\n  a: 1\n}\n',
      options
    );
    expect(svg).toContain('>%% kept</tspan>');
    expect(svg).not.toContain('a real comment');
  });

  it('lists notes and specifications under the lanes', () => {
    const src =
      'eventmodeling\ntf 01 cmd Update\ntf 02 evt Updated\nentity Updated\nnote 02 `md` {\n  first line\n  second line\n}\n' +
      'gwt 01\n given\n  evt Updated\n  evt Other\n when\n  cmd Update\n then\n  evt Updated\ngwt 02 given evt A then evt B\n';
    const { svg } = render(src, options);
    expect(count(svg, /class="pele-em-note"/g)).toBe(1);
    expect(count(svg, /class="pele-em-specification"/g)).toBe(2);
    expect(svg).toContain('Note · 02 Updated');
    expect(svg).toContain('>second line</tspan>');
    expect(svg).toContain('>Given  Updated, Other</tspan>');
    expect(svg).toContain('>When  Update</tspan>');
    const model = parse(src);
    if (model.type === 'eventmodeling') {
      expect(model.specifications[1]).toEqual({ frame: '02', given: ['A'], when: [], then: ['B'] });
      expect(model.entities).toEqual(['Updated']);
    }
  });

  it('reads titles written the only way the grammar allows, and front matter titles', () => {
    const { svg } = render(
      'eventmodeling title Checkout\ntf 01 ui A accTitle: Acc\ntf 02 cmd B accDescr: Described\n',
      options
    );
    expect(svg).toContain('>Checkout<');
    expect(svg).toContain('<title id="pele-title">Acc</title>');
    expect(svg).toContain('<desc id="pele-desc">Described</desc>');
    expect(render('---\ntitle: Front\n---\neventmodeling\ntf 01 ui A\n', options).svg).toContain('>Front<');
  });

  it('reads entity names that start with title', () => {
    const model = parse('eventmodeling\ntf 01 ui titles\ntf 02 cmd title_x\nentity titles\n');
    expect(model.type).toBe('eventmodeling');
    if (model.type === 'eventmodeling') {
      expect(model.boxes.map((b) => b.name)).toEqual(['titles', 'title_x']);
      expect(model.entities).toEqual(['titles']);
    }
  });

  it('draws an empty model and is deterministic', () => {
    expect(render('eventmodeling', options).svg).toContain('<svg');
    expect(render(BASIC, options).svg).toBe(render(BASIC, options).svg);
  });

  it("rejects what Mermaid rejects, with Mermaid's message for a reused frame number", () => {
    try {
      render('eventmodeling\n  tf 01 ui UI\n  %% a comment line\n  rf 01 evt Event\n', options);
      expect.unreachable();
    } catch (error) {
      expect(error).toBeInstanceOf(PeleError);
      expect((error as PeleError).message).toBe('Duplicate event modeling frame ID "01" on line 4');
      expect((error as PeleError).line).toBe(4);
    }
    expect(() => render('eventmodeling\ntf 01 cmd\n%% a comment line\n\n', options)).toThrow(
      expect.objectContaining({ line: 2, column: 10 })
    );
    for (const src of [
      'eventmodeling\ntitle T\n',
      'eventmodeling\ntf 01 evt data\n',
      'eventmodeling\ntf 01 evt A {\n',
      'eventmodeling\ntf 1234 evt A\n',
    ]) {
      expect(() => render(src, options), src).toThrow(PeleError);
    }
  });
});
