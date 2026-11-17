import * as reference from '@mermaid-js/parser';
import type { TokenType } from '../../src/diagrams/common/tokens.js';
import { ABNF_TOKENS, parseAbnf } from '../../src/diagrams/railroad/abnf.js';
import { EBNF_TOKENS, parseEbnf } from '../../src/diagrams/railroad/ebnf.js';
import { IR_TOKENS, parseIr } from '../../src/diagrams/railroad/ir.js';
import { PEG_TOKENS, parsePeg } from '../../src/diagrams/railroad/peg.js';
import { loadCorpus } from './corpus.js';
import { patchTitle, patchTokens } from './token-patch.js';

for (const [module, rest] of [
  ['RailroadModule', String.raw`[\w-]`],
  ['RailroadEbnfModule', String.raw`[\w-]`],
  ['RailroadAbnfModule', '[A-Za-z0-9-]'],
  ['RailroadPegModule', String.raw`[\w-]`],
]) {
  patchTokens(module, (tokens) =>
    patchTitle(tokens, rest, 'TITLE', String.raw`title(?:[\t ][^\n\r]*?(?=%%)|[\t ][^\n\r]*|)`)
  );
}

// The four railroad grammars, each with what the differential tests need: Mermaid's services and
// parser name, Pelers's token table and parser, and three sources of test input.

type Rnd = () => number;

export interface Notation {
  name: string;
  keyword: string;
  // The service factory and key in `@mermaid-js/parser`, and the name its `parse` takes.
  create: string;
  key: string;
  type: string;
  tokens: readonly TokenType[];
  parse: (src: string) => unknown;
  fragments: string[];
  seeds: string[];
  // A well-formed rule of random shape.
  rule: (rnd: Rnd) => string;
}

function pick<X>(rnd: Rnd, list: readonly X[]): X {
  return list[Math.floor(rnd() * list.length)];
}

function several(rnd: Rnd, max: number, make: () => string, joints: readonly string[]): string {
  const parts: string[] = [];
  for (let i = 1 + Math.floor(rnd() * max); i > 0; i--) parts.push(make());
  return parts.join(pick(rnd, joints));
}

const STRINGS = [
  '"a"',
  "'b'",
  '"x y"',
  '"\\n"',
  '"\\""',
  '"q\\\\"',
  '""',
  '"+"',
  '"é"',
  "'it\\'s'",
  '"a  b"',
  '"?"',
  '";"',
  '"(*"',
];
const NAMES = [
  'r',
  'rule',
  'my_rule',
  'a-b',
  'A1',
  'terminals',
  'x_',
  'choice2',
  'accTitles',
  'digit',
  'Expr',
  'b',
  'c',
  'd',
  'e',
  'f',
  'g',
  'h',
  'i',
  'titles',
  'title-x',
];

function irExpression(rnd: Rnd, depth: number): string {
  const r = rnd();
  if (depth <= 0 || r < 0.4) return `${pick(rnd, ['terminal', 'nonterminal', 'special'])}(${pick(rnd, STRINGS)})`;
  if (r < 0.75) {
    return `${pick(rnd, ['sequence', 'choice'])}(${several(rnd, 3, () => irExpression(rnd, depth - 1), [', ', ',', ' , ', ',\n  '])})`;
  }
  return `${pick(rnd, ['optional', 'oneOrMore', 'zeroOrMore'])}( ${irExpression(rnd, depth - 1)} )`;
}

function ebnfPrimary(rnd: Rnd, depth: number): string {
  const r = rnd();
  if (depth <= 0 || r < 0.78)
    return rnd() < 0.1 ? pick(rnd, ['? any ?', '?x?', '? two words ?']) : pick(rnd, rnd() < 0.5 ? STRINGS : NAMES);
  const [open, close] = pick(rnd, [
    ['( ', ' )'],
    ['[ ', ' ]'],
    ['{ ', ' }'],
    ['(', ')'],
  ]);
  return open + ebnfChoice(rnd, depth - 1) + close;
}

