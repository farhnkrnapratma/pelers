import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/class-oracle.js';

// Compares Pelers's class diagram lexer and parser with the parser Mermaid generates from
// classDiagram.jison: the same tokens, the same accept or reject decision, and the same calls
// into the model. Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '\n',
  '\r\n',
  '\t',
  ';',
  ':',
  '::',
  ':::',
  ': x',
  ',',
  '.',
  '..',
  '-',
  '--',
  '<|',
  '|>',
  '<',
  '>',
  '<<',
  '>>',
  '*',
  'o',
  ' o ',
  '()',
  '(',
  ')',
  '{',
  '}',
  '{\n',
  '\n}',
  '[',
  ']',
  '[*]',
  '"',
  '""',
  '"x"',
  '`',
  '`a b`',
  '~',
  '~T~',
  '#',
  '%',
  '%%',
  '%% c\n',
  '+',
  '=',
  '!',
  '$',
  '&',
  '?',
  '/',
  '\\',
  '_',
  'A',
  'B',
  'Class01',
  '1',
  'é',
  'ö',
  '中',
  'class ',
  'class',
  'classDiagram',
  'classDiagram-v2',
  'classDef ',
  'cssClass ',
  'style ',
  'namespace ',
  'namespace N {\n',
  'note ',
  'note for ',
  'note for A "x"',
  'link ',
  'click ',
  'callback ',
  'href ',
  'call ',
  'call f()',
  'call f(1, "a")',
  '_self',
  '_blank',
  '_parent',
  '_top',
  'direction TB',
  'direction BT',
  'direction RL',
  'direction LR',
  'direction',
  'accTitle: ',
  'accTitle',
  'accDescr: ',
  'accDescr {',
  'accDescr { x }',
  '<|--',
  '--|>',
  '*--',
  '--*',
  'o--',
  '--o',
  '-->',
  '<--',
  '..>',
  '<..',
  '..|>',
  '<|..',
  '()--',
  '--()',
  '"1" ',
  ' "many"',
  '<<interface>>',
  'fill:#f9f',
  'stroke-width:2px',
  '+foo()',
  '-bar: int',
  'x(y) z$',
  ' \n',
  '}\n',
  'A.B',
  'a-b',
];

// The specs build most diagrams by joining string literals, so the pieces without a header line
// are used twice: with a header put in front, and as fragments for mutation.
const literals = loadCorpus('class', /[^]/);
const pieces = literals.filter((s) => !s.includes('classDiagram'));
const corpus = [
  ...literals.filter((s) => s.includes('classDiagram')),
  ...pieces
    .filter((s) => !/^(?:should|when|given|does|gives|emits|moves|defaults) /.test(s))
    .map((s) => 'classDiagram\n' + s),
];
const fragments = [...FRAGMENTS, ...pieces];

