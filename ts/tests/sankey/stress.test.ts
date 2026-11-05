import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/sankey/lexer.js';
import { prepareTextForParsing } from '../../src/diagrams/sankey/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const HEAD = 'sankey\n';

const CASES: [string, () => unknown][] = [
  [
    'preparing text with long runs of blanks',
    () => prepareTextForParsing(HEAD + ('a,b,1' + ' '.repeat(500) + '\n').repeat(N / 506)),
  ],
  // Mermaid's own regex for this step rescans the run from each of its positions and takes seconds here.
  [
    'preparing one run of 50,000 blanks inside the text',
    () => prepareTextForParsing(HEAD + 'a,b,1' + ' '.repeat(N) + 'x'),
  ],
  ['preparing 50,000 line breaks', () => prepareTextForParsing(HEAD + '\r\n'.repeat(N / 2) + 'a,b,1')],
  ['one field of 50,000 characters', () => tokenize(HEAD + 'x'.repeat(N) + ',b,1')],
  ['one unclosed quoted field', () => tokenize(HEAD + '"' + 'x'.repeat(N))],
  ['a quoted field made of escaped quotes', () => tokenize(HEAD + '"' + '""'.repeat(N / 2) + '",b,1')],
  ['quotes and nothing else', () => tokenize(HEAD + '"'.repeat(N))],
  ['commas and nothing else', () => tokenize(HEAD + ','.repeat(N))],
  ['a character outside the alphabet after a long field', () => tokenize(HEAD + 'x'.repeat(N) + '\t')],
  ['the keyword repeated', () => tokenize('sankey'.repeat(N / 6))],
  ['a quoted field spanning 10,000 lines', () => parse(HEAD + '"' + 'line\n'.repeat(10000) + '",b,1', big)],
  ['8,000 links between 8,001 nodes in a chain', () => render(HEAD + repeat(8000, (i) => `${i},${i + 1},1\n`), big)],
  ['a star of 6,000 targets', () => render(HEAD + repeat(6000, (i) => `c,t${i},${i + 1}\n`), big)],
  ['6,000 sources into one target', () => render(HEAD + repeat(6000, (i) => `s${i},c,${i + 1}\n`), big)],
  ['8,000 parallel links', () => render(HEAD + 'a,b,1\n'.repeat(8000), big)],
  [
    'a hub joined to 2,500 columns',
    () => render(HEAD + repeat(2500, (i) => `n${i},n${i + 1},1\nhub,n${i + 1},1\n`), big),
  ],
  [
    'a dense two-column mesh',
    () => {
      let s = HEAD;
      for (let i = 0; i < 70; i++) for (let j = 0; j < 70; j++) s += `a${i},b${j},${(i * 7 + j) % 13}\n`;
      return render(s, big);
    },
  ],
  [
    'a layered mesh of 40 columns',
    () => {
      let s = HEAD;
      for (let l = 0; l < 40; l++)
        for (let i = 0; i < 12; i++)
          for (let j = 0; j < 12; j += 3) s += `${l}_${i},${l + 1}_${(i + j) % 12},${1 + ((i + j) % 5)}\n`;
      return render(s, big);
    },
  ],
  ['a cycle of 8,000', () => render(HEAD + repeat(8000, (i) => `${i},${(i + 1) % 8000},1\n`), big)],
  [
    'a long tail hanging off a small cycle',
    () => render(HEAD + 'a,b,1\nb,a,1\n' + repeat(7000, (i) => `${i},${i + 1},1\n`) + '7000,a,1\n', big),
  ],
  ['8,000 self links', () => render(HEAD + repeat(8000, (i) => `${i},${i},1\n`), big)],
  [
    'values at the edge of what a number holds',
    () => render(HEAD + repeat(3000, (i) => `a${i % 50},b${i % 60},${i % 2 ? '1e308' : '5e-324'}\n`), big),
  ],
  ['values that are not numbers', () => render(HEAD + repeat(5000, (i) => `a${i % 50},b${i % 60},x\n`), big)],
  [
    'node names of 5,000 characters',
    () => render(HEAD + repeat(5, (i) => `${String(i).repeat(5000)},${String(i + 1).repeat(5000)},1\n`), big),
  ],
];

describe('sankey worst cases', () => {
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

  it('refuses source over the default limit', () => {
    expect(() => render(HEAD + 'a,b,1\n'.repeat(10000))).toThrow(/limit of 50000/);
    expect(render(HEAD + 'a,b,1\n'.repeat(10000), big).svg).toContain('<svg');
  });
});
