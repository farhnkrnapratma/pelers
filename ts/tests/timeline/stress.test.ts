import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/timeline/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const draw = (body: string) => (): unknown => render('timeline\n' + body, big);
const drawTD = (body: string) => (): unknown => render('timeline TD\n' + body, big);

const CASES: [string, () => unknown][] = [
  ['8,000 periods with one event each', draw('p : e\n'.repeat(N / 6))],
  ['8,000 periods with one event each, top down', drawTD('p : e\n'.repeat(N / 6))],
  ['one period with 12,000 events', draw('p' + ' : e'.repeat(N / 4))],
  ['one period with 12,000 events, top down', drawTD('p' + ' : e'.repeat(N / 4))],
  ['5,000 sections with one period each', draw('section s\np\n'.repeat(N / 10))],
  ['5,000 empty sections, top down', drawTD('section s\n'.repeat(N / 10))],
  ['one period of 50,000 characters without a space', draw('p'.repeat(N))],
  ['one event of 50,000 characters without a space', draw('p : ' + 'e'.repeat(N))],
  ['one event of 10,000 words', draw('p : ' + 'word '.repeat(N / 5))],
  ['one event of 10,000 words, top down', drawTD('p : ' + 'word '.repeat(N / 5))],
  ['one event of 12,000 line breaks', draw('p : ' + 'a<br>'.repeat(N / 5))],
  ['one section name of 50,000 characters', draw('section ' + 's'.repeat(N) + '\np')],
  ['a title of 50,000 characters', draw('title ' + 't '.repeat(N / 2) + '\np')],
  ['a title of 50,000 characters without a space', draw('title ' + 't'.repeat(N) + '\np')],
  ['event of tag-like text with no closing bracket', draw('p : ' + '<a'.repeat(N / 2))],
  ['event of tag openers followed by spaces', draw('p : ' + '<a '.repeat(N / 3))],
  ['event of entities', draw('p : ' + '#35;&amp;'.repeat(N / 9))],
  ['event of unclosed entity placeholders', draw('p : ' + 'ﬂ°'.repeat(N / 2))],
  ['event of emphasis markers', draw('p : ' + '*a _b '.repeat(N / 6))],
  ['event of surrogate pairs', draw('p : ' + '😀'.repeat(N / 2))],
  ['colons with no space', draw('p' + ':'.repeat(N))],
  ['colons each followed by a space', draw('p' + ': '.repeat(N / 2))],
  ['colons without spaces inside one event', draw('p : ' + 'a:'.repeat(N / 2))],
  ['keyword followed by spaces and no direction', () => tokenize('timeline' + ' '.repeat(N) + 'LX')],
  ['repeated keyword and spaces', () => tokenize('timeline   \t'.repeat(N / 12))],
  ['accTitle followed by spaces and no colon', draw('accTitle' + ' '.repeat(N))],
  ['accTitle followed by line breaks and no colon', draw('accTitle' + '\n'.repeat(N))],
  ['repeated accDescr with no colon', draw('accDescr \n'.repeat(N / 10))],
  ['unclosed accDescr block', draw('accDescr {' + ' a\n'.repeat(N / 3))],
  ['accDescr blocks, opened and closed', draw('accDescr { a }\n'.repeat(N / 15))],
  ['title keyword on every line', draw('title\n'.repeat(N / 6))],
  ['section keyword on every line', draw('section\n'.repeat(N / 8))],
  ['percent comments', draw('x%%\n%\n%{\n'.repeat(N / 9))],
  ['hash comments', draw('p # c\n'.repeat(N / 6))],
  ['blank lines and spaces', draw((' '.repeat(9) + '\n').repeat(N / 10) + 'p')],
  ['one long line of spaces', draw(' '.repeat(N) + 'p : e')],
  ['carriage returns', draw('p : e\r'.repeat(N / 6))],
];

describe('timeline worst cases', () => {
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
        expect(performance.now() - started).toBeLessThan(1500);
      },
      30_000
    );
  }
});
