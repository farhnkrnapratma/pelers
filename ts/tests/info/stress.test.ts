import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { parseInfo } from '../../src/diagrams/info/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };

const CASES: [string, () => unknown][] = [
  ['a 50,000 character title', () => render('info\ntitle ' + 'word '.repeat(N / 5), big)],
  ['6,000 titles', () => render('info\n' + 'title t\n'.repeat(N / 8), big)],
  ['a title of entities and line breaks', () => render('info\ntitle ' + '#35;&amp;<br>'.repeat(N / 13), big)],
  ['25,000 blank lines', () => render('info' + '\n '.repeat(N / 2) + 'showInfo', big)],
  ['a line of spaces', () => parseInfo('info' + ' '.repeat(N) + 'showInfo')],
  ['20,000 comments', () => parseInfo('info\n' + '%% c\n'.repeat(N / 5))],
  ['unclosed directives', () => parseInfo('info\n' + '%%{\n'.repeat(N / 4))],
  ['front matter fences', () => parseInfo('info\n' + '---\n'.repeat(N / 4))],
  ['an unclosed accDescr block', () => parseInfo('info\naccDescr {' + ' a\n'.repeat(N / 3))],
  ['a closed accDescr block of 50,000 characters', () => render('info\naccDescr {' + ' a\n'.repeat(N / 3) + '}', big)],
  ['a long accTitle', () => render('info\naccTitle: ' + 'x'.repeat(N), big)],
  ['the keyword 10,000 times', () => parseInfo('info '.repeat(N / 5))],
  ['showInfo run together', () => parseInfo('info ' + 'showInfo'.repeat(N / 8))],
];

describe('info worst cases', () => {
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
