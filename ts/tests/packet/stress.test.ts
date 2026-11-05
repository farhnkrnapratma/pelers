import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { parsePacket } from '../../src/diagrams/packet/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const config = (packet: string, body: string): string => `---\nconfig:\n  packet:\n${packet}\n---\n${body}`;

const CASES: [string, () => unknown][] = [
  ['4,000 single-bit fields', () => render('packet\n' + repeat(4000, (i) => `${i}: "f${i}"\n`), big)],
  ['5,000 counted fields', () => render('packet\n' + repeat(5000, (i) => `+${(i % 40) + 1}: "f"\n`), big)],
  ['one field of a billion bits', () => render('packet\n+1000000000: "wide"\n+1: "after"', big)],
  ['a field of more bits than a number holds', () => render('packet\n+' + '9'.repeat(400) + ': "x"\n+1: "y"', big)],
  ['ten thousand rows of one bit', () => render(config('    bitsPerRow: 1', 'packet\n+20000: "a"'), big)],
  ['a 50,000 character label on one field', () => render('packet\n0-31: "' + 'word '.repeat(N / 5) + '"', big)],
  [
    'a 40,000 character label on ten thousand rows',
    () => render(config('    bitsPerRow: 1', 'packet\n+20000: "' + 'x'.repeat(40000) + '"'), big),
  ],
  ['a label of entities', () => render('packet\n0-31: "' + '#35;&amp;'.repeat(N / 9) + '"', big)],
  ['a 50,000 digit number', () => parsePacket('packet\n' + '1'.repeat(N) + ': "a"')],
  ['50,000 zeros', () => parsePacket('packet\n' + '0'.repeat(N))],
  ['50,000 dashes', () => parsePacket('packet\n0' + '-'.repeat(N))],
  ['dashes that open front matter 12,000 times', () => parsePacket('packet\n' + '---\n'.repeat(N / 4))],
  ['an unclosed string', () => parsePacket('packet\n0: "' + 'a'.repeat(N))],
  ['a string of backslashes', () => parsePacket('packet\n0: "' + '\\'.repeat(N))],
  ['unclosed directives', () => parsePacket('packet\n' + '%%{\n'.repeat(N / 4))],
  ['an unclosed accDescr block', () => parsePacket('packet\naccDescr {' + ' a\n'.repeat(N / 3))],
  ['a long title', () => render('packet title ' + 'x'.repeat(N), big)],
  ['a line of spaces', () => parsePacket('packet\n' + ' '.repeat(N) + '0: "a"')],
  ['20,000 blank lines', () => parsePacket('packet\n' + '\n \t'.repeat(N / 3) + '0: "a"')],
  ['20,000 comments', () => parsePacket('packet\n' + '%% c\n'.repeat(N / 5) + '0: "a"')],
  ['a title on every line', () => parse('packet\n' + 'title t\n'.repeat(N / 8), big)],
];

describe('packet worst cases', () => {
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
