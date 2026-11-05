import { describe, expect, it } from 'vitest';
import { INFO_TOKENS, parseInfo } from '../../src/diagrams/info/parser.js';
import { tokenize } from '../../src/diagrams/common/tokens.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { peleAst, peleTokens, referenceAst, referenceTokens } from '../support/langium.js';
import {
  loadTestStrings,
  peleLexer,
  peleLongerAlts,
  referenceLexer,
  referenceLongerAlts,
} from '../support/langium-extra.js';

const FRAGMENTS = [
  'info',
  'showInfo',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  'accDescr {}',
  '}',
  ':',
  '\n',
  '\r\n',
  ' ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---',
  '---\n',
  'x',
  'é',
  ';',
  '#35;',
  'info showInfo',
  '\ntitle t',
  'Info',
];

const corpus = [
  ...loadCorpus('info', /info/),
  ...loadTestStrings('info', /info/),
  'info',
  'info\naccDescr {}\ntitle t',
  'info showInfo',
  'info\nshowInfo\n',
  '\n info \n showInfo \n title x',
  'info title x',
  'info\ntitle x\naccTitle: a\naccDescr: d',
  'info showInfo title x\n\n',
  'info title x\nshowInfo',
  'info\ntitle x\n\n\ntitle y',
  'info\ntitle x\ninfo',
  'infox',
  'info showInfox',
  'showInfo',
  'info info',
  'info showInfo showInfo',
  'info%% c\nshowInfo%% d',
  'info\naccDescr { a\n b }\n',
  'info\naccDescr { a\n b } title x',
  'info unsupported',
  '---\ntitle: x\n---\ninfo',
  '%%{init: {"theme":"dark"}}%%\ninfo showInfo %% c\n',
];

const referenceLex = referenceLexer('createInfoServices', 'Info');
const peleLex = peleLexer(INFO_TOKENS, (src) => tokenize(src, INFO_TOKENS, 'info'));

async function compare(src: string): Promise<void> {
  expect(peleLex(src), `tokens of ${JSON.stringify(src)}`).toBe(referenceLex(src));
  const expected = await referenceAst('info', src);
  const actual = peleAst(parseInfo, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('info parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(peleTokens(INFO_TOKENS)).toEqual(referenceTokens('createInfoServices', 'Info'));
    expect(peleLongerAlts(INFO_TOKENS)).toEqual(referenceLongerAlts('createInfoServices', 'Info'));
  });

  it('agrees on every spec input', async () => {
    expect(corpus.length).toBeGreaterThan(20);
    for (const src of corpus) await compare(src);
  });

  it('agrees on mutated inputs', async () => {
    const count = Number(process.env.FUZZ ?? 3000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 2000) await compare(src);
    }
  }, 600_000);
});