function ebnfTerm(rnd: Rnd, depth: number): string {
  let s = ebnfPrimary(rnd, depth);
  while (rnd() < 0.25)
    s += pick(rnd, ['?', '*', '+', ' - ' + ebnfPrimary(rnd, depth - 1), '-' + ebnfPrimary(rnd, depth - 1)]);
  return s;
}

function ebnfChoice(rnd: Rnd, depth: number): string {
  return several(rnd, 3, () => several(rnd, 3, () => ebnfTerm(rnd, depth), [' ', ' , ', ',', ' (* c *) ']), [
    ' | ',
    '|',
  ]);
}

function abnfAlternation(rnd: Rnd, depth: number): string {
  const element = (): string => {
    const repeat = pick(rnd, ['', '', '', '*', '1*', '2*3', '3', '*4', '0*1', '1', '0']);
    if (depth <= 0 || rnd() < 0.78)
      return repeat + pick(rnd, ['"s"', '"a b"', '""', '%x41', '%d1.2', '%b0-1', 'name', 'DIGIT', 'rule-name', 'a1']);
    const [open, close] = pick(rnd, [
      ['( ', ' )'],
      ['[ ', ' ]'],
      ['(', ')'],
    ]);
    return repeat + open + abnfAlternation(rnd, depth - 1) + close;
  };
  return several(rnd, 3, () => several(rnd, 3, element, [' ', '  ', '\n  ']), [' / ', '/']);
}

function pegChoice(rnd: Rnd, depth: number): string {
  const prefix = (): string => {
    const head = pick(rnd, ['', '', '', '&', '!']);
    const tail = pick(rnd, ['', '', '', '?', '*', '+']);
    if (depth <= 0 || rnd() < 0.78) return head + (rnd() < 0.1 ? '.' : pick(rnd, rnd() < 0.5 ? STRINGS : NAMES)) + tail;
    return `${head}( ${pegChoice(rnd, depth - 1)} )${tail}`;
  };
  return several(rnd, 3, () => several(rnd, 3, prefix, [' ', '  ', ' # c\n  ']), [' / ', '/']);
}

const COMMON = [
  'title',
  'title ',
  'title "T"',
  "title 'T'",
  'title "a\\"b"',
  'title "',
  'accTitle: ',
  'accTitle',
  'accDescr: ',
  'accDescr {',
  'accDescr{x}',
  '}',
  '\n',
  '\r\n',
  ' ',
  '\t',
  '%%',
  '%% c',
  '%%{init: {}}%%',
  '---',
  '---\n',
  '---\nx: 1\n---\n',
  '\\',
  '\\"',
  "\\'",
  '\\n',
  '"',
  "'",
  '"a"',
  "'b'",
  '""',
  '"a b"',
  '"\\t\\\\"',
  ';',
  ' ;',
  ';\n',
  '=',
  '(',
  ')',
  ',',
  'x',
  'é',
  '#35;',
  '-',
  '_',
  '0',
  '9',
  'a-b',
  'A_1',
  '/*',
  '*/',
  '/* c */',
  'titles',
  'title-x',
  'title_',
];

