import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { brokenRuleText, oracleParse, oracleTokens, peleParse, peleTokens } from '../support/xychart-oracle.js';

// Compares Pele's XY chart lexer and parser with the parser Mermaid generates from xychart.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of mutated inputs.

const BROKEN = brokenRuleText();

const FRAGMENTS = [
  ' ',
  '  ',
  '\n',
  '\r\n',
  '\r',
  ' \n',
  '\n ',
  '\t',
  ';',
  ':',
  '[',
  ']',
  '[ ',
  ' ]',
  ',',
  ', ',
  '1',
  '23',
  '4.5',
  '.5',
  '5.',
  '-1',
  '+2',
  '-.3',
  '1.2.3',
  '012',
  '-',
  '--',
  '-->',
  ' --> ',
  '->',
  '>',
  '"',
  '""',
  '"a b"',
  '"`',
  '`"',
  '`',
  '`)',
  '%%',
  '%% c',
  'a%%b',
  '}%%',
  '%%{init: {}}%%',
  'xychart',
  'xychart-beta',
  'XYChart ',
  'xychart-',
  'xycharts',
  'vertical',
  'horizontal',
  ' Horizontal ',
  'verticals',
  'x-axis ',
  'y-axis ',
  'X-Axis',
  'x-axis_',
  'x-axi',
  'line ',
  'bar ',
  'Line',
  'BAR',
  'lines',
  'bars',
  'bar1',
  'line [1]',
  'bar [2, 3]',
  'title',
  'title ',
  'Title x',
  'titles',
  'accTitle: ',
  'accTitle :',
  'accTitle',
  'accDescr: ',
  'accDescr { ',
  'accDescr{',
  '}',
  '{',
  '#',
  '_',
  '&',
  '+',
  '=',
  '*',
  '.',
  '!',
  '$',
  '%',
  "'",
  '?',
  '\\',
  '/',
  '(',
  ')',
  '<',
  '@',
  '|',
  '~',
  '^',
  'A',
  'b',
  'cat',
  'é',
  ' ',
  ' ',
  '\v',
  '\f',
  '\u0001',
  'x',
  'y',
  'l',
  'v',
  'h',
  '#35;',
  BROKEN,
  BROKEN.toUpperCase(),
  BROKEN.slice(0, 40),
];

const corpus = [
  ...loadCorpus('xychart', /xychart/i),
  'xychart\n  title "T"\n  accTitle: a title\n  accDescr: a description\n  line [1, 2, 3]',
  'xychart horizontal\n  accDescr {\n    many\n    lines\n  }\n  x-axis months [jan, "feb", mar]\n  bar "a" [1, 2, 3]\n  line b [3 "x", 2, 1]',
  'xychart;title a-b_c;x-axis 1 --> 10;y-axis "y" -5 --> +5.5;line [1,2];bar [.5]',
  'xychart-beta\n%% comment\n  x-axis x 0 --> 1 %% trailing\n  y-axis 0 --> 1   \n  line [0.1, 0.9]   \n  bar [0.5, 0.2]',
  'xychart\n  x-axis "title only"\n  y-axis title\n  line "s" [1]\n',
  `xychart\nx-axis a ${BROKEN} [b]\nline [1]`,
  `xychart\nline [1 ${BROKEN} 2]\nbar [3]`,
];

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('XY chart parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(60);
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
