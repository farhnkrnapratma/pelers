import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, peleParse, peleTokens } from '../support/er-oracle.js';

// Compares Pele's ER lexer and parser with the parser Mermaid generates from erDiagram.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of mutated inputs and SEED to change them.

const FRAGMENTS = [
  ' ',
  '\n',
  '\t',
  '\r\n',
  ' \n',
  ';',
  ':',
  ':::',
  ',',
  '#',
  '"',
  '""',
  '`',
  '``',
  '~',
  '{',
  '}',
  '[',
  ']',
  '(',
  ')',
  '?',
  '*',
  '.',
  '-',
  '_',
  '|',
  '%',
  '\\',
  '||',
  '|o',
  'o|',
  '}o',
  'o{',
  '}|',
  '|{',
  '--',
  '..',
  '.-',
  '-.',
  'u',
  'u--',
  'u..',
  '||--o{',
  '}|..|{',
  '|o--o|',
  ' 1 ',
  '1',
  '1+',
  '0+',
  '1.5',
  '12',
  '0',
  '7',
  '2',
  '4',
  '6',
  '9',
  ' one ',
  ' only one ',
  ' one or zero ',
  ' one or more ',
  ' one or many ',
  ' zero or one ',
  ' zero or more ',
  ' zero or many ',
  ' many(0) ',
  ' many(1) ',
  ' many ',
  ' to ',
  ' optionally to ',
  'one',
  'only',
  'zero',
  'or',
  'many',
  'to',
  'optionally',
  'A',
  'B',
  'id1',
  'CUSTOMER',
  'LINE-ITEM',
  'é',
  'µ',
  'ö',
  '\u00a0',
  '\u2028',
  'PK',
  'FK',
  'UK',
  'pk',
  ' PK, FK ',
  'string',
  'int',
  'type~T~',
  'List~int~',
  ' "comment" ',
  '"a b"',
  '"a%b"',
  'erDiagram',
  'ERDIAGRAM',
  'style ',
  'style',
  'classDef ',
  'classDef',
  'class ',
  'class',
  'subgraph ',
  'subgraph',
  'end',
  'end\n',
  'END ',
  'direction TB',
  'direction BT',
  'direction RL',
  'direction LR',
  'direction',
  'DIRECTION\ntb',
  'accTitle: ',
  'accDescr: ',
  'accDescr { ',
  'acctitle:',
  'title',
  'fill:#f9f',
  'stroke-width:2px',
  'color:red',
  'default',
  ' : ',
  ' : label\n',
  '{\n',
  '\n}\n',
  'A { string b }',
  'p[Person]',
  'a["x y"]',
  '%%',
  '\v',
  '\f',
];

// The specs build most of their inputs from template placeholders, so these cover the constructs directly.
const SEEDS = [
  'erDiagram\nCUSTOMER ||--o{ ORDER : places\nORDER ||--|{ LINE-ITEM : contains\n',
  'erDiagram\nA |o--o| B : "a b"\nC }o..o{ D : ""\nE }|.-|{ F : x\nG ||-.|| H : y\nI u--o{ J : z\n',
  'erDiagram\nA one or zero to one or more B : r\nC zero or many optionally to many(1) D : r\nE only one to 1+ F : r\n',
  'erDiagram\nA 1 to zero or more B : r\nC many(0) optionally to 0+ D : r\nE 1--1 F : r\nG 1..many H : r\nx 1--1 12 : r\n',
  'erDiagram\nBOOK {\n  string title PK "The title"\n  string? subtitle\n  int *pages UK\n  List~string~ tags FK, UK "many"\n  `my type` `my name` PK,FK\n}\n',
  'erDiagram\nA{}\nB { }\nC {string name}\nD:::x { int id PK }\nE:::x,y\nF:::x {}\n',
  'erDiagram\np[Person] {\n string firstName\n}\na["Customer Account"]:::big {\n string email\n}\nb[B]\nc[C]:::k\nd[D] {}\ne[E]:::k {}\np ||--o| a : has\n',
  'erDiagram\nA:::x ||--o{ B:::y,z : rel\nC:::x ||--o{ D : rel\nE ||--o{ F:::y : "r"\n',
  'erDiagram\nA\nB\nstyle A fill:#f9f,stroke:#333,stroke-width:4px\nstyle A,B color:red\nclassDef big,small font-size:12pt\nclassDef default fill:#f9f;\nclass A big\nclass A,B big,small\n',
  'erDiagram\ndirection LR\nsubgraph one1\n A\n B ||--|| C : x\n subgraph "Two Words"\n  direction TB\n  D { int id }\n end\n subgraph s3 [Title of three]\n  E\n end\nend\none1 ||--|| E : links\nstyle one1 fill:red\nclass s3 big\n',
  'erDiagram\naccTitle: A title\naccDescr: A description\nA\n',
  'erDiagram\naccDescr {\n several\n lines\n}\nsubgraph g\naccTitle: inside\nclassDef c fill:red\nstyle A fill:red\nclass A c\nA\nend\n',
  'erDiagram\n"Quoted name" ||--|| "Other ❤ name" : "a label"\n1.5 ||--|| 2 : n\n_x ||--|| y-z : n\nA.B ||--|| C*D : n\n',
  'erDiagram A B C\nD { a b c d } E\n',
];

const corpus = [...new Set([...loadCorpus('er', /erDiagram/), ...SEEDS])];

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('ER parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(100);
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
  }, 1_200_000);

  // Mutation rarely joins two tokens without a space, which is where keyword boundaries and
  // lookahead matter. This strings random pieces together, in each lexer state.
  it('agrees on random sequences of tokens', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 1);
    const pieces = [...new Set([...FRAGMENTS, ...FRAGMENTS.map((f) => f.trim()).filter((f) => f !== '')])];
    const pick = (): string => pieces[Math.floor(rnd() * pieces.length)];
    const openers = [
      'erDiagram\n',
      'erDiagram\nA {\n',
      'erDiagram\nstyle A ',
      'erDiagram\nA ',
      'erDiagram\nsubgraph s\n',
      '',
    ];
    for (let i = 0; i < count; i++) {
      let src = openers[Math.floor(rnd() * openers.length)];
      for (let k = 1 + Math.floor(rnd() * 10); k > 0; k--) src += pick() + (rnd() < 0.3 ? ' ' : '');
      compare(src);
    }
  }, 1_200_000);
});
