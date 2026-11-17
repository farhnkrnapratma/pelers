import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const HEAD = 'wardley-beta\n';
const frac = (i: number): string => (0.001 * (i % 1000)).toFixed(3);

const CASES: [string, string][] = [
  ['a run of zeros', HEAD + 'component A [0.5, 0.5]\n' + '0'.repeat(N - 100)],
  ['digits and dots in turn', HEAD + 'component A [0.5, 0.5]\n' + '1.'.repeat(N / 2 - 100)],
  ['2,000 components', HEAD + repeat(2000, (i) => `component c${i} [${frac(i)}, ${frac(i * 7)}]\n`)],
  [
    'chain of 1,500 links',
    HEAD +
      repeat(1500, (i) => `component c${i} [${frac(i)}, ${frac(i * 7)}]\n`) +
      repeat(1500, (i) => `c${i} -> c${i + 1}\n`),
  ],
  [
    '3,000 links to unknown names',
    HEAD + repeat(500, (i) => `component c${i} [0.5, ${frac(i)}]\n`) + repeat(3000, (i) => `u${i} -> v${i}\n`),
  ],
  [
    '3,000 links between two nodes, with labels',
    HEAD + 'component A [0.9, 0.1]\ncomponent B [0.1, 0.9]\n' + repeat(3000, (i) => `A +<> B; label ${i}\n`),
  ],
  ['the same component 3,000 times', HEAD + 'component A [0.5, 0.5]\n'.repeat(3000)],
  [
    'pipeline of 2,000 components',
    HEAD + 'component P [0.5, 0.5]\npipeline P {\n' + repeat(2000, (i) => ` component p${i} [${frac(i)}]\n`) + '}\n',
  ],
  [
    '1,000 pipelines',
    HEAD + repeat(1000, (i) => `component P${i} [${frac(i)}, 0.5]\npipeline P${i} {\n component x [0.2]\n}\n`),
  ],
  [
    '2,000 annotations',
    HEAD + 'annotations [0.9, 0.1]\n' + repeat(2000, (i) => `annotation ${i},[${frac(i)}, 0.5] "text ${i}"\n`),
  ],
  [
    '2,000 notes and forces',
    HEAD +
      repeat(
        700,
        (i) =>
          `note "n${i}" [0.5, ${frac(i)}]\naccelerator a${i} [0.2, ${frac(i)}]\ndeaccelerator d${i} [0.8, ${frac(i)}]\n`
      ),
  ],
  [
    '2,000 evolves',
    HEAD + repeat(1000, (i) => `component c${i} [0.5, 0.1]\n`) + repeat(1000, (i) => `evolve c${i} 0.9\n`),
  ],
  ['evolution with 5,000 stages', HEAD + 'evolution ' + repeat(5000, (i) => `s${i}@0.5 -> `) + 'end\n'],
  ['one 50,000 character name', HEAD + 'component ' + 'a'.repeat(N) + ' [0.5, 0.5]'],
  ['one name of 12,000 words', HEAD + 'component ' + 'ab '.repeat(N / 4) + '[0.5, 0.5]'],
  ['one 50,000 character quoted name', HEAD + 'component "' + 'word '.repeat(N / 5) + '" [0.5, 0.5]'],
  ['one 50,000 character note', HEAD + 'note "' + 'word '.repeat(N / 5) + '" [0.5, 0.5]'],
  [
    'one 50,000 character link label',
    HEAD + 'component A [0.1, 0.1]\ncomponent B [0.9, 0.9]\nA -> B; ' + 'x'.repeat(N),
  ],
  ['names followed by long blanks', HEAD + ('a' + ' '.repeat(99)).repeat(N / 100) + '1'],
  ['words that almost continue a name', HEAD + ('a' + ' '.repeat(49) + '1' + ' '.repeat(49)).repeat(N / 100)],
  ['a digit then 50,000 hyphens', HEAD + '1' + '-'.repeat(N) + '!'],
  ['digits and hyphens, many times', HEAD + ('1' + '-'.repeat(99) + ' ').repeat(N / 101)],
  ['50,000 hyphens', HEAD + '-'.repeat(N)],
  ['lines of three hyphens', HEAD + '---\n'.repeat(N / 4)],
  ['unclosed flow labels', HEAD + "A +'x\n".repeat(N / 6)],
  ['one unclosed flow label', HEAD + "A +'" + 'x'.repeat(N)],
  ['unclosed strings', HEAD + 'component "a\n'.repeat(N / 13)],
  ['unclosed accDescr on every line', HEAD + 'accDescr {\n'.repeat(N / 11)],
  ['accTitle with no colon, many times', HEAD + ('accTitle' + ' '.repeat(40)).repeat(N / 48)],
  ['unclosed directive openers', HEAD + '%%{\n'.repeat(N / 4)],
  ['a 50,000 digit number', HEAD + 'size [' + '9'.repeat(N) + ', 1]'],
  ['a 50,000 digit fraction', HEAD + 'component A [0.' + '5'.repeat(N) + ', 0.5]'],
  ['digits before a dot, many times', HEAD + ('1'.repeat(98) + '. ').repeat(N / 100)],
  ['blank lines', HEAD + '\n'.repeat(N)],
  ['comments', HEAD + 'component A [0.5, 0.5] %% c\n'.repeat(N / 28)],
  ['keywords run together', HEAD + 'componentannotationsevolve'.repeat(N / 26)],
];

describe('wardley worst cases', () => {
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
    expect(() => render(HEAD + 'component A [0.5, 0.5]\n'.repeat(3000))).toThrow(/limit of 50000/);
  });
});
