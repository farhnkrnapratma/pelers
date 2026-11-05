import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/kanban/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const linked = { ...big, config: { kanban: { ticketBaseUrl: 'https://example.com/browse/#TICKET#' } } };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['one line of shaped nodes', () => tokenize('kanban\n' + '(a) '.repeat(N / 4))],
  ['blank lines of spaces', () => tokenize('kanban\ntodo\n' + ' \n'.repeat(N / 2) + ' a')],
  [
    'a newline then a long run of spaces, repeated',
    () => tokenize('kanban\ntodo' + ('\n' + ' '.repeat(99) + 'a').repeat(N / 100)),
  ],
  ['at signs', () => tokenize('kanban\ntodo\n a' + '@'.repeat(N))],
  ['metadata openers', () => tokenize('kanban\ntodo\n a' + '@{'.repeat(N / 2))],
  ['empty metadata blocks', () => tokenize('kanban\ntodo\n a' + '@{}'.repeat(N / 3))],
  ['unclosed metadata', () => tokenize('kanban\ntodo\n a@{' + ' a: 1,\n'.repeat(N / 7))],
  ['unclosed metadata string', () => tokenize('kanban\ntodo\n a@{ label: "' + 'x\n   '.repeat(N / 5))],
  ['metadata of quotes', () => tokenize('kanban\ntodo\n a@{' + '"'.repeat(N))],
  ['metadata of carets and braces', () => tokenize('kanban\ntodo\n' + ' a@{ ^ }\n'.repeat(N / 9))],
  ['lone closing braces in a node', () => tokenize('kanban\ntodo[' + '}\n'.repeat(N / 2) + ']')],
  ['16,000 cards in one column, parsed', () => parse('kanban\ntodo\n' + ' a\n'.repeat(16000), big)],
  ['16,000 cards in one column, rendered', () => render('kanban\ntodo\n' + ' a\n'.repeat(16000), big)],
  ['16,000 columns', () => render('kanban\n' + 'a\n'.repeat(16000), big)],
  ['4,000 columns with two cards each', () => render('kanban\n' + 'c[Column]\n a[Card]\n b[Card]\n'.repeat(4000), big)],
  [
    '750 cards with every kind of metadata',
    () =>
      render(
        'kanban\ntodo\n' + " a[Card]@{ ticket: T-1, assigned: 'k', priority: 'High', icon: star }\n".repeat(750),
        linked
      ),
  ],
  [
    '1,000 cards with metadata over several lines',
    () =>
      render('kanban\ntodo\n' + ' a@{\n  assigned: k\n  ticket: T-1\n  label: "x\n   y"\n }\n'.repeat(1000), linked),
  ],
  [
    'a metadata block with 7,000 keys',
    () => render('kanban\ntodo\n a@{ ' + repeat(7000, (i) => `k${i}: 1, `) + 'label: x }', big),
  ],
  ['deeply bracketed metadata', () => parse('kanban\ntodo\n a@{ a: ' + '['.repeat(20000) + ' }\n', big)],
  ['metadata key followed by spaces', () => parse('kanban\ntodo\n a@{\n a' + ' '.repeat(N) + '\n}\n', big)],
  ['one 50,000 character card', () => render('kanban\ntodo\n a[' + 'word '.repeat(N / 5) + ']', big)],
  ['one 50,000 character word in a card', () => render('kanban\ntodo\n a[' + 'w'.repeat(N) + ']', big)],
  ['one 50,000 character word as a column title', () => render('kanban\ntodo[' + 'w'.repeat(N) + ']', big)],
  ['card of emphasis markers', () => render('kanban\ntodo\n a["`' + '*a _b '.repeat(N / 6) + '`"]', big)],
  ['card of line breaks and entities', () => render('kanban\ntodo\n a[' + '#35;&amp;<br>'.repeat(N / 13) + ']', big)],
  ['a ticket of 50,000 characters', () => render('kanban\ntodo\n a@{ ticket: "' + 'T'.repeat(N) + '" }', linked)],
  [
    'a ticket wrapped in layers of entity encoding',
    () => render('kanban\ntodo\n a@{ ticket: "&' + '#38;'.repeat(N / 4) + 'x" }', linked),
  ],
  [
    'an assignee of 50,000 characters',
    () => render('kanban\ntodo\n a@{ assigned: "' + 'ab '.repeat(N / 3) + '" }', big),
  ],
  ['a class list of 25,000 names', () => render('kanban\ntodo\n:::' + 'a '.repeat(N / 2), big)],
  ['a misplaced card after 16,000 cards', () => render('kanban\n todo\n' + '  a\n'.repeat(16000) + 'b\nc\n', big)],
];

describe('kanban worst cases', () => {
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

  it('keeps a long word inside its card', () => {
    const { width, svg } = render('kanban\ntodo\n a[' + 'w'.repeat(N) + ']', big);
    expect(width).toBe(216);
    expect(svg.match(/class="pele-label"/g)?.length).toBeGreaterThan(1000);
  });
});
