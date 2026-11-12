import * as reference from '@mermaid-js/parser';
import { describe, expect, it } from 'vitest';
import { ARCHITECTURE_TOKENS, parseArchitecture } from '../../src/diagrams/architecture/parser.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { pelersAst, pelersTokens, referenceAst, referenceTokens } from '../support/langium.js';
import { patchTitle, patchTokens } from '../support/token-patch.js';

patchTokens('ArchitectureModule', (tokens) => patchTitle(tokens, String.raw`[-\w]*\w`));

const FRAGMENTS = [
  'architecture-beta',
  'architecture',
  'service ',
  'group ',
  'junction ',
  'align ',
  'row ',
  'column ',
  ' in ',
  'in',
  'title',
  'title ',
  'accTitle: ',
  'accDescr: ',
  'accDescr {',
  '}',
  ':',
  ' : ',
  '--',
  '-',
  '---',
  '---\n',
  '<',
  '>',
  '-->',
  '<--',
  '<-->',
  '{group}',
  '{',
  'L',
  'R',
  'T',
  'B',
  ':L',
  'R:',
  ':T -- B:',
  ':R --> L:',
  ' -[x]- ',
  '-[',
  ']-',
  '(',
  ')',
  '(cloud)',
  '(a:b-c)',
  '()',
  '[',
  ']',
  '[x]',
  '[]',
  '["',
  '"]',
  "['",
  "']",
  '["a b"]',
  "['a']",
  '[\\"]',
  '"',
  "'",
  '"t"',
  '\\',
  '\\"',
  "\\'",
  '\n',
  '\r\n',
  ' ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  'a',
  'b',
  'db',
  'x-y',
  'a-',
  '-a',
  '_',
  '1',
  'é',
  '采',
  ';',
  ',',
  '#35;',
  'service a',
  '\nservice b(server)[B] in g',
  '\ngroup g(cloud)[G]',
  '\na:L -- R:b',
  '\njunction j',
  '\nalign row a b',
  'titles',
  'title-x',
  'title_',
  's',
  '-s',
];

const corpus = [
  ...loadCorpus('architecture', /architecture/),
  'architecture-beta',
  'architecture-beta title T\nservice a(server)[A]\nservice b "txt" [B]\na:R --> L:b',
  'architecture-beta\n accTitle: at\n accDescr { multi\n line }\n title t\n group g(cloud)[G]\n service a in g\n junction j in g\n a:B -[lbl]- T:j',
  'architecture-beta\ngroup one\ngroup two in one\nservice a(disk)["Q \\" q"] in two\nservice b(x:y-z)[\'s \\\' s\'] in one\na{group}:T <--> B:b{group}',
  '---\ntitle: x\n---\narchitecture-beta\nservice a\nservice b\nalign column a b',
  '%%{init: {"theme":"dark"}}%%\narchitecture-beta %% c\nservice a %% d\nservice b\na:L<-[ t ]->R:b %% e\n',
  'architecture-beta\nservice a\nservice b\nservice c\na:R -- L:b\nb:R -- T:c\nalign row a b c\n',
  'architecture-beta\nservice a\nservice b\na:R -- L:b\n---\nskipped: yes\n---\nservice c\nb:B -[x]- T:c\n  ---\n---\n',
  'architecture-beta\nservice a\n  %%{init: {}}%%\nservice b %%{ x }%%\n',
  'architecture-beta\nservice a\nservice b\na:R >--< L:b\na:T >-[x]-< B:b\na{group}:L <-- R:b\na:B --> T:b{group}\na : L - [ y ] - R : b\n',
  'architecture-beta\ngroup titles(cloud)[G]\nservice title-x(database)[DB] in titles\njunction title1 in titles\ntitle-x:L -- R:title1\nalign row title-x title1\n',
  'architecture-beta\nservice a\n  title\n  title-\n',
  'architecture-beta\nservice a\ntitleservice b\n',
  'architecture-beta\ngroup g\ngroup h(a-b:c)["t"] in g\nservice s \'i\'[t] in h\nservice in-x(_)[ ] in g\njunction j in g\nalign column s j in-x\n',
];

async function compare(src: string): Promise<boolean> {
  const expected = await referenceAst('architecture', src);
  const actual = pelersAst(parseArchitecture, src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.ast, `tree of ${JSON.stringify(src)}`).toEqual(expected.ast);
  return expected.ok;
}

const IDS = [
  'a',
  'b',
  'db',
  'x-y',
  'n1',
  '_',
  '1',
  'in1',
  'rows',
  'servicex',
  'La',
  'Tx',
  'B',
  'in',
  'row',
  'group',
  'title',
  'titles',
  'accTitle',
  'a-',
];
const ICONS = ['(cloud)', '(a:b-c)', '(_)', '(server)', '(x-1)', '()', '(a b)'];
const TITLES = [
  '[T]',
  '["q"]',
  "['q']",
  '[ sp  aced ]',
  '[]',
  '["]',
  '[a]',
  '[\\"]',
  '["a\\"b"]',
  '[a"b]',
  '[é 采]',
  '["a\nb"]',
];
const STRINGS = ['"s"', "'s'", '"a\\"b"', '""', '"two\nlines"'];
const SIDES = ['L', 'R', 'T', 'B'];
const LINES = [
  'title A title',
  'title',
  'accTitle: An acc title',
  'accDescr: An acc descr',
  'accDescr {\n many\n lines\n}',
  '%% comment',
  '%%{init: {}}%%',
];
const WORDS = [
  'service',
  'group',
  'junction',
  'align',
  'row',
  'column',
  'in',
  ':',
  '--',
  '-',
  '---',
  '<',
  '>',
  '{group}',
  'architecture-beta',
];
const SEPARATORS = [' ', ' ', ' ', ' ', ' ', ' ', ' ', '', '  ', '\t'];
const BREAKS = [
  '\n',
  '\n',
  '\n',
  '\n',
  '\n',
  '\n',
  '\n',
  '\n',
  '\n',
  '\n',
  '\n',
  '\n\n',
  '\r\n',
  ' %% c\n',
  '\n  ',
  ' ',
  '',
];

