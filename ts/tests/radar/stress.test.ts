import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { parseRadar } from '../../src/diagrams/radar/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const list = (count: number, item: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => item(i)).join(',');

const CASES: [string, () => unknown][] = [
  [
    '5,000 axes and one curve',
    () => render(`radar-beta\naxis ${list(5000, (i) => `a${i}`)}\ncurve c{${list(5000, (i) => String(i))}}`, big),
  ],
  [
    '5,000 axes with a polygon graticule of 32 ticks',
    () =>
      render(
        `radar-beta\naxis ${list(5000, (i) => `a${i}`)}\ncurve c{${list(5000, () => '1')}}\ngraticule polygon\nticks 32`,
        big
      ),
  ],
  [
    '2,000 axes and curves of named entries',
    () => {
      const axes = list(2000, (i) => `a${i}`);
      const entries = list(2000, (i) => `a${1999 - i} ${i}`);
      return render(`radar-beta\naxis ${axes}\ncurve x{${entries}}\ncurve y{${entries}}`, big);
    },
  ],
  [
    '4,000 curves',
    () =>
      render('radar-beta\naxis a,b,c\n' + Array.from({ length: 4000 }, (_, i) => `curve c${i}{1,2,3}\n`).join(''), big),
  ],
  ['5,000 options', () => render('radar-beta\n' + 'ticks 3\nmax 9\n'.repeat(2500), big)],
  ['50,000 zeros as curve values', () => parseRadar('radar-beta\ncurve c{' + '0'.repeat(N) + '}')],
  ['50,000 zeros before a decimal point', () => parseRadar('radar-beta\nmax ' + '0'.repeat(N) + '.5')],
  ['a 50,000 digit number', () => render('radar-beta\naxis a\ncurve c{1}\nmax ' + '9'.repeat(N), big)],
  ['numbers and dots', () => parseRadar('radar-beta\ncurve c{' + '1.2'.repeat(N / 3) + '}')],
  ['a 50,000 character name', () => render('radar-beta\naxis ' + 'a'.repeat(N), big)],
  ['a name of dashes', () => parseRadar('radar-beta\naxis a' + '-'.repeat(N))],
  ['keywords run together', () => parseRadar('radar-beta\n' + 'axis'.repeat(N / 4))],
  ['accDescr opened 5,000 times', () => parseRadar('radar-beta\n' + 'accDescr {'.repeat(N / 10))],
  ['an unclosed accDescr block', () => parseRadar('radar-beta\naccDescr {' + ' a\n'.repeat(N / 3))],
  ['an unclosed string', () => parseRadar('radar-beta\naxis a["' + 'x'.repeat(N))],
  [
    'a 50,000 character label',
    () => render('radar-beta\naxis a["' + 'word '.repeat(N / 5) + '"], b\ncurve c{1,2}', big),
  ],
  [
    'a label of entities and line breaks',
    () => render('radar-beta\naxis a["' + '#35;&amp;<br>'.repeat(N / 13) + '"], b\ncurve c{1,2}', big),
  ],
  ['25,000 open brackets', () => parseRadar('radar-beta\naxis a' + '['.repeat(N / 2))],
  [
    '20,000 line breaks inside a curve',
    () => render('radar-beta\naxis a\ncurve c{' + '\n'.repeat(20000) + '1' + '\n'.repeat(20000) + '}', big),
  ],
  ['unclosed directives', () => parseRadar('radar-beta\n' + '%%{\n'.repeat(N / 4))],
  ['front matter fences', () => parseRadar('radar-beta\n' + '---\n'.repeat(N / 4))],
  ['a long title', () => render('radar-beta\ntitle ' + 'x'.repeat(N), big)],
  ['a line of spaces', () => parseRadar('radar-beta\n' + ' '.repeat(N) + 'axis a')],
];

describe('radar worst cases', () => {
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
});
