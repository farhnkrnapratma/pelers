import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/timeline-oracle.js';

// Compares Pelers's timeline lexer and parser with the parser Mermaid generates from timeline.jison:
// the same tokens, the same accept or reject decision, the same calls into the model, and the same
// "Expecting ..." line on a syntax error. Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  'timeline',
  'timeline ',
  'timeline LR',
  'timeline TD',
  'timeline\tTD',
  'Timeline lr',
  'TIMELINE',
  ' LR',
  ' TD',
  'LR',
  'TD',
  'lr',
  'td',
  'LRx',
  'TD_',
  'timelines',
  'title ',
  'title',
  'title\n',
  'TITLE ',
  'titles',
  'section ',
  'section',
  'section\n',
  'Section ',
  'sections',
  'accTitle: ',
  'accTitle',
  'accTitle :',
  'acctitle:',
  'accDescr: ',
  'accDescr',
  'accDescr {',
  'ACCDESCR{',
  'accDescr\n:',
  'accTitle\n\n:\n',
  '{',
  '}',
  '}\n',
  ':',
  ' : ',
  ': ',
  ' :',
  ':\n',
  '::',
  ': :',
  ':x',
  ' ',
  '  ',
  '\n',
  '\n\n',
  '\t',
  '\r',
  '\r\n',
  '\u000b',
  '\u000c',
  '\u00a0',
  '\u2028',
  '\u2029',
  '\ufeff',
  '#',
  '# c',
  ';',
  '%',
  '%%',
  '%% c',
  '%{',
  '%%{',
  '%%{init: {}}%%',
  'a%%b',
  '}%%',
  '\n%%',
  ' %%',
  '<br>',
  '<br/>',
  'x',
  'é',
  '日本',
  '1',
  '2002',
  '_',
  '-',
  '.',
  ',',
  '"',
  "'",
  '2002 : a',
  ' : b : c',
  '\n     : d',
];

const SEEDS = [
  'timeline',
  'timeline LR',
  'timeline TD\n',
  'timeline\n    title History\n    2002 : LinkedIn\n    2004 : Facebook : Google\n         : Orkut\n    2005 : YouTube\n',
  'timeline TD\n  title T\n  section One\n    A : a1 : a2\n    B\n  section Two\n    C : c1\n',
  'timeline\n  accTitle: Acc title\n  accDescr: Acc description\n  2001 : a\n',
  'timeline\n  accDescr {\n    Several\n    lines\n  }\n  2001 : a\n',
  'timeline\n  accDescr {}title x\n  2001 : a',
  'timeline\n  %% comment\n  2001 : a %% not a comment\n  # comment\n  2002 # comment\n  x%%hidden\n',
  'timeline\n  title\nnext line\n  section\nnext line\n  a :\nb\n',
  'timeline\n  a : b:c : d::e : f:\n  a :b\n  a : \n',
  'timeline\n  a : 50% done\n  % comment\n  %{ not a comment\n',
  'TIMELINE lr\n  TITLE Caps\n  SECTION Caps\n  ACCTITLE: caps\n  ACCDESCR: caps\n  AccDescr { caps }\n',
  'timeline\r\n  title Windows\r\n  2001 : a\r\n',
  'timeline\n  section A : B\n  timeline\n',
  '  \n  timeline\n\n\n  2001\n\n  : late event\n',
  'timeline\n  accTitle:',
  'timeline\n  accDescr {\n  never closed',
  'timeline title T section S p : e',
  'timeline\n  section S: e\n  ; : e ; f\n',
];

const corpus = [...loadCorpus('timeline', /timeline/), ...SEEDS];

const expecting = (message: string | undefined): string => (message ?? '').split('\n').pop() ?? '';

function compare(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
  else expect(expecting(actual.error), `error for ${JSON.stringify(src)}`).toBe(expecting(expected.error));
}

describe('timeline parser against the Mermaid grammar', () => {
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
});
