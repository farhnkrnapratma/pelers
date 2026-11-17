import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/journey-oracle.js';

// Compares Pelers's journey lexer and parser with the parser Mermaid generates from journey.jison:
// the same tokens, the same accept or reject decision, the same calls into the model, and the same
// "Expecting ..." line on a syntax error. Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  'journey',
  'journey ',
  'Journey',
  'JOURNEY',
  'journeys',
  'journey_',
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
  ': 5',
  ': 5: Me',
  ': 3: Me, Cat',
  ':x',
  ',',
  ', ',
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
  ';\n',
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
  '5',
  '-1',
  '3.5',
  'NaN',
  '_',
  '-',
  '.',
  '"',
  "'",
  'Make tea: 5: Me',
  '\n  Do work: 1: Me, Cat',
];

const SEEDS = [
  'journey',
  'journey\n',
  'journey\n    title My working day\n    section Go to work\n      Make tea: 5: Me\n      Go upstairs: 3: Me\n      Do work: 1: Me, Cat\n    section Go home\n      Go downstairs: 5: Me\n      Sit down: 5: Me\n',
  'journey\n  A task: 5\n  B task: 3:\n  C task: 1: a,b , c: ignored\n  D task:: x\n',
  'journey\n  accTitle: Acc title\n  accDescr: Acc description\n  Task: 5: Me\n',
  'journey\n  accDescr {\n    Several\n    lines\n  }\n  Task: 5: Me\n',
  'journey\n  accDescr {}title x\n  Task: 5',
  'journey\n  %% comment\n  Task: 5: Me %% not a comment\n  # comment\n  Task # comment\n  x%%hidden\n',
  'journey\n  title\nnext line\n  section\nnext line\n  Task\n: 5\n',
  'journey\n  Task: 5: Child#1\n  Task: 5; Task: 4\n  title a;b\n',
  'journey\n  Task:\n  Task\n',
  'journey\n  50% done: 5\n  % comment\n  %{ not a comment: 1\n',
  'JOURNEY\n  TITLE Caps\n  SECTION Caps\n  ACCTITLE: caps\n  ACCDESCR: caps\n  AccDescr { caps }\n  Task: 5\n',
  'journey\r\n  title Windows\r\n  Task: 5: Me\r\n',
  'journey\n  section A: 5\n  journey\n',
  '  \n  journey\n\n\n  Task\n\n  : 5\n',
  'journey\n  accTitle:',
  'journey\n  accDescr {\n  never closed',
  'journey title T section S Task: 5: Me',
  'journey\n  Task: abc: Me\n  Task: 100: Me\n  Task: -3: Me\n  Task: 1e3\n  Task: : Me\n',
  'journey\ntitle Adding journey diagram functionality to mermaid\nsection Documentation\nA task: 5: Alice, Bob, Charlie\nB task: 3:Bob, Charlie\nC task: 5\nD task: 5: Charlie, Alice\nE task: 5:\nsection Another section\nP task: 5:\nQ task: 5:\nR task: 5:',
  'journey\naccDescr {\n        A user journey for\n        family shopping\n      }title Adding journey diagram functionality to mermaid\naccTitle: Adding acc journey diagram functionality to mermaid\nsection Order from website',
  'journey\ntitle Adding gantt diagram functionality to mermaid\nsection Line1<br>Line2<br/>Line3</br />Line4<br\t/>Line5',
];

const corpus = [...loadCorpus('journey', /journey/), ...SEEDS];

const expecting = (message: string | undefined): string => (message ?? '').split('\n').pop() ?? '';

function compare(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
  else expect(expecting(actual.error), `error for ${JSON.stringify(src)}`).toBe(expecting(expected.error));
}

describe('journey parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(25);
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
