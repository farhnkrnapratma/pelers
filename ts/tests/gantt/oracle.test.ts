import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/gantt-oracle.js';

// Compares Pelers's gantt lexer and parser with the parser Mermaid generates from gantt.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model.
// Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '\n',
  '\n\n',
  '\t',
  '\r\n',
  ' \n',
  ':',
  ';',
  '#',
  ',',
  '%',
  '%%',
  '%%{',
  '}',
  '{',
  '"',
  '(',
  ')',
  '()',
  '( )',
  'gantt',
  'gantt\n',
  'GANTT',
  'dateFormat ',
  'dateFormat YYYY-MM-DD',
  'dateformat X',
  'inclusiveEndDates',
  'topAxis',
  'axisFormat ',
  'axisFormat %Y-%m-%d',
  'tickInterval ',
  'tickInterval 1day',
  'includes ',
  'excludes ',
  'excludes weekends',
  'todayMarker ',
  'todayMarker off',
  'todayMarker stroke:#0f0,opacity:0.5',
  'weekday ',
  'weekday monday',
  'weekday  tuesday',
  'weekday\nwednesday',
  'weekday thursday',
  'weekday friday',
  'weekday saturday',
  'weekday sunday',
  'weekday sundays',
  'Weekday Monday',
  'weekend ',
  'weekend friday',
  'weekend saturday',
  'weekend sunday',
  'title ',
  'title A',
  'accTitle: ',
  'accTitle : x',
  'accDescr: ',
  'accDescr { ',
  'accDescr {\n x\n}',
  'accDescription ',
  'accDescription x',
  'section ',
  'section A',
  'click ',
  'click a ',
  'click a,b ',
  'href ',
  'href "',
  'href "https://example.com"',
  'call ',
  'call f()',
  'call f(a, "b")',
  'call f( )',
  '2014-01-01',
  '2014-01-01x',
  ' 2014-01-01',
  'a1',
  'des1, ',
  'after a1',
  'until a1',
  '3d',
  '1w',
  'crit, ',
  'done, ',
  'active, ',
  'milestone, ',
  'vert, ',
  'A task :a1, 2014-01-01, 30d',
  'Task :',
  ': 5d',
  'x%y',
  '% z',
  'é',
  'ö',
  ' ',
  ' ',
  'ſection x',
  'TITLE x',
  'Section b',
];

const corpus = loadCorpus('gantt', /gantt/);

function compare(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
  else expect(lastLine(actual.error), `error of ${JSON.stringify(src)}`).toBe(lastLine(expected.error));
}

// The line of a parse error that names the expected and the found tokens.
function lastLine(message: string | undefined): string {
  return (message ?? '').split('\n').pop()!;
}

describe('gantt parser against the Mermaid grammar', () => {
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
