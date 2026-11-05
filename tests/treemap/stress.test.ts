import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { parseTreemap } from '../../src/diagrams/treemap/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['5,000 leaves', () => render('treemap\n' + repeat(5000, (i) => `"l${i}": ${i + 1}\n`), big)],
  ['5,000 leaves of equal value in one section', () => render('treemap\n"s"\n' + repeat(5000, () => ' "l": 1\n'), big)],
  [
    '5,000 leaves of very unequal value',
    () => render('treemap\n' + repeat(5000, (i) => `"l": ${i % 2 ? '0.0001' : '99999999'}\n`), big),
  ],
  [
    '3,000 sections with one leaf each',
    () => render('treemap\n' + repeat(3000, (i) => `"s${i}"\n  "l": ${i + 1}\n`), big),
  ],
  [
    '300 levels of nesting',
    () => render('treemap\n' + repeat(300, (i) => ' '.repeat(i) + `"s${i}"\n`) + ' '.repeat(300) + '"leaf": 1\n', big),
  ],
  ['20,000 sections that step in and out', () => parse('treemap\n' + repeat(10000, () => '"s"\n "t"\n'), big)],
  ['5,000 sections and no leaves', () => render('treemap\n' + repeat(5000, (i) => `"s${i}"\n`), big)],
  [
    '3,000 class definitions',
    () => render('treemap\n"a": 1:::c1\n' + repeat(3000, (i) => `classDef c${i} fill:red\n`), big),
  ],
  [
    'one class with 5,000 declarations',
    () => render('treemap\n"a": 1:::c1\nclassDef c1 ' + 'fill:red,'.repeat(5000) + 'stroke:blue;\n', big),
  ],
  [
    'a class applied to 3,000 leaves',
    () =>
      render('treemap\nclassDef c1 fill:red,stroke:blue,color:white\n' + repeat(3000, (i) => `"l${i}": 5:::c1\n`), big),
  ],
  ['a 50,000 character name', () => render('treemap\n"' + 'word '.repeat(N / 5) + '": 1', big)],
  ['a 50,000 character section name', () => render('treemap\n"' + 'x'.repeat(N) + '"\n "a": 1', big)],
  ['a name of entities', () => render('treemap\n"' + '#35;&amp;'.repeat(N / 9) + '": 1', big)],
  ['a 50,000 digit value', () => render('treemap\n"a": ' + '9'.repeat(N), big)],
  ['a value of dots and commas', () => render('treemap\n"a": ' + '.,_'.repeat(N / 3), big)],
  ['classDef followed by 50,000 spaces', () => parseTreemap('treemap\nclassDef' + ' '.repeat(N) + '1')],
  ['classDef 6,000 times with one-letter names', () => parseTreemap('treemap\n' + 'classDef a\n'.repeat(N / 8))],
  ['classDef and 25,000 blank lines', () => parseTreemap('treemap\nclassDef' + '\n '.repeat(N / 2) + 'ab')],
  ['a class style of 50,000 characters', () => render('treemap\n"a": 1:::cc\nclassDef cc ' + 'x'.repeat(N), big)],
  ['accDescr opened 5,000 times', () => parseTreemap('treemap\n' + 'accDescr\n'.repeat(N / 10) + '{')],
  ['an unclosed accDescr block', () => parseTreemap('treemap\naccDescr {' + ' a\n'.repeat(N / 3))],
  ['an unclosed string', () => parseTreemap('treemap\n"' + 'a'.repeat(N))],
  ['25,000 quotes', () => parseTreemap('treemap\n' + '"'.repeat(N / 2))],
  ['50,000 colons', () => parseTreemap('treemap\n"a"' + ':'.repeat(N))],
  ['a line of spaces', () => parseTreemap('treemap\n' + ' '.repeat(N) + '"a": 1')],
  ['20,000 comments', () => parseTreemap('treemap\n' + '%% c\n'.repeat(N / 5) + '"a": 1')],
  ['a long title', () => render('treemap\ntitle ' + 'x'.repeat(N), big)],
  [
    'a 50,000 character value format',
    () => render('---\nconfig:\n  treemap:\n    valueFormat: "' + '0'.repeat(N) + 'x"\n---\ntreemap\n"a": 1', big),
  ],
];

describe('treemap worst cases', () => {
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
});
