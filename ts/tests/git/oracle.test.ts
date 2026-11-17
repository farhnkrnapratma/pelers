import * as reference from '@mermaid-js/parser';
import { describe, expect, it } from 'vitest';
import { GIT_TOKENS, parseGit } from '../../src/diagrams/git/parser.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { pelersAst, pelersTokens, referenceAst, referenceTokens } from '../support/langium.js';
import { patchTitle, patchTokens } from '../support/token-patch.js';

patchTokens('GitGraphModule', (tokens) => patchTitle(tokens, String.raw`[-./\w]*[-\w]`));

const FRAGMENTS = [
  'gitGraph',
  'gitGraph:',
  'gitGraph ',
  ' LR:',
  ' TB:',
  ' BT:',
  'LR',
  'TB',
  'BT',
  ':',
  ' : ',
  'commit',
  'commit ',
  'branch ',
  'merge ',
  'checkout ',
  'switch ',
  'cherry-pick ',
  'cherry-pick',
  'id:',
  'id: ',
  'msg:',
  'tag:',
  'tag: ',
  'type:',
  'type: ',
  'parent:',
  'order:',
  'order: ',
  'NORMAL',
  'REVERSE',
  'HIGHLIGHT',
  'id',
  'tag',
  'order',
  'main',
  'develop',
  'a/b',
  'a.b',
  'a-',
  'a.',
  '-a',
  '_a',
  '1.0.1',
  'title',
  'title ',
  ' title x',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  '}',
  '{',
  '"',
  "'",
  '"a"',
  "'b'",
  '""',
  '"x y"',
  '0',
  '1',
  '12',
  '01',
  '1.',
  '1.5',
  '-1',
  '\n',
  '\r\n',
  ' ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---',
  '---\n',
  'titles',
  'title-',
  'title.x',
  'title/x',
  'title.',
  's',
  '/s',
  '.s',
  '\\',
  '\\"',
  'x',
  'é',
  ';',
  ',',
  '.',
  '-',
  '/',
  '#35;',
  ' id:"a"',
  ' tag:"t"',
  '\ncommit',
  '\nbranch x',
  '\nmerge x',
  '\ncheckout main',
];

const corpus = [
  ...loadCorpus('git', /gitGraph/),
  'gitGraph',
  'gitGraph:',
  'gitGraph LR:',
  'gitGraph TB:\ncommit',
  'gitGraph BT :\ncommit id:"a" msg:"m" tag:"t" tag:"u" type:REVERSE',
  'gitGraph commit "msg" id: "x"',
  'gitGraph: commit\nbranch "quoted name" order: 3\ncheckout "quoted name"\ncommit\nswitch main\nmerge "quoted name" id:"m" tag:"v" type:HIGHLIGHT',
  'gitGraph\n title A title\n accTitle: at\n accDescr { multi\n line }\n commit\n cherry-pick id:"a" parent:"b" tag:""',
  '\n\n  gitGraph\n\n  commit\n\n\n  branch 1.0.1\n  branch feature/x_y-z order: 10\n',
  '---\ntitle: x\n---\ngitGraph\ncommit',
  '%%{init: {"theme":"dark"}}%%\ngitGraph %% c\ncommit %% d\n',
  'gitGraph\ncommit\nbranch commits\ncheckout TBD\nmerge LRU\nbranch title1',
  'gitGraph\ncommit\nbranch titles\ncheckout titles\ncommit\nbranch title/x order: 2\nswitch title/x\nmerge titles\nbranch title.v2\nbranch title-\n',
  'gitGraph\ncommit\nbranch title.\n',
  'gitGraph\ncommit\n  title\ntitlecommit\n',
];

// Random sequences of whole tokens reach parser states that character edits to real diagrams rarely do.
const WORDS = [
  'gitGraph',
  'gitGraph:',
  'LR',
  'TB',
  'BT',
  ':',
  'commit',
  'branch',
  'merge',
  'checkout',
  'switch',
  'cherry-pick',
  'id:',
  'msg:',
  'tag:',
  'type:',
  'parent:',
  'order:',
  'NORMAL',
  'REVERSE',
  'HIGHLIGHT',
  '"s"',
  "'t'",
  '""',
  'name',
  'a/b.c-d',
  '0',
  '7',
  '12',
  '\n',
  '\n',
  '\n',
  'titles',
  'title x',
  'accTitle: a',
  'accDescr: d',
  'accDescr { x\n y }',
  '%% c',
];
const HEADERS = [
  'gitGraph',
  'gitGraph:',
  'gitGraph :',
  'gitGraph LR:',
  'gitGraph TB :',
  'gitGraph BT:',
  'gitGraph\n',
  'gitGraph:\n',
  'gitGraph TB:\n',
  '',
];

