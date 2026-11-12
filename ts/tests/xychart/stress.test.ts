import { describe, expect, it } from 'vitest';
import { tokenize } from '../../src/diagrams/xychart/lexer.js';
import { PelersError, parse, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

// Inputs of about 50,000 characters built to hit the XY chart's worst cases. Each must finish
// quickly and either succeed or fail with a PelersError. A regression here is seconds or a hang.

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, item: (i: number) => string, join = ''): string =>
  Array.from({ length: count }, (_, i) => item(i)).join(join);
const withLabels = '---\nconfig:\n  xyChart:\n    showDataLabel: true\n---\n';

const CASES: [string, () => unknown][] = [
  ['a run of spaces and line breaks', () => tokenize('xychart\n' + ' \n'.repeat(N / 2) + 'line [1]')],
  ['a run of dashes', () => tokenize('xychart\nx-axis a ' + '-'.repeat(N))],
  ['signs with no digits in a series', () => tokenize('xychart\nline [' + '+-'.repeat(N / 2) + ']')],
  ['dots and digits in a series', () => tokenize('xychart\nline [' + '1.'.repeat(N / 2) + ']')],
  ['line breaks each followed by a comment', () => tokenize('xychart' + '\n%% c'.repeat(N / 5))],
  ['comment markers with no line break', () => tokenize('xychart\n' + 'a%%'.repeat(N / 3))],
  ['accTitle with no colon, many times', () => tokenize('xychart\n' + ('accTitle' + ' '.repeat(40)).repeat(1000))],
  [
    'backticks that almost start the broken rule',
    () => tokenize('xychart\n' + ('`)' + ' '.repeat(36) + '{ this').repeat(1000)),
  ],
  [
    'keywords that are not quite keywords',
    () => tokenize('xychart\n' + 'xychart-bet x-axi linex bar_ titl '.repeat(N / 34)),
  ],
  ['an unclosed accDescr block', () => parse('xychart\naccDescr {' + ' a\n'.repeat(N / 3), big)],
  ['an unclosed string', () => parse('xychart\ntitle "' + 'a'.repeat(N), big)],
  ['10,000 nested axis and series keywords', () => parse('xychart\n' + 'x-axis line bar y-axis '.repeat(N / 23), big)],
  ['10,000 chart keywords', () => parse(repeat(10000, () => 'xychart\n') + 'line [1]', big)],
  ['20,000 values in a line', () => render('xychart\nline [' + repeat(20000, (i) => String(i % 97), ',') + ']', big)],
  [
    '16,000 values in a bar series',
    () => render('xychart\nbar [' + repeat(16000, (i) => String(i % 97), ',') + ']', big),
  ],
  [
    '8,000 bars with their values shown',
    () => render(withLabels + 'xychart\nbar [' + repeat(8000, (i) => String(i), ',') + ']', big),
  ],
  [
    '8,000 categories',
    () => render('xychart\nx-axis [' + repeat(8000, (i) => 'c' + i, ',') + ']\nbar [' + '1,'.repeat(7999) + '1]', big),
  ],
  [
    '8,000 categories on their side',
    () =>
      render(
        'xychart horizontal\nx-axis [' + repeat(8000, (i) => 'c' + i, ',') + ']\nline [' + '1,'.repeat(7999) + '1]',
        big
      ),
  ],
  ['5,000 labelled points', () => render('xychart\nline [' + repeat(5000, (i) => `${i} "p${i}"`, ',') + ']', big)],
  ['4,000 series', () => render('xychart\n' + repeat(4000, (i) => (i % 2 ? 'bar' : 'line') + ' [1, 2]\n'), big)],
  ['3,000 named series', () => render('xychart\n' + repeat(3000, (i) => `line "series ${i}" [1, 2]\n`), big)],
  [
    'one category of 50,000 characters',
    () => render('xychart\nx-axis ["' + 'word '.repeat(N / 5) + '"]\nbar [1]', big),
  ],
  [
    'a title of 50,000 characters',
    () => render('xychart\ntitle "' + 'long title '.repeat(N / 11) + '"\nline [1, 2]', big),
  ],
  [
    'a title of 50,000 words without quotes',
    () => render('xychart\ntitle ' + 'a '.repeat(N / 2) + '\nline [1, 2]', big),
  ],
  [
    'labels of entities and line breaks',
    () => render('xychart\nx-axis "' + '#35;&amp;<br>'.repeat(N / 13) + '" [a]\nline [1]', big),
  ],
  ['a value of 50,000 digits', () => render('xychart\nline [' + '9'.repeat(N) + ', 1]', big)],
  [
    'a range of 400-digit numbers',
    () => render(`xychart\ny-axis -${'9'.repeat(400)} --> ${'9'.repeat(400)}\nbar [1, 2]`, big),
  ],
];

describe('XY chart worst cases', () => {
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
    expect(() => render('xychart\n' + 'line [1, 2, 3]\n'.repeat(5000))).toThrow(/limit of 50000/);
  });
});
