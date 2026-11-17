import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const HEAD = 'eventmodeling\n';
const id = (i: number): string => String(i % 1000).padStart(3, '0');
const KINDS = ['ui', 'cmd', 'evt', 'rmo', 'pcr'];

const CASES: [string, string][] = [
  ['1,000 frames across the lanes', HEAD + repeat(1000, (i) => `tf ${id(i)} ${KINDS[i % 5]} E${i}\n`)],
  ['1,000 frames in one lane', HEAD + repeat(1000, (i) => `tf ${id(i)} evt E${i}\n`)],
  ['1,000 frames in 1,000 namespaces', HEAD + repeat(1000, (i) => `tf ${id(i)} evt N${i}.E\n`)],
  [
    '1,000 frames that each name every earlier frame',
    HEAD + repeat(1000, (i) => `tf ${id(i)} evt E${i}${repeat(Math.min(i, 9), (k) => ` ->> ${id(i - k - 1)}`)}\n`),
  ],
  ['one frame with 9,000 sources', HEAD + 'tf 001 cmd A\ntf 002 evt B' + ' ->> 001'.repeat(9000) + '\n'],
  ['a frame number reused 5,000 times', HEAD + 'tf 01 evt A\n'.repeat(5000)],
  [
    '500 data blocks, each referenced',
    HEAD +
      repeat(500, (i) => `tf ${id(i)} evt E${i} [[D${i}]]\n`) +
      repeat(500, (i) => `data D${i} {\n  a: ${i}\n  b: true\n}\n`),
  ],
  ['one data block of 10,000 lines', HEAD + 'tf 01 evt A [[D]]\ndata D {\n' + 'a: 1\n'.repeat(10000) + '}\n'],
  ['one data line of 50,000 characters', HEAD + 'tf 01 evt A { ' + 'x'.repeat(N) + ' }\n'],
  ['inline data of 25,000 braces', HEAD + 'tf 01 evt A ' + '{'.repeat(N / 2) + '}'.repeat(N / 2) + '\n'],
  [
    '2,000 notes and specifications',
    HEAD + 'tf 01 evt A\n' + repeat(1000, (i) => `note 01 {\n n${i}\n}\ngwt 01 given evt A then evt B\n`),
  ],
  [
    'a specification of 10,000 statements',
    HEAD + 'tf 01 evt A\ngwt 01 given' + ' evt A'.repeat(5000) + ' then' + ' evt B'.repeat(5000) + '\n',
  ],
  ['a qualified name of 10,000 parts', HEAD + 'tf 01 evt ' + 'a.'.repeat(10000) + 'z\n'],
  ['one 50,000 character name', HEAD + 'tf 01 evt ' + 'a'.repeat(N) + '\n'],
  ['unclosed braces, one per line', HEAD + 'tf 01 evt A {\n'.repeat(N / 14)],
  ['an open brace and many closing braces that do not end a block', HEAD + 'data D {\n' + '}x\n'.repeat(N / 3)],
  ['lines of opening braces', HEAD + ('{'.repeat(99) + '\n').repeat(N / 100)],
  ['one line of quotes', HEAD + 'tf 01 evt A ' + '"'.repeat(N)],
  ['unclosed quotes, one per line', HEAD + "tf 01 evt A '\n".repeat(N / 14)],
  ['unclosed block comment', HEAD + '/*' + ' x\n'.repeat(N / 3)],
  ['many block comment openers', HEAD + '/* '.repeat(N / 3)],
  ['line comments', HEAD + '// c\n'.repeat(N / 5)],
  ['percent comments', HEAD + '%% c\n'.repeat(N / 5)],
  ['unclosed accDescr after every frame', HEAD + 'tf 01 evt A accDescr {\n'.repeat(N / 23)],
  ['accDescr after long blanks', HEAD + ('x' + ' '.repeat(90) + 'accDescr ').repeat(N / 100)],
  ['title after every identifier', HEAD + 'a title b\n'.repeat(N / 10)],
  ['unclosed directive openers', HEAD + 'x %%{\n'.repeat(N / 6)],
  ['unclosed front matter fences', HEAD + 'x\n---\n'.repeat(N / 6)],
  ['blank lines and blanks', HEAD + ' \n\t'.repeat(N / 3)],
  ['digits', HEAD + 'tf ' + '1'.repeat(N)],
  ['keywords run together', HEAD + 'tfevtuidatanote'.repeat(N / 15)],
];

describe('eventmodeling worst cases', () => {
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
    expect(() => render(HEAD + 'tf 01 evt A\n'.repeat(5000))).toThrow(/limit of 50000/);
  });
});
