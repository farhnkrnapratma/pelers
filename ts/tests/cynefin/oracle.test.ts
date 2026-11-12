import { describe, expect, it } from 'vitest';
import { CYNEFIN_TOKENS, parseCynefin } from '../../src/diagrams/cynefin/parser.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { pelersAst, pelersTokens, referenceAst, referenceTokens } from '../support/langium.js';
import { pelersLongerAlts, referenceLongerAlts } from '../support/langium-extra.js';

const FRAGMENTS = [
  'cynefin-beta',
  'cynefin-beta:',
  'cynefin-beta\n',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  '}',
  ':',
  ' : ',
  '-->',
  ' --> ',
  '--',
  '->',
  '>',
  '"',
  "'",
  '"a"',
  "'b'",
  '"a\\"b"',
  '\\',
  '\\n',
  'complex',
  'complicated',
  'clear',
  'chaotic',
  'confusion',
  'compl',
  'complexity',
  'Clear',
  '\n',
  '\r\n',
  '\r',
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
  ',',
  '#35;',
  'complex --> clear',
  'clear --> clear : "x"',
  '\n"item"',
  '\nchaotic\n',
  '1',
];

// Seeds written for this test, checked against the reference below so each keeps covering what it is meant to.
const VALID = [
  'cynefin-beta',
  'cynefin-beta:',
  'cynefin-beta:\ncomplex\n"a"',
  '\n\n  cynefin-beta\n',
  'cynefin-beta title T\n',
  'cynefin-beta complex "a" "b" clear\n"c"',
  'cynefin-beta\ncomplex --> clear\nclear --> chaotic : "x"\nchaotic --> chaotic',
  'cynefin-beta\naccTitle: at\naccDescr {\n multi\n line\n}\ntitle t\ncomplex\n\n\n"a"\n\n"b"\n',
  'cynefin-beta\ncomplex\n"a"\ncomplex\n"b"\ncomplex --> complicated : \'single\'\n',
  '---\ntitle: x\n---\ncynefin-beta\nclear\n"a"',
  '%%{init: {"theme":"dark"}}%%\ncynefin-beta %% c\nclear %% d\n"a" %% e\n',
  'cynefin-beta\r\ncomplex\r\n"a"\r\n',
];

const INVALID: string[] = [];

const corpus = [...loadCorpus('cynefin', /cynefin-beta/), ...VALID, ...INVALID];

async function compare(src: string): Promise<void> {
  const expected = await referenceAst('cynefin', src);
  const actual = pelersAst(parseCynefin, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('cynefin parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(pelersTokens(CYNEFIN_TOKENS)).toEqual(referenceTokens('createCynefinServices', 'Cynefin'));
    expect(pelersLongerAlts(CYNEFIN_TOKENS)).toEqual(referenceLongerAlts('createCynefinServices', 'Cynefin'));
  });

  it('agrees on every spec and documentation input', async () => {
    expect(corpus.length).toBeGreaterThan(20);
    for (const src of corpus) await compare(src);
    for (const src of VALID) expect((await referenceAst('cynefin', src)).error, src).toBeUndefined();
    for (const src of INVALID) expect((await referenceAst('cynefin', src)).ok, src).toBe(false);
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
