import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/venn-oracle.js';

// Compares Pelers's venn lexer and parser with the parser Mermaid generates from venn.jison:
// the same tokens, the same accept or reject decision, and the same calls into the model,
// including the ones the lexer makes to ask about indentation.
// Set FUZZ to raise the number of generated inputs and SEED to change them.

const FRAGMENTS = [
  ' ',
  '  ',
  '\t',
  '\n',
  '\r\n',
  '\r',
  '\n\n',
  ' \n',
  '\n  ',
  '\n\t',
  ',',
  ':',
  ';',
  '#',
  '"',
  '""',
  '[',
  ']',
  '(',
  ')',
  '{',
  '}',
  '%',
  '%%',
  '%% c',
  '%%{',
  '%%{init}%%',
  'x%%c',
  '}%%c',
  '\n%% c',
  ' %%c\n',
  '-',
  '+',
  '.',
  '_',
  'A',
  'B',
  'a-b',
  '_x',
  'A1',
  'é',
  '日本',
  ' ',
  'venn-beta',
  'VENN-BETA',
  'venn-betax',
  'venn-beta-x',
  'venn',
  'set',
  'SET',
  'set ',
  'sets',
  'set-a',
  'union',
  'Union ',
  'unions',
  'text',
  'TEXT',
  'text ',
  'texts',
  'text-x',
  'style',
  'style ',
  'styles',
  'title',
  'title x',
  'title\nx',
  'TITLE a b',
  'title #x',
  'title a;b',
  'title y',
  'titlex',
  '  text',
  '\n  text ',
  '\n    text T1',
  '\n  text "q"["l"]',
  '\n\ttext 5',
  '\n  texts',
  '\n  text\n',
  '\n  Text x',
  '["l"]',
  '[l m]',
  '[ l ]',
  '[""]',
  '[]',
  '["l]',
  '[l"]',
  '["a"b]',
  '["x\ny"]',
  ':5',
  ': 5',
  ':-5',
  ':+.5',
  ':1.5',
  ':1.',
  ':.',
  ':5px',
  '5',
  '-5',
  '.5',
  '1.5.2',
  '+',
  '"s"',
  '"s t"',
  '"a,b"',
  '"x\ny"',
  '"unclosed',
  '#fff',
  '#ff6b6b',
  '#12345678',
  '#123456789',
  '#ff',
  '#ggg',
  'rgb(1,2,3)',
  'rgb( 1 , 2 , 3 )',
  'RGB(1,2,3)',
  'rgb(1,2)',
  'rgb(1.5,.2,3.)',
  'rgba(1,2,3,0.5)',
  'rgba(1,2,3)',
  'rgba (1,2,3,4)',
  'rgb(1,\n2,3)',
  'rgbx',
  'A,B',
  'A, B',
  'A,',
  ',B',
  'A,B,C',
  'fill:#f00',
  'color:red',
  'fill:red,color:blue',
  'stroke-width:2px',
  'fill:"a b"',
  'fill:red blue 5',
  'fill:',
  'fill',
  'set A',
  'set A["Alpha"]:20',
  'set "Foo Bar"',
  'union A,B',
  'union A,B["AB"]:3',
  'union A',
  'text A foo',
  'text A,B "q"["l"]',
  'text A 5',
  'text A 5["l"]',
  '\n  text 5[l]',
  'style A fill:#ff6b6b',
  'style A,B color:#333, fill:rgb(1,2,3)',
  'style A1 color:red',
];

const SEEDS = [
  'venn-beta\nset A\nset B\nunion A,B\n',
  'venn-beta\n  set A["Alpha"]:20\n    text A1["React"]\n    text A2\n    text "q"\n    text 5\n  set B[Beta]:12\n  union A,B["AB"]:3\n    text AB1["OpenAPI"]\n  style A fill:#ff6b6b\n  style A,B color:#333\n  style A1 color:red\n',
  'venn-beta\ntitle A title\nset A\n  text x\nset B set C\nunion A,B,C[all]:1.5 union A,C\ntext A,B t1["one"]\ntext C "two"\ntext A 3\n',
  '\n\n  venn-beta\n%% comment\n  set A %% trailing\n  set B\n\n   \n  union A,B\n    text T["x"]\n\n    text U\ntext A V\n',
  'venn-beta\nset A\nstyle A fill:rgb(255, 0, 128), color:rgba(1,2,3,0.5), stroke:"dashed red", stroke-width:2 px, x:#fff 5 a\n',
  'VENN-BETA\nSET A\nSet B:5\nUNION A,B\n  TEXT t\nSTYLE A FILL:Red\nTITLE Shout\n',
  'venn-beta\nset "Foo Bar"\nset Buz\nunion "Foo Bar",Buz\n  text "a note"["with label"]\n',
  'venn-beta\r\nset A\r\n  text x\r\nset B\r\n',
  'venn-beta\nset A\n  text a\n  text b\nunion A,A\n  text c\ntitle last one\n',
];

