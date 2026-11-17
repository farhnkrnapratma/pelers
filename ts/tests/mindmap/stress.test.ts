import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/mindmap/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity, outputLimit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['one line of shaped nodes', () => tokenize('mindmap\n' + '(a) '.repeat(N / 4))],
  ['one line of spaces between ids', () => tokenize('mindmap\n' + 'a(b)' + ' '.repeat(N) + 'x')],
  ['blank lines of spaces', () => tokenize('mindmap\nroot\n' + ' \n'.repeat(N / 2) + ' a')],
  [
    'a newline then a long run of spaces, repeated',
    () => tokenize('mindmap\nroot' + ('\n' + ' '.repeat(99) + 'a').repeat(N / 100)),
  ],
  ['whitespace that never reaches a comment', () => tokenize('mindmap\nroot\n' + ' \t\n'.repeat(N / 3) + '%')],
  ['comment markers', () => tokenize('mindmap\nroot\n' + '%%'.repeat(N / 2))],
  ['comment lines', () => tokenize('mindmap\nroot\n' + ' %% c\n'.repeat(N / 6))],
  ['lone closing braces in a node', () => tokenize('mindmap\nroot[' + '}\n'.repeat(N / 2) + ']')],
  ['closing braces on one line in a node', () => tokenize('mindmap\nroot[' + '} '.repeat(N / 2) + ']')],
  ['unclosed quoted text', () => tokenize('mindmap\nroot["' + 'a'.repeat(N))],
  ['unclosed markdown text', () => tokenize('mindmap\nroot["`' + 'a\n'.repeat(N / 2))],
  ['backticks in markdown text', () => tokenize('mindmap\nroot["`' + 'a`'.repeat(N / 2))],
  ['unclosed icon', () => tokenize('mindmap\nroot\n::icon(' + 'x\n'.repeat(N / 2))],
  ['empty class lines', () => tokenize('mindmap\nroot\n' + ':::\n'.repeat(N / 4))],
  ['dashes and brackets', () => tokenize('mindmap\n' + '-)(-'.repeat(N / 4))],
  ['keyword prefixes', () => tokenize('mindma'.repeat(N / 6))],
  ['16,000 nodes on one level, parsed', () => parse('mindmap\nroot\n' + ' a\n'.repeat(16000), big)],
  ['16,000 nodes on one level, rendered', () => render('mindmap\nroot\n' + ' a\n'.repeat(16000), big)],
  ['a chain as deep as indentation allows', () => render('mindmap\n' + repeat(310, (i) => ' '.repeat(i) + 'n\n'), big)],
  ['3,000 levels of nesting', () => render('mindmap\n' + repeat(3000, (i) => ' '.repeat(i) + 'n\n'), big)],
  [
    'indentation that rises and falls',
    () => render('mindmap\nroot\n' + repeat(2400, (i) => ' '.repeat(1 + (i % 40)) + 'n\n'), big),
  ],
  [
    'deep branches each followed by a shallow one',
    () => render('mindmap\nroot\n' + repeat(160, () => repeat(24, (i) => ' '.repeat(1 + i) + 'n\n')), big),
  ],
  ['5,000 branches with two leaves each', () => render('mindmap\nroot\n' + ' b\n  l\n  l\n'.repeat(5000), big)],
  [
    '1,400 shaped nodes with icons and classes',
    () => render('mindmap\nroot\n' + ' a((b))\n ::icon(fa fa-x)\n :::c d\n'.repeat(1400), big),
  ],
  ['one 50,000 character label', () => render('mindmap\nroot[' + 'word '.repeat(N / 5) + ']', big)],
  ['one 50,000 character word', () => render('mindmap\nroot[' + 'w'.repeat(N) + ']', big)],
  ['label of emphasis markers', () => render('mindmap\nroot["`' + '*a _b '.repeat(N / 6) + '`"]', big)],
  ['label of line breaks and entities', () => render('mindmap\nroot[' + '#35;&amp;<br>'.repeat(N / 13) + ']', big)],
  ['a class list of 25,000 names', () => render('mindmap\nroot\n:::' + 'a '.repeat(N / 2), big)],
  ['an icon name of 50,000 characters', () => render('mindmap\nroot\n::icon(' + 'fa '.repeat(N / 3) + ')', big)],
  ['a second root after 16,000 nodes', () => render('mindmap\n root\n' + '  a\n'.repeat(16000) + ' b\n', big)],
];

describe('mindmap worst cases', () => {
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

  it('lays out wide and deep trees without overflow', () => {
    const wide = render('mindmap\nroot\n' + ' a\n'.repeat(16000), big);
    expect(Number.isFinite(wide.height)).toBe(true);
    expect(wide.svg.match(/class="pelers-edge"/g)?.length).toBe(16000);
    const deep = render('mindmap\n' + repeat(3000, (i) => ' '.repeat(i) + 'n\n'), big);
    expect(deep.svg.match(/class="pelers-node/g)?.length).toBe(3000);
  });
});
