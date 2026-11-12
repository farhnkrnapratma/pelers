import { describe, expect, it } from 'vitest';
import { PIE_TOKENS, parsePie } from '../../src/diagrams/pie/parser.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { pelersAst, pelersTokens, referenceAst, referenceTokens } from '../support/langium.js';

const FRAGMENTS = [
  'pie',
  'showData',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  '}',
  ':',
  ' : ',
  '"',
  "'",
  '"a"',
  "'b'",
  '1',
  '0',
  '-1',
  '1.5',
  '01',
  '1.',
  '.5',
  '1e3',
  '\n',
  '\r\n',
  ' ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---',
  '---\n',
  '\\',
  '\\"',
  'x',
  'é',
  ';',
  ',',
  '#35;',
  '"a" : 1',
  '\n"b":2',
];

const corpus = [
  ...loadCorpus('pie', /pie/),
  'pie',
  'pie showData title T\n"a":1\n"b":2.5',
  'pie\n accTitle: at\n accDescr { multi\n line }\n title t\n "a" : 1',
  '---\ntitle: x\n---\npie\n"a":1',
  '%%{init: {"theme":"dark"}}%%\npie title A %% c\n"a":1 %% d\n',
];

async function compare(src: string): Promise<void> {
  const expected = await referenceAst('pie', src);
  const actual = pelersAst(parsePie, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('pie parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(pelersTokens(PIE_TOKENS)).toEqual(referenceTokens('createPieServices', 'Pie'));
  });

  it('agrees on every spec and documentation input', async () => {
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