const corpus = [...new Set([...loadCorpus('venn', /venn-beta/), ...SEEDS])];

function compare(src: string): boolean {
  for (const indent of [false, true]) {
    expect(pelersTokens(src, indent), `tokens of ${JSON.stringify(src)} with indent ${indent}`).toEqual(
      oracleTokens(src, indent)
    );
  }
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
  return expected.ok;
}

describe('venn parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(25);
  });

  it('agrees on every spec and documentation input', () => {
    for (const src of corpus) compare(src);
    for (const src of SEEDS) expect(oracleParse(src).error, src).toBeUndefined();
  });

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000 && compare(src)) accepted++;
    }
    expect(accepted).toBeGreaterThan(count / 20);
  }, 1_200_000);

  // The spec corpus is thin, and mutation rarely joins two tokens without a space. This strings
  // random pieces together, after each kind of statement and in each lexer state.
  it('agrees on random sequences of tokens', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 1);
    const pieces = [...new Set([...FRAGMENTS, ...FRAGMENTS.map((f) => f.trim()).filter((f) => f !== '')])];
    const pick = (): string => pieces[Math.floor(rnd() * pieces.length)];
    const openers = [
      'venn-beta\n',
      'venn-beta\nset A\n',
      'venn-beta\nset A\nset B\nunion A,B\n  ',
      'venn-beta\nset A\n  text ',
      'venn-beta\nset A\nstyle A ',
      'venn-beta\nset A\nstyle A fill:',
      'venn-beta\nset A\ntext A ',
      'venn-beta ',
      '',
    ];
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      let src = openers[Math.floor(rnd() * openers.length)];
      for (let k = 1 + Math.floor(rnd() * 8); k > 0; k--) src += pick() + (rnd() < 0.4 ? ' ' : rnd() < 0.3 ? '\n' : '');
      if (compare(src)) accepted++;
    }
    expect(accepted).toBeGreaterThan(count / 50);
  }, 1_200_000);

  // Whole statements over a few ids: most parse, and exercise the indentation rule from every side.
  it('agrees on random diagrams built from statements', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 2);
    const pick = <X>(list: X[]): X => list[Math.floor(rnd() * list.length)];
    const id = (): string => pick(['A', 'B', 'C', '"D e"', 'x-1', '_y']);
    const label = (): string => pick(['', '', '["L"]', '[L m]', '[""]']);
    const size = (): string => pick(['', '', ':5', ': 2.5', ':-1', ':.5']);
    const indent = (): string => pick(['', '', '  ', '    ', '\t', ' ']);
    const tail = (): string => pick(['t1', '"q r"', '7', 't2["L"]', '"q"[L]', '7["L"]', 't3 %% c', 'text', 'set']);
    const statements: (() => string)[] = [
      () => `set ${id()}${label()}${size()}`,
      () => `union ${id()},${id()}${label()}${size()}`,
      () => `union ${id()},${id()},${id()}${label()}`,
      () => `text ${tail()}`,
      () => `text ${id()} ${tail()}`,
      () => `text ${id()},${id()} ${tail()}`,
      () => `Text ${tail()}`,
      () => `style ${id()} fill:${pick(['red', '#ff6b6b', 'rgb(1, 2, 3)', '"x y"', 'a b 5'])}`,
      () => `style ${id()},${id()} color:#333, stroke-width:${pick(['2', '2px', '.5'])}`,
      () => `title ${pick(['T', 'A b c', '"quoted"', 'x # y', 'a; b'])}`,
      () => pick(['', '   ', '%% comment', 'x%% eaten', '%%{ directive }%%']),
    ];
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      let src = pick(['venn-beta', '  venn-beta', '\nvenn-beta']) + pick(['\n', '\n', ' ', '\r\n']);
      for (let k = 1 + Math.floor(rnd() * 8); k > 0; k--)
        src += indent() + pick(statements)() + pick(['\n', '\n', '\n', ' ', '\r\n', '\n\n']);
      if (compare(src)) accepted++;
    }
    expect(accepted).toBeGreaterThan(count / 10);
  }, 1_200_000);
});
