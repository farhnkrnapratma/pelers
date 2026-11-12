import { describe, expect, it } from 'vitest';
import { tokenize } from '../../src/diagrams/quadrant/lexer.js';
import { parseStyles } from '../../src/diagrams/quadrant/db.js';
import { PelersError, parse, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

// Inputs of about 50,000 characters built to hit the quadrant chart's worst cases. Each must finish
// quickly and either succeed or fail with a PelersError. A regression here is seconds or a hang.

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['a run of spaces with no keyword after it', () => tokenize('quadrantChart\n' + ' '.repeat(N) + '!')],
  ['a run of tabs before a colon and no bracket', () => tokenize('quadrantChart\nA' + '\t'.repeat(N) + ':x')],
  [
    'runs of tabs and colons, never a bracket',
    () => tokenize('quadrantChart\n' + ('\t'.repeat(500) + ':').repeat(100)),
  ],
  ['colons and spaces in turn', () => tokenize('quadrantChart\n' + ': '.repeat(N / 2))],
  ['a run of dashes with no arrowhead', () => tokenize('quadrantChart\nx-axis a ' + '-'.repeat(N))],
  [
    'spaces before dashes with no arrowhead',
    () => tokenize('quadrantChart\nx-axis a' + ('  ' + '-'.repeat(50)).repeat(1000)),
  ],
  ['line breaks each followed by a comment', () => tokenize('quadrantChart' + '\n%% c'.repeat(N / 5))],
  ['comment markers with no line break', () => tokenize('quadrantChart\n' + 'a%%'.repeat(N / 3))],
  [
    'accTitle with no colon, many times',
    () => tokenize('quadrantChart\n' + ('accTitle' + ' '.repeat(40)).repeat(1000)),
  ],
  ['an unclosed accDescr block', () => parse('quadrantChart\naccDescr {' + ' a\n'.repeat(N / 3), big)],
  ['an unclosed string', () => parse('quadrantChart\nquadrant-1 "' + 'a'.repeat(N), big)],
  ['an unclosed markdown string', () => parse('quadrantChart\nquadrant-1 "`' + 'a '.repeat(N / 2), big)],
  ['a point number followed by digits', () => tokenize('quadrantChart\nA: [0.' + '5'.repeat(N) + ', 1]')],
  ['space before the comma of a point', () => tokenize('quadrantChart\nA: [0' + ' '.repeat(N) + '1]')],
  ['a style of spaces around one colon', () => parseStyles(['radius' + ' '.repeat(N) + ':' + ' '.repeat(N) + '5'])],
  ['a style of spaces and no colon', () => parseStyles(['a' + ' '.repeat(N) + 'b'])],
  [
    'a class of 8,000 styles',
    () => render('quadrantChart\nA:::c: [0.5, 0.5]\nclassDef c ' + 'radius: 5,'.repeat(8000) + 'radius: 6', big),
  ],
  [
    'a point with 6,000 styles',
    () => render('quadrantChart\nA: [0.5, 0.5] ' + 'color: #abc ,'.repeat(6000) + 'radius: 6', big),
  ],
  [
    '3,500 points',
    () => render('quadrantChart\n' + repeat(3500, (i) => `P${i}: [0.${i % 100}, 0.${(i * 7) % 100}]\n`), big),
  ],
  ['300 points at one place', () => render('quadrantChart\n' + repeat(300, (i) => `Point ${i}: [0.5, 0.5]\n`), big)],
  [
    '3,000 classes',
    () => render('quadrantChart\n' + repeat(3000, (i) => `classDef c${i} radius: ${i}\n`) + 'A:::c7: [0.5, 0.5]', big),
  ],
  [
    'one point with a 50,000 character label',
    () => render('quadrantChart\n' + 'word '.repeat(N / 5) + ': [0.5, 0.5]', big),
  ],
  [
    'a quadrant label of emphasis markers',
    () => render('quadrantChart\nquadrant-1 "`' + '*a _b '.repeat(N / 6) + '`"', big),
  ],
  [
    'labels of entities and line breaks',
    () => render('quadrantChart\nx-axis "' + '#35;&amp;<br>'.repeat(N / 13) + '"', big),
  ],
  ['a title of 50,000 characters', () => render('quadrantChart\ntitle ' + 'long title '.repeat(N / 11), big)],
  ['a class name of 50,000 characters', () => render('quadrantChart\nA:::' + 'c'.repeat(N) + ': [0.5, 0.5]', big)],
  ['10,000 statements on one line', () => render('quadrantChart;' + 'quadrant-1 a;'.repeat(10000), big)],
  [
    'leading spaces before every statement',
    () => render('quadrantChart\n' + (' '.repeat(100) + 'quadrant-2 b\n').repeat(450), big),
  ],
  ['a radius of 400 digits', () => render('quadrantChart\nA: [0.5, 0.5] radius: ' + '9'.repeat(400), big)],
];

describe('quadrant chart worst cases', () => {
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
    expect(() => render('quadrantChart\n' + 'A: [0.5, 0.5]\n'.repeat(5000))).toThrow(/limit of 50000/);
  });
});
