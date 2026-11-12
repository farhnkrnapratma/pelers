import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { pelersParse, pelersTokens, referenceParse, referenceTokens, specCorpus } from '../support/usecase-oracle.js';

// Compares Pelers's use case lexer and parser with Mermaid's own, bundled as a reference: the same
// tokens or the same lexer error, the same accept or reject decision with the same message, and,
// when the text is accepted, the same published model and AST down to every source span.
// Set FUZZ to raise the number of generated inputs and SEED to change them.

const FRAGMENTS = [
  ' ',
  '\n',
  '\t',
  '\r\n',
  '\r',
  ' \n',
  '\n\n',
  ';',
  ':',
  '::',
  ':::',
  ',',
  '\\,',
  '#',
  '#f96',
  '"',
  '""',
  "'",
  "''",
  '`',
  '"`',
  '`"',
  '"`x`"',
  '"`a\nb`"',
  '{',
  '}',
  '[',
  ']',
  '(',
  ')',
  '@',
  '@{',
  '@{}',
  '@{ ',
  ' }',
  '%',
  '%%',
  '%% c',
  '.',
  '..',
  '..>',
  '...>',
  '-',
  '--',
  '---',
  '----',
  '-->',
  '--->',
  '---->',
  '<--',
  '<---',
  '--o',
  'o--',
  '--x',
  'x--',
  '--|>',
  '---|>',
  '<',
  '<<',
  '>>',
  '<<a>>',
  '<< a b >>',
  '<<>>',
  '<< >>',
  '<<  >>',
  '<<\n',
  '>',
  '|',
  '*',
  '_',
  '~',
  '!',
  '$',
  '&',
  '+',
  '/',
  '=',
  '?',
  '^',
  '\\',
  'A',
  'B',
  'a1',
  '_x',
  'o',
  'x',
  'ox',
  'A-B',
  'a-b-c',
  '--a',
  '1',
  '1mg',
  '1.5',
  '1.5px',
  '.5',
  '12',
  '4px',
  'é',
  'Ä',
  '日本語',
  ' ',
  ' ',
  '\u0001',
  '\u007f',
  'usecase-beta',
  'usecase-betax',
  'usecase',
  'actor',
  'actor ',
  'actors',
  'actor A[x]',
  'systemBoundary',
  'systemBoundary ',
  'end',
  'end\n',
  'ends',
  'endx-y',
  'direction',
  'direction TB',
  'direction LR',
  'TD',
  'TB',
  'BT',
  'LR',
  'RL',
  'TB1',
  'note',
  'note for ',
  'note-x',
  'for',
  'for-x',
  'format',
  'json',
  'json a@{}',
  'json a @ {"b":1}',
  'json x@{',
  'jsonx',
  'classDef',
  'classDef c fill:red',
  'class',
  'class A c',
  'classes',
  'style',
  'style A fill:red',
  'include',
  'INCLUDE',
  'iNclude',
  'included',
  'extend',
  'Extend',
  'eXtend',
  'enduser-x',
  'true',
  'false',
  'trueish',
  ': include ',
  ': extend ',
  '..> : include ',
  '..> : eXtend ',
  'accTitle',
  'accTitle: t',
  'accTitle : t',
  'accDescr',
  'accDescr: d',
  'accDescr {',
  'accDescr {\n',
  'accDescr { d }',
  'accDescr{\n a\n b\n}',
  'type',
  'type: hollow',
  'type: package',
  'business: true',
  'icon: "fa:user"',
  'animate: true',
  'animation: fast',
  '"k": "v"',
  'k: true',
  '"k": false',
  'k: "v w"',
  'true: k',
  ', k: v',
  '\nk: v',
  ' A@{ type: hollow }',
  ' e@--> ',
  'e@',
  '("x")',
  '(x y)',
  '[x]',
  '["x"]',
  '("`x`")',
  '"s"(x)',
  '"s"[x]',
  '"s" (',
  '"`m`"[',
  ':::c',
  ':::c,d',
  ' -- l --> ',
  ' -- "l" -- ',
  ' -- "" --> ',
  ' <-- l -- ',
  ' <--- l -- ',
  ' --- l --> ',
  '"\'',
  '\'"',
  ' o-- l -- ',
  ' x-- l -- ',
  ' -- l --o ',
  ' -- l --x ',
  'package',
  'rectangle',
  'newpage',
  'skinparam',
  'allowmixing',
  'fill:#fee',
  'stroke-width:2px',
  'a:b c',
  '--x:1',
  '{"a":1}',
  '{"a":{"b":[1,{"c":null}]}}',
  '{"a":1,"a":{"b":2}}',
  '"a":',
  '[1,2]',
  'null',
  '\\"',
  '\\u0041',
  ',\n',
  ',\n\n',
  '\n,',
  '\n,\n',
  '\n\n\n',
  '\n\n,',
  ',\n}',
  ',\n\n}',
  '\n\n}',
  '\n\n\n}',
  '  %% c\n',
  'A %% c',
  '\n%%',
  'x--y',
  'o--o',
  'a--b',
  'a---b',
  'a-->b',
  'a<--b',
  'a--|>b',
  'a..>b',
  'a:::b',
  'a@b',
  'a@{b',
  '<<a',
  'a>>',
  '<<a>>b',
];