export const NOTATIONS: Notation[] = [
  {
    name: 'railroad',
    keyword: 'railroad-beta',
    create: 'createRailroadServices',
    key: 'Railroad',
    type: 'railroad',
    tokens: IR_TOKENS,
    parse: parseIr,
    fragments: [
      ...COMMON,
      'railroad-beta',
      'terminal',
      'nonterminal',
      'special',
      'sequence',
      'choice',
      'optional',
      'oneOrMore',
      'zeroOrMore',
      'terminal("a")',
      'nonterminal("b")',
      'special("c")',
      'sequence(',
      'choice(',
      'optional(',
      'oneOrMore(',
      'zeroOrMore(',
      'terminalx',
      'xterminal',
      'choice1',
      'r = terminal("a") ;',
      ', ',
      '))',
      '"),',
    ],
    seeds: [
      'railroad-beta',
      'railroad-beta\ntitle Example Grammar\naccTitle: Accessible Railroad\naccDescr: Railroad description\nrule = terminal("a") ;',
      'railroad-beta\nr1 = optional(terminal("a")) ;\nr2 = oneOrMore(terminal("b")) ;\nr3 = zeroOrMore(terminal("c")) ;',
      'railroad-beta\n/* comment before the first rule */\nrule = terminal("a") ;\nnext = nonterminal("rule") ;',
      'railroad-beta\naccDescr {\n  two\n  lines\n}\ntitle "Quoted \\"title\\"" %% c\nr = special(\'x\\ty\') ; %% d\n',
      'railroad-beta title x\ntitles = sequence(choice(terminal("a")), optional(oneOrMore(zeroOrMore(nonterminal("n"))))) ;',
      '---\ntitle: x\n---\nrailroad-beta\nr = choice(terminal("a"), sequence(terminal("b"), terminal("c")), special("d")) ;',
    ],
    rule: (rnd) => `${pick(rnd, NAMES)} = ${irExpression(rnd, 4)} ;`,
  },
  {
    name: 'EBNF',
    keyword: 'railroad-ebnf-beta',
    create: 'createRailroadEbnfServices',
    key: 'RailroadEbnf',
    type: 'railroadEbnf',
    tokens: EBNF_TOKENS,
    parse: parseEbnf,
    fragments: [
      ...COMMON,
      'railroad-ebnf-beta',
      '::=',
      '|',
      '[',
      ']',
      '{',
      '}',
      '?',
      '*',
      '+',
      ' - ',
      '? x ?',
      '?  ?',
      '??',
      '? a; ?',
      '?x?',
      '(*',
      '*)',
      '(* c *)',
      '(**)',
      '(*)',
      '( a )',
      '[ a ]',
      '{ a }',
      'a?',
      'a*',
      'a+',
      'a - b',
      'a , b',
      'a | b',
      'r = a ;',
      'r ::= "a" ;',
      '- (',
      '-- ',
      '---x',
    ],
    seeds: [
      'railroad-ebnf-beta',
      'railroad-ebnf-beta\nr = a , b , c ;\ns ::= "x" | [ y ] | { z } | ( a b ) ;',
      'railroad-ebnf-beta\nr = ? any character ? - "x" ;\ns = a - ( b | c )* - d? ;',
      'railroad-ebnf-beta\n(* iso *) /* w3c */ %% mermaid\nr = a? b* c+ ;',
      'railroad-ebnf-beta\naccDescr {\n  d\n}\nr = { a } ;\naccDescr { e',
      'railroad-ebnf-beta\nr = a? b? ;\ns = a ? b ;',
      'railroad-ebnf-beta\ntitle "T"\nr = "\\"" \'\\\'\' "\\n" ;',
    ],
    rule: (rnd) => `${pick(rnd, NAMES)} ${pick(rnd, ['=', '::='])} ${ebnfChoice(rnd, 3)} ;`,
  },
  {
    name: 'ABNF',
    keyword: 'railroad-abnf-beta',
    create: 'createRailroadAbnfServices',
    key: 'RailroadAbnf',
    type: 'railroadAbnf',
    tokens: ABNF_TOKENS,
    parse: parseAbnf,
    fragments: [
      ...COMMON,
      'railroad-abnf-beta',
      '/',
      '[',
      ']',
      '*',
      '1*',
      '*2',
      '2*4',
      '3',
      '03',
      '%x41',
      '%d65',
      '%b1000001',
      '%x30-39',
      '%x0D.0A',
      '%x',
      '%X4f.',
      '%x1-',
      '; comment',
      ';x',
      '; ',
      ' ;\n',
      '*a',
      '1*( a )',
      '[ a ]',
      '( a / b )',
      'a / b',
      'r = a ;\n',
      'r = "a" ;',
      'rule-name',
      '2DIGIT',
      '**',
      '1*2*3',
    ],
    seeds: [
      'railroad-abnf-beta',
      'railroad-abnf-beta\nr = 2*4a 3b *c 1*d *5e [ f ] ( g / h ) ;\ns = %x41 %d65.66 %b1-0 "str" ;',
      'railroad-abnf-beta\nr = a ; a comment\n  / b ;\n',
      'railroad-abnf-beta\ntitle "T"\naccTitle: at\nr = "a" ;\n; trailing comment',
      'railroad-abnf-beta\nr = *( a / "b" ) 1*2[ c ] ;\n',
    ],
    rule: (rnd) =>
      `${pick(rnd, ['r', 'rule', 'a-b', 'A1', 'titles'])} = ${abnfAlternation(rnd, 3)} ;${pick(rnd, ['', '', '', ' c'])}`,
  },
  {
    name: 'PEG',
    keyword: 'railroad-peg-beta',
    create: 'createRailroadPegServices',
    key: 'RailroadPeg',
    type: 'railroadPeg',
    tokens: PEG_TOKENS,
    parse: parsePeg,
    fragments: [
      ...COMMON,
      'railroad-peg-beta',
      '<-',
      '<',
      '/',
      '&',
      '!',
      '?',
      '*',
      '+',
      '.',
      '# comment',
      '#x',
      '#',
      '&a',
      '!a',
      '!(',
      '&"x"',
      'a?',
      'a*',
      'a+',
      '( a )',
      'a / b',
      'R <- a ;',
      'R <- "a" ;',
      '!.',
      '&&',
      '??',
      '..',
    ],
    seeds: [
      'railroad-peg-beta',
      'railroad-peg-beta\nR <- &a !b c? d* e+ . ;\nS <- ( a / b ) / !( c d )* ;',
      'railroad-peg-beta\n# a comment\nR <- "a" # another\n  / \'b\' ;',
      'railroad-peg-beta\ntitle "T"\nEnd <- !. ;\nAny <- &"x" . ;',
    ],
    rule: (rnd) => `${pick(rnd, NAMES)} <- ${pegChoice(rnd, 3)} ;`,
  },
];

