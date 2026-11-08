import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/journey/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const draw = (body: string) => (): unknown => render('journey\n' + body, big);
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['6,000 tasks', draw('t: 5: a\n'.repeat(N / 8))],
  ['5,000 tasks, each with its own actor', draw(repeat(5000, (i) => `t: ${i % 6}: a${i}\n`))],
  ['one task with 16,000 actors', draw('t: 3: ' + repeat(16000, (i) => `a${i % 9000},`))],
  ['one task with the same actor 25,000 times', draw('t: 3: ' + 'a,'.repeat(N / 2))],
  ['one task with 25,000 empty actors', draw('t: 3: ' + ','.repeat(N / 2))],
  ['4,000 sections with one task each', draw(repeat(4000, (i) => `section s${i}\nt: 1\n`))],
  ['5,000 sections with no tasks', draw('section s\n'.repeat(N / 10))],
  ['one task name of 50,000 characters without a space', draw('t'.repeat(N) + ': 5')],
  ['one task name of 10,000 words', draw('word '.repeat(N / 5) + ': 5')],
  ['one task name of 12,000 line breaks', draw('a<br>'.repeat(N / 5) + ': 5')],
  ['one actor name of 50,000 characters', draw('t: 5: ' + 'a'.repeat(N))],
  ['one actor name of 10,000 words', draw('t: 5: ' + 'word '.repeat(N / 5))],
  ['one section name of 50,000 characters', draw('section ' + 's'.repeat(N) + '\nt: 5')],
  ['a title of 50,000 characters', draw('title ' + 't '.repeat(N / 2) + '\nt: 5')],
  ['a score of 50,000 digits', draw('t: ' + '9'.repeat(N))],
  ['a task line of 25,000 colons', draw('t' + ':'.repeat(N / 2) + 'x')],
  ['task name of tag-like text with no closing bracket', draw('<a'.repeat(N / 2) + ': 5')],
  ['task name of entities', draw('&amp;'.repeat(N / 5) + ': 5')],
  ['task name of unclosed entity placeholders', draw('ﬂ°'.repeat(N / 2) + ': 5')],
  ['task name of surrogate pairs', draw('😀'.repeat(N / 2) + ': 5')],
  ['semicolons', draw(';'.repeat(N))],
  ['colons on their own lines', draw(':\n'.repeat(N / 2))],
  ['repeated keyword', () => tokenize('journey '.repeat(N / 8))],
  ['accTitle followed by spaces and no colon', draw('accTitle' + ' '.repeat(N))],
  ['accTitle followed by line breaks and no colon', draw('accTitle' + '\n'.repeat(N))],
  ['repeated accDescr with no colon', draw('accDescr \n'.repeat(N / 10))],
  ['unclosed accDescr block', draw('accDescr {' + ' a\n'.repeat(N / 3))],
  ['accDescr blocks, opened and closed', draw('accDescr { a }\n'.repeat(N / 15))],
  ['title keyword on every line', draw('title\n'.repeat(N / 6))],
  ['section keyword on every line', draw('section\n'.repeat(N / 8))],
  ['percent comments', draw('x%%\n%\n%{: 1\n'.repeat(N / 12))],
  ['hash comments', draw('t: 5 # c\n'.repeat(N / 9))],
  ['blank lines and spaces', draw((' '.repeat(9) + '\n').repeat(N / 10) + 't: 5')],
  ['one long line of spaces', draw(' '.repeat(N) + 't: 5')],
];

describe('journey worst cases', () => {
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
        expect(performance.now() - started).toBeLessThan(1500);
      },
      30_000
    );
  }
});