// Constructs the specs touch lightly or not at all. Mermaid accepts every one of these.
const SEEDS = [
  'usecase-beta\nactor A, B("Bee")@{ type: hollow } <<Role>>, "C d":::c\nA --> U\n',
  'usecase-beta\nactor A x@--> U(Use it)@{ business: true } <<S>>:::c\nx@{ animate: true }\nclass x,A,U c,d\nstyle x stroke:red\n',
  'usecase-beta\n"A b" --> "C d"\n"`E *f*`" -- "g h" --> I[Rect]\nI <-- "j" -- K\nK o-- l m -- M\nM x-- "n" -- N\nN -- o --o P\nP -- q --x Q\n',
  'usecase-beta\nA ---> B\nB <---- C\nC ----- D\nD -- e ----> F\nF -- g ---- H\nH <-- i ---- J\nJ -- "" --> K\n',
  'usecase-beta\nA ..> : include B\nB ..> : EXTEND C\nC ..> : eXtend D\nC --|> D\nactor E\nactor F\nE --|> F\n',
  'usecase-beta\nsystemBoundary S["T t"]@{\n  type: package,\n}:::c,d\n  %% in\n\n  actor A, B\n  U(Use)\n  "V w"@{ business: true } <<S>>:::c\nend\nS@{ type: rect }\n',
  'usecase-beta\nsystemBoundary "A b"\nend\nA_b@{ type: package }\nnote for U "n"\nnote for U n o p\nU\n',
  'usecase-beta\njson P@{"2":"b","1":"a","n":{"x":[1,{"y":null}],"e":{}},"n":{"z":true}}:::c\njson Q @ {}\nU --> P\nP -- Q\nclassDef c,d fill:#eef,stroke:#369\nstyle P stroke-width:4px\n',
  'usecase-beta\nA@{ type: hollow }\nactor A\nB@{\n  business: true\n\n  , "business": false\n}\nB\nactor C@{ icon: "fa:user" }\nC@{ icon: "x", type: normal }\n',
  'usecase-beta\nclassDef a,b fill:#fff,stroke-width:3px,font-family:Arial Black,-- x:1,a-b:c\\,d,x:"q" 1.5px #f00 50% (a) [b] {c} @ -- !\nA\nclass A a,b\nstyle A stroke-dasharray:5\\,5,fill:red\n',
  'usecase-beta\naccTitle: T t\naccDescr: D d\naccDescr {\n  a\n   b\n}\ndirection TD\ndirection RL\n',
  'usecase-beta\r\nactor A\r\nA --> B\r%% c\r\n',
  'usecase-beta\nA <<x>>\nA <<x>>:::c\nB(l)\nB(l)\nactor C("l")\nactor C("l")\nD <<y>> --> E\nD[D]\n',
  'usecase-beta\n1 --> 2\n1mg --> 3rd\nactor 4("x")\n5[y]\n',
  'usecase-beta\nA e@--> B\ne@{ animation: fast, animate: false }\nB --> C\nD\nC(C)\n',
];

