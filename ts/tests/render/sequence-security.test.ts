import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };

function check(src: string, extra: object = {}): boolean {
  let svg: string;
  try {
    svg = render(src, { ...options, ...extra }).svg;
  } catch (error) {
    if (error instanceof PeleError) return false;
    throw new Error(
      `Unexpected ${(error as Error).name} for ${JSON.stringify(src).slice(0, 200)}: ${(error as Error).message}`
    );
  }
  assertInert(svg, JSON.stringify(src).slice(0, 160));
  return true;
}

const json = (p: string): string => JSON.stringify(p);

// Hostile text in every position the sequence grammar has.
const TEMPLATES: ((p: string) => string)[] = [
  (p) => `sequenceDiagram\n  participant ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  actor ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A as ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  actor A as ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A@{ "type": ${json(p)} }\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A@{ "type": "database", "alias": ${json(p)} }\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A@{ ${json(p)}: "x", "alias": "y" }\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A@{ ${p} }\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A@{ "type": "queue" } as ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  ${p}->>B: x`,
  (p) => `sequenceDiagram\n  A->>${p}: x`,
  (p) => `sequenceDiagram\n  A->>B: ${p}`,
  (p) => `sequenceDiagram\n  A->>B: wrap: ${p} and some more words to wrap around the corner of the label`,
  (p) => `sequenceDiagram\n  A->>A: ${p}`,
  (p) => `sequenceDiagram\n  A-x+B: ${p}\n  B--)-A: ${p}`,
  (p) => `sequenceDiagram\n  A()->>()B: ${p}`,
  (p) => `sequenceDiagram\n  A ${p} B: x`,
  (p) => `sequenceDiagram\n  Note left of A: ${p}`,
  (p) => `sequenceDiagram\n  Note right of A: ${p}`,
  (p) => `sequenceDiagram\n  Note over A,B: ${p}`,
  (p) => `sequenceDiagram\n  Note over ${p}: x`,
  (p) => `sequenceDiagram\n  Note over A: wrap: ${p}`,
  (p) => `sequenceDiagram\n  loop ${p}\n  A->>B: x\n  end`,
  (p) => `sequenceDiagram\n  alt ${p}\n  A->>B: x\n  else ${p}\n  B->>A: y\n  end`,
  (p) => `sequenceDiagram\n  opt ${p}\n  A->>B: x\n  end`,
  (p) => `sequenceDiagram\n  par ${p}\n  A->>B: x\n  and ${p}\n  B->>A: y\n  end`,
  (p) => `sequenceDiagram\n  par_over ${p}\n  A->>B: x\n  end`,
  (p) => `sequenceDiagram\n  critical ${p}\n  A->>B: x\n  option ${p}\n  B->>A: y\n  end`,
  (p) => `sequenceDiagram\n  break ${p}\n  A->>B: x\n  end`,
  (p) => `sequenceDiagram\n  rect ${p}\n  A->>B: x\n  end`,
  (p) => `sequenceDiagram\n  rect rgb(0, 0, 0)${p}\n  A->>B: x\n  end`,
  (p) => `sequenceDiagram\n  box ${p}\n  participant A\n  end\n  A->>B: x`,
  (p) => `sequenceDiagram\n  box red ${p}\n  participant A\n  end\n  A->>B: x`,
  (p) => `sequenceDiagram\n  box rgb(1, 2, 3)${p}\n  participant A\n  end\n  A->>B: x`,
  (p) => `sequenceDiagram\n  box rgb(${p}) T\n  participant A\n  end\n  A->>B: x`,
  (p) => `sequenceDiagram\n  title ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  title: ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  accTitle: ${p}\n  accDescr: ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  accDescr {\n ${p}\n }\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A\n  links A: {${json(p)}: ${json(p)}}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A\n  links A: ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A\n  link A: ${p} @ ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A\n  link A: Docs @ ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A\n  properties A: {"class": ${json(p)}, ${json(p)}: ${json(p)}}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A\n  properties A: ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  participant A\n  details A: ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  autonumber ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  autonumber\n  A->>B: ${p}\n  A->>A: ${p}`,
  (p) => `sequenceDiagram\n  A->>B: x\n  create participant ${p}\n  A->>${p}: y`,
  (p) => `sequenceDiagram\n  A->>B: x\n  create actor C as ${p}\n  A->>C: y\n  destroy C\n  C-xA: ${p}`,
  (p) => `sequenceDiagram\n  activate ${p}\n  A->>B: x`,
  (p) => `sequenceDiagram\n  A->>B: x\n  %% ${p}\n  B->>A: y # ${p}`,
  (p) => `sequenceDiagram ${p}\n  A->>B: x`,
  (p) =>
    `---\ntitle: ${json(p)}\nconfig:\n  sequence:\n    messageAlign: ${json(p)}\n    noteAlign: ${json(p)}\n---\nsequenceDiagram\n  A->>B: x`,
  (p) =>
    `%%{init: {"sequence": {"width": ${json(p)}, "actorMargin": ${json(p)}, "mirrorActors": ${json(p)}, "messageFontFamily": ${json(p)}}, "themeVariables": {"fontFamily": ${json(p)}}}}%%\nsequenceDiagram\n  A->>B: x`,
];

