import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/flow-oracle.js';

// Compares Pelers's flowchart lexer and parser with the parser Mermaid generates from flow.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '\n',
  ';',
  '-->',
  '---',
  '--',
  '==>',
  '===',
  '==',
  '-.->',
  '-.-',
  '-.',
  '.->',
  '~~~',
  '--x',
  '--o',
  'x--',
  'o--',
  '<--',
  '<-->',
  '|',
  '[',
  ']',
  '(',
  ')',
  '{',
  '}',
  '((',
  '))',
  '[[',
  ']]',
  '([',
  '])',
  '[(',
  ')]',
  '(((',
  ')))',
  '{{',
  '}}',
  '[/',
  '/]',
  '[\\',
  '\\]',
  '>',
  '(-',
  '-)',
  '"',
  '`',
  '"`',
  '`"',
  '@',
  '@{',
  'e1@',
  ':::',
  ':',
  '&',
  ' & ',
  ',',
  '*',
  '#',
  '^',
  'v',
  '-',
  '=',
  '.',
  '~',
  '<',
  '\\|',
  '[|',
  'x',
  'o',
  'A',
  'B',
  'id1',
  '1',
  '23',
  'text',
  'é',
  'ö',
  'style ',
  'classDef ',
  'class ',
  'click ',
  'href ',
  'call ',
  'linkStyle ',
  'interpolate ',
  'default',
  'subgraph ',
  'subgraph',
  'end',
  'end\n',
  'direction TB',
  'direction LR',
  'graph ',
  'flowchart ',
  'TD',
  'LR',
  'TB',
  'BT',
  'RL',
  '_blank',
  '_self',
  'accTitle: ',
  'accDescr: ',
  'accDescr { ',
  'fill:#f9f',
  'stroke-width:2px',
  'shape: rect',
  'label: "x"',
  '%%',
  '\t',
  '\r\n',
  ' \n',
  '}\n',
  'a()',
  '()',
  'foo',
];

const corpus = loadCorpus('flowchart', /graph|flowchart/);

function compare(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('flowchart parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(300);
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
});
