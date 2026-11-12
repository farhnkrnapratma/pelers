import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { RequirementDb } from '../../src/diagrams/requirement/db.js';
import { renderRequirement } from '../../src/diagrams/requirement/render.js';
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

const BODY = 'requirement a {\nid: 1\n}\nelement b {\ntype: t\n}\n';

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `requirementDiagram\nrequirement ${p} {\nid: 1\n}\n`,
  (p) => `requirementDiagram\nrequirement "${p}" {\nid: 1\n}\n`,
  (p) => `requirementDiagram\nelement ${p} {\ntype: t\n}\n`,
  (p) => `requirementDiagram\nelement "${p}" {\ntype: t\n}\n"${p}" - contains -> "${p}"\n`,
  (p) => `requirementDiagram\nrequirement a {\nid: ${p}\ntext: ${p}\n}\n`,
  (p) => `requirementDiagram\nrequirement a {\nid: "${p}"\ntext: "${p}"\n}\n`,
  (p) => `requirementDiagram\nrequirement a {\nrisk: ${p}\nverifymethod: ${p}\n}\n`,
  (p) => `requirementDiagram\nelement b {\ntype: ${p}\ndocref: ${p}\n}\n`,
  (p) => `requirementDiagram\nelement b {\ntype: "${p}"\ndocRef: "${p}"\n}\n`,
  (p) => `requirementDiagram\n${BODY}a - ${p} -> b\n`,
  (p) => `requirementDiagram\n${BODY}a - contains -> ${p}\n`,
  (p) => `requirementDiagram\n${BODY}"${p}" <- traces - a\n`,
  (p) => `requirementDiagram\nrequirement a:::${p} {\nid: 1\n}\n`,
  (p) => `requirementDiagram\nrequirement a:::"${p}" {\nid: 1\n}\nelement b:::"${p}","${p}" {\n}\n`,
  (p) => `requirementDiagram\n${BODY}a:::${p}\n`,
  (p) => `requirementDiagram\n${BODY}a:::"${p}"\n`,
  (p) => `requirementDiagram\n${BODY}class a ${p}\n`,
  (p) => `requirementDiagram\n${BODY}class a,b "${p}"\n`,
  (p) => `requirementDiagram\n${BODY}class "${p}" c\n`,
  (p) => `requirementDiagram\n${BODY}classDef ${p} fill:red\nclass a ${p}\n`,
  (p) => `requirementDiagram\n${BODY}classDef "${p}" fill:red\nclass a "${p}"\n`,
  (p) => `requirementDiagram\n${BODY}classDef c fill:${p}\nclass a c\n`,
  (p) => `requirementDiagram\n${BODY}classDef c ${p}:red\nclass a c\n`,
  (p) => `requirementDiagram\n${BODY}style a fill:${p}\n`,
  (p) => `requirementDiagram\n${BODY}style a ${p}\n`,
  (p) => `requirementDiagram\n${BODY}style a color:${p},stroke:${p},font-family:${p}\n`,
  (p) => `requirementDiagram\n${BODY}style "${p}" fill:red\n`,
  (p) => `requirementDiagram\naccTitle: ${p}\naccDescr: ${p}\n${BODY}`,
  (p) => `accTitle: ${p}\nrequirementDiagram\n${BODY}`,
  (p) => `requirementDiagram\naccDescr {\n ${p}\n }\n${BODY}`,
  (p) => `requirementDiagram\ndirection ${p}\n${BODY}`,
  (p) => `requirementDiagram\n${p} direction LR\n${BODY}`,
  (p) => `requirementDiagram\n# ${p}\n${BODY}`,
  (p) => `requirementDiagram\n%% ${p}\n${BODY}`,
  (p) => `requirementDiagram ${p}\n${BODY}`,
  (p) =>
    `---\ntitle: "${p}"\nconfig:\n  requirement:\n    fontSize: "${p}"\n  theme: "${p}"\n---\nrequirementDiagram\n${BODY}`,
  (p) =>
    `%%{init: {"requirement": {"rect_fill": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\nrequirementDiagram\n${BODY}`,
];

describe('inert requirement output', () => {
  const corpus = loadCorpus('requirement', /requirementDiagram/i);

  it('holds for every corpus input', () => {
    for (const src of corpus) check(src);
  }, 120_000);

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(550);
  }, 120_000);

  it('escapes what it shows', () => {
    const { svg } = render(
      'requirementDiagram\nrequirement "<script>alert(1)</script>" {\ntext: "<img src=x onerror=alert(1)>"\n}\n',
      options
    );
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('<img');
    expect(svg).toContain('data-id="&lt;script&gt;alert(1)&lt;/script&gt;"');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(
        check(`requirementDiagram\naccTitle: t\n${BODY}a - contains -> b\n`, { idPrefix: payload, fontFamily: payload })
      ).toBe(true);
    }
  }, 120_000);

  it('holds for a model filled in directly, which the grammar would not allow', () => {
    for (const payload of PAYLOADS) {
      const db = new RequirementDb();
      db.title = payload;
      db.setAccTitle(payload);
      db.setAccDescription(payload);
      db.setDirection(payload);
      db.setNewReqId(payload);
      db.setNewReqText(payload);
      db.setNewReqRisk(payload);
      db.setNewReqVerifyMethod(payload);
      db.addRequirement(payload, payload);
      db.setNewElementType(payload);
      db.setNewElementDocRef(payload);
      db.addElement('e');
      db.addRelationship(payload, payload, 'e');
      db.addRelationship(payload, 'e', 'e');
      db.defineClass(
        [payload],
        [payload, `fill:${payload}`, `stroke:${payload}`, `color:${payload}`, `font-family:${payload}`]
      );
      db.setClass([payload, 'e'], [payload]);
      db.setCssStyle(['e'], [`fill:${payload},stroke-dasharray:${payload}`, `font-size:${payload}`, `${payload}:red`]);
      assertInert(renderRequirement(db, {}, options).svg, JSON.stringify(payload));
    }
  }, 120_000);

  it('holds for mutated inputs', () => {
    const next = mutator(
      corpus,
      [
        ...PAYLOADS,
        '"',
        '{',
        '}',
        ':',
        ':::',
        '\n',
        'style a fill:',
        'classDef c ',
        'class a ',
        ' - contains -> ',
        'accTitle: ',
      ],
      random(3)
    );
    const count = Number(process.env.FUZZ ?? 4000);
    let rendered = 0;
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000 && check(src)) rendered++;
    }
    expect(rendered).toBeGreaterThan(count / 20);
  }, 600_000);

  it('does not pollute prototypes', () => {
    render(
      'requirementDiagram\nrequirement __proto__ {\nid: constructor\n}\nelement constructor {\ntype: toString\n}\n__proto__ - contains -> constructor\nclassDef __proto__ fill:red\nclass __proto__,constructor __proto__,toString\n__proto__:::constructor\nstyle __proto__ fill:red\n',
      options
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, 'default')).toBe(false);
    expect(Object.keys(Object.prototype)).toEqual([]);
  });
});