const HEADERS = [
  'title T\n',
  'title "Quoted \\" t"\n',
  "title 'q'\n",
  'accTitle: a  b\n',
  'accDescr: d\n',
  'accDescr {\n multi\n line\n}\n',
  'title\n',
];

// Well-formed text of random shape in one notation.
export function grammar(notation: Notation, rnd: Rnd): string {
  let s = notation.keyword + pick(rnd, ['\n', ' ', '\n\n']);
  while (rnd() < 0.25) s += pick(rnd, HEADERS);
  for (let i = Math.floor(rnd() * 4); i > 0; i--) s += notation.rule(rnd) + pick(rnd, ['\n', '\n\n', '\n%% c\n']);
  return s;
}

// Token-level pieces in a random order, most of them well-formed on their own.
export function pieces(notation: Notation, rnd: Rnd): string {
  let s = rnd() < 0.9 ? notation.keyword + pick(rnd, ['\n', ' ', '\n\n', '']) : '';
  for (let i = 1 + Math.floor(rnd() * 14); i > 0; i--)
    s += pick(rnd, notation.fragments) + pick(rnd, ['', ' ', ' ', '\n']);
  return s;
}

const RE_KEYWORD = /railroad-(?:ebnf-|abnf-|peg-)?beta/;
let loaded: string[] | undefined;

// The notation's own spec and documentation inputs, then its seeds, then the other notations'
// inputs under this keyword, which exercise the error paths and the shared constructs.
export function corpusFor(notation: Notation): { own: string[]; corpus: string[] } {
  loaded ??= loadCorpus('railroad', RE_KEYWORD);
  const own = loaded.filter((src) => src.includes(notation.keyword));
  const borrowed = loaded
    .filter((src) => !src.includes(notation.keyword))
    .map((src) => src.replace(RE_KEYWORD, notation.keyword));
  return { own, corpus: [...own, ...notation.seeds, ...borrowed] };
}

// Names of each token type's LONGER_ALT types in the reference lexer, in table order.
export function longerAlternatives(notation: Notation): string[][] {
  type Token = { name: string; LONGER_ALT?: Token | Token[] };
  type Services = Record<string, { parser: { Lexer: { chevrotainLexer: { lexerDefinition: Token[] } } } }>;
  const services = (reference as unknown as Record<string, () => Services>)[notation.create]()[notation.key];
  return services.parser.Lexer.chevrotainLexer.lexerDefinition.map((t) =>
    [t.LONGER_ALT ?? []].flat().map((alt) => alt.name)
  );
}