// Text that stops where a statement is not finished, or that Mermaid's lexer gives up on.
const REJECTED = [
  'usecase-beta\nA e@',
  'usecase-beta\nA -->',
  'usecase-beta\nA -- l',
  'usecase-beta\nA <--- l -- B',
  'usecase-beta\nA --- l --> B',
  'usecase-beta\nA <-- l --> B',
  'usecase-beta\nA o-- l --o B',
  'usecase-beta\nA x-- "l" --x B',
  'usecase-beta\nA -- "l" "m" --> B',
  'usecase-beta\nactor',
  'usecase-beta\nA@{',
  'usecase-beta\nA@{ k: v,\n\n}',
  'usecase-beta\nstyle A fill:',
  'usecase-beta\nnote for',
  'usecase-beta\nsystemBoundary S\nA',
  'usecase-beta\nA <<',
  'usecase-beta\nA <<\n\r\nB',
  'usecase-beta\nA "\'\nB',
  'usecase-beta\nA \'"\'" x\nB',
  'usecase-beta\nA "b\nC',
  'usecase-beta\njson P@{"a":',
  'usecase-beta\n"`a',
];

const spec = specCorpus();
const corpus = [...new Set([...loadCorpus('usecase', /usecase-beta/), ...spec.diagrams, ...SEEDS, ...REJECTED])];

function compareTokens(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(referenceTokens(src));
}

async function compare(src: string): Promise<boolean> {
  compareTokens(src);
  const expected = await referenceParse(src);
  const actual = pelersParse(src);
  if (actual.ok !== expected.ok || actual.error !== expected.error) {
    expect(actual.error, `result of ${JSON.stringify(src)}`).toBe(expected.error);
  }
  if (expected.ok) expect(actual.model, `model of ${JSON.stringify(src)}`).toEqual(expected.model);
  return expected.ok;
}

const JSON_SEEDS = [
  '{}',
  '{"a":1,"b":[true,false,null],"c":{"d":"e"}}',
  '{"2":"b","1":"a","n":{"x":[1,{"y":null}],"e":{}},"n":{"z":-1.5e+3}}',
  '{ "s" : "a\\"b\\\\c\\u0041\\n" , "t" : [ [ ] , { } , [ { "k" : 0 } ] ] }',
];
const JSON_FRAGMENTS = [
  '{',
  '}',
  '[',
  ']',
  ',',
  ':',
  '"',
  '"a"',
  '"a":',
  '1',
  '-',
  '0',
  '01',
  '1.',
  '.5',
  '1e',
  'e5',
  '+',
  'true',
  'tru',
  'false',
  'null',
  'nul',
  ' ',
  '\n',
  '\t',
  '\\',
  '\\u00',
  '\\x',
  '\u0001',
  '{}',
  '[]',
  '"a":{}',
  ',,',
  'x',
];

