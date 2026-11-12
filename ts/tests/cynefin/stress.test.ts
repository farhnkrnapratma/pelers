import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const HEAD = 'cynefin-beta\n';

const CASES: [string, string][] = [
  ['10,000 items in one domain', HEAD + 'complex\n' + repeat(10000, (i) => `"i${i}"\n`)],
  ['10,000 items in confusion', HEAD + 'confusion\n' + '"x"\n'.repeat(N / 4)],
  ['8,000 domain blocks', HEAD + 'clear\n"a"\n'.repeat(N / 10)],
  ['5,000 transitions', HEAD + repeat(5000, (i) => `complex --> clear : "t${i}"\n`)],
  [
    '5,000 transitions between every pair',
    HEAD +
      repeat(
        5000,
        (i) =>
          ['complex', 'clear', 'chaotic', 'confusion', 'complicated'][i % 5] +
          ' --> ' +
          ['clear', 'chaotic', 'confusion', 'complicated', 'complex'][(i + (i % 3)) % 5] +
          '\n'
      ),
  ],
  ['one 50,000 character item', HEAD + 'complex\n"' + 'word '.repeat(N / 5) + '"'],
  ['one 50,000 character word', HEAD + 'complex\n"' + 'a'.repeat(N) + '"'],
  ['one 50,000 character transition label', HEAD + 'complex --> clear : "' + 'word '.repeat(N / 5) + '"'],
  ['one 50,000 character title', HEAD + 'title ' + 'x'.repeat(N)],
  ['unclosed string', HEAD + 'complex\n"' + 'a'.repeat(N)],
  ['string of backslashes', HEAD + 'complex\n"' + '\\'.repeat(N)],
  ['string of escaped quotes', HEAD + 'complex\n"' + '\\"'.repeat(N / 2) + '"'],
  ['many unclosed strings', HEAD + 'complex\n' + '"a\n'.repeat(N / 3)],
  ['unclosed accDescr', HEAD + 'accDescr {' + ' a\n'.repeat(N / 3)],
  ['unclosed directive openers', HEAD + '%%{\n'.repeat(N / 4)],
  ['unclosed front matter fences', HEAD + 'complex\n---\n'.repeat(N / 12)],
  ['blank lines', HEAD + '\n'.repeat(N)],
  ['spaces before an item', HEAD + 'complex\n' + ' '.repeat(N) + '"a"'],
  ['comments', HEAD + '%% c\n'.repeat(N / 5)],
  ['entities and line breaks in an item', HEAD + 'complex\n"' + '#35;&amp;<br>'.repeat(N / 13) + '"'],
  ['domain names run together', HEAD + 'complex'.repeat(N / 7)],
];

describe('cynefin worst cases', () => {
  for (const [name, src] of CASES) {
    it(
      name,
      () => {
        const started = performance.now();
        try {
          render(src, big);
        } catch (error) {
          expect(error, (error as Error).message).toBeInstanceOf(PelersError);
        }
        expect(performance.now() - started).toBeLessThan(2000);
      },
      30_000
    );
  }

  it('refuses source over the default limit', () => {
    expect(() => render(HEAD + 'complex\n' + '"a"\n'.repeat(15000))).toThrow(/limit of 50000/);
  });
});
