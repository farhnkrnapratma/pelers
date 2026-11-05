import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, peleParse, peleTokens } from '../support/c4-oracle.js';

// Compares Pele's C4 lexer and parser with the parser Mermaid generates from c4Diagram.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '  ',
  '\n',
  '\r\n',
  '\t',
  ',',
  ',,',
  ', ',
  '(',
  ')',
  '( ,',
  '(,',
  '()',
  '{',
  '}',
  ' {',
  '{\n',
  '\n{',
  '}\n',
  '"',
  '""',
  ' ""',
  ' "',
  '$',
  ' $',
  '=',
  '="',
  '= "',
  '$a="b"',
  '$link="x"',
  '$tags="t"',
  '$offsetX="5"',
  '$=""',
  '$x=',
  '#',
  ';',
  '%%',
  '%% c\n',
  '%%{',
  'a',
  'b',
  'x1',
  '"l"',
  'é',
  'title ',
  'title\n',
  'title x',
  'accDescription ',
  'accDescription d',
  'accTitle: ',
  'accTitle',
  'accDescr: ',
  'accDescr',
  'accDescr {',
  'accDescr { d }',
  'direction',
  'direction TB',
  'direction  BT',
  'direction\nRL',
  'direction LR',
  'C4Context',
  'C4Container',
  'C4Component',
  'C4Dynamic',
  'C4Deployment',
  'Person',
  'Person_Ext',
  'Person(',
  'System',
  'SystemDb',
  'SystemQueue',
  'System_Ext',
  'SystemDb_Ext',
  'SystemQueue_Ext',
  'Container',
  'ContainerDb',
  'ContainerQueue',
  'Container_Ext',
  'ContainerDb_Ext',
  'ContainerQueue_Ext',
  'Component',
  'ComponentDb',
  'ComponentQueue',
  'Component_Ext',
  'ComponentDb_Ext',
  'ComponentQueue_Ext',
  'Boundary',
  'Boundary(b, "B") {\n',
  'Enterprise_Boundary',
  'System_Boundary',
  'Container_Boundary',
  'Deployment_Node',
  'Node',
  'Node_L',
  'Node_R',
  'Node(n, "N")\n{',
  'Rel',
  'Rel(a, b, "l")',
  'BiRel',
  'Rel_Up',
  'Rel_U',
  'Rel_Down',
  'Rel_D',
  'Rel_Left',
  'Rel_L',
  'Rel_Right',
  'Rel_R',
  'Rel_Back',
  'RelIndex',
  'RelIndex(1, a, b, "l")',
  'UpdateElementStyle',
  'UpdateRelStyle',
  'UpdateLayoutConfig',
  'UpdateLayoutConfig("3", "1")',
  '_',
  'Ext',
  '__proto__',
  '$__proto__="x"',
];

// Whole statements and the separators between them, to build documents that reach deep into the grammar.
const STATEMENTS = [
  'title T',
  'accTitle: T',
  'accDescr: D',
  'accDescr {\n d\n}',
  'accDescription D',
  'Person(a, "A")',
  'Person_Ext(a, "A", "d")',
  'System(b, "B", $link="l")',
  'SystemDb(c, , "d")',
  'Container(d, "D", "t", "d", $tags="x")',
  'Component(e,"E")',
  'Rel(a, b, "x")',
  'BiRel(a, b, "x", "t")',
  'RelIndex(1, a, b, "x")',
  'Rel_Back(a,b,"x")',
  'UpdateLayoutConfig("2")',
  'UpdateElementStyle(a, $bgColor="red")',
  'UpdateRelStyle(a, b, "red", $offsetX="1")',
  'Person(a, unquoted)',
  'Person(a)',
  'Boundary(g, "G") {',
  'Enterprise_Boundary(g, "G")\n{',
  'System_Boundary(g, "G")\n  {\n',
  'Container_Boundary(g, "G"){',
  'Deployment_Node(n, "N") {',
  'Node_L(n, "N", "t")\n{',
  'Node_R(n, "N") {',
  'Boundary(g, "G")',
  '{',
  '}',
  '}',
  '}',
];
const SEPARATORS = ['\n', '\n', '\n', '\n\n', '\n  ', ' \n', ' ', '', '\r\n', '\n%% c\n', ' %% c\n', ';'];
const HEADERS = ['C4Context', 'C4Container', 'C4Component', 'C4Dynamic', 'C4Deployment'];

function generator(rnd: () => number): () => string {
  const pick = <X>(list: X[]): X => list[Math.floor(rnd() * list.length)];
  return () => {
    let src = pick(HEADERS) + pick(SEPARATORS);
    const count = 1 + Math.floor(rnd() * 8);
    for (let i = 0; i < count; i++) src += pick(STATEMENTS) + pick(SEPARATORS);
    return src;
  };
}

const corpus = loadCorpus('c4', /C4(?:Context|Container|Component|Dynamic|Deployment)/);

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('C4 parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(40);
  });

  it('agrees on every spec and documentation input', () => {
    for (const src of corpus) {
      compare(src);
      compare(src + '\n');
    }
  });

  it('agrees on generated documents', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = generator(random(Number(process.env.SEED ?? 7)));
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      const src = next();
      compare(src);
      if (oracleParse(src).ok) accepted++;
    }
    expect(accepted).toBeGreaterThan(count / 50);
  }, 600_000);

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) compare(src);
    }
  }, 600_000);
});
