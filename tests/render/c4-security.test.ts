import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };
const corpus = loadCorpus('c4', /C4(?:Context|Container|Component|Dynamic|Deployment)/);

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

const TWO = 'System(a, "A")\nSystem(b, "B")\n';

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `C4Context\nPerson(${p}, "L")`,
  (p) => `C4Context\nPerson("${p}", "L")`,
  (p) => `C4Context\nPerson(a, ${p}, "d")`,
  (p) => `C4Context\nPerson(a, "${p}")`,
  (p) => `C4Context\nPerson_Ext(a, "L", "${p}")`,
  (p) => `C4Context\nSystem(a, "L", "d", "${p}", "${p}", "${p}")`,
  (p) => `C4Context\nSystemDb(a, "L", $link="${p}")`,
  (p) => `C4Context\nSystemQueue_Ext(a, "L", $tags="${p}", $sprite="${p}")`,
  (p) => `C4Context\nSystem(a, "L", $descr="${p}", $techn="${p}", $type="${p}", $label="${p}")`,
  (p) => `C4Context\nSystem(a, "L", $bgColor="${p}", $fontColor="${p}", $borderColor="${p}", $shape="${p}")`,
  (p) => `C4Context\nSystem(a, "L", $${p}="x")`,
  (p) => `C4Context\nSystem($${p}="x", "L")`,
  (p) => `C4Context\nSystem(a, $link="${p}")`,
  (p) => `C4Container\nContainer(a, "${p}", "${p}", "${p}")`,
  (p) => `C4Container\nContainerDb_Ext(a, "L", "${p}", "d", "s", "t", "${p}")`,
  (p) => `C4Component\nComponentQueue(a, "L", ${p}, ${p})`,
  (p) => `C4Context\nBoundary(${p}, "L") {\nSystem(a, "A")\n}`,
  (p) => `C4Context\nBoundary(b, "${p}", "${p}") {\nSystem(a, "A")\n}`,
  (p) => `C4Context\nEnterprise_Boundary(b, "L", "${p}", "${p}") {\nSystem(a, "A")\n}`,
  (p) => `C4Context\nSystem_Boundary(b, "L", $link="${p}", $tags="${p}", $type="${p}") {\nSystem(a, "A")\n}`,
  (p) =>
    `C4Container\nContainer_Boundary(b, "L", $bgColor="${p}", $borderColor="${p}", $fontColor="${p}") {\nSystem(a, "A")\n}`,
  (p) => `C4Deployment\nDeployment_Node(n, "${p}", "${p}", "${p}", "${p}", "${p}", "${p}") {\nContainer(a, "A")\n}`,
  (p) => `C4Deployment\nNode_L(n, "L", $descr="${p}") {\nNode_R(m, "M", $link="${p}")\n{\nContainer(a, "A")\n}\n}`,
  (p) => `C4Context\n${TWO}Rel(a, b, "${p}", "${p}", "${p}")`,
  (p) => `C4Context\n${TWO}Rel(a, b, ${p})`,
  (p) => `C4Context\n${TWO}BiRel(a, b, "L", "t", "d", "${p}", "${p}", "${p}")`,
  (p) => `C4Context\n${TWO}Rel_Back(a, b, "L", $link="${p}", $techn="${p}", $descr="${p}")`,
  (p) => `C4Context\nSystem(${p}, "A")\nSystem(b, "B")\nRel(${p}, b, "L")`,
  (p) => `C4Context\nSystem(a, "A")\nRel(a, a, "${p}", "${p}")`,
  (p) => `C4Dynamic\n${TWO}RelIndex(${p}, a, b, "${p}")`,
  (p) => `C4Context\n${TWO}Rel(a, b, "L")\nRel(b, a, "${p}")`,
  (p) => `C4Context\n${TWO}Rel(a, b, "L")\nUpdateRelStyle(a, b, "${p}", "${p}", "${p}", "${p}")`,
  (p) =>
    `C4Context\n${TWO}Rel(a, b, "L")\nUpdateRelStyle(a, b, $textColor="${p}", $lineColor="${p}", $offsetX="${p}", $offsetY="${p}")`,
  (p) => `C4Context\n${TWO}Rel(a, b, "L")\nUpdateRelStyle(a, b, $offsetX="${p}", $${p}="1")`,
  (p) =>
    `C4Context\n${TWO}UpdateElementStyle(a, "${p}", "${p}", "${p}", "${p}", "${p}", "${p}", "${p}", "${p}", "${p}")`,
  (p) => `C4Context\n${TWO}UpdateElementStyle(a, $bgColor="${p}", $fontColor="${p}", $borderColor="${p}")`,
  (p) => `C4Context\n${TWO}UpdateElementStyle(a, $shape="${p}", $techn="${p}", $legendText="${p}", $label="${p}")`,
  (p) => `C4Context\n${TWO}UpdateLayoutConfig("${p}", "${p}")`,
  (p) => `C4Context\n${TWO}UpdateLayoutConfig($c4ShapeInRow="${p}", $c4BoundaryInRow="${p}")`,
  (p) => `C4Context\ntitle ${p}\n${TWO}`,
  (p) => `C4Context\naccTitle: ${p}\naccDescr: ${p}\n${TWO}`,
  (p) => `C4Context\naccDescr {\n ${p}\n }\n${TWO}`,
  (p) => `C4Context\naccDescription ${p}\n${TWO}`,
  (p) => `C4Context ${p}\n${TWO}`,
  (p) => `C4Context\n%% ${p}\n${TWO}`,
  (p) => `---\ntitle: "${p}"\nconfig:\n  c4:\n    c4ShapeInRow: "${p}"\n    wrap: "${p}"\n---\nC4Context\n${TWO}`,
  (p) => `%%{init: {"c4": {"c4ShapeInRow": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\nC4Context\n${TWO}`,
];

