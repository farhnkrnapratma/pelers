import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, pelersParse, pelersTokens } from '../support/state-oracle.js';

// Compares Pelers's state diagram lexer and parser with the parser Mermaid generates from
// stateDiagram.jison: the same tokens, the same accept or reject decision, and the same calls
// into the model. Set FUZZ to raise the number of mutated inputs.

const FRAGMENTS = [
  ' ',
  '\n',
  '\n\n',
  '\t',
  '\r\n',
  '\r',
  '\u2028',
  '\u00a0',
  ' \n',
  ';',
  ':',
  '::',
  ':::',
  ' : ',
  '-->',
  '--',
  '-',
  '->',
  '[*]',
  '[*] --> ',
  ' --> [*]',
  '{',
  '}',
  ' {\n',
  '\n}\n',
  '{ ',
  ' }',
  '"',
  '""',
  '"a b"',
  '%%',
  '%%{',
  '%% c\n',
  '#',
  '# c',
  'state ',
  'state',
  'State ',
  'STATE ',
  'state "',
  '" as ',
  ' as ',
  'as ',
  'as',
  'AS ',
  'note ',
  'note',
  'Note ',
  'note left of ',
  'note right of ',
  'left of',
  'right of ',
  'end note',
  '\nend note',
  '\n  end note\n',
  'End Note',
  'note "',
  '<<fork>>',
  '<<join>>',
  '<<choice>>',
  '[[fork]]',
  '[[join]]',
  '[[choice]]',
  ' <<FORK>>',
  '<<',
  '>>',
  'direction ',
  'direction TB',
  'direction BT',
  'direction RL',
  'direction LR',
  'Direction lr',
  'direction\nTB',
  'direction TD',
  'direction',
  'TB',
  'LR',
  'scale ',
  'scale 350 width',
  ' width',
  'width',
  '350',
  'accTitle: ',
  'accDescr: ',
  'accDescr { ',
  'accDescr {\n',
  'acctitle : ',
  'classDef ',
  'classDef c fill:#f00',
  'classdef ',
  'class ',
  'class a,b c',
  'Class ',
  'style ',
  'style a fill:#f00',
  'Style ',
  'default',
  'default ',
  'DEFAULT ',
  'click ',
  'click a "u" "t"',
  'click a href "u"',
  'href ',
  'href',
  'hide empty description',
  'Hide Empty Description',
  'hide empty',
  'stateDiagram',
  'stateDiagram ',
  'stateDiagram-v2',
  'stateDiagram-v2\n',
  'STATEDIAGRAM\n',
  'a',
  'b',
  'S1',
  'id_1',
  'nl',
  'é',
  'ö',
  '1',
  ',',
  ', ',
  'fill:#f9f',
  'stroke-width:2px',
  'a:b',
  'a::b',
  'x%%y',
  'x%%{y',
  ' %%',
  '.',
  '*',
  '[',
  ']',
  '<',
  '>',
  '/',
  '\\',
  '(',
  ')',
];

const corpus = loadCorpus('state', /stateDiagram/);

function compare(src: string): void {
  expect(pelersTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = pelersParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('state diagram parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(100);
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
});
