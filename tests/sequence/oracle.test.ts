import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, peleParse, peleTokens } from '../support/sequence-oracle.js';

// Compares Pele's sequence lexer and parser with the parser Mermaid generates from
// sequenceDiagram.jison: the same tokens, the same accept or reject decision, and the same calls
// into the model. Set FUZZ to raise the number of mutated inputs and SEED to vary them.

const FRAGMENTS = [
  ' ',
  '\n',
  ';',
  ',',
  ':',
  '#',
  '%%',
  '%',
  '@',
  '@{',
  '}',
  '{',
  '"',
  "'",
  '+',
  '-',
  '--',
  '()',
  '(',
  ')',
  '<',
  '>',
  '/',
  '\\',
  '|',
  '->>',
  '-->>',
  '->',
  '-->',
  '<<->>',
  '<<-->>',
  '-x',
  '--x',
  '-X',
  '-)',
  '--)',
  '-|\\',
  '-|/',
  '-\\\\',
  '-//',
  '--|\\',
  '--|/',
  '--\\\\',
  '--//',
  '/|-',
  '\\|-',
  '//-',
  '\\\\-',
  '/|--',
  '\\|--',
  '//--',
  '\\\\--',
  'participant ',
  'actor ',
  'create ',
  'destroy ',
  'box ',
  'end',
  'end\n',
  'loop ',
  'rect ',
  'opt ',
  'alt ',
  'else ',
  'par ',
  'par_over ',
  'and ',
  'critical ',
  'option ',
  'break ',
  'note ',
  'Note ',
  'left of ',
  'right of ',
  'over ',
  'links ',
  'link ',
  'properties ',
  'details ',
  'activate ',
  'deactivate ',
  'autonumber',
  'autonumber ',
  'off',
  ' off\n',
  '1',
  '10 ',
  '.5 ',
  '1.25\n',
  '10.001',
  'title ',
  'title: ',
  'title\n',
  'accTitle: ',
  'accDescr: ',
  'accDescr { ',
  'accDescr{',
  'sequenceDiagram',
  'sequenceDiagram\n',
  ' as ',
  ' AS ',
  'as',
  'wrap: ',
  'nowrap: ',
  ':wrap:',
  'A',
  'B',
  'Alice',
  'Bob',
  'x',
  'é',
  'ö',
  '日本',
  '\t',
  '\r\n',
  ' \n',
  '\u2028',
  '\u00a0',
  'rgb(0, 255, 0)',
  'rgba(0,0,0,0.1)',
  'green ',
  'transparent',
  '{ "type": "database" }',
  '"alias": "x"',
  '<br/>',
  'A->>B: hi',
  'A-->>-B: ok\n',
  'A->>+B: go\n',
  'A()->>()B: c\n',
  '%%{',
  'PARTICIPANT ',
  'End',
  'LOOP ',
  'Link',
  'end-user',
  'endpoint',
  '12',
  '.',
  '.5',
  '0.25',
  '\r',
  '\f',
  '\v',
  '_',
  'e',
  's',
  'of',
  'left',
  'right',
  'note',
  'over',
  'link',
  'links',
  'title',
  'box',
  'loop',
  'par',
  'opt',
  'wrap',
  'no',
  'acc',
  'accTitle',
  'accDescr',
  'Title',
  'Descr',
  '-|',
  '/|',
  '\\|',
  '//',
  '\\\\',
  '<<',
  '>>',
  '<<-',
  '->>()',
  '#35;',
  '\ufeff',
  '\u017f',
  '\u212a',
];