const CLAUSES = [
  'id:"a"',
  'id: "b"',
  'msg:"m"',
  '"m"',
  'tag:"t"',
  'tag: ""',
  'type:NORMAL',
  'type: REVERSE',
  'type:HIGHLIGHT',
  'type:"x"',
  'parent:"p"',
  'order: 1',
  'order:20',
  'order:x',
  'name',
  'main',
  '"quoted name"',
  '7',
  'LR',
  ':',
];
const STATEMENTS: [string, string[]][] = [
  ['commit', ['id:"a"', 'msg:"m"', '"m"', 'tag:"t"', 'type:NORMAL', 'type: REVERSE']],
  ['branch', ['name', '"quoted name"', 'order: 1', 'order:20', 'titles', 'title/x']],
  ['merge', ['name', 'main', 'id:"a"', 'tag:"t"', 'type:HIGHLIGHT']],
  ['checkout', ['name', 'main']],
  ['switch', ['name', '"quoted name"']],
  ['cherry-pick', ['id:"a"', 'tag:"t"', 'tag: ""', 'parent:"p"']],
  ['title t', []],
  ['accTitle: a', []],
  ['accDescr { d }', []],
  ['', []],
];

function soup(rnd: () => number): () => string {
  const pick = <X>(list: X[]): X => list[Math.floor(rnd() * list.length)];
  const gap = (): string => {
    const r = rnd();
    return r < 0.75 ? ' ' : r < 0.9 ? '\n' : '';
  };
  return () => {
    let s = pick(HEADERS);
    if (rnd() < 0.5) {
      for (let k = Math.floor(rnd() * 12); k > 0; k--) s += gap() + pick(WORDS);
      return s;
    }
    for (let k = Math.floor(rnd() * 5); k > 0; k--) {
      const [statement, own] = pick(STATEMENTS);
      s += (rnd() < 0.9 ? '\n' : ' ') + statement;
      for (let c = Math.floor(rnd() * 5); c > 0; c--) s += gap() + pick(own.length > 0 && rnd() < 0.8 ? own : CLAUSES);
    }
    return s;
  };
}

async function compare(src: string): Promise<void> {
  const expected = await referenceAst('gitGraph', src);
  const actual = pelersAst(parseGit, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
}

interface ReferenceToken {
  name: string;
  LONGER_ALT?: ReferenceToken | ReferenceToken[];
}

describe('git graph parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(pelersTokens(GIT_TOKENS)).toEqual(referenceTokens('createGitGraphServices', 'GitGraph'));
  });

  it('gives the same token types a longer alternative', () => {
    const services = (reference as unknown as Record<string, () => Record<string, unknown>>).createGitGraphServices()
      .GitGraph;
    const definition = (services as { parser: { Lexer: { chevrotainLexer: { lexerDefinition: ReferenceToken[] } } } })
      .parser.Lexer.chevrotainLexer.lexerDefinition;
    const expected = definition.map((t) => [t.name, [t.LONGER_ALT ?? []].flat().map((alt) => alt.name)]);
    expect(GIT_TOKENS.map((t) => [t.name, (t.longer ?? []).map((alt) => alt.name)])).toEqual(expected);
  });

  it('agrees on every spec and documentation input', async () => {
    expect(corpus.length).toBeGreaterThan(100);
    for (const src of corpus) await compare(src);
  });

  it('agrees on mutated inputs', async () => {
    const count = Number(process.env.FUZZ ?? 10000);
    const rnd = random(Number(process.env.SEED ?? 7));
    const mutated = mutator(corpus, FRAGMENTS, rnd);
    const words = soup(rnd);
    for (let i = 0; i < count; i++) {
      const src = i % 3 === 0 ? words() : mutated();
      if (src.length <= 2000) await compare(src);
    }
  }, 3_600_000);
});
