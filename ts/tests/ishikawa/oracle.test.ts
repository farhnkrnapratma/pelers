import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/ishikawa-oracle.js';

// Compares Pelers's Ishikawa lexer and parser with the parser Mermaid generates from ishikawa.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of generated inputs and SEED to change them.

const FRAGMENTS = [
  ' ',
  '  ',
  '    ',
  '\t',
  '\n',
  '\n\n',
  '\r',
  '\r\n',
  ' \n',
  '\n ',
  '\n  \n',
  '\n\t\n  ',
  ' ',
  ' ',
  ' ',
  '﻿',
  '\v',
  '\f',
  '%',
  '%%',
  '%% c',
  ' %% c',
  '\n%% c',
  '\n  %% c\n',
  'x %% y',
  '%%\r',
  '%% x',
  'a',
  'Cause',
  'Sub cause',
  'A b c',
  '  indented',
  '\n    deeper',
  'ishikawa',
  'ishikawa-beta',
  'ISHIKAWA',
  'Ishikawa-Beta',
  'ishikawax',
  'ishikawa-betax',
  'ishikawa-',
  'ishikawa_',
  'ishikawa beta',
  'ishikawa-beta\n',
  '\nishikawa',
  ' ishikawa',
  'é',
  '日本',
  '<b>x</b>',
  '"q"',
  '#35;',
  '-',
  ':',
  '0',
];

const SEEDS = [
  'ishikawa-beta\n    Blurry Photo\n        Process\n            Out of focus\n        User\n            Shaky hands\n',
  'ishikawa-beta\nProblem\nCause A\n  Subcause A1\nCause B\n',
  'ishikawa-beta\n    Problem\nCause A\n  Subcause A1\nCause B\n',
  'ishikawa\nEffect\n\tOne\n\t\tTwo\n\t\t\tThree\n\t\t\t\tFour\n\tFive\n',
  '%% first\n\n%% second\nishikawa-beta\n%% inside\n  Effect\n\n\n  %% between\n  Cause\n    Sub %% not a comment\n\n',
  'ishikawa-beta Effect on one line',
  'ISHIKAWA-BETA\nEffect\r\n  Cause\r\n    Sub\r\n',
  'ishikawa-beta\nEffect\n  A\n      B\n    C\n  D\n E\nF\n',
  'ishikawa-beta%% nothing else',
  'ishikawa-beta\n  Effect\n   \n\n  Cause\n   ',
];

const corpus = [...new Set([...loadCorpus('ishikawa', /ishikawa/i), ...SEEDS])];

function compare(src: string): boolean {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
  return expected.ok;
}

describe('Ishikawa parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(12);
  });

  it('agrees on every spec and documentation input', () => {
    for (const src of corpus) compare(src);
    for (const src of SEEDS) expect(oracleParse(src).error, src).toBeUndefined();
  });

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000 && compare(src)) accepted++;
    }
    expect(accepted).toBeGreaterThan(count / 10);
  }, 1_200_000);

  // The spec corpus is three diagrams. This strings random pieces together, so that every kind
  // of white space meets every other and the keyword turns up where it should not.
  it('agrees on random sequences of tokens', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 1);
    const pick = (): string => FRAGMENTS[Math.floor(rnd() * FRAGMENTS.length)];
    const openers = [
      'ishikawa-beta\n',
      'ishikawa-beta\nEffect\n',
      'ishikawa\n  E\n',
      'ishikawa-beta',
      '%% c\nishikawa-beta\n',
      '\n',
      '',
    ];
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      let src = openers[Math.floor(rnd() * openers.length)];
      for (let k = 1 + Math.floor(rnd() * 8); k > 0; k--) src += pick();
      if (compare(src)) accepted++;
    }
    expect(accepted).toBeGreaterThan(count / 20);
  }, 1_200_000);

  // Whole lines at random depths, with blank lines and comments between them.
  it('agrees on random outlines', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 2);
    const pick = <X>(list: X[]): X => list[Math.floor(rnd() * list.length)];
    for (let i = 0; i < count; i++) {
      let src = pick(['ishikawa-beta', 'ishikawa', 'Ishikawa-BETA']) + pick(['\n', '\n', '\r\n', ' ', '\n\n']);
      for (let k = Math.floor(rnd() * 12); k > 0; k--) {
        src +=
          pick(['', ' ', '  ', '    ', '      ', '\t', '\t\t', ' \t ']) +
          pick(['Cause', 'A b', 'x', '%% note', '', 'ishikawa']) +
          pick(['\n', '\n', '\n', '\r\n', '\n\n', ' \n']);
      }
      compare(src);
    }
  }, 1_200_000);
});