const SLOTS: Record<string, string[]> = {
  actor: [
    'A',
    'B',
    'Alice',
    'Bob',
    'a b',
    'A-B',
    'end-user',
    'link',
    'Link',
    'x',
    'é',
    '1',
    '2.5',
    'A#1',
    'loop1',
    'as',
  ],
  arrow: [
    '->>',
    '-->>',
    '->',
    '-->',
    '<<->>',
    '<<-->>',
    '-x',
    '--x',
    '-)',
    '--)',
    '-|\\',
    '-|/',
    '-\\\\',
    '-//',
    '--|\\',
    '--|/',
    '--\\\\',
    '--//',
    '/|-',
    '\\|-',
    '//-',
    '\\\\-',
    '/|--',
    '\\|--',
    '//--',
    '\\\\--',
  ],
  mark: ['+', '-', '()'],
  central: ['()'],
  txt: [': hi', ':', ' : a b', ': wrap: x', ':nowrap:y', ': a # b', ': a; b', ': <br/>x', ': -x'],
  kwId: ['participant ', 'actor ', 'destroy ', 'activate ', 'deactivate ', 'Participant '],
  create: ['create '],
  as: [' as ', ' AS ', ' as', 'as ', '\tas\t', ' as\n'],
  text: ['Name', 'a b c', '', 'x # y', 'rgb(0, 0, 255) T', 'green', 'wrap: t', 'Aqua x'],
  config: ['@{ "type": "queue" }', '@{type: actor}', ' @{ "alias": "x" }', '@{}', '@{ a', '@{ "type": "x" } '],
  note: ['note ', 'Note '],
  place: ['left of ', 'right of ', 'over ', 'Left Of '],
  over: ['over '],
  comma: [',', ', ', ' , '],
  block: ['loop ', 'rect ', 'opt ', 'alt ', 'par ', 'par_over ', 'critical ', 'break ', 'box ', 'loop', 'alt\n'],
  divider: ['else ', 'and ', 'option ', 'else'],
  end: ['end', 'End', 'end '],
  autonumber: ['autonumber', 'autonumber ', 'Autonumber '],
  num: ['1', '10', '.5', '2.25', '1.234', '1 ', '3\t'],
  off: ['off', ' off', 'OFF'],
  menu: ['links ', 'link ', 'properties ', 'details ', 'link\t', 'links  '],
  misc: ['title T', 'title: T', 'accTitle: t', 'accDescr: d', 'accDescr { a\n b }', '%% c', '# c', 'x%%y'],
};
const TEMPLATES = [
  'actor arrow actor txt',
  'actor arrow actor txt',
  'actor arrow mark actor txt',
  'actor central arrow actor txt',
  'actor central arrow central actor txt',
  'kwId actor',
  'kwId actor as text',
  'kwId actor config',
  'kwId actor config as text',
  'create kwId actor',
  'create kwId actor as text',
  'note place actor txt',
  'note over actor comma actor txt',
  'block text',
  'block text',
  'end',
  'end',
  'divider text',
  'autonumber',
  'autonumber num',
  'autonumber num num',
  'autonumber off',
  'menu actor txt',
  'misc',
  // Near misses: one part too many, too few, or out of place.
  'actor central arrow mark actor txt',
  'actor arrow mark mark actor txt',
  'actor mark arrow actor txt',
  'actor arrow txt',
  'actor txt',
  'actor arrow actor',
  'note place actor comma actor txt',
  'note actor txt',
  'kwId actor as text config',
  'kwId config',
  'kwId actor comma actor',
  'create actor arrow actor txt',
  'create block text',
  'autonumber off num',
  'autonumber num off',
  'autonumber num num num',
  'menu txt',
  'menu actor',
  'menu actor arrow actor txt',
  'block text end',
  'divider text end',
  'end end',
].map((t) => t.split(' '));
const SLOT_NAMES = Object.keys(SLOTS);

const corpus = loadCorpus('sequence', /sequenceDiagram/i);

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('sequence parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(150);
  });

  it('agrees on every spec and documentation input', () => {
    for (const src of corpus) compare(src);
  });

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) compare(src);
    }
  }, 600_000);

  // Mutation keeps most of each input well formed. Strings of fragments put every token next to
  // every other, which is where the lookaheads and word boundaries of the lexer rules decide.
  it('agrees on strings of fragments', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 1);
    for (let i = 0; i < count; i++) {
      let src = rnd() < 0.8 ? 'sequenceDiagram\n' : '';
      for (let k = 2 + Math.floor(rnd() * 14); k > 0; k--) src += FRAGMENTS[Math.floor(rnd() * FRAGMENTS.length)];
      compare(src);
    }
  }, 600_000);

  // Statements built from the grammar's own shapes, with parts swapped, dropped, and repeated.
  it('agrees on generated statements', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 2);
    const pick = <X>(list: X[]): X => list[Math.floor(rnd() * list.length)];
    for (let i = 0; i < count; i++) {
      let src = 'sequenceDiagram\n';
      for (let lines = 1 + Math.floor(rnd() * 6); lines > 0; lines--) {
        for (const slot of pick(TEMPLATES)) {
          const r = rnd();
          if (r < 0.06) continue;
          src += pick(SLOTS[r < 0.18 ? pick(SLOT_NAMES) : slot]);
          if (r > 0.94) src += pick(SLOTS[pick(SLOT_NAMES)]);
          if (rnd() < 0.3) src += ' ';
        }
        src += rnd() < 0.85 ? '\n' : pick([';', ' \n', '', '\n\n', ' ']);
      }
      compare(src);
    }
  }, 600_000);
});