describe('use case parser against the Mermaid parser', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(300);
  });

  it('agrees on every spec and documentation input', async () => {
    let accepted = 0;
    for (const src of corpus) if (await compare(src)) accepted++;
    expect(accepted).toBeGreaterThan(100);
    const rejected: string[] = [];
    for (const src of SEEDS) {
      const result = await referenceParse(src);
      if (!result.ok) rejected.push(`${JSON.stringify(src)}: ${result.error}`);
    }
    expect(rejected).toEqual([]);
  });

  it('lexes the statements the specs use alike', () => {
    for (const src of spec.fragments) compareTokens(src);
  });

  it('agrees on mutated inputs', async () => {
    const count = Number(process.env.FUZZ ?? 3000);
    const rnd = random(Number(process.env.SEED ?? 7));
    const next = mutator(corpus, FRAGMENTS, rnd);
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      let src = next();
      // A statement cut off by the end of the text ends on a different error than one cut off by a line break.
      if (i % 6 === 0) src = src.slice(0, Math.floor(rnd() * (src.length + 1)));
      if (src.length <= 4000 && (await compare(src))) accepted++;
    }
    // The comparison only means something if a fair share of the inputs gets past the parser.
    expect(accepted).toBeGreaterThan(count / 50);
  }, 2_400_000);

  // Engines without a position in their JSON.parse errors make both parsers walk the text
  // themselves to find the fault. V8 gives a position, so this hides it to compare the walkers.
  it('locates invalid JSON alike when the engine does not', async () => {
    const count = Number(process.env.FUZZ ?? 3000) / 2;
    const next = mutator(JSON_SEEDS, JSON_FRAGMENTS, random(Number(process.env.SEED ?? 7) + 2));
    const parse = JSON.parse;
    JSON.parse = ((text: string) => {
      try {
        return parse(text);
      } catch {
        throw new SyntaxError('not valid');
      }
    }) as typeof JSON.parse;
    try {
      for (let i = 0; i < count; i++) await compare(`usecase-beta\njson P@${next()}\n`);
    } finally {
      JSON.parse = parse;
    }
  }, 2_400_000);

  // Mutated text is mostly rejected by the grammar. Whole statements over a handful of ids mostly
  // get through it, and then agree or clash in every way the model builder has to resolve.
  it('agrees on random diagrams built from statements', async () => {
    const count = Number(process.env.FUZZ ?? 3000);
    const rnd = random(Number(process.env.SEED ?? 7) + 3);
    const pick = <X>(list: X[]): X => list[Math.floor(rnd() * list.length)];
    const id = (): string => pick(['A', 'B', 'C', 'D', 'e', 'f', 'S', 'P', 'A_b', '__proto__']);
    const text = (): string => pick(['A', 'A b', 'B', 'x y', 'A-b', '']);
    const arrow = (): string =>
      pick([
        '-->',
        '<--',
        '--',
        '--o',
        'o--',
        '--x',
        'x--',
        '--->',
        '<---',
        '---',
        '--|>',
        '..> : include',
        '..> : extend',
        '-- l -->',
        '-- "l" --',
        '<-- l --',
        'o-- l --',
        'x-- l --',
        '-- l --o',
        '-- l --x',
        '-- "`l`" ---->',
        '-- "" -->',
        '<--- l --',
        '--- l -->',
      ]);
    const meta = (): string =>
      pick([
        'type: hollow',
        'type: normal',
        'type: awesome',
        'type: package',
        'type: rect',
        'icon: "i"',
        'business: true',
        'business: false',
        'animate: true',
        'animate: false',
        'animation: fast',
        'animation: slow',
        'animation: x',
        'k: v',
        '"type": "hollow"',
      ]);
    const suffix = (): string =>
      pick([
        '',
        '',
        '',
        `@{ ${meta()} }`,
        `@{ ${meta()}, ${meta()} }`,
        ' <<S>>',
        ' <<T>>',
        ':::c',
        ':::c,d',
        `@{ ${meta()} } <<S>>:::c`,
      ]);
    // A label is often the id itself, which is what lets two declarations of one element agree.
    const named = (open: string, close: string): string => {
      const name = id();
      return `${name}${open}${pick([name, name, text() || 'l', `"${name}"`])}${close}`;
    };
    const element = (): string =>
      pick([id(), id(), named('(', ')'), named('[', ']'), `"${text()}"`, `"\`${text()}\`"`]) + suffix();
    const actor = (): string => pick([id(), named('(', ')'), named('[', ']'), `"${text()}"`]) + suffix();
    const statements: (() => string)[] = [
      () => `actor ${actor()}`,
      () => `actor ${actor()}, ${actor()}`,
      () => `actor ${actor()} ${arrow()} ${element()}`,
      () => element(),
      () => `${element()} ${arrow()} ${element()}`,
      () => `${element()} ${id()}@${arrow()} ${element()}`,
      () =>
        `systemBoundary ${pick([id(), named('[', ']'), `"${text()}"`])}${pick(['', '', `@{ ${meta()} }`, ':::c'])}\n  ${element()}\n  actor ${actor()}\nend`,
      () => `systemBoundary ${id()}\n  ${element()}\nend`,
      () => `${pick([id(), `"${text()}"`])}@{ ${meta()} }`,
      () => `${id()}@{ ${meta()}, ${meta()} }`,
      () => {
        // A metadata block with every mix of commas and line breaks between and after its properties.
        const gap = (): string =>
          pick(['', ' ', ',', '\n', ',\n', '\n,', '\n\n', ',\n\n', '\n,\n', '\n\n,', '\n\n\n', ', ', ',,']);
        let block = `${pick([id(), `actor ${id()}`, `systemBoundary ${id()}`])}@{${pick(['', ' ', '\n', '\n\n'])}`;
        for (let k = Math.floor(rnd() * 4); k > 0; k--) block += meta() + gap();
        return block + (rnd() < 0.2 ? gap() : '') + '}' + (block.startsWith('system') ? '\nend' : '');
      },
      () => `note for ${id()} ${pick(['"n"', 'n o', '"`n`"'])}`,
      () =>
        `json ${id()}@${pick(['{}', '{"a":1}', '{"b":{"c":[1,2]},"a":null}', '{"a":}', '[1]', '{"a":1,"a":{"b":2}}'])}${pick(['', ':::c'])}`,
      () => `class ${id()}${pick(['', `,${id()}`])} c${pick(['', ',d'])}`,
      () => `style ${id()} ${pick(['fill:red,stroke-width:2px', 'stroke-dasharray:5\\,5,border:1px solid red'])}`,
      () => `classDef ${pick(['c', 'd', 'default', 'c,d'])} fill:#eee,stroke:#333`,
      () => `direction ${pick(['TB', 'TD', 'BT', 'LR', 'RL'])}`,
      () => pick(['accTitle: T', 'accDescr: D', 'accDescr {\n a\n  b\n}', '%% c', '']),
    ];
    let accepted = 0;
    for (let i = 0; i < count; i++) {
      let src = 'usecase-beta\n';
      for (let k = 1 + Math.floor(rnd() * 7); k > 0; k--) src += pick(statements)() + '\n';
      if (await compare(src)) accepted++;
    }
    expect(accepted).toBeGreaterThan(count / 20);
  }, 2_400_000);

  // Mutation rarely joins two tokens without a space, which is where keyword boundaries, operator
  // precedence and the three-token lookahead matter. This strings random pieces together.
  it('agrees on random sequences of tokens', async () => {
    const count = Number(process.env.FUZZ ?? 3000);
    const rnd = random(Number(process.env.SEED ?? 7) + 1);
    const pieces = [...new Set([...FRAGMENTS, ...FRAGMENTS.map((f) => f.trim()).filter((f) => f !== '')])];
    const pick = (): string => pieces[Math.floor(rnd() * pieces.length)];
    const openers = [
      'usecase-beta\n',
      'usecase-beta\nactor ',
      'usecase-beta\nA ',
      'usecase-beta\nA@{',
      'usecase-beta\nsystemBoundary S\n',
      'usecase-beta\nstyle A ',
      'usecase-beta\nA -- ',
      'usecase-beta\njson P@',
      '',
    ];
    for (let i = 0; i < count; i++) {
      let src = openers[Math.floor(rnd() * openers.length)];
      for (let k = 1 + Math.floor(rnd() * 10); k > 0; k--) src += pick() + (rnd() < 0.3 ? ' ' : '');
      await compare(src);
    }
  }, 2_400_000);
});
