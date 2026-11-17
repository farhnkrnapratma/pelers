import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };

function check(src: string, extra: object = {}): boolean {
  let svg: string;
  try {
    svg = render(src, { ...options, ...extra }).svg;
  } catch (error) {
    if (error instanceof PelersError) return false;
    throw new Error(
      `Unexpected ${(error as Error).name} for ${JSON.stringify(src).slice(0, 200)}: ${(error as Error).message}`
    );
  }
  assertInert(svg, JSON.stringify(src).slice(0, 160));
  return true;
}

// Hostile text in every position the class diagram syntax has.
const TEMPLATES: ((p: string) => string)[] = [
  (p) => `classDiagram\n  class ${p}`,
  (p) => `classDiagram\n  class \`${p}\``,
  (p) => `classDiagram\n  class A["${p}"]`,
  (p) => `classDiagram\n  class A~${p}~`,
  (p) => `classDiagram\n  class A:::${p}`,
  (p) => `classDiagram\n  class A <<${p}>>`,
  (p) => `classDiagram\n  <<${p}>> A`,
  (p) => `classDiagram\n  class A {\n    ${p}\n  }`,
  (p) => `classDiagram\n  class A {\n    +${p}()\n    -${p} x$\n    #f(${p}) ${p}*\n    <<${p}>>\n  }`,
  (p) => `classDiagram\n  class A {\n    +List~${p}~ x\n    +f(Map~${p}, ${p}~ m) List~${p}~\n  }`,
  (p) => `classDiagram\n  A : ${p}`,
  (p) => `classDiagram\n  A : +${p}()`,
  (p) => `classDiagram\n  ${p} <|-- B`,
  (p) => `classDiagram\n  \`${p}\` <|-- \`${p}2\``,
  (p) => `classDiagram\n  A <|-- B : ${p}`,
  (p) => `classDiagram\n  A "${p}" --> "${p}" B : ${p}`,
  (p) => `classDiagram\n  A "${p}" --> "${p}" A : ${p}`,
  (p) => `classDiagram\n  ${p} ()-- A`,
  (p) => `classDiagram\n  A --() \`${p}\``,
  (p) => `classDiagram\n  note "${p}"`,
  (p) => `classDiagram\n  class A\n  note for A "${p}"`,
  (p) => `classDiagram\n  note for \`${p}\` "x"`,
  (p) => `classDiagram\n  namespace ${p} {\n    class A\n  }`,
  (p) => `classDiagram\n  namespace \`${p}\` {\n    class A\n  }`,
  (p) => `classDiagram\n  namespace N["${p}"] {\n    class A\n    note "${p}"\n  }`,
  (p) => `classDiagram\n  class A\n  link A "${p}"`,
  (p) => `classDiagram\n  class A\n  link A "${p}" "${p}" _blank`,
  (p) => `classDiagram\n  class A\n  link A "https://example.com" ${p}`,
  (p) => `classDiagram\n  class A\n  click A href "${p}" "${p}"`,
  (p) => `classDiagram\n  class A\n  click A href "x" "${p}" _top`,
  (p) => `classDiagram\n  class A\n  click A call ${p}()`,
  (p) => `classDiagram\n  class A\n  click A call f(${p}) "${p}"`,
  (p) => `classDiagram\n  class A\n  callback A "${p}" "${p}"`,
  (p) => `classDiagram\n  class A\n  style A fill:${p}`,
  (p) => `classDiagram\n  class A\n  style A ${p}:red`,
  (p) => `classDiagram\n  class A\n  style A color:${p},stroke:${p},font-family:${p},font-size:${p}`,
  (p) => `classDiagram\n  class A\n  style ${p} fill:red`,
  (p) => `classDiagram\n  class A:::c\n  classDef c fill:${p},color:${p}`,
  (p) => `classDiagram\n  class A {\n +x$\n}\n  classDef default color:${p},stroke:${p}`,
  (p) => `classDiagram\n  class A\n  classDef ${p} fill:red\n  cssClass "A" ${p}`,
  (p) => `classDiagram\n  class A\n  cssClass "${p}" c`,
  (p) => `classDiagram\n  class A\n  cssClass "A" ${p}`,
  (p) => `classDiagram\n  accTitle: ${p}\n  accDescr: ${p}\n  class A`,
  (p) => `classDiagram\n  accDescr {\n ${p}\n }\n  class A`,
  (p) => `classDiagram\n  direction ${p}\n  class A`,
  (p) => `classDiagram\n  %% ${p}\n  class A`,
  (p) => `classDiagram ${p}\n  class A`,
  (p) =>
    `---\ntitle: "${p}"\nconfig:\n  class:\n    hideEmptyMembersBox: "${p}"\n    nodeSpacing: "${p}"\n    hierarchicalNamespaces: "${p}"\n---\nclassDiagram\n  class A`,
  (p) =>
    `%%{init: {"class": {"rankSpacing": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\nclassDiagram\n  A --> B`,
];

describe('class diagram output is inert', () => {
  it('for hostile text in every syntax position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
        if (check(template(payload.replace(/"/g, '#quot;')))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(1500);
  }, 60_000);

  it('for hostile options', () => {
    for (const payload of PAYLOADS) {
      expect(
        check('classDiagram\n  accTitle: t\n  A "1" --> "2" B : x', { idPrefix: payload, fontFamily: payload })
      ).toBe(true);
      expect(
        check('classDiagram\n  A --> B', { config: { class: { nodeSpacing: payload, rankSpacing: payload } } })
      ).toBe(true);
    }
    for (const value of [NaN, Infinity, -Infinity, -1, 1e308]) {
      expect(
        check('classDiagram\n  A "1" --> "2" B : x', { config: { class: { nodeSpacing: value, rankSpacing: value } } })
      ).toBe(true);
    }
  });

  it('never emits a script URL as a link, nor anything from a callback', () => {
    for (const url of [
      'javascript:alert(1)',
      'JAVASCRIPT:alert(1)',
      ' javascript:alert(1)',
      'java\tscript:alert(1)',
      'data:text/html,x',
      'vbscript:x',
      '&#106;avascript:alert(1)',
    ]) {
      const { svg, links } = render(`classDiagram\n  class A\n  link A "${url}"`, options);
      expect(svg).not.toMatch(/<a\b/);
      expect(links).toEqual([]);
    }
    const { svg, links } = render(
      'classDiagram\n  class A\n  click A call stealCookies("now") "tip"\n  callback A "stealCookies"',
      options
    );
    expect(svg).not.toContain('stealCookies');
    expect(svg).not.toMatch(/<a\b/);
    expect(links).toEqual([]);
  });

  it('for mutated corpus inputs', () => {
    const corpus = loadCorpus('class', /classDiagram/);
    const fragments = [
      ...PAYLOADS,
      '"',
      '`',
      '~',
      '<<',
      '>>',
      ':::',
      '{',
      '}',
      '\n',
      ' : ',
      'link A "',
      'style A fill:',
      'note "',
      'namespace N {\n',
      'classDef c ',
      '()--',
      '"1" ',
    ];
    const next = mutator(corpus, fragments, random(3));
    let rendered = 0;
    for (let i = 0; i < 3000; i++) if (check(next())) rendered++;
    expect(rendered).toBeGreaterThan(300);
  }, 60_000);
});
