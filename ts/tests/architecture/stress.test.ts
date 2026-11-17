import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const head = 'architecture-beta\n';
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const services = (count: number): string => repeat(count, (i) => `service n${i}\n`);

function completeGraph(): string {
  let s = head + services(70);
  for (let i = 0; i < 70; i++) for (let j = i + 1; j < 70; j++) s += `n${i}:R -- L:n${j}\n`;
  return s;
}

const diagonalGroup = (g: number, side: string): string =>
  `group g${g}\n` +
  repeat(12, (i) => `service m${g}x${i} in g${g}\n`) +
  repeat(11, (i) => `m${g}x${i}:${side}:m${g}x${i + 1}\n`);

const VALID: [string, string][] = [
  ['4,500 unconnected services', head + services(4500)],
  ['a chain of 2,000', head + services(2000) + repeat(1999, (i) => `n${i}:R -- L:n${i + 1}\n`)],
  ['a diagonal of 2,000', head + services(2000) + repeat(1999, (i) => `n${i}:R -- T:n${i + 1}\n`)],
  ['a star of 2,000 all wanting one cell', head + services(2000) + repeat(1999, (i) => `n0:R -- L:n${i + 1}\n`)],
  ['a star of 2,000 bends', head + services(2000) + repeat(1999, (i) => `n0:B -- L:n${i + 1}\n`)],
  ['a star of 2,000 like sides', head + services(2000) + repeat(1999, (i) => `n0:R -- R:n${i + 1}\n`)],
  [
    'a column of hubs with two leaves each',
    head +
      services(1800) +
      repeat(
        599,
        (i) => `n${i * 3}:B -- T:n${i * 3 + 3}\nn${i * 3}:R -- L:n${i * 3 + 1}\nn${i * 3}:R -- L:n${i * 3 + 2}\n`
      ),
  ],
  ['3,000 edges between two services', head + 'service a\nservice b\n' + 'a:R <-[x]-> L:b\n'.repeat(3000)],
  ['2,500 self edges', head + 'service a\n' + 'a:R --> L:a\n'.repeat(2500)],
  ['a complete graph of 70', completeGraph()],
  [
    '2,000 nested groups',
    head + 'group g0\n' + repeat(1999, (i) => `group g${i + 1} in g${i}\n`) + 'service a in g1999\n',
  ],
  [
    '1,000 nested groups with titles and edges to the outside',
    head +
      'group g0[T]\n' +
      repeat(999, (i) => `group g${i + 1}[T] in g${i}\n`) +
      'service a in g999\nservice b\n' +
      'a:R -- L:b\n'.repeat(500),
  ],
  [
    '1,200 sibling groups in a chain',
    head +
      repeat(1200, (i) => `group g${i}\nservice n${i} in g${i}\n`) +
      repeat(1199, (i) => `n${i}:R -- L:n${i + 1}\n`),
  ],
  [
    '1,000 groups each wanting the same place',
    head + 'service hub\n' + repeat(1000, (i) => `group g${i}\nservice n${i} in g${i}\nhub:R -- L:n${i}\n`),
  ],
  [
    'large groups all wanting the same place',
    head + 'service hub\n' + repeat(40, (g) => diagonalGroup(g, 'R -- T') + `hub:R -- L:m${g}x0\n`),
  ],
  [
    'large groups beside a column of 1,500',
    head +
      services(1500) +
      repeat(1499, (i) => `n${i}:B -- T:n${i + 1}\n`) +
      repeat(20, (g) => diagonalGroup(g, 'B -- R') + `n0:R -- L:m${g}x0\n`),
  ],
  [
    'titled groups over long rows',
    head +
      'group g0[A title]\n' +
      repeat(300, (i) => `group g${i + 1}[A title] in g${i}\n`) +
      repeat(1500, (i) => `service n${i} in g${i % 301}\n`) +
      repeat(1499, (i) => `n${i}:R -- L:n${i + 1}\n`),
  ],
  [
    'many groups crossed by many vertical edges',
    head +
      repeat(600, (i) => `group g${i}[Title]\nservice n${i} in g${i}\n`) +
      repeat(599, (i) => `n${i}:B --> T:n${i + 1}\n`) +
      'n0:B -- T:n599\n'.repeat(800),
  ],
  [
    'align directives over 2,000 members',
    head +
      services(2000) +
      'align row ' +
      repeat(1000, (i) => `n${i} `) +
      '\n' +
      repeat(1000, (i) => `align column n${i} n${i + 1000}\n`),
  ],
  ['one 50,000 character service title', head + 'service a[' + 'word '.repeat(N / 5) + ']'],
  ['one 50,000 character unbroken title', head + 'service a[' + 'w'.repeat(N) + ']'],
  ['one 50,000 character icon text', head + 'service a "' + 'word '.repeat(N / 5) + '"'],
  ['one 50,000 character unbroken icon text', head + 'service a "' + '\ud83d\ude00'.repeat(N / 2) + '"'],
  [
    'a 25,000 character group title and edge label',
    head +
      'group g[' +
      'word '.repeat(N / 10) +
      ']\nservice a in g\nservice b\na:R -[' +
      '*a _b '.repeat(N / 12) +
      ']- L:b',
  ],
  ['title of entities and line breaks', head + 'service a["' + '#35;&amp;<br>'.repeat(N / 13) + '"]'],
  ['quoted titles that never close, one per line', head + repeat(N / 20, (i) => `service n${i}["a]\n`)],
  ['single quoted titles that never close, one per line', head + repeat(N / 20, (i) => `service n${i}['a]\n`)],
  [
    'titles of escaped quotes after an open one',
    head + 'service a["x]\n' + repeat(N / 20, (i) => `service n${i}[\\"\\']\n`),
  ],
  ['title keyword on every line', head + 'title a %% b\n'.repeat(N / 13)],
  ['one line of 50,000 spaces', head + ' '.repeat(N) + 'service a'],
  ['one identifier of 50,000 characters', head + 'service ' + 'a-'.repeat(N / 2) + 'a'],
  ['keywords glued to identifiers', head + repeat(N / 20, (i) => `service service${i}\n`) + 'service in-in-in\n'],
  ['50,000 line breaks around the header', '\n'.repeat(3000) + head + '\n'.repeat(N) + 'service a'],
];

