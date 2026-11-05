import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { T, VennLexer } from '../../src/diagrams/venn/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const V = 'venn-beta\n';
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

function lex(src: string, indent = false): number {
  const lexer = new VennLexer(src, { getIndentMode: () => indent, setIndentMode: () => undefined });
  let tokens = 0;
  for (let type = lexer.next(); type !== T.END && type !== T.ERROR; type = lexer.next()) tokens++;
  return tokens;
}

// Nothing, less than nothing, more than a number can hold, and next to nothing.
const NONSENSE = `set A:0\nset B:-1000000000\nset C:${'9'.repeat(400)}\nset D:.${'0'.repeat(300)}1\nunion A,B:${'9'.repeat(400)}\nunion C,D:-5\nunion A,B,C,D["x"]:0\n`;

const CASES: [string, () => unknown][] = [
  ['unclosed bracket labels', () => lex(V + 'set A["x'.repeat(N / 8))],
  ['bracket labels that end without their bracket', () => lex(V + 'set A["x" '.repeat(N / 10))],
  ['unclosed strings', () => lex(V + 'set "a '.repeat(N / 7))],
  ['one long unquoted bracket label', () => lex(V + 'set A[' + 'x '.repeat(N / 2))],
  ['rgb that never closes', () => lex(V + 'style A fill:' + 'rgb(1,2,'.repeat(N / 8))],
  ['rgb of digits and spaces', () => lex(V + 'style A fill:rgba(' + '1 '.repeat(N / 2))],
  ['rgb of digits and line breaks', () => lex(V + 'style A fill:rgb(' + '1\n'.repeat(N / 2))],
  ['comments after every character', () => lex(V + 'a%%\n'.repeat(N / 4))],
  ['lines of percent signs', () => lex(V + ('%'.repeat(99) + '\n').repeat(N / 100))],
  ['directive openers', () => lex(V + '%%{\n'.repeat(N / 4))],
  ['blank lines of spaces', () => lex(V + (' '.repeat(99) + '\n').repeat(N / 100))],
  ['one line of 50,000 spaces before text', () => lex(V + 'set A\n' + ' '.repeat(N) + 'text x', true)],
  ['indented text lines, no set before them', () => lex(V + '  text a\n'.repeat(N / 9))],
  ['indented text lines after a set', () => lex(V + '  text a\n'.repeat(N / 9), true)],
  ['title lines', () => lex(V + 'title a b c\n'.repeat(N / 12))],
  ['the word title with nothing after it', () => lex(V + 'title\n'.repeat(N / 6))],
  ['one title of 50,000 characters', () => render(V + 'title ' + 'word '.repeat(N / 5) + '\nset A', big)],
  ['keywords run together', () => lex(V + 'setunionstyletextvenn-beta'.repeat(N / 26))],
  ['hex colors of every length', () => lex(V + 'style A fill:' + '#123456789abcdef '.repeat(N / 17))],
  ['numbers and signs', () => lex(V + 'set A:' + '+1.5-.5 '.repeat(N / 8))],
  ['2,000 sets', () => render(V + repeat(2000, (i) => `set s${i}\n`), big)],
  ['5,000 sets', () => render(V + repeat(5000, (i) => `set s${i}\n`), big)],
  [
    '300 sets in a chain of unions',
    () => render(V + repeat(300, (i) => `set s${i}\n`) + repeat(299, (i) => `union s${i},s${i + 1}\n`), big),
  ],
  [
    '60 sets, every pair a union',
    () => {
      let s = V + repeat(60, (i) => `set s${i}\n`);
      for (let i = 0; i < 60; i++) for (let j = i + 1; j < 60; j++) s += `union s${i},s${j}\n`;
      return render(s, big);
    },
  ],
  [
    'one union of 300 sets',
    () =>
      render(V + repeat(300, (i) => `set s${i}\n`) + 'union ' + repeat(299, (i) => `s${i},`) + 's299["all"]\n', big),
  ],
  [
    '40 unions of 60 sets each',
    () =>
      render(
        V + repeat(60, (i) => `set s${i}\n`) + repeat(40, () => 'union ' + repeat(59, (i) => `s${i},`) + 's59\n'),
        big
      ),
  ],
  ['one set stated 8,000 times', () => render(V + 'set A["x"]:5\n'.repeat(8000), big)],
  ['one union stated 5,000 times', () => render(V + 'set A\nset B\n' + 'union A,B["x"]:1\n'.repeat(5000), big)],
  ['8,000 texts in one set', () => render(V + 'set A\n' + '  text t\n'.repeat(8000), big)],
  [
    '5,000 texts in the overlap of three sets',
    () => render(V + 'set A\nset B\nset C\nunion A,B,C\n' + '  text "t t"["l"]\n'.repeat(5000), big),
  ],
  ['5,000 styles for one set', () => render(V + 'set A\n' + 'style A fill:red, color:blue\n'.repeat(5000), big)],
  [
    'one style of 8,000 declarations',
    () => render(V + 'set A\nstyle A ' + repeat(8000, (i) => `p${i}:1,`) + 'fill:red\n', big),
  ],
  ['one style value of 20,000 words', () => render(V + 'set A\nstyle A fill:' + 'a '.repeat(20000) + '\n', big)],
  [
    '5,000 styled overlaps',
    () => render(V + 'set A\nset B\nset C\nunion A,B,C["x"]\n' + 'style A,B,C fill:gold\n'.repeat(5000), big),
  ],
  ['one 50,000 character label', () => render(V + 'set A["' + 'word '.repeat(N / 5) + '"]', big)],
  ['one 50,000 character word', () => render(V + 'set A[' + 'a'.repeat(N) + ']', big)],
  ['a label of entities and markup', () => render(V + 'set A["' + '#35;&amp;<br>'.repeat(N / 13) + '"]', big)],
  ['sizes that make no sense', () => render(V + NONSENSE, big)],
  [
    'a size of 50,000 digits',
    () => render(V + 'set A:' + '9'.repeat(N) + '\nset B\nunion A,B:' + '9'.repeat(400), big),
  ],
  [
    'a union of one set with itself, many times',
    () => render(V + 'set A\n' + 'union A,A,A,A["x"]\n'.repeat(3000), big),
  ],
];

describe('venn worst cases', () => {
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

  it('keeps every number finite, whatever the sizes', () => {
    const { svg, width, height } = render(V + NONSENSE, big);
    expect(svg).not.toMatch(/NaN|Infinity/);
    expect(Number.isFinite(width) && Number.isFinite(height)).toBe(true);
  });

  it('refuses source over the default limit', () => {
    expect(() => render(V + 'set A\n'.repeat(9000))).toThrow(/limit of 50000/);
    expect(() => parse(V + 'set A\n'.repeat(9000))).toThrow(/limit of 50000/);
  });
});
