import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { parseGenericTypes as genericTypes } from '../../src/diagrams/common/generics.js';
import { tokenize } from '../../src/diagrams/er/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['long line that mentions direction', () => tokenize('erDiagram\n' + 'A '.repeat(N / 2) + 'direction XX')],
  ['one line of direction words', () => tokenize('erDiagram\n' + 'direction '.repeat(N / 10))],
  [
    'direction followed by blank lines, many times',
    () => tokenize('erDiagram\n' + ('direction' + '\n'.repeat(9)).repeat(N / 18) + 'TB'),
  ],
  ['many lines that each mention direction', () => tokenize('erDiagram\n' + 'A direction B\n'.repeat(N / 14))],
  ['direction after a long run of spaces', () => tokenize('erDiagram\ndirection' + ' '.repeat(N) + 'XX')],
  ['accTitle with no colon, many times', () => tokenize('erDiagram\n' + ('accTitle' + ' '.repeat(40)).repeat(N / 48))],
  ['ones followed by long spaces', () => tokenize('erDiagram\n' + ('1' + ' '.repeat(99)).repeat(N / 100) + '!')],
  ['runs of cardinality symbols', () => tokenize('erDiagram\n' + '|o}o}|o{|{||--..'.repeat(N / 16))],
  ['unclosed quotes', () => tokenize('erDiagram\n' + '"a%'.repeat(N / 3))],
  [
    'block of single tildes on one line',
    () => tokenize('erDiagram\nA {\n' + '! '.repeat(N / 4) + '~' + ' !'.repeat(N / 4)),
  ],
  ['block of unmatched characters before two tildes', () => tokenize('erDiagram\nA {\n' + '!'.repeat(N) + '~~')],
  ['block with a tilde on every line', () => tokenize('erDiagram\nA {\n' + 'a~ b\n'.repeat(N / 5) + '}')],
  ['block of tilde pairs', () => tokenize('erDiagram\nA {\n' + '~a~ '.repeat(N / 4) + '}')],
  ['unclosed backtick in a block', () => tokenize('erDiagram\nA {\n' + '`' + 'a '.repeat(N / 2))],
  ['block of backtick pairs', () => tokenize('erDiagram\nA {\n' + '`a``b` '.repeat(N / 7) + '}')],
  ['unclosed comment quotes in a block', () => tokenize('erDiagram\nA {\n' + 'a b "'.repeat(N / 5))],
  ['unclosed multi-line description', () => tokenize('erDiagram\naccDescr {' + ' text\n'.repeat(N / 6))],
  ['style statement of colons and hashes', () => parse('erDiagram\nA\nstyle A ' + ':#'.repeat(N / 2) + '\n', big)],
  [
    'style statement with 10,000 ids',
    () => parse('erDiagram\nstyle ' + repeat(10000, (i) => `e${i},`) + 'z fill:red\n', big),
  ],
  [
    'style list of 10,000 declarations',
    () => render('erDiagram\nA\nstyle A ' + repeat(10000, (i) => `x${i}:1,`) + 'fill:red\n', big),
  ],
  [
    'class statement with 10,000 ids and classes',
    () =>
      parse('erDiagram\nclass ' + repeat(10000, (i) => `e${i},`) + 'z ' + repeat(10000, (i) => `c${i},`) + 'z\n', big),
  ],
  ['20,000 class names on one entity', () => render('erDiagram\nA:::' + repeat(20000, (i) => `c${i},`) + 'z\n', big)],
  [
    '20,000 nested subgraphs',
    () => parse('erDiagram\n' + 'subgraph s\n'.repeat(20000) + 'A\n' + 'end\n'.repeat(20000), big),
  ],
  [
    '3,000 nested subgraphs, rendered',
    () =>
      render('erDiagram\n' + repeat(3000, (i) => `subgraph s${i}\n`) + 'A ||--|| B : x\n' + 'end\n'.repeat(3000), big),
  ],
  ['5,000 sibling subgraphs', () => render('erDiagram\n' + repeat(5000, (i) => `subgraph s${i}\nn${i}\nend\n`), big)],
  [
    '5,000 subgraphs that list the same entities',
    () => render('erDiagram\n' + repeat(5000, () => 'subgraph s\nA\nB\nC\nend\n'), big),
  ],
  [
    'one subgraph of 10,000 entities',
    () => parse('erDiagram\nsubgraph s\n' + repeat(10000, (i) => `n${i}\n`) + 'end\n', big),
  ],
  ['unclosed subgraphs', () => parse('erDiagram\n' + 'subgraph s\n'.repeat(N / 11), big)],
  ['entity with 5,000 attributes', () => render('erDiagram\nA {\n' + repeat(5000, (i) => `t${i} n${i}\n`) + '}', big)],
  ['attribute with 10,000 keys', () => render('erDiagram\nA {\n string id ' + 'PK,'.repeat(10000) + 'FK\n}', big)],
  ['5,000 attribute blocks for one entity', () => render('erDiagram\n' + repeat(5000, (i) => `A { t n${i} }\n`), big)],
  ['one 50,000 character comment', () => render('erDiagram\nA {\n string id "' + 'word '.repeat(N / 5) + '"\n}', big)],
  ['one 50,000 character attribute name', () => render('erDiagram\nA {\n string ' + 'a'.repeat(N) + '\n}', big)],
  ['attribute type of 50,000 tildes', () => render('erDiagram\nA {\n ' + '~'.repeat(N) + ' name\n}', big)],
  [
    'attribute type of tilde and comma pairs',
    () => render('erDiagram\nA {\n ' + 'a~b,'.repeat(N / 4) + ' name\n}', big),
  ],
  ['generic type of nested tildes', () => genericTypes('a~'.repeat(N / 2) + ','.repeat(1000) + '~b'.repeat(N / 2))],
  ['comment of emphasis markers', () => render('erDiagram\nA {\n string id "' + '*a _b '.repeat(N / 6) + '"\n}', big)],
  ['entity name of entities and line breaks', () => render('erDiagram\n"' + '#35;&amp;<br>'.repeat(N / 13) + '"', big)],
  [
    'one 50,000 character relationship label',
    () => render('erDiagram\nA ||--o{ B : "' + 'word '.repeat(N / 5) + '"', big),
  ],
  ['5,000 entities', () => render('erDiagram\n' + repeat(5000, (i) => `n${i}\n`), big)],
  ['chain of 5,000', () => render('erDiagram\n' + repeat(5000, (i) => `n${i} ||--o{ n${i + 1} : r\n`), big)],
  ['star of 5,000', () => render('erDiagram\n' + repeat(5000, (i) => `c ||--o{ n${i} : r\n`), big)],
  [
    '2,000 parallel relationships and 2,000 self relationships',
    () => render('erDiagram\n' + 'A ||--o{ B : r\n'.repeat(2000) + 'A ||--|| A : s\n'.repeat(2000), big),
  ],
  [
    'complete graph of 50',
    () => {
      let s = 'erDiagram\n';
      for (let i = 0; i < 50; i++) for (let j = i + 1; j < 50; j++) s += `n${i} }o--o{ n${j} : r\n`;
      return render(s, big);
    },
  ],
  [
    '2,000 relationships between subgraphs',
    () =>
      render(
        'erDiagram\nsubgraph a\nA\nend\nsubgraph b\nB\nend\n' + 'a ||--|| b : r\nA ||--|| b : s\n'.repeat(1000),
        big
      ),
  ],
  ['alias repeated 10,000 times', () => render('erDiagram\n' + 'a["Some Alias"]\n'.repeat(10000), big)],
];

describe('ER worst cases', () => {
  for (const [name, run] of CASES) {
    it(
      name,
      () => {
        const started = performance.now();
        try {
          run();
        } catch (error) {
          expect(error, (error as Error).message).toBeInstanceOf(PelersError);
        }
        expect(performance.now() - started).toBeLessThan(3000);
      },
      30_000
    );
  }

  it('refuses source over the default limit', () => {
    expect(() => render('erDiagram\n' + 'A ||--o{ B : r\n'.repeat(5000))).toThrow(/limit of 50000/);
  });
});
