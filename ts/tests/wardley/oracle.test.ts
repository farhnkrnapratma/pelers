import { describe, expect, it } from 'vitest';
import { WARDLEY_TOKENS, parseWardley } from '../../src/diagrams/wardley/parser.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { pelersAst, pelersTokens, referenceAst, referenceTokens } from '../support/langium.js';
import { pelersLongerAlts, referenceLongerAlts } from '../support/langium-extra.js';

const FRAGMENTS = [
  'wardley-beta',
  'wardley-beta\n',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  '}',
  '{',
  '[',
  ']',
  '(',
  ')',
  ',',
  '@',
  '/',
  '-',
  '--',
  '---',
  '->',
  '-->',
  '-.->',
  '>',
  '+>',
  '+<',
  '+<>',
  "+'x'>",
  "+'y'<",
  "+'z'<>",
  "+'",
  "'",
  '"',
  '"a b"',
  "'c'",
  ';',
  '; note',
  ';x',
  'size',
  'size [800, 600]',
  'evolution',
  'evolution A -> B',
  'anchor',
  'component',
  'component A [0.5, 0.5]',
  'label',
  'label [-10, 5]',
  'inertia',
  '(inertia)',
  'evolve',
  'evolve A 0.9',
  'pipeline',
  'pipeline A {\n',
  'note',
  'annotations',
  'annotation',
  'annotation 1,[0.5, 0.5] "t"',
  'accelerator',
  'deaccelerator',
  'build',
  'buy',
  'outsource',
  'market',
  '(buy)',
  '0.5',
  '0',
  '1',
  '12',
  '1.',
  '.5',
  '05',
  '1.5',
  '100',
  '0.50',
  '\n',
  '\r\n',
  '\r',
  ' ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---\n',
  'x',
  'A',
  'B',
  'a-b',
  'A B',
  '_',
  '9',
  'é',
  '#35;',
  '&',
  '\\',
  'Tea',
  'Kettle',
  ' -> ',
  ' --> ',
  ' -.-> ',
  ' +> ',
  ' +<> ',
  '\nA -> B',
  '\nA --> B; l',
];

// Seeds written for this test. The reference must accept every one, so each keeps covering what it is meant to.
const VALID = [
  'wardley-beta',
  'wardley-beta title T\n',
  '\n\nwardley-beta\naccTitle: A\naccDescr {\n multi\n line\n}\ntitle T\n',
  'wardley-beta\nsize [800, 600]\nevolution "A b"@0.25 / c -> B@0.5 -> C_1@0.75 / "d e" -> D@1.0\n',
  'wardley-beta\ncomponent A [0.5, 0.5] label [-10, 5] (build) (inertia)\ncomponent B [0.1, 0.9] inertia\ncomponent "C d" [0.3, 0.3] (market)\n',
  'wardley-beta\ncomponent A [0.5, 0.5] label [10, -5] (inertia)\ncomponent _b [0.1, 0.9] (buy) inertia\ncomponent _9 [0.3, 0.3] (outsource)\n',
  'wardley-beta\ncomponent A [0.5, 0.5]\npipeline A {\n\n  component X [0.2] label [1, -2]\n\n  component "Y z" [0.8]\n}\n',
  'wardley-beta\nA -> B\nA --> B\nA -.-> B; dashed\n',
  'wardley-beta\nA +> B\nA +< B\nA +<> B\n',
  "wardley-beta\nA +'text'> B\nA +'t'< B\nA +''<> B; label\n",
  'wardley-beta\n"A" "B"\n_1 _2\nA > B\nA->B\nfoo-bar->baz\n',
  'wardley-beta\nA +> B +<; both\nA +< -> B +>\n"x y" -.-> \'z\'\n',
  'wardley-beta\nannotations [0.1, 90]\nannotation 1,[0.5, 50] "one"\nannotation 0,[1, 0] \'two\'\nnote "n" [0.2, 0.3]\n',
  'wardley-beta\naccelerator "Go" [0.5, 0.5]\ndeaccelerator Slow down [0.1, 0.2]\nevolve A 0.9\nevolve "B c" 0.10\n',
  'wardley-beta\r\ncomponent A [0.5, 0.5]\r\nA->B\r\n',
  '---\ntitle: x\n---\nwardley-beta\ncomponent A [0.5, 0.5] %% trailing\n',
  'wardley-beta\n---\nx\n---\nA->B\n',
];

const INVALID = [
  'wardley-beta\ncomponent market [0.1, 0.2]\n',
  'wardley-beta\ncomponent labelled thing [0.1, 0.2]\n',
  'wardley-beta\n1 2\n',
  'wardley-beta\nA B\n',
  'wardley-beta\ncomponent A [1, 2]\n',
  'wardley-beta\nevolution A / b@0.5 -> B\n',
  'wardley-beta\ncomponent A [0.5, 0.5] (inertia) (buy)\n',
];

const corpus = [...loadCorpus('wardley', /wardley-beta/), ...VALID, ...INVALID];

async function compare(src: string): Promise<void> {
  const expected = await referenceAst('wardley', src);
  const actual = pelersAst(parseWardley, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('wardley parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(pelersTokens(WARDLEY_TOKENS)).toEqual(referenceTokens('createWardleyServices', 'Wardley'));
    expect(pelersLongerAlts(WARDLEY_TOKENS)).toEqual(referenceLongerAlts('createWardleyServices', 'Wardley'));
  });

  it('agrees on every spec and documentation input', async () => {
    expect(corpus.length).toBeGreaterThan(40);
    for (const src of corpus) await compare(src);
    for (const src of VALID) expect((await referenceAst('wardley', src)).error, src).toBeUndefined();
    for (const src of INVALID) expect((await referenceAst('wardley', src)).ok, src).toBe(false);
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
