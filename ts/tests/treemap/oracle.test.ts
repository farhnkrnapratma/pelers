import { describe, expect, it } from 'vitest';
import { TREEMAP_TOKENS, parseTreemap } from '../../src/diagrams/treemap/parser.js';
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
import { patchTitle, patchTokens } from '../support/token-patch.js';

patchTokens('TreemapModule', (tokens) => patchTitle(tokens, String.raw`\w`));

const FRAGMENTS = [
  'treemap',
  'treemap-beta',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  'accDescr {}',
  '}',
  ':',
  ' : ',
  '::',
  ':::',
  ':::c1',
  ',',
  ' , ',
  '"',
  "'",
  '"a"',
  "'b'",
  '"a": 1',
  '"s"\n',
  '1',
  '0',
  '1.5',
  '1,000',
  '_',
  '.',
  '1e3',
  '-1',
  '\n',
  '\r\n',
  '\r',
  ' ',
  '  ',
  '    ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---',
  'x',
  'ab',
  'é',
  ';',
  '#35;',
  'classDef',
  'classDef ',
  'classDef c1 fill:red',
  'classDef c1 fill:red,stroke:blue;',
  'class',
  'titles',
  ':::titles',
  'title1',
  '\n  "b": 2',
  '\n    "c":3:::c1',
];

const corpus = [
  ...loadCorpus('treemap', /treemap/),
  ...loadTestStrings('treemap', /treemap/),
  'treemap',
  'treemap\naccDescr {}\n"A"',
  'treemap\nclassDef ab ;\n"A"',
  'treemap\nclassDef ab \n\n',
  'treemap-beta\n"A"\n  "B": 10\n',
  'treemap-betax',
  '  treemap',
  '\ntreemap\n',
  'treemap\n"A"   \n"B"',
  'treemap "A" "B"',
  'treemap\n"A" : 1,000.5:::cls\n',
  'treemap\n"A":1\n"B",2\n',
  'treemap\n"A" :::x',
  'treemap\n"A":::x1 "B"',
  'treemap\n  classDef foo fill:red,stroke:blue;\n"A"',
  'treemap\nclassDef foo\n',
  'treemap\nclassDef foo fill:red; "A"',
  'treemap\nclassDef\n\nfoo   a b c  \n"A"',
  'treemap\nclassDef a fill:red',
  'treemap title Hello\n"A"',
  'treemap\n  title   Hello  world %% c\n accTitle: x\n accDescr { a\n b }\n"A"',
  'treemap\n"A": _',
  'treemap\n"A": .',
  'treemap\n"A": 1.2.3',
  'treemap\n"A": ,5',
  'treemap\n"A":\n 5',
  'treemap\n"A"\n: 5',
  "treemap\n'A': 5 %% comment\n'B'",
  'treemap\n%% c\n"A"',
  'treemap\n"A\n multi": 5',
  'treemap\n"A" ',
  'treemap ',
  'treemap\n\t"A"\n\t\t"B":1',
  'treemap\n"A": 5 :::c',
  'treemap\n"A": 5::: c',
  'treemap\n"A"\n\n\n   \n"B"',
  '---\ntitle: x\n---\ntreemap\n"A"',
  '%%{init: {}}%%\ntreemap\n"A"',
  'treemap\n"A": 5 6',
  'treemap\n"A": 5 "B": 6',
  'treemap\nclass foo bar',
  'treemap\ntitle\n"A"',
  'treemap\ntitle a\ntitle b\n"A"',
  'treemap\r\n"A"\r\n  "B": 3\r\n',
  'treemap\n"A"  ,  7',
  'treemap\n"A"  ,',
  'treemap\n"A":::',
  'treemap\n "A"\n"B"',
  'treemap treemap',
  'treemap\n"A": 5:::c d',
  'treemap\n"A"\n classDef ab x\n classDef cd',
  'treemap\nclassDef ab x;;',
  'treemap\nclassDef titles fill:red\nclassDef title_2 fill:blue\n"A":::titles\n  "B": 1:::title_2\n',
  'treemap\n"A":::title\n',
  'treemap\n"A"\n  titles\n',
];

const referenceLex = referenceLexer('createTreemapServices', 'Treemap');
const peleLex = peleLexer(TREEMAP_TOKENS, (src) => tokenize(src, TREEMAP_TOKENS, 'treemap'));

async function compare(src: string): Promise<void> {
  expect(peleLex(src), `tokens of ${JSON.stringify(src)}`).toBe(referenceLex(src));
  const expected = await referenceAst('treemap', src);
  const actual = peleAst(parseTreemap, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

describe('treemap parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(peleTokens(TREEMAP_TOKENS)).toEqual(referenceTokens('createTreemapServices', 'Treemap'));
    expect(peleLongerAlts(TREEMAP_TOKENS)).toEqual(referenceLongerAlts('createTreemapServices', 'Treemap'));
  });

  it('agrees on every spec and documentation input', async () => {
    expect(corpus.length).toBeGreaterThan(60);
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