describe('inert C4 output', () => {
  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(20);
  }, 60_000);

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(700);
  }, 60_000);

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(
        check('C4Context\naccDescr: d\nSystem(a, "A")\nSystem(b, "B")\nRel(a, b, "x")', {
          idPrefix: payload,
          fontFamily: payload,
        })
      ).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const next = mutator(
      corpus,
      [
        ...PAYLOADS,
        '"',
        '(',
        ')',
        ',',
        '\n',
        '{',
        '}',
        '$link="',
        '$bgColor="',
        '$offsetX="',
        'Rel(a, b, "',
        'UpdateElementStyle(a, "',
      ],
      random(3)
    );
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('never emits an unsafe link', () => {
    for (const url of [
      'javascript:alert(1)',
      ' JaVaScRiPt:alert(1)',
      'data:text/html,x',
      'vbscript:x',
      'jav&#x09;ascript:alert(1)',
    ]) {
      const { svg, links } = render(
        `C4Context\nSystem(a, "A", $link="${url}")\nBoundary(b, "B", $link="${url}") {\nSystem(c, "C")\n}\nRel(a, c, "x", $link="${url}")`,
        options
      );
      expect(svg).not.toContain('<a ');
      expect(links).toEqual([]);
    }
  });

  it('drops colours that are more than a colour', () => {
    const { svg } = render(
      `C4Context\nSystem(a, "A")\nSystem(b, "B")\nRel(a, b, "x")\nUpdateElementStyle(a, "red;background:url(//evil.example/x)", "expression(alert(1))", "url(#x)")\nUpdateRelStyle(a, b, "red}", "blue<")`,
      options
    );
    expect(svg).not.toMatch(/url\(|expression|evil|red|blue/);
  });

  it('does not pollute prototypes', () => {
    render(
      'C4Context\nSystem(__proto__, "A", $__proto__="polluted", $constructor="x")\nSystem(constructor, "B", $tags="__proto__,constructor", $shape="toString")\nRel(__proto__, constructor, "x")\nUpdateElementStyle(__proto__, $prototype="x")\nUpdateRelStyle(__proto__, constructor, $__proto__="1")',
      options
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, 'x')).toBe(false);
  });
});
