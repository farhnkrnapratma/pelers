import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const HEAD = 'treeView-beta\n';

const CASES: [string, string][] = [
  ['25,000 flat entries', HEAD + 'a\n'.repeat(N / 2)],
  ['300 levels of nesting', HEAD + repeat(300, (i) => ' '.repeat(i) + 'd/\n')],
  ['310 levels indented with tabs and spaces', HEAD + repeat(310, (i) => '\t'.repeat(i % 2) + ' '.repeat(i) + 'x\n')],
  ['one 50,000 character name', HEAD + 'a'.repeat(N)],
  ['a name with one long run of blanks', HEAD + 'a' + ' '.repeat(N) + 'b'],
  ['a name of many runs of blanks', HEAD + ('a' + ' '.repeat(99)).repeat(N / 100) + 'b'],
  ['runs of blanks that almost start a class', HEAD + ('a :::' + ' '.repeat(95)).repeat(N / 100) + '1'],
  ['one line of almost-annotations', HEAD + 'a' + ' ::: 1 icon[ #'.repeat(N / 14)],
  ['blanks then a colon, many lines', HEAD + (' '.repeat(98) + ':\n').repeat(N / 100)],
  ['unclosed accDescr on every line', HEAD + 'accDescr {\n'.repeat(N / 11)],
  ['unclosed accDescr after names', HEAD + 'a\n' + ' accDescr {\n'.repeat(N / 12)],
  ['accDescr followed by blank lines', HEAD + ('accDescr' + '\n'.repeat(40)).repeat(N / 48)],
  ['unclosed quotes', HEAD + '"a\n'.repeat(N / 3)],
  ['10,000 annotations on one entry', HEAD + 'a' + ' :::b icon(c) ## d\n x'.slice(0, 13).repeat(N / 13)],
  ['10,000 classes on one entry', HEAD + 'a' + ' :::highlight'.repeat(N / 13)],
  ['every row highlighted with a description', HEAD + repeat(2500, (i) => ` f${i} :::highlight ## d${i}\n`)],
  ['comments', HEAD + '%% c\n'.repeat(N / 5)],
  ['one 50,000 character description', HEAD + 'a ## ' + 'word '.repeat(N / 5)],
  ['entities', HEAD + ' ' + '#35;&amp;'.repeat(N / 9)],
  ['box drawing, 10,000 rows', HEAD + '├── a\n'.repeat(N / 6)],
  ['box drawing, deep columns', HEAD + repeat(300, (i) => '│'.repeat(i) + '├─ x\n')],
  ['box drawing, one far branch per line', HEAD + '├ a\n' + (' '.repeat(490) + '└ b\n').repeat(100)],
  ['box decoration only', HEAD + '│   │   │\n'.repeat(N / 10)],
  ['box drawing, tabs', HEAD + ('\t'.repeat(40) + '├── a\n').repeat(N / 46)],
  ['box characters inside names', HEAD + 'a ─ b\n'.repeat(N / 6)],
  [
    'icons on every row',
    '---\nconfig:\n  treeView:\n    showIcons: true\n---\n' + HEAD + repeat(5000, (i) => ` f${i}.ts icon(p:i${i})\n`),
  ],
];

describe('treeView worst cases', () => {
  for (const [name, src] of CASES) {
    it(
      name,
      () => {
        const started = performance.now();
        try {
          render(src, big);
        } catch (error) {
          expect(error, (error as Error).message).toBeInstanceOf(PeleError);
        }
        expect(performance.now() - started).toBeLessThan(2000);
      },
      30_000
    );
  }

  it('refuses source over the default limit', () => {
    expect(() => render(HEAD + 'a\n'.repeat(30000))).toThrow(/limit of 50000/);
  });
});