const ANY: [string, string][] = [
  ['unclosed brackets', head + 'service a' + '['.repeat(N)],
  ['unclosed parentheses', head + 'service a(' + 'a:-'.repeat(N / 3)],
  ['unclosed string', head + 'service a "' + 'a\\"'.repeat(N / 3)],
  ['strings that never end, one per line', head + repeat(N / 14, (i) => `service n${i} "a\n`)],
  ['runs of dashes', head + 'service a\nservice b\na:R ' + '-'.repeat(N) + ' L:b'],
  ['fence openers with no closer', head + 'service a\nservice b\n' + 'a:R ---\n'.repeat(N / 8)],
  ['unclosed directives on every line', head + '%%{\n'.repeat(N / 4)],
  ['accDescr with a brace that never closes', head + 'accDescr\n'.repeat(N / 12) + 'accDescr {' + ' a\n'.repeat(1000)],
  ['direction letters', head + 'L'.repeat(N)],
  ['fences that never close, one per line', head + repeat(N / 20, (i) => `service n${i} ---\n`)],
  ['accDescr words before one closed brace', head + 'accDescr\n'.repeat(N / 12) + 'accDescr { a }\n'],
  ['arrows', head + 'service a\nservice b\na:R ' + '<>'.repeat(N / 2)],
  ['group modifiers', head + 'service a\na' + '{group}'.repeat(N / 7)],
];

describe('architecture worst cases', () => {
  for (const [name, src] of VALID) {
    it(
      name,
      () => {
        const started = performance.now();
        expect(render(src, big).svg).toContain('</svg>');
        expect(performance.now() - started).toBeLessThan(3000);
      },
      30_000
    );
  }

  for (const [name, src] of ANY) {
    it(
      name,
      () => {
        const started = performance.now();
        try {
          render(src, big);
        } catch (error) {
          expect(error, (error as Error).message).toBeInstanceOf(PelersError);
        }
        expect(performance.now() - started).toBeLessThan(3000);
      },
      30_000
    );
  }

  it('refuses source over the default limit', () => {
    expect(() => render(head + services(6000))).toThrow(/limit of 50000/);
  });
});
