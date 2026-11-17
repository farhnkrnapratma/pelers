import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/block/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['one row of 16,000 blocks', () => render('block\n' + repeat(16000, (i) => `b${i} `), big)],
  ['one column of 10,000 blocks', () => render('block\ncolumns 1\n' + repeat(10000, (i) => `b${i}\n`), big)],
  ['grid of 10,000 labelled blocks', () => render('block\ncolumns 100\n' + repeat(8000, (i) => `b${i}["x"] `), big)],
  ['8,000 nested composites', () => render('block\n' + 'block\n'.repeat(8000) + 'a\n' + 'end\n'.repeat(8000), big)],
  [
    '5,000 nested titled composites',
    () => render('block\n' + repeat(5000, (i) => `block:g${i}\n`) + 'a\n' + 'end\n'.repeat(5000), big),
  ],
  ['12,000 unclosed composites', () => parse('block\n' + 'block\n'.repeat(12000) + 'a', big)],
  ['12,000 stray ends', () => parse('block\na\n' + 'end\n'.repeat(12000), big)],
  ['5,000 sibling composites', () => render('block\ncolumns 50\n' + repeat(5000, (i) => `block\nn${i}\nend\n`), big)],
  ['one statement of 8,000 links', () => render('block\na0' + repeat(8000, (i) => `-->a${i + 1}`), big)],
  [
    '4,000 edges across a grid',
    () =>
      render(
        'block\ncolumns 20\n' +
          repeat(400, (i) => `n${i} `) +
          '\n' +
          repeat(4000, (i) => `n${i % 400} --> n${(i * 37 + 11) % 400}\n`),
        big
      ),
  ],
  [
    'complete graph of 70',
    () => {
      let s = 'block\ncolumns 9\n';
      for (let i = 0; i < 70; i++) for (let j = i + 1; j < 70; j++) s += `n${i}-->n${j}\n`;
      return render(s, big);
    },
  ],
  [
    '4,000 parallel edges and 4,000 self loops',
    () => render('block\nA space B\n' + 'A-->B\n'.repeat(4000) + 'A-->A\n'.repeat(4000), big),
  ],
  ['3,000 labelled edges', () => render('block\nA space:3 B\n' + 'A -- "label text" --> B\n'.repeat(2500), big)],
  ['more space than allowed', () => render('block\n' + 'space:99999 '.repeat(4000), big)],
  ['space count of 400 digits', () => render('block\na space:' + '9'.repeat(400) + ' b', big)],
  ['99,999 spaces in one row', () => render('block\na space:99999 b', big)],
  ['block width of 400 digits', () => render('block\ncolumns 3\na:' + '9'.repeat(400) + ' b c', big)],
  ['column count of 400 digits', () => render('block\ncolumns ' + '9'.repeat(400) + '\na b', big)],
  ['many wide blocks', () => render('block\n' + repeat(5000, (i) => `b${i}:999 `), big)],
  ['one 50,000 character label', () => render('block\nA["' + 'word '.repeat(N / 5) + '"]', big)],
  ['label of entities and line breaks', () => render('block\nA["' + '#35;&amp;<br>'.repeat(N / 13) + '"]', big)],
  ['unclosed label', () => parse('block\nA["' + 'x'.repeat(N), big)],
  ['unclosed markdown label', () => parse('block\nA["`' + 'x`'.repeat(N / 2), big)],
  ['arrow label then nothing', () => parse('block\nA<["' + 'x'.repeat(N) + '"]>', big)],
  ['arrow with 10,000 directions', () => render('block\nA<["x"]>(' + 'up, down , x,y '.repeat(2500) + ')', big)],
  ['arrow directions made of spaces', () => parse('block\nA<["x"]>(' + ' '.repeat(N) + 'sideways)', big)],
  ['arrow close then spaces', () => parse('block\nA<["x"]>' + ' '.repeat(N) + 'x', big)],
  [
    'class list of 25,000 ids',
    () => render('block\nA\nclass ' + 'A,'.repeat(25000) + 'A hot\nclassDef hot fill:#f96', big),
  ],
  [
    'class list with spaces after each comma',
    () => parse('block\nA\nclass A' + (',' + ' '.repeat(40)).repeat(1200) + ' x', big),
  ],
  ['style list of commas', () => render('block\nA\nstyle A ' + ','.repeat(N), big)],
  ['style of semicolons', () => render('block\nA\nstyle A ' + 'fill:red;'.repeat(N / 9), big)],
  ['classDef with no semicolon', () => render('block\nA\nclassDef c ' + 'x'.repeat(N) + '\nclass A c', big)],
  [
    'classDef of 10,000 entries',
    () => render('block\nA\nclassDef c ' + 'fill:red,'.repeat(10000) + 'x\nclass A c', big),
  ],
  [
    '5,000 classes on one block',
    () => render('block\nA\n' + repeat(2500, (i) => `classDef c${i} fill:red\nclass A c${i}\n`), big),
  ],
  ['classDef id then spaces', () => parse('block\nclassDef ' + 'a'.repeat(N / 2) + ' '.repeat(N / 2), big)],
  ['columns then spaces', () => tokenize('block\n' + ('columns' + ' '.repeat(40) + 'x ').repeat(1000))],
  ['accTitle then spaces', () => tokenize('block\n' + ('accTitle' + ' '.repeat(40) + 'x ').repeat(1000))],
  ['accDescr never closed', () => parse('block\nA\naccDescr {' + ' a\n'.repeat(15000), big)],
  ['runs of dashes, dots, equals', () => tokenize('block\nA' + '-'.repeat(N) + '.'.repeat(N) + '='.repeat(N))],
  ['dotted link that never ends', () => tokenize('block\nA -' + '.'.repeat(N) + 'x')],
  ['link label then spaces', () => tokenize('block\nA -- "x"' + ' '.repeat(N) + 'y')],
  ['links without blocks', () => parse('block\n' + '--> '.repeat(N / 4), big)],
  ['one id of 50,000 characters', () => render('block\n' + 'a'.repeat(N), big)],
  ['25,000 one-letter statements on one line', () => parse('block ' + 'a;'.repeat(N / 2), big)],
  ['brackets only', () => tokenize('block\n' + '(([[{{'.repeat(N / 6))],
  [
    'prototype names',
    () =>
      render(
        'block\n__proto__ constructor toString\nclassDef __proto__ fill:red\nclass constructor,toString __proto__\nstyle __proto__ fill:blue\n__proto__ --> constructor',
        big
      ),
  ],
];

describe('block worst cases', () => {
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

  it('refuses space counts past the limit and keeps the ones within it', () => {
    expect(() => parse('block\na space:100001 b')).toThrow(/at most 100000 space blocks/);
    expect(() => parse('block\n' + 'space:60000 '.repeat(2))).toThrow(PelersError);
    const model = parse('block\na space:5 b');
    expect(model.type === 'block' && model.blocks.length).toBe(7);
  });

  it('keeps every number in the output finite', () => {
    for (const src of [
      'block\ncolumns 3\na:' + '9'.repeat(400) + ' b c',
      'block\na:' + '9'.repeat(400) + ' b c',
      'block\na:0 b:00 c',
      'block\ncolumns 99999999999999999999\na b',
      'block\nblock:g:99999999\na\nend\nb',
    ]) {
      const { svg, width, height } = render(src, big);
      expect(Number.isFinite(width) && Number.isFinite(height), src.slice(0, 40)).toBe(true);
      expect(svg, src.slice(0, 40)).not.toMatch(/NaN|Infinity/);
    }
  });
});
