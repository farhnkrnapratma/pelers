import { describe, expect, it } from 'vitest';
import { PeleError, parse } from '../../src/index.js';
import type { RailroadModel, RailroadNode } from '../../src/diagrams/railroad/types.js';

// How each notation's syntax tree becomes the common rule model. The expected values follow
// Mermaid's railroadParser.ts, ebnfParser.ts, abnfParser.ts, and pegParser.ts.

function model(src: string): RailroadModel {
  const result = parse(src);
  if (result.type !== 'railroad') throw new Error('not a railroad diagram');
  return result;
}

const t = (value: string): RailroadNode => ({ type: 'terminal', value });
const n = (name: string): RailroadNode => ({ type: 'nonterminal', name });
const s = (text: string): RailroadNode => ({ type: 'special', text });
const seq = (...elements: RailroadNode[]): RailroadNode => ({ type: 'sequence', elements });
const alt = (...alternatives: RailroadNode[]): RailroadNode => ({ type: 'choice', alternatives });
const opt = (element: RailroadNode): RailroadNode => ({ type: 'optional', element });
const rep = (element: RailroadNode, min: number, max = Infinity): RailroadNode => ({
  type: 'repetition',
  element,
  min,
  max,
});

const definition = (keyword: string, body: string): RailroadNode => model(`${keyword}\n${body}`).rules[0].definition;
const ir = (expression: string) => definition('railroad-beta', `r = ${expression} ;`);
const ebnf = (expression: string) => definition('railroad-ebnf-beta', `r = ${expression} ;`);
const abnf = (expression: string) => definition('railroad-abnf-beta', `r = ${expression} ;`);
const peg = (expression: string) => definition('railroad-peg-beta', `R <- ${expression} ;`);