describe('inert sequence output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('sequence', /sequenceDiagram/i)) check(src);
  }, 120_000);

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(900);
  }, 120_000);

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      const src =
        'sequenceDiagram\n  accTitle: t\n  accDescr: d\n  autonumber\n  A->>+B: x\n  Note over A: n\n  B-->>-A: y';
      expect(check(src, { idPrefix: payload, fontFamily: payload })).toBe(true);
      expect(
        check(src, { config: { sequence: { messageAlign: payload, width: payload, mirrorActors: payload } } })
      ).toBe(true);
    }
  }, 120_000);

  it('holds for extreme numbers in the config', () => {
    for (const value of [NaN, Infinity, -Infinity, -1, 0, 1e308, 1e-308, '12', null]) {
      const sequence = {
        width: value,
        height: value,
        actorMargin: value,
        messageMargin: value,
        noteMargin: value,
        boxMargin: value,
      };
      const src =
        'sequenceDiagram\n  box B\n  participant A\n  end\n  A->>C: x\n  Note over A,C: n\n  loop l\n  C->>C: y\n  end';
      expect(check(src, { config: { sequence } as never })).toBe(true);
    }
    expect(
      check('sequenceDiagram\n  autonumber ' + '9'.repeat(400) + ' ' + '9'.repeat(400) + '\n  A->>B: x\n  B->>A: y')
    ).toBe(true);
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('sequence', /sequenceDiagram/i);
    const fragments = [
      ...PAYLOADS,
      '"',
      ':',
      '\n',
      '@{ "type": "',
      '" }',
      ' as ',
      'rect ',
      'box ',
      'links A: {"a": "',
      'properties A: {"class": "',
      'Note over A: ',
      'title ',
      'end\n',
      'loop ',
      '<br/>',
      'wrap: ',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('never emits a link for participant menus', () => {
    const { svg } = render(
      'sequenceDiagram\n  participant A\n  link A: x @ javascript:alert(1)\n  links A: {"y": "javascript:alert(1)"}\n  A->>B: hi',
      options
    );
    expect(svg).not.toContain('javascript');
    expect(svg).not.toContain('<a');
  });

  it('passes colors through the style filter', () => {
    const { svg } = render(
      'sequenceDiagram\n  rect url(javascript:alert(1))\n  A->>B: hi\n  end\n  rect red\n  B->>A: yo\n  end',
      options
    );
    expect(svg).not.toContain('url(');
    expect(svg).toContain('style="rx:var(--_r);fill:red;"');
  });

  it('does not pollute prototypes', () => {
    render(
      'sequenceDiagram\n  participant __proto__\n  participant constructor\n  __proto__->>constructor: toString\n  Note over __proto__,constructor: hasOwnProperty',
      options
    );
    render(
      'sequenceDiagram\n  participant A@{ "__proto__": "polluted", "constructor": 1, "type": "__proto__", "alias": "constructor" }\n  A->>A: x',
      options
    );
    render(
      'sequenceDiagram\n  participant A\n  links A: {"__proto__": {"polluted": true}}\n  properties A: {"__proto__": {"polluted": true}, "constructor": {"prototype": {"polluted": true}}}',
      options
    );
    const model = parse('sequenceDiagram\n  participant A\n  links A: {"__proto__": "x", "toString": "y"}');
    if (model.type !== 'sequence') throw new Error('not a sequence diagram');
    expect([...model.actors.get('A')!.links.keys()]).toEqual(['__proto__', 'toString']);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, 'x')).toBe(false);
  });
});
