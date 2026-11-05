import { describe, expect, it } from 'vitest';
import { TREEVIEW_TOKENS, parseTreeView } from '../../src/diagrams/treeview/parser.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { peleAst, peleTokens, referenceAst, referenceTokens } from '../support/langium.js';
import { peleLongerAlts, referenceLongerAlts } from '../support/langium-extra.js';
import { patchTitle, patchTokens } from '../support/token-patch.js';

patchTokens('TreeViewModule', (tokens) => patchTitle(tokens, String.raw`(?!%%)[^\n\r]`));

const FRAGMENTS = [
  'treeView-beta',
  'treeView-beta\n',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  '}',
  ':::',
  ' :::',
  ' :::x',
  ' ::: y',
  ':::1',
  'icon(',
  ' icon(',
  ' icon()',
  ' icon(a:b)',
  ' icon(a:)',
  ' icon(none)',
  ')',
  '##',
  ' ##',
  ' ## d',
  '#',
  '"',
  "'",
  '"a b"',
  "'c'",
  '""',
  '\n',
  '\r\n',
  '\r',
  ' ',
  '  ',
  '\t',
  '    ',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---',
  '/',
  'src/',
  'a.js',
  '├── ',
  '└── ',
  '│   ',
  'x',
  'é',
  ':',
  '::',
  '-',
  '_',
  ' ',
  '#35;',
  'icon',
  'a b',
  ' \n',
  '"a" b',
  'titles',
  'title.md',
  'title ',
  's',
  '.s',
];

// Seeds written for this test, checked against the reference below so each keeps covering what it is meant to.
const VALID = [
  'treeView-beta',
  'treeView-beta\n',
  'treeView-beta\ntitle T\naccTitle: A\naccDescr: D\nroot/\n  a.js :::highlight icon(x:y) ## note\n  "b c" ## d\n',
  'treeView-beta\naccDescr {\n multi\n line\n}\n"a"\n    \'b\'\n',
  'treeView-beta\n%% comment\nsrc/\n    my file.ts ## some description\n    x :::a :::b icon() icon(c)\n',
  'treeView-beta\r\na\r\n  b\r\n',
  'treeView-beta\na:::b icon(c)d ##e\n',
  'treeView-beta\n"a" "b" c\n',
  'treeView-beta\ntitle%% c\naccTitle: A\nroot\n',
  'treeView-beta\ntitle T\ntitles\n  title.txt\n  title-page.md ## d\n  title"q"\n  titles :::x\n',
];

const INVALID: string[] = ['treeView-beta :::x\n', 'treeView-beta title T\n', 'treeView-beta\n  a\n  title\n  b\n'];

const corpus = [...loadCorpus('treeview', /treeView-beta/), ...VALID, ...INVALID];

async function compare(src: string): Promise<void> {
  const expected = await referenceAst('treeView', src);
  const actual = peleAst(parseTreeView, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('treeView parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(peleTokens(TREEVIEW_TOKENS)).toEqual(referenceTokens('createTreeViewServices', 'TreeView'));
    expect(peleLongerAlts(TREEVIEW_TOKENS)).toEqual(referenceLongerAlts('createTreeViewServices', 'TreeView'));
  });

  it('agrees on every spec and documentation input', async () => {
    expect(corpus.length).toBeGreaterThan(20);
    for (const src of corpus) await compare(src);
    for (const src of VALID) expect((await referenceAst('treeView', src)).error, src).toBeUndefined();
    for (const src of INVALID) expect((await referenceAst('treeView', src)).ok, src).toBe(false);
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