describe('railroad model', () => {
  it('builds the constructor notation node for node', () => {
    expect(ir('terminal("a")')).toEqual(t('a'));
    expect(ir("nonterminal('b')")).toEqual(n('b'));
    expect(ir('special("any character")')).toEqual(s('any character'));
    expect(ir('sequence(terminal("a"), nonterminal("b"))')).toEqual(seq(t('a'), n('b')));
    expect(ir('choice(terminal("a"), nonterminal("b"), special("c"))')).toEqual(alt(t('a'), n('b'), s('c')));
    expect(ir('optional(terminal("a"))')).toEqual(opt(t('a')));
    expect(ir('oneOrMore(terminal("a"))')).toEqual(rep(t('a'), 1));
    expect(ir('zeroOrMore(terminal("a"))')).toEqual(rep(t('a'), 0));
  });

  it('replaces a sequence or choice of one by its only member', () => {
    expect(ir('sequence(choice(sequence(terminal("a"))))')).toEqual(t('a'));
    expect(ir('optional(choice(terminal("a")))')).toEqual(opt(t('a')));
    expect(ebnf('( ( a ) )')).toEqual(n('a'));
    expect(abnf('( ( a ) )')).toEqual(n('a'));
    expect(peg('( ( a ) )')).toEqual(n('a'));
  });

  it('decodes three escapes in strings and keeps any other escaped character', () => {
    expect(ir('terminal("a\\nb\\tc\\rd")')).toEqual(t('a\nb\tc\rd'));
    expect(ir('terminal("\\"\\\\\\x\\0")')).toEqual(t('"\\x0'));
    expect(ebnf("'it\\'s'")).toEqual(t("it's"));
    expect(peg('"\\b"')).toEqual(t('b'));
    // ABNF strings have no escapes.
    expect(abnf('"a\\" b')).toEqual(seq(t('a\\'), n('b')));
  });

  it('reads EBNF in both styles', () => {
    expect(ebnf('a b , c')).toEqual(seq(n('a'), n('b'), n('c')));
    expect(ebnf('a | "b" c')).toEqual(alt(n('a'), seq(t('b'), n('c'))));
    expect(ebnf('[ a ]')).toEqual(opt(n('a')));
    expect(ebnf('{ a | b }')).toEqual(rep(alt(n('a'), n('b')), 0));
    expect(ebnf('a? b* c+')).toEqual(seq(opt(n('a')), rep(n('b'), 0), rep(n('c'), 1)));
    expect(ebnf('?  any character  ?')).toEqual(s('any character'));
    expect(definition('railroad-ebnf-beta', 'r ::= a ;')).toEqual(n('a'));
  });

  it('applies EBNF postfix operators left to right to everything before them', () => {
    expect(ebnf('a?*+')).toEqual(rep(rep(opt(n('a')), 0), 1));
    expect(ebnf('a - b')).toEqual(seq(n('a'), t('-'), n('b')));
    expect(ebnf('a - b - "c"')).toEqual(seq(seq(n('a'), t('-'), n('b')), t('-'), t('c')));
    expect(ebnf('a - ( b | c )*')).toEqual(rep(seq(n('a'), t('-'), alt(n('b'), n('c'))), 0));
    expect(ebnf('a* - [ b ]')).toEqual(seq(rep(n('a'), 0), t('-'), opt(n('b'))));
  });

  it('lets an EBNF special sequence start at any question mark that has a partner', () => {
    expect(ebnf('a? b?')).toEqual(seq(n('a'), s('b')));
    expect(model('railroad-ebnf-beta\nr = a? ;\nq = b? ;').rules.map((rule) => rule.definition)).toEqual([
      opt(n('a')),
      opt(n('b')),
    ]);
  });

  it('reads ABNF repeats', () => {
    expect(abnf('*a')).toEqual(rep(n('a'), 0));
    expect(abnf('1*a')).toEqual(rep(n('a'), 1));
    expect(abnf('2*4a')).toEqual(rep(n('a'), 2, 4));
    expect(abnf('*4a')).toEqual(rep(n('a'), 0, 4));
    expect(abnf('3a')).toEqual(rep(n('a'), 3, 3));
    expect(abnf('1a')).toEqual(rep(n('a'), 1, 1));
    expect(abnf('007a')).toEqual(rep(n('a'), 7, 7));
    expect(abnf('0*1a')).toEqual(opt(n('a')));
    expect(abnf('*1"a"')).toEqual(opt(t('a')));
    expect(abnf('2*( a / b )')).toEqual(rep(alt(n('a'), n('b')), 2));
    expect(abnf('3[ a ]')).toEqual(rep(opt(n('a')), 3, 3));
    expect(abnf('9'.repeat(400) + 'a')).toEqual(rep(n('a'), Infinity, Infinity));
  });

  it('reads ABNF values, groups, and alternation', () => {
    expect(abnf('%x41 %d65.66 %b1000001 %x30-39')).toEqual(seq(t('%x41'), t('%d65.66'), t('%b1000001'), t('%x30-39')));
    expect(abnf('a / "b" c / [ d ]')).toEqual(alt(n('a'), seq(t('b'), n('c')), opt(n('d'))));
    expect(abnf('rule-name DIGIT')).toEqual(seq(n('rule-name'), n('DIGIT')));
  });

  it('ends an ABNF rule only with a semicolon that ends its line', () => {
    const { rules } = model('railroad-abnf-beta\nr = a ; a comment, so the rule goes on\n  b ;\ns = c ;');
    expect(rules).toEqual([
      { name: 'r', definition: seq(n('a'), n('b')) },
      { name: 's', definition: n('c') },
    ]);
    expect(() => parse('railroad-abnf-beta\nr = a ; \n')).toThrow(PeleError);
  });

  it('reads PEG suffixes, and shows predicates as special text', () => {
    expect(peg('a? b* c+')).toEqual(seq(opt(n('a')), rep(n('b'), 0), rep(n('c'), 1)));
    expect(peg('a / "b" c')).toEqual(alt(n('a'), seq(t('b'), n('c'))));
    expect(peg('.')).toEqual(s('.'));
    expect(peg('&a')).toEqual(s('&a'));
    expect(peg('!"x"')).toEqual(s('!"x"'));
    expect(peg('!.')).toEqual(s('!.'));
    expect(() => peg('!&a')).toThrow(PeleError);
    expect(peg('!( a b )')).toEqual(s('!(...)'));
    expect(peg('!a*')).toEqual(s('!(...)'));
    expect(peg('&( a )')).toEqual(s('&a'));
    expect(peg('( !a )+')).toEqual(rep(s('!a'), 1));
    expect(() => peg('a**')).toThrow(PeleError);
  });

  it('reads the title and accessibility statements', () => {
    const quoted = model(
      'railroad-ebnf-beta\ntitle   "A \\"quoted\\"\\ttitle"   \naccTitle:   Acc   title\naccDescr: One   line\nr = a ;'
    );
    expect(quoted.title).toBe('A "quoted"\ttitle');
    expect(quoted.accTitle).toBe('Acc title');
    expect(quoted.accDescr).toBe('One line');
    expect(model("railroad-peg-beta\ntitle 'single'\naccDescr {\n   Two\n   lines\n}\nR <- a ;")).toMatchObject({
      title: 'single',
      accDescr: 'Two\nlines',
    });
    expect(model('railroad-beta title Plain  title %% comment\nr = terminal("a") ;').title).toBe('Plain title');
    // ABNF titles lose their quotes but keep their backslashes.
    expect(model('railroad-abnf-beta\ntitle "a\\tb"\nr = a ;').title).toBe('a\\tb');
    expect(model('railroad-ebnf-beta\ntitle first\ntitle second\nr = a ;').title).toBe('second');
    expect(model('railroad-ebnf-beta\nr = a ;').title).toBeUndefined();
  });

  it('prefers its own title to the front matter title', () => {
    expect(model('---\ntitle: Front\n---\nrailroad-ebnf-beta\nr = a ;').title).toBe('Front');
    expect(model('---\ntitle: Front\n---\nrailroad-ebnf-beta\ntitle Own\nr = a ;').title).toBe('Own');
    expect(model('---\ntitle: Front\n---\nrailroad-ebnf-beta\ntitle\nr = a ;').title).toBe('Front');
  });

  it('reads names that start with title, but no title after the first rule', () => {
    expect(model('railroad-ebnf-beta\ntitles = a ;')).toMatchObject({
      title: undefined,
      rules: [{ name: 'titles', definition: n('a') }],
    });
    expect(() => parse('railroad-ebnf-beta\nr = a ;\ntitle Late')).toThrow(PeleError);
    expect(() => parse('railroad-ebnf-beta\nr = title ;')).toThrow(PeleError);
  });

  it('keeps keywords out of names only when they stand alone', () => {
    expect(model('railroad-beta\nterminals = nonterminal("choice") ;').rules[0]).toEqual({
      name: 'terminals',
      definition: n('choice'),
    });
    expect(() => parse('railroad-beta\nterminal = terminal("a") ;')).toThrow(PeleError);
    expect(() => parse('railroad-beta-x\nr = terminal("a") ;')).toThrow(/railroad-beta/);
  });

  it('skips comments in each notation', () => {
    expect(ebnf('a (* iso *) /* w3c */ b')).toEqual(seq(n('a'), n('b')));
    expect(ir('/* c */ terminal("a") /* d */')).toEqual(t('a'));
    expect(peg('a # to the end of the line\n b')).toEqual(seq(n('a'), n('b')));
    expect(() => ebnf('a (*) b')).toThrow(PeleError);
  });

  it('rejects what the grammars reject', () => {
    for (const src of [
      'railroad-ebnf-beta\nr = ;',
      'railroad-ebnf-beta\nr = a',
      'railroad-ebnf-beta\nr = ( ) ;',
      'railroad-ebnf-beta\nr = a | ;',
      'railroad-ebnf-beta\nr = a , ;',
      'railroad-ebnf-beta\nr = [ a ) ;',
      'railroad-ebnf-beta\nr = a - ;',
      'railroad-beta\nr = sequence() ;',
      'railroad-beta\nr = optional(terminal("a"), terminal("b")) ;',
      'railroad-beta\nr = terminal(a) ;',
      'railroad-beta\nr = terminal("unclosed) ;',
      'railroad-abnf-beta\nr = a /\n ;',
      'railroad-abnf-beta\nr = 2* ;',
      'railroad-peg-beta\nR <- ;',
      'railroad-peg-beta\nR = a ;',
      'railroad-peg-beta\nR <- a ) ;',
      'railroad-peg-beta\nR <- & ;',
      'r = "a" ;',
    ]) {
      expect(() => parse(src), src).toThrow(PeleError);
    }
  });
});
