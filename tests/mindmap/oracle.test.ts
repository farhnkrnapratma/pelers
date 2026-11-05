import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, peleParse, peleTokens } from '../support/mindmap-oracle.js';
import { failure } from '../support/outline-oracle.js';

// Compares Pele's mindmap lexer and parser with the parser Mermaid generates from mindmap.jison:
// the same tokens, the same accept or reject decision, the same calls into the model, and on
// rejection the same kind of error. Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '  ',
  '    ',
  '\t',
  '\n',
  '\n\n',
  ' \n',
  '\n ',
  '\r\n',
  '\r',
  ' ',
  ' ',
  'mindmap',
  'MindMap',
  'mindmap\n',
  'mindmapx',
  '%%',
  ' %% c',
  '%%{',
  ':::',
  ':::a b',
  '::icon(',
  '::ICON(fa fa-x)',
  '::',
  ':',
  '(',
  ')',
  '((',
  '))',
  '[',
  ']',
  '{{',
  '}}',
  '{',
  '}',
  '(-',
  '-)',
  '-',
  '"',
  '`',
  '"`',
  '`"',
  '""',
  '"a"',
  '["a"]',
  '["`a`"]',
  '(a)',
  '((a))',
  '))a((',
  ')a(',
  '{{a}}',
  '[a]',
  '(a]',
  '[a)',
  'id',
  'root',
  'A',
  'é',
  '🤓',
  '@',
  '@{',
  '^',
  '<br/>',
  '**b**',
  '#',
  ';',
  ',',
  '.',
];

const corpus = loadCorpus('mindmap', /mindmap/);

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
  if (!expected.ok) expect(failure(actual.error), `error of ${JSON.stringify(src)}`).toBe(failure(expected.error));
}

describe('mindmap parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(30);
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
