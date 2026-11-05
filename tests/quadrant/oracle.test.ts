import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, peleParse, peleTokens } from '../support/quadrant-oracle.js';

// Compares Pele's quadrant chart lexer and parser with the parser Mermaid generates from quadrant.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '  ',
  '\n',
  '\r\n',
  '\t',
  ';',
  ':',
  '::',
  ':::',
  ': [',
  ':[',
  ' : [ ',
  '[',
  ']',
  ' ] ',
  ',',
  ', ',
  '0',
  '1',
  '0.5',
  '0.25',
  '1.5',
  '0,5',
  '0x5',
  '05',
  '055',
  '10',
  '.',
  '-',
  '--',
  '-->',
  '--->',
  ' --> ',
  '->',
  '>',
  '"',
  '"`',
  '`"',
  '`',
  '"a"',
  '"`md`"',
  '%%',
  '%% c',
  'a%%b',
  '}%%',
  '%%{init: {}}%%',
  'quadrantChart',
  'QuadrantChart ',
  'quadrant-1 ',
  'quadrant-2 ',
  'quadrant-3 ',
  'quadrant-4 ',
  'quadrant-5',
  'quadrant-',
  'quadrant',
  'x-axis ',
  'y-axis ',
  ' X-Axis ',
  'x-axi',
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
  'classDef ',
  'classDef a ',
  'classDefs',
  'class1',
  'radius: 10',
  'color: #ff0000',
  'stroke-color: #f0f',
  'stroke-width: 5px',
  ' ,',
  '#',
  '_',
  '&',
  '+',
  '=',
  '*',
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
  '>',
  '@',
  '|',
  '~',
  '^',
  'A',
  'b',
  'Point',
  'é',
  '需',
  ' ',
  ' ',
  '\v',
  '\f',
  '\r',
  '\u0001',
  'x',
  'y',
  'q',
  't',
  'c',
  '#35;',
];

const corpus = [
  ...loadCorpus('quadrant', /quadrant/i),
  'quadrantChart\n  title T\n  accTitle: a title\n  accDescr: a description\n  A: [0.1, 0.2]',
  'quadrantChart\n  accDescr {\n    many\n    lines\n  }\n  x-axis "`left`" --> "`right`"\n  "`**P**`":::c: [1, 0] radius: 3',
  'quadrantChart;quadrant-1 a;quadrant-2 b;A: [0.5, 0.5];',
  'quadrantChart\n%% comment\n  A: [0.3, 0.6] %% trailing\n  B:::k: [ 0 , 1 ] color: #abc , radius : 4\n  classDef k stroke-width: 2px',
  '  \n ;quadrantChart x-axis a-b --> c d\n y-axis "only"\n',
];

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('quadrant chart parser against the Mermaid grammar', () => {
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
