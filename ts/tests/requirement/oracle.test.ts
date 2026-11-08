import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, peleParse, peleTokens } from '../support/requirement-oracle.js';

// Compares Pele's requirement lexer and parser with the parser Mermaid generates from
// requirementDiagram.jison: the same tokens, the same accept or reject decision, and the same
// calls into the model. Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '  ',
  '\n',
  '\n\n',
  ' \n',
  '\r\n',
  '\r',
  '\t',
  '\u2028',
  '\u00a0',
  ';',
  ':',
  ':::',
  '::',
  ',',
  ', ',
  '{',
  '}',
  '{\n',
  '}\n',
  '-',
  '->',
  '<-',
  '<',
  '>',
  '=',
  '"',
  '""',
  '"a b"',
  '#',
  '%',
  '%%',
  '# c',
  '.',
  '(',
  ')',
  '_',
  'é',
  'ö',
  '1',
  '1.2',
  '42',
  'requirementDiagram',
  'requirementDiagram\n',
  'RequirementDiagram',
  'requirement ',
  'functionalRequirement ',
  'interfaceRequirement ',
  'performanceRequirement ',
  'physicalRequirement ',
  'designConstraint ',
  'element ',
  'ELEMENT ',
  'id',
  'id: ',
  'ID: ',
  'text',
  'text: ',
  'risk',
  'risk: ',
  'verifymethod: ',
  'verifyMethod',
  'type',
  'type: ',
  'docref: ',
  'docRef',
  'low',
  'medium',
  'high',
  'High',
  'analysis',
  'demonstration',
  'inspection',
  'test',
  'Test',
  'contains',
  'copies',
  'derives',
  'satisfies',
  'verifies',
  'refines',
  'traces',
  ' - contains -> ',
  ' <- copies - ',
  'style ',
  'style',
  'classDef ',
  'classdef',
  'class ',
  'class',
  'fill:#f9f',
  'stroke-width:2px',
  'color: red',
  'font-weight:bold',
  '50%',
  'direction TB',
  'direction BT',
  'direction RL',
  'direction LR',
  'direction',
  'direction\nTB',
  'DIRECTION lr',
  'direction  TB x',
  'title x',
  'title ',
  'Title\nx',
  'accTitle: ',
  'accTitle',
  'accDescr: ',
  'accDescr { ',
  'accDescr{',
  'acc',
  'a',
  'b',
  'test_req',
  'test_entity',
  'foo bar',
  '__proto__',
  'constructor',
  'x:::y',
  'a,b',
  '*x*',
  '**y**',
];

const corpus = [
  ...loadCorpus('requirement', /requirementDiagram/i),
  'requirementDiagram\n\nrequirement a {\nid: 1\ntext: "t"\nrisk: low\nverifymethod: test\n}\nelement e {\ntype: x\ndocref: y\n}\ne - satisfies -> a\na <- traces - e\n',
  'accTitle: t\naccDescr: d\nrequirementDiagram\naccDescr {\n multi\n line\n}\nrequirement "a b":::c1, c2 {\n}\n',
  'requirementDiagram\ndirection LR\nstyle a,b fill:#f9f,stroke:#333;stroke-width:4px\nclassDef c1,c2 color: red, font-weight:bold\nclass a,b c1,c2\na:::c1\n',
  'requirementDiagram\nelement "e 1":::x {\ntype: "t t"\ndocRef: "d/e"\n}\n"e 1" - contains -> "e 1"\n',
  'requirementDiagram\n\nrequirement r {\n}\nelement e {\n}\nclassDef c1,c2 fill:#f9f,stroke:#333,stroke-width:4px\nclass r,e c1,c2\nstyle r,e fill:#f9f,stroke:#333,stroke-width:4px\n',
  'requirementDiagram\n\nrequirement r:::c1 {\n}\nelement e:::c1,c2 {\n}\n\nclassDef c1 fill:#f9f\nclassDef c2 color:blue\nr:::c1,c2\n',
  // The specs build their documents line by line, so the same shapes are spelled out here.
  ...[
    'requirement',
    'functionalRequirement',
    'interfaceRequirement',
    'performanceRequirement',
    'physicalRequirement',
    'designConstraint',
  ].flatMap((type, k) => [
    `requirementDiagram\n\n${type} test_req {\nid: test_id\ntext: the test text.\nrisk: ${['low', 'medium', 'high'][k % 3]}\nverifymethod: ${
      ['analysis', 'demonstration', 'inspection', 'test'][k % 4]
    }\n}`,
  ]),
  ...['contains', 'copies', 'derives', 'satisfies', 'verifies', 'refines', 'traces'].flatMap((type) => [
    `requirementDiagram\n\na - ${type} -> b`,
    `requirementDiagram\n\na <- ${type} - b\n`,
  ]),
  ...['TB', 'BT', 'LR', 'RL'].map((dir) => `requirementDiagram\n\ndirection ${dir}\n`),
];

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('requirement parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(40);
  });

  it('agrees on every spec and documentation input', () => {
    for (const src of corpus) compare(src);
  });

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) compare(src);
    }
  }, 600_000);

  it('agrees on inputs assembled from fragments', () => {
    const count = Number(process.env.FUZZ ?? 5000) / 2;
    const rnd = random(Number(process.env.SEED ?? 7) + 1);
    for (let i = 0; i < count; i++) {
      let src = rnd() < 0.7 ? 'requirementDiagram\n' : '';
      for (let k = 1 + Math.floor(rnd() * 14); k > 0; k--) src += FRAGMENTS[Math.floor(rnd() * FRAGMENTS.length)];
      compare(src);
    }
  }, 600_000);
});