// Statements built from the grammar, most of them valid, then spoiled by a few token edits.
function generator(rnd: () => number): () => string {
  const pick = <X>(list: X[]): X => list[Math.floor(rnd() * list.length)];
  const maybe = (p: number, ...tokens: string[]): string[] => (rnd() < p ? tokens : []);
  const id = (): string => (rnd() < 0.93 ? pick(IDS.slice(0, 7)) : pick(IDS));
  const statement = (): string[] => {
    const r = rnd();
    if (r < 0.25) {
      const icon = rnd() < 0.5 ? maybe(0.7, pick(ICONS)) : maybe(0.5, pick(STRINGS));
      return ['service', id(), ...icon, ...maybe(0.6, pick(TITLES)), ...maybe(0.3, 'in', id())];
    }
    if (r < 0.4)
      return ['group', id(), ...maybe(0.5, pick(ICONS)), ...maybe(0.6, pick(TITLES)), ...maybe(0.3, 'in', id())];
    if (r < 0.5) return ['junction', id(), ...maybe(0.4, 'in', id())];
    if (r < 0.85) {
      const line = rnd() < 0.6 ? ['--'] : ['-', pick(TITLES), '-'];
      return [
        id(),
        ...maybe(0.2, '{group}'),
        ':',
        pick(SIDES),
        ...maybe(0.3, pick(['<', '>'])),
        ...line,
        ...maybe(0.4, pick(['<', '>'])),
        pick(SIDES),
        ':',
        id(),
        ...maybe(0.2, '{group}'),
      ];
    }
    if (r < 0.93) return ['align', pick(['row', 'column']), id(), id(), ...maybe(0.4, id()), ...maybe(0.2, id())];
    return [pick(LINES)];
  };
  const vocabulary = [...IDS, ...ICONS, ...TITLES, ...STRINGS, ...SIDES, ...LINES, ...WORDS];
  return () => {
    let src =
      rnd() < 0.92 ? 'architecture-beta' : pick(['\n architecture-beta ', 'architecture', '', 'architecture-beta-x']);
    src += pick(BREAKS);
    for (let count = Math.floor(rnd() * 4); count >= 0; count--) {
      const tokens = statement();
      if (rnd() < 0.3) {
        for (let edits = 1 + Math.floor(rnd() * 2); edits > 0; edits--) {
          const at = Math.floor(rnd() * (tokens.length + 1));
          const r = rnd();
          if (r < 0.3) tokens.splice(at, 1);
          else if (r < 0.55) tokens.splice(at, 0, pick(vocabulary));
          else if (r < 0.7) tokens.splice(at, 1, pick(vocabulary));
          else if (r < 0.85 && at < tokens.length) tokens.splice(at, 0, tokens[at]);
          else if (at + 1 < tokens.length) tokens.splice(at, 2, tokens[at + 1], tokens[at]);
        }
      }
      for (const token of tokens) src += token + pick(SEPARATORS);
      src += pick(BREAKS);
    }
    return src;
  };
}

interface ReferenceToken {
  name: string;
  LONGER_ALT?: ReferenceToken | ReferenceToken[];
}

describe('architecture parser against the Mermaid grammar', () => {
  it('uses the same token types in the same order', () => {
    expect(pelersTokens(ARCHITECTURE_TOKENS)).toEqual(referenceTokens('createArchitectureServices', 'Architecture'));
  });

  it('gives each token type the same longer alternatives', () => {
    const create = (reference as unknown as Record<string, () => Record<string, unknown>>).createArchitectureServices;
    const services = create().Architecture as {
      parser: { Lexer: { chevrotainLexer: { lexerDefinition: ReferenceToken[] } } };
    };
    const expected = services.parser.Lexer.chevrotainLexer.lexerDefinition.map((t) => [
      t.name,
      [t.LONGER_ALT ?? []].flat().map((alt) => alt.name),
    ]);
    expect(ARCHITECTURE_TOKENS.map((t) => [t.name, (t.longer ?? []).map((alt) => alt.name)])).toEqual(expected);
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

  it('agrees on generated statements with token edits', async () => {
    const count = Number(process.env.FUZZ ?? 3000);
    const next = generator(random(Number(process.env.SEED ?? 7) + 1));
    let accepted = 0;
    for (let i = 0; i < count; i++) if (await compare(next())) accepted++;
    expect(accepted / count).toBeGreaterThan(0.2);
  }, 600_000);
});
