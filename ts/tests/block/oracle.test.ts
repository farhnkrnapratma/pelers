import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/block-oracle.js';

// Compares Pelers's block lexer and parser with the parser Mermaid generates from block.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '\n',
  '\t',
  '\r\n',
  ' ',
  'block',
  'block-beta',
  'block:',
  'block:g',
  'block\n',
  'end',
  'end\n',
  ' end ',
  'columns 3',
  'columns auto',
  'columns ',
  'columns 0',
  'columns\n2',
  'space',
  'space:2',
  'space:',
  ':2',
  ':',
  ':03',
  '-->',
  '---',
  '--',
  '--x',
  '--o',
  'x--',
  'o--',
  '<--',
  '<-->',
  '==>',
  '===',
  '==',
  '<==>',
  '-.->',
  '-.-',
  '-.',
  '.->',
  '<-.->',
  '-..-',
  '~~~',
  '->',
  '-',
  '=',
  '.',
  ' -- "x" --> ',
  ' == "x" ==> ',
  ' -. "x" .-> ',
  '"',
  '`',
  '"`',
  '`"',
  '"x"',
  '""',
  '["x"]',
  '("x")',
  '(("x"))',
  '((("x")))',
  '{"x"}',
  '{{"x"}}',
  '(["x"])',
  '[["x"]]',
  '[("x")]',
  '>"x"]',
  '[/"x"/]',
  '[\\"x"\\]',
  '[/"x"\\]',
  '[\\"x"/]',
  '(-"x"-)',
  '[|"x"]',
  '<["x"]>(right)',
  '<["x"]>(up, down)',
  '<["x"]>( x )',
  '<["x"]>(x,y)',
  '<[',
  ']>',
  ']>(',
  '(',
  ')',
  '((',
  '))',
  '(((',
  ')))',
  '[',
  ']',
  '[[',
  ']]',
  '{',
  '}',
  '{{',
  '}}',
  '([',
  '])',
  '[(',
  ')]',
  '[/',
  '/]',
  '[\\',
  '\\]',
  '(-',
  '-)',
  '>',
  '<',
  '[|',
  ',',
  ', ',
  'right',
  'left',
  'up',
  'down',
  'x',
  'y',
  'o',
  'A',
  'B',
  'id1',
  'a b',
  '1',
  '23',
  'é',
  ';',
  'classDef ',
  'classDef a fill:#f96',
  'classDef DEFAULT x',
  'classDef a\n',
  'class ',
  'class A a',
  'class A,B a',
  'class A, B a',
  'style ',
  'style A fill:#f96,stroke:#333',
  'style A,B x',
  'default',
  'linkStyle',
  'interpolate',
  'accTitle: t',
  'accDescr: d',
  'accDescr { d }',
  'accDescr {}',
  'accTitle',
  '%%',
  '#',
  '&nbsp;',
  '__proto__',
  'constructor',
];

const corpus = loadCorpus('block', /block/);

function compare(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
  else expect(actual.error, `error for ${JSON.stringify(src)}`).toMatch(/^(Parse|Lexical) error on line \d+/);
}

describe('block parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(40);
  });

  it('agrees on every spec and documentation input', () => {
    for (const src of corpus) {
      compare(src);
      compare(src + '\n');
    }
  });

  it('agrees on statements with several links, apart from keeping every link', () => {
    for (const src of [
      'block\n A --> B --> C',
      'block\n A --> B --> C --> D\n E',
      'block\n A["a"] -- "x" --> B(("b")) ==> C\n D --> E\n',
      'block\n block:A --> B --> C\n D\n end\n E --> F --> G',
      'block\n block\n A --> B --> C\n end',
    ]) {
      compare(src);
    }
  });

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) compare(src);
    }
  }, 600_000);
});