// One input for each way through each rule of the grammar, valid or not.
function tour(): string[] {
  const names = [
    'A',
    'A.B',
    'A-B',
    'A B',
    '`a b`',
    'A`b`',
    'A~T~',
    '`a`~T~',
    'A.`b`~T~',
    'é',
    '1',
    'A.',
    '.A',
    'A~T~~U~',
    '`a`B',
    'o',
    'link',
  ];
  const ends = ['', '<|', '|>', '<', '>', '*', 'o', '()'];
  const out: string[] = [];
  for (const a of names) {
    out.push(
      a,
      `${a} : label`,
      `class ${a}`,
      `namespace ${a} {\nclass X\n}`,
      `<<i>> ${a}`,
      `note for ${a} "n"`,
      `click ${a} href "u"`
    );
    for (const b of ['A', 'B~T~', '`b`']) out.push(`${a} --> ${b}`, `${a} "1" --> "2" ${b} : t`);
  }
  for (const left of ends) {
    for (const line of ['--', '..', '-', '']) {
      for (const right of ends) {
        out.push(`A ${left}${line}${right} B`, `A "1" ${left}${line}${right} "2" B : t`, `A ${left}${line}${right}`);
      }
    }
  }
  const bodies = ['', ' {\n}', '{}', '{ x }', ' {\n+a\n-b()\n}', ' {\n"q"\n}', ' {\n[*]\n}', ' { {', ' {\n+a', ' }'];
  for (const label of ['', '["L"]', '[L]', '["L"']) {
    for (const mid of ['', ':::c', ':::', ' <<i>>', ' <<i', ' : x', ' B']) {
      for (const body of bodies)
        out.push(`class A${label}${mid}${body}`, `namespace N {\nclass A${label}${mid}${body}\n}`);
    }
  }
  const items = [
    'class A',
    'class B {\n+x\n}',
    'note "n"',
    'note for A "n"',
    'namespace M {\nclass C\n}',
    'A --> B',
    '',
  ];
  for (const label of ['', '["L"]']) {
    for (const open of ['{', '{\n', ' {\n\n']) {
      for (const first of items) {
        for (const sep of ['\n', ' ', '\n\n']) {
          for (const second of items) {
            out.push(
              `namespace N${label} ${open}${first}${sep}${second}}`,
              `namespace N${label} ${open}${first}${sep}${second}\n}\nclass Z`
            );
          }
        }
      }
    }
  }
  out.push(
    'namespace A {\nnamespace B {\nnamespace C {\nclass X\n}\n}\nclass Y\n}',
    'namespace A { namespace B { class X } }',
    'namespace A {\nclass X\n}}',
    'namespace namespace A {\nclass X\n}'
  );
  for (const head of [
    'callback A',
    'link A',
    'click A href',
    'click A call f()',
    'click A call f(1, "a")',
    'click A call f',
    'click A',
  ]) {
    for (const tail of [
      '',
      ' "a"',
      ' "a" "b"',
      ' "a" _blank',
      ' "a" "b" _self',
      ' "a" "b" "c"',
      ' _top',
      ' "a" _parent "b"',
    ])
      out.push(head + tail);
  }
  for (const head of [
    'style A',
    'style',
    'classDef a',
    'classDef a,b',
    'classDef a,',
    'classDef',
    'cssClass "A"',
    'cssClass "A,B"',
    'cssClass A',
  ]) {
    for (const tail of [
      '',
      ' fill:#f9f',
      ' fill:#f9f,stroke:#333,stroke-width:4px',
      ' x,y',
      ' c',
      ' x:1;',
      ' a % # : style',
      ' "s"',
    ])
      out.push(head + tail);
  }
  for (const text of ['"n"', '', 'n', '"a" "b"', '""'])
    out.push(`note ${text}`, `note for A ${text}`, `note for ${text}`, `notefor A ${text}`);
  for (const line of [
    'direction TB',
    'direction BT',
    'direction RL',
    'direction LR',
    'direction TD',
    'x direction LR y',
    'direction\nTB',
    'direction LR direction TB',
    'accTitle: t',
    'accTitle:',
    'accTitle : t',
    'accDescr: d',
    'accDescr { a\nb }',
    'accDescr {}',
    'accDescr { a',
    'accTitle t',
    '<<i>> A',
    '<<i>>',
    '<<i> A',
    '<< i >> A~T~',
    '%% c',
    'A %% c',
    '[*] --> A',
    'A --> [*]',
    'call f()',
    'A::B',
    'A:::c',
    'A;',
  ]) {
    out.push(line, `${line}\nclass A`, `class A\n${line}`);
  }
  return [...out.map((s) => 'classDiagram\n' + s), ...out.map((s) => 'classDiagram\n' + s + '\n'), ...out];
}

function compare(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('class diagram parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(500);
  });

  it('agrees on every spec and documentation input', () => {
    for (const src of corpus) compare(src);
  });

  it('agrees on a tour of the grammar', () => {
    for (const src of tour()) compare(src);
  });

  it('agrees on inputs with no header line', () => {
    for (const src of ['A~x', 'class A\n"', 'A <|-- B\nnote "', 'click A call f', 'accTitle:', 'A\n', 'A'])
      compare(src);
  });

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator([...corpus, ...tour()], fragments, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) compare(src);
    }
  }, 600_000);
});
