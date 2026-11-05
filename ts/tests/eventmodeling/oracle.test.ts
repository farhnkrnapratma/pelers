import { describe, expect, it } from 'vitest';
import { EVENTMODELING_TOKENS, parseEventModel } from '../../src/diagrams/eventmodeling/parser.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { peleTree, referenceTree } from '../support/eventmodeling-oracle.js';
import { peleTokens, referenceTokens } from '../support/langium.js';
import { peleLongerAlts, referenceLongerAlts } from '../support/langium-extra.js';

const FRAGMENTS = [
  'eventmodeling',
  'eventmodeling\n',
  'tf',
  'timeframe',
  'rf',
  'resetframe',
  'tf 01 evt A',
  '\ntf 09 cmd B.C',
  'rf 7 ui U',
  'entity',
  'entity A',
  'data',
  'data D {\n x\n}\n',
  'note',
  'note 01 {\n n\n}',
  'gwt',
  'gwt 01 given evt A then evt B',
  'given',
  'when',
  'then',
  'rmo',
  'readmodel',
  'ui',
  'cmd',
  'command',
  'evt',
  'event',
  'pcr',
  'processor',
  'json',
  'jsobj',
  'figma',
  'salt',
  'uri',
  'md',
  'html',
  'text',
  '`json`',
  '`md`',
  '`',
  '.',
  '->>',
  ' ->> 01',
  '->',
  '>>',
  '[[',
  ']]',
  '[[D]]',
  '{',
  '}',
  '{ a: 1 }',
  '{}',
  '{\n',
  '\n}',
  '\n}\n',
  '"',
  "'",
  '"q"',
  "'s'",
  '0',
  '01',
  '123',
  '1234',
  ' title T',
  ' accTitle: T',
  ' accDescr: D',
  ' accDescr {',
  'title',
  'accTitle',
  '\n',
  '\r\n',
  '\r',
  ' ',
  '\t',
  ' ',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---',
  '---\n',
  '/*',
  '*/',
  '/* c */',
  '//',
  '// c',
  'x',
  'A',
  '_a',
  'é',
  ':',
  ';',
  ',',
  '#35;',
  'tfx',
  'eventx',
  'evt2',
  'titles',
  ' titles',
  'title_',
  'title1',
];

// Seeds written for this test, checked against the reference below so each keeps covering what it is meant to.
const VALID = [
  'eventmodeling',
  'eventmodeling\n',
  'eventmodeling\ntf 01 cmd A . B.C ->> 02 ->> 3 [[D]] `json`{ a: 1 }\nrf 02 evt X {q}\nentity N.M\ndata D `md` {\n  x\n}\nnote 01 {\n hi\n}\n',
  'eventmodeling\ngwt 01 given evt A cmd B when ui C then evt D\ngwt 2\n given\n  rmo A\n then\n  pcr B\n  processor C\n',
  'eventmodeling title Foo\ntf 01 ui A accDescr: d\ntf 02 cmd B accTitle: t\n',
  'eventmodeling accDescr {\n multi\n line\n}\ntf 1 event E "inline"\ntf 2 command C \'single\'\ntf 3 readmodel R `text`"t"\n',
  'eventmodeling\n/* block\n comment */\ntf 01 ui A // line comment\n%% percent comment\ntf 02 cmd B %% trailing\n',
  'eventmodeling\r\ntimeframe 01 ui A\r\nresetframe 02 processor P ->> 01\r\ndata D {\r\n x\r\n}\r\n',
  'eventmodeling\ndata D {\n}\ndata E `jsobj` {  \n  { a: {\n    d: true\n  }}\n}\nnote 1 `html` {\n <b>x</b>\n}',
  'eventmodeling\ntf 123 evt _a1.b_2 { { nested } } \ntf 4 evt B { a } [[x]] { b }\n',
  '---\ntitle: x\n---\neventmodeling\ntf 01 ui A\n',
  'eventmodeling%%c\ntf 01 ui tfx\ntf 02 ui eventx\n',
  'eventmodeling title T\ntf 01 ui titles\ntf 02 cmd title_x ->> 01 [[titleData]]\nentity titles.title2\ndata titleData {\n x\n}\ngwt 02 given evt titles then rmo title_x\n',
];

const INVALID = [
  'eventmodeling\ntitle Foo\n',
  'eventmodeling\ntf 01 evt data\n',
  'eventmodeling\ntf 01 evt title\n',
  'eventmodeling\ntf 1234 evt A\n',
  'eventmodeling\ntf 01 evt A {\n',
  'eventmodeling\ngwt 01 given then evt A\n',
  'eventmodelingx\n',
  'eventmodeling\ntf 01 A\n',
];

const corpus = [...loadCorpus('eventmodeling', /eventmodeling/), ...VALID, ...INVALID];

function compare(src: string, expected: Awaited<ReturnType<typeof referenceTree>>): void {
  const actual = peleTree(parseEventModel, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('eventmodeling parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(peleTokens(EVENTMODELING_TOKENS)).toEqual(referenceTokens('createEventModelingServices', 'EventModel'));
    expect(peleLongerAlts(EVENTMODELING_TOKENS)).toEqual(
      referenceLongerAlts('createEventModelingServices', 'EventModel')
    );
  });

  it('agrees on every spec and documentation input', async () => {
    expect(corpus.length).toBeGreaterThan(30);
    for (const src of corpus) compare(src, await referenceTree(src));
    for (const src of VALID) expect((await referenceTree(src)).error, src).toBeUndefined();
    for (const src of INVALID) expect((await referenceTree(src)).ok, src).toBe(false);
  });

  it('agrees on mutated inputs', async () => {
    const count = Number(process.env.FUZZ ?? 3000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 2000) compare(src, await referenceTree(src));
    }
  }, 600_000);
});
