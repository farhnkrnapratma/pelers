import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/common/tokens.js';
import { ABNF_TOKENS } from '../../src/diagrams/railroad/abnf.js';
import { EBNF_TOKENS } from '../../src/diagrams/railroad/ebnf.js';
import { IR_TOKENS } from '../../src/diagrams/railroad/ir.js';
import { PEG_TOKENS } from '../../src/diagrams/railroad/peg.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, part: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => part(i)).join('');
const lex = (types: typeof IR_TOKENS, src: string) => tokenize(src, types, 'railroad');

const IR = 'railroad-beta\n';
const EBNF = 'railroad-ebnf-beta\n';
const ABNF = 'railroad-abnf-beta\n';
const PEG = 'railroad-peg-beta\n';

const CASES: [string, () => unknown][] = [
  [
    'special sequence opener before a long run with no closer',
    () => lex(EBNF_TOKENS, EBNF + 'r = ?' + ' a'.repeat(N / 2)),
  ],
  ['question marks separated by blanks', () => lex(EBNF_TOKENS, EBNF + 'r = ' + '?  '.repeat(N / 3))],
  [
    'question marks each followed by a long run and a semicolon',
    () => lex(EBNF_TOKENS, EBNF + ('? ' + 'a '.repeat(50) + ';').repeat(N / 103)),
  ],
  ['unclosed ISO comment openers', () => lex(EBNF_TOKENS, EBNF + 'r = ' + '(* a '.repeat(N / 5))],
  ['ISO comment openers that never get a full closer', () => lex(EBNF_TOKENS, EBNF + 'r = ' + '(*)'.repeat(N / 3))],
  ['unclosed block comment openers', () => lex(IR_TOKENS, IR + '/* a '.repeat(N / 5))],
  ['block comment open and close, many times', () => lex(EBNF_TOKENS, EBNF + '/**/(**)'.repeat(N / 8))],
  ['braced descriptions that are never closed', () => lex(EBNF_TOKENS, EBNF + 'accDescr { '.repeat(N / 11))],
  [
    'descriptions with long blank runs and no brace',
    () => lex(EBNF_TOKENS, EBNF + ('accDescr' + ' '.repeat(92)).repeat(N / 100)),
  ],
  [
    'one-line descriptions with no comment marker',
    () => lex(IR_TOKENS, IR + ('accDescr: ' + '% '.repeat(45) + '\n').repeat(N / 101)),
  ],
  ['accTitle with no colon, many times', () => lex(PEG_TOKENS, PEG + ('accTitle' + ' '.repeat(42)).repeat(N / 50))],
  ['one long title with single percent signs', () => lex(IR_TOKENS, IR + 'title ' + 'a % '.repeat(N / 4))],
  ['title words on one line', () => lex(ABNF_TOKENS, ABNF + 'title '.repeat(N / 6))],
  ['titles with no space after them', () => lex(IR_TOKENS, IR + 'titlea'.repeat(N / 6))],
  ['unclosed double quote', () => lex(IR_TOKENS, IR + 'r = terminal("' + 'a\\"'.repeat(N / 3))],
  [
    'quotes that open and never close, with backslashes',
    () => lex(PEG_TOKENS, PEG + 'R <- ' + "'" + '\\\\'.repeat(N / 2)),
  ],
  ['unclosed ABNF string', () => lex(ABNF_TOKENS, ABNF + 'r = "' + 'a '.repeat(N / 2))],
  ['front matter fences that do not close', () => lex(EBNF_TOKENS, EBNF + 'r = a ' + '---\n'.repeat(N / 4))],
  ['directive openers that do not close', () => lex(IR_TOKENS, IR + '%%{ a '.repeat(N / 6))],
  ['dashes', () => lex(EBNF_TOKENS, EBNF + 'r = a ' + '-'.repeat(N))],
  ['digits with no star', () => lex(ABNF_TOKENS, ABNF + 'r = ' + '1'.repeat(N) + 'a ;')],
  ['digit runs each with a star', () => lex(ABNF_TOKENS, ABNF + 'r = ' + '12*34 '.repeat(N / 6))],
  ['numeric values with long tails', () => lex(ABNF_TOKENS, ABNF + 'r = %x' + '41.'.repeat(N / 3) + ' ;')],
  ['semicolon comments', () => lex(ABNF_TOKENS, ABNF + '; a comment\n'.repeat(N / 12))],
  ['hash comments', () => lex(PEG_TOKENS, PEG + '# a comment\n'.repeat(N / 12))],
  ['keywords glued into one name', () => lex(IR_TOKENS, IR + 'terminal'.repeat(N / 8))],
  ['one 50,000 character name', () => render(EBNF + 'a'.repeat(N) + ' = ' + 'b'.repeat(N) + ' ;', big)],
  ['one 50,000 character terminal', () => render(EBNF + 'r = "' + 'word '.repeat(N / 5) + '" ;', big)],
  ['terminal of entity codes and markup', () => render(EBNF + 'r = "' + '#35;&amp;<br>*_'.repeat(N / 15) + '" ;', big)],
  ['terminal of escapes', () => render(PEG + 'R <- "' + '\\n\\t\\\\'.repeat(N / 6) + '" ;', big)],
  ['25,000 nested groups', () => render(EBNF + 'r = ' + '('.repeat(N / 2) + 'a' + ')'.repeat(N / 2) + ' ;', big)],
  ['12,000 nested optionals', () => render(EBNF + 'r = ' + '['.repeat(12000) + 'a' + ']'.repeat(12000) + ' ;', big)],
  ['12,000 nested repetitions', () => render(EBNF + 'r = ' + '{'.repeat(12000) + 'a' + '}'.repeat(12000) + ' ;', big)],
  ['50,000 postfix operators', () => render(EBNF + 'r = a' + '*+?'.repeat(N / 3) + ' ;', big)],
  ['chain of 12,000 exceptions', () => render(EBNF + 'r = a' + ' - b'.repeat(12000) + ' ;', big)],
  [
    'exceptions nested 8,000 deep',
    () => render(EBNF + 'r = a' + ' - ( b'.repeat(8000) + ' )'.repeat(8000) + ' ;', big),
  ],
  ['unclosed brackets', () => parse(EBNF + 'r = ' + '([{'.repeat(N / 3), big)],
  ['choice of 12,000 alternatives', () => render(EBNF + 'r = ' + repeat(12000, (i) => `a${i} | `) + 'z ;', big)],
  ['sequence of 12,000 elements', () => render(EBNF + 'r = ' + 'a '.repeat(12000) + ';', big)],
  [
    'choices nested 5,000 deep',
    () => render(EBNF + 'r = ' + '( a | '.repeat(5000) + 'b' + ' )'.repeat(5000) + ' ;', big),
  ],
  ['5,000 rules', () => render(EBNF + repeat(5000, (i) => `r${i} = a ;\n`), big)],
  ['one rule name defined 5,000 times', () => render(EBNF + 'r = a | b ;\n'.repeat(5000), big)],
  [
    'constructor calls nested 5,000 deep',
    () => render(IR + 'r = ' + 'optional(oneOrMore('.repeat(2500) + 'terminal("a")' + '))'.repeat(2500) + ' ;', big),
  ],
  [
    'sequence constructors nested 6,000 deep',
    () => render(IR + 'r = ' + 'sequence('.repeat(6000) + 'terminal("a")' + ')'.repeat(6000) + ' ;', big),
  ],
  [
    'choice call with 6,000 arguments',
    () => render(IR + 'r = choice(' + 'terminal("a"),'.repeat(6000) + 'special("b")) ;', big),
  ],
  ['unclosed constructor calls', () => parse(IR + 'r = ' + 'choice('.repeat(N / 7), big)],
  [
    'ABNF groups nested 10,000 deep with repeats',
    () => render(ABNF + 'r = ' + '2*3( '.repeat(10000) + 'a' + ' )'.repeat(10000) + ' ;', big),
  ],
  [
    'ABNF optional groups nested 12,000 deep',
    () => render(ABNF + 'r = ' + '['.repeat(12000) + 'a' + ']'.repeat(12000) + ' ;', big),
  ],
  [
    'ABNF repeat of 400 digits',
    () => render(ABNF + 'r = ' + '9'.repeat(400) + '*' + '9'.repeat(400) + 'a ' + '9'.repeat(400) + 'b ;', big),
  ],
  ['ABNF alternation of 12,000', () => render(ABNF + 'r = ' + 'a / '.repeat(12000) + 'b ;', big)],
  [
    'PEG predicates nested 10,000 deep',
    () => render(PEG + 'R <- ' + '!( &( '.repeat(5000) + 'a' + ' ) )'.repeat(5000) + ' ;', big),
  ],
  [
    'PEG groups with suffixes nested 10,000 deep',
    () => render(PEG + 'R <- ' + '( '.repeat(10000) + 'a' + ' )*'.repeat(10000) + ' ;', big),
  ],
  ['PEG run of operators', () => parse(PEG + 'R <- ' + '!&'.repeat(N / 2) + 'a ;', big)],
  ['PEG sequence of 12,000 predicates', () => render(PEG + 'R <- ' + '!a &"b" '.repeat(6000) + ';', big)],
];

describe('railroad worst cases', () => {
  for (const [name, run] of CASES) {
    it(
      name,
      () => {
        const started = performance.now();
        try {
          run();
        } catch (error) {
          expect(error, (error as Error).message).toBeInstanceOf(PeleError);
        }
        expect(performance.now() - started).toBeLessThan(3000);
      },
      30_000
    );
  }

  it('refuses source over the default limit', () => {
    expect(() => render(EBNF + 'r = a | b ;\n'.repeat(5000))).toThrow(/limit of 50000/);
  });
});
