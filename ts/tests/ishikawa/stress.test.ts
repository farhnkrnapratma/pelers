import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/ishikawa/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const I = 'ishikawa-beta\n';
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['50,000 spaces', () => tokenize(I + ' '.repeat(N))],
  ['50,000 line breaks', () => tokenize(I + '\n'.repeat(N))],
  ['spaces and line breaks by turns', () => tokenize(I + ' \n'.repeat(N / 2))],
  ['long runs of spaces, each ending in a line break', () => tokenize(I + (' '.repeat(999) + '\n').repeat(N / 1000))],
  ['long runs of spaces, each ending in text', () => tokenize(I + (' '.repeat(999) + 'x\n').repeat(N / 1000))],
  ['a line break and then 50,000 spaces', () => tokenize(I + 'x\n' + ' '.repeat(N))],
  ['white space of every kind before a comment that never comes', () => tokenize(I + ' \t\r  \n'.repeat(N / 6) + '%')],
  ['white space of every kind before a comment', () => tokenize(I + ' \t\r  \n'.repeat(N / 6) + '%% c')],
  ['comment lines', () => tokenize(I + '  %% c\n'.repeat(N / 7))],
  ['percent signs', () => tokenize(I + '% '.repeat(N / 2))],
  ['carriage returns', () => tokenize(I + 'x\r'.repeat(N / 2))],
  ['the keyword over and over', () => tokenize('ishikawa-beta '.repeat(N / 14))],
  ['the keyword without its ending, over and over', () => tokenize('ishikawa-bet\n'.repeat(N / 13))],
  ['one line of 50,000 characters', () => render(I + '  ' + 'a'.repeat(N), big)],
  ['one line of 10,000 words', () => render(I + '  Effect\n    ' + 'word '.repeat(10000), big)],
  [
    'a cause of entities and markup',
    () => render(I + '  Effect\n    Category\n      ' + '#35;&amp;<br>'.repeat(N / 13), big),
  ],
  ['12,000 categories', () => render(I + '  E\n' + '    c\n'.repeat(12000), big)],
  ['12,000 causes in one category', () => render(I + '  E\n    C\n' + '      c\n'.repeat(12000), big)],
  ['12,000 sub-causes of one cause', () => render(I + '  E\n    C\n      c\n' + '        s\n'.repeat(12000), big)],
  ['an outline 300 levels deep', () => render(I + repeat(300, (i) => ' '.repeat(i + 1) + 'n\n'), big)],
  ['an outline 3,000 levels deep', () => render(I + repeat(3000, (i) => ' '.repeat(i + 1) + 'n\n'), big)],
  [
    'an outline 3,000 levels deep, twice',
    () => render(I + ' E\n' + repeat(2, () => repeat(3000, (i) => ' '.repeat(i + 2) + 'n\n')), big),
  ],
  [
    'indentation that goes up and down',
    () => render(I + repeat(8000, (i) => ' '.repeat(1 + ((i * 7) % 13)) + 'n\n'), big),
  ],
  ['a sawtooth of indentation', () => parse(I + repeat(200, () => repeat(40, (i) => ' '.repeat(i + 1) + 'n\n')), big)],
  [
    'indentation by 2,000 spaces',
    () => render(I + ' E\n' + repeat(20, (i) => ' '.repeat(1 + (i % 2) * 2000) + 'n\n'), big),
  ],
  [
    '4,000 categories with a cause and a sub-cause each',
    () => render(I + '  E\n' + '    c\n      d\n        e\n'.repeat(4000), big),
  ],
];

describe('Ishikawa worst cases', () => {
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

  it('keeps a deep outline finite and in order', () => {
    const { svg, width, height } = render(I + repeat(400, (i) => ' '.repeat(i + 1) + `n${i}\n`), big);
    expect(svg).not.toMatch(/NaN|Infinity/);
    expect(Number.isFinite(width) && Number.isFinite(height)).toBe(true);
    expect(svg).toContain('>n399</text>');
  });

  it('refuses source over the default limit', () => {
    expect(() => render(I + '  cause\n'.repeat(8000))).toThrow(/limit of 50000/);
  });
});
