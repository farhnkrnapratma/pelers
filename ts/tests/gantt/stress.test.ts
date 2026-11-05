import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/gantt/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity, now: new Date(2024, 0, 15) };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const head = 'gantt\ndateFormat YYYY-MM-DD\n';

const CASES: [string, () => unknown][] = [
  ['weekday followed by a run of spaces', () => tokenize('gantt\nweekday' + ' '.repeat(N) + 'x')],
  ['weekday keywords over newlines', () => tokenize('gantt\n' + 'weekday\n\n\n'.repeat(N / 10))],
  [
    'accTitle with no colon',
    () => tokenize('gantt\n' + 'accTitle   '.repeat(N / 11) + '\n' + 'accDescr \n '.repeat(2000)),
  ],
  ['unclosed accDescr block', () => tokenize('gantt\naccDescr {' + ' x\n'.repeat(N / 3))],
  ['percent signs and comment starts', () => tokenize('gantt\n' + 'a%'.repeat(N / 4) + '\n' + '%\n'.repeat(N / 4))],
  ['runs of colons', () => tokenize('gantt\n' + ':'.repeat(N) + '\n' + ':;'.repeat(N / 2))],
  [
    'click and call with open parentheses',
    () => tokenize('gantt\n' + 'click a call f(' + ' '.repeat(N) + '\n' + 'click a call (('.repeat(1000)),
  ],
  ['href with no closing quote', () => tokenize('gantt\nclick a href "' + 'x '.repeat(N / 2))],
  [
    'keywords with nothing after them',
    () => tokenize('gantt\n' + 'dateFormat \ntitle \nsection \nexcludes \n'.repeat(N / 40)),
  ],
  ['8,000 tasks', () => render(head + repeat(8000, (i) => `T${i} :2024-01-01, ${1 + (i % 30)}d\n`), big)],
  [
    '5,000 tasks in a chain',
    () => render(head + 'A :t0, 2024-01-01, 1d\n' + repeat(5000, (i) => `T :t${i + 1}, after t${i}, 1d\n`), big),
  ],
  [
    '3,000 tasks that each wait for the next',
    () => render(head + repeat(3000, (i) => `T :t${i}, after t${i + 1}, 1d\n`) + 'Z :t3000, 2024-01-01, 1d\n', big),
  ],
  [
    'one task after 12,000 ids',
    () => render(head + 'A :a, 2024-01-01, 1d\nB :b, after' + ' a zz'.repeat(6000) + ', 1d\n', big),
  ],
  [
    '500 tasks after 30 unknown ids each',
    () => render(head + repeat(500, () => 'T :after' + ' q'.repeat(30) + ', 1d\n'), big),
  ],
  ['5,000 sections', () => render(head + repeat(5000, (i) => `section S${i}\nT :2024-01-01, 2d\n`), big)],
  [
    'compact mode with 6,000 overlapping tasks',
    () => render('---\ndisplayMode: compact\n---\n' + head + repeat(6000, (i) => `T :2024-01-01, ${i + 1}d\n`), big),
  ],
  ['one 50,000 character task name', () => render(head + 'word '.repeat(N / 5) + ':2024-01-01, 2d\n', big)],
  [
    'one 50,000 character section name',
    () => render(head + 'section ' + 'word '.repeat(N / 5) + '\nT :2024-01-01, 2d\n', big),
  ],
  [
    'one 50,000 character title of entities',
    () => render(head + 'title ' + '#35;&amp;<br>'.repeat(N / 13) + '\nT :2024-01-01, 2d\n', big),
  ],
  [
    'excluded weekends over 400 tasks of 30 years',
    () => render(head + 'excludes weekends\n' + repeat(400, () => 'T :2024-01-01, 10950d\n'), big),
  ],
  [
    'every weekday excluded',
    () =>
      render(
        head +
          'excludes weekends monday tuesday wednesday thursday friday\n' +
          repeat(2000, () => 'T :2024-01-01, 7d\n'),
        big
      ),
  ],
  [
    '12,000 excluded dates over long tasks',
    () =>
      render(
        head +
          'excludes ' +
          repeat(4000, (i) => `2024-01-${10 + (i % 20)} `) +
          '\n' +
          repeat(300, () => 'T :2024-01-01, 3000d\n'),
        big
      ),
  ],
  [
    'a custom format checked for every excluded day',
    () =>
      render(
        'gantt\ndateFormat DD.MM.YYYY [' +
          'x'.repeat(2000) +
          ']\nexcludes 01.01.2024\n' +
          repeat(500, () => 'T :2024-01-01, 20000d\n'),
        big
      ),
  ],
  [
    'a date format of 25,000 tokens',
    () =>
      render(
        'gantt\ndateFormat ' + 'D '.repeat(12500) + '\n' + 'T :' + '1 '.repeat(8000) + ', ' + '2 '.repeat(4000) + '\n',
        big
      ),
  ],
  [
    'a date format of words against digits at the far end',
    () =>
      render(
        'gantt\ndateFormat ' + 'D-'.repeat(6000) + '\n' + repeat(3, () => 'T :' + 'x'.repeat(8000) + '1, 3d\n'),
        big
      ),
  ],
  [
    'long junk dates in 2,000 tasks',
    () => render(head + repeat(2000, (i) => `T :t${i}, after t${i + 1}, ${'9'.repeat(18)}\n`), big),
  ],
  ['a task that lasts as long as dates go', () => render(head + 'excludes weekends\nT :2024-01-01, 99999999d\n', big)],
  ['a chart from year one to the last date', () => render(head + 'A :0001-01-01, 1d\nB :2024-01-01, 99990000d\n', big)],
  [
    'a tick for every millisecond of a century',
    () => render(head + 'tickInterval 1millisecond\nA :1950-01-01, 36500d\n', big),
  ],
  [
    'a tick interval that matches every minute',
    () => render(head + 'tickInterval 99999999second\nA :1950-01-01, 36500d\n', big),
  ],
  [
    'a tick interval of many weeks over a long chart',
    () => render(head + 'tickInterval 100000week\nA :0100-01-01, 99000000d\n', big),
  ],
  [
    'an axis format of 12,000 directives',
    () => render(head + 'axisFormat ' + '%c%j%U%V'.repeat(3000) + '\nA :2024-01-01, 30d\n', big),
  ],
  [
    'a today marker of 25,000 declarations',
    () => render(head + 'todayMarker ' + 'stroke:red,'.repeat(4500) + '\nA :2024-01-01, 30d\n', big),
  ],
  ['timestamps too large for a date', () => parse('gantt\ndateFormat x\nA :' + '9'.repeat(N) + ', 1d\n', big)],
  [
    '2,000 links to 2,000 tasks',
    () =>
      render(
        head +
          repeat(2000, (i) => `T :t${i}, 2024-01-01, 2d\n`) +
          repeat(2000, (i) => `click t${i} href "https://example.com/${i}"\n`),
        big
      ),
  ],
  [
    'a click on 12,000 ids',
    () => render(head + 'T :t, 2024-01-01, 2d\nclick ' + 't,'.repeat(12000) + 't call f()\n', big),
  ],
];

describe('gantt worst cases', () => {
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

  it('refuses a schedule that would take too long, as a limit', () => {
    const src = head + 'excludes weekends\n' + repeat(400, () => 'T :2024-01-01, 10950d\n');
    let code: string | undefined;
    try {
      render(src, big);
    } catch (error) {
      code = (error as PeleError).code;
    }
    expect(code).toBe('limit');
  }, 30_000);
});
