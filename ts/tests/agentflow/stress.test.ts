import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/agentflow/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const H = 'agentflow-beta TB\n';
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

const CASES: [string, () => unknown][] = [
  ['long line that mentions direction', () => tokenize(H + 'A '.repeat(N / 2) + 'direction XX')],
  ['one line of direction words', () => tokenize(H + 'direction '.repeat(N / 10))],
  [
    'direction followed by blank lines, many times',
    () => tokenize(H + ('direction' + '\n'.repeat(9)).repeat(N / 18) + 'TB'),
  ],
  ['many lines that each mention direction', () => tokenize(H + 'A direction B\n'.repeat(N / 14))],
  ['direction after a long run of spaces', () => tokenize(H + 'direction' + ' '.repeat(N) + 'XX')],
  ['50,000 spaces', () => tokenize(H + ' '.repeat(N) + 'a')],
  ['spaces before a lone dash', () => tokenize(H + 'a' + ' '.repeat(N) + '-b')],
  ['spaces before a lone dash, many times', () => tokenize(H + ('a' + ' '.repeat(99) + '-b\n').repeat(N / 103))],
  [
    'spaces before a run of dots that closes nothing',
    () => tokenize(H + 'a' + ' '.repeat(N / 2) + '-' + '.'.repeat(N / 2)),
  ],
  ['spaces before dashes that reach no arrow', () => tokenize(H + 'a' + ' '.repeat(N / 2) + '-'.repeat(N / 2) + 'b')],
  ['spaces before a lone percent sign', () => tokenize(H + ' '.repeat(N) + '%x')],
  ['spaces before a directive opener, many times', () => tokenize(H + (' '.repeat(99) + '%%{\n').repeat(N / 103))],
  ['edge text of spaces before a lone dash', () => tokenize(H + 'a -- ' + ' '.repeat(N) + '-b')],
  ['edge text of spaced dashes', () => tokenize(H + 'a -- ' + ' -'.repeat(N / 2))],
  [
    'edge text of spaces before dashes that reach no arrow',
    () => tokenize(H + 'a -- x' + ' '.repeat(N / 2) + '-'.repeat(N / 2)),
  ],
  ['edge text of spaced words', () => tokenize(H + 'a -- ' + 'x '.repeat(N / 2) + '--> b')],
  ['run of link ids', () => tokenize(H + 'a@'.repeat(N / 2))],
  ['run of at signs before a brace', () => tokenize(H + '@'.repeat(N) + '{')],
  ['unclosed quotes', () => tokenize(H + 'a["' + 'x%'.repeat(N / 2))],
  ['unclosed markdown string', () => tokenize(H + 'a["`' + 'x '.repeat(N / 2))],
  ['label of comment lines', () => tokenize(H + 'a[' + 'x %% c\n'.repeat(N / 7) + ']')],
  ['ellipse label of comment lines', () => tokenize(H + 'a(-' + 'x %% c\n'.repeat(N / 7) + '-)')],
  ['trapezoid label of comment lines', () => tokenize(H + 'a[/' + 'x %% c\n'.repeat(N / 7) + '/]')],
  ['nested openers', () => tokenize(H + 'a' + '[('.repeat(N / 2))],
  ['unclosed metadata', () => tokenize(H + 'a@{' + ' k: "v",'.repeat(N / 8))],
  ['accTitle with no colon, many times', () => tokenize(H + ('accTitle' + ' '.repeat(40)).repeat(N / 48))],
  ['unclosed multi-line description', () => tokenize(H + 'accDescr {' + ' text\n'.repeat(N / 6))],
  ['10,000 comment lines', () => parse(H + '  %% note\n'.repeat(10000) + 'a --> b\n', big)],
  ['braces followed by spaces', () => parse(H + ('}' + ' '.repeat(49)).repeat(N / 50), big)],
  ['style statement of colons and hashes', () => parse(H + 'A\nstyle A ' + ':#'.repeat(N / 2) + '\n', big)],
  [
    'style list of 10,000 declarations',
    () => render(H + 'A\nstyle A ' + repeat(10000, (i) => `x${i}:1,`) + 'fill:red\n', big),
  ],
  [
    'class statement with 10,000 ids and classes',
    () => parse(H + 'class ' + repeat(10000, (i) => `e${i},`) + 'z ' + repeat(10000, (i) => `c${i},`) + 'z\n', big),
  ],
  ['20,000 class names on one node', () => render(H + 'A:::' + repeat(20000, (i) => `c${i},`) + 'z\n', big)],
  [
    'linkStyle with 10,000 indexes',
    () => render(H + 'a --> b\nlinkStyle ' + '0,'.repeat(10000) + '0 stroke:red\n', big),
  ],
  ['20,000 nested flows', () => parse(H + 'flow s\n'.repeat(20000) + 'A\n' + 'end\n'.repeat(20000), big)],
  [
    '3,000 nested flows, rendered',
    () => render(H + repeat(3000, (i) => `flow s${i}\n`) + 'A --> B\n' + 'end\n'.repeat(3000), big),
  ],
  [
    '3,000 nested collapsed flows',
    () => render(H + repeat(3000, (i) => `flow s${i}@{ view: collapsed }\n`) + 'A --> B\n' + 'end\n'.repeat(3000), big),
  ],
  [
    '1,500 nested flows crossed by 1,000 edges',
    () =>
      render(H + repeat(1500, (i) => `flow s${i}\n`) + 'A\n' + 'end\n'.repeat(1500) + 'A --> B\n'.repeat(1000), big),
  ],
  ['5,000 sibling flows', () => render(H + repeat(5000, (i) => `flow s${i}\nn${i}\nend\n`), big)],
  ['5,000 flows that list the same nodes', () => render(H + repeat(5000, () => 'flow s\nA\nB\nC\nend\n'), big)],
  ['one flow of 10,000 nodes', () => parse(H + 'flow s\n' + repeat(10000, (i) => `n${i}\n`) + 'end\n', big)],
  ['unclosed flows', () => parse(H + 'flow s\n'.repeat(N / 7), big)],
  [
    '3,000 flows that each name the next one',
    () => render(H + repeat(3000, (i) => `flow f${i}\n a${i} --> f${i + 1}\nend\n`), big),
  ],
  [
    '2,000 flows that each name the one before',
    () => render(H + repeat(2000, (i) => `flow f${i}\n a${i} --> f${i === 0 ? 1999 : i - 1}\nend\n`), big),
  ],
  [
    '3,000 global blocks after a flow of 3,000 nodes',
    () =>
      render(
        H + 'flow f\n' + repeat(3000, (i) => `n${i}\n`) + 'end\n' + repeat(3000, (i) => `global\nn${i}\nend\n`),
        big
      ),
  ],
  ['5,000 connectors', () => render(H + repeat(5000, (i) => `connector c${i}["C"]@{ protocol: "mcp" }\n`), big)],
  ['5,000 metadata blocks on one node', () => render(H + 'a\n' + repeat(5000, (i) => `a@{ k${i}: ${i} }\n`), big)],
  [
    '5,000 metadata blocks on one edge',
    () => render(H + 'a e1@--> b\n' + repeat(5000, (i) => `e1@{ k${i}: ${i} }\n`), big),
  ],
  [
    'one metadata block of 5,000 lines with trailing commas',
    () => render(H + 'a@{\n' + repeat(5000, (i) => `  k${i}: "v",\n`) + '}\n', big),
  ],
  ['metadata of nested brackets', () => render(H + 'a@{ k: ' + '['.repeat(N / 2) + ' }\n', big)],
  ['metadata of quotes and commas', () => render(H + 'a@{\n' + "  k: 'a,\n".repeat(N / 9) + '}\n', big)],
  ['4,000 nodes with a removed shape', () => render(H + repeat(4000, (i) => `n${i}((x))\n`), big)],
  ['3,000 nodes with an unsupported shape', () => render(H + repeat(3000, (i) => `n${i}@{ shape: tri }\n`), big)],
  ['one 50,000 character label', () => render(H + 'a["' + 'word '.repeat(N / 5) + '"]', big)],
  ['label of emphasis markers', () => render(H + 'a["`' + '*a _b '.repeat(N / 6) + '`"]', big)],
  ['label of entities and line breaks', () => render(H + 'a["' + '#35;&amp;<br>'.repeat(N / 13) + '"]', big)],
  ['one 50,000 character edge label', () => render(H + 'a -- "' + 'word '.repeat(N / 5) + '" --> b', big)],
  ['one 50,000 character id', () => render(H + 'a'.repeat(N) + ' --> b', big)],
  ['5,000 nodes', () => render(H + repeat(5000, (i) => `n${i}\n`), big)],
  ['chain of 5,000', () => render(H + repeat(5000, (i) => `n${i} --> n${i + 1}\n`), big)],
  ['one statement chaining 5,000', () => render(H + 'n' + repeat(5000, (i) => ` --> n${i}`) + '\n', big)],
  ['star of 5,000', () => render(H + repeat(5000, (i) => `c --> n${i}\n`), big)],
  [
    'fan-out of 150 by 150',
    () => render(H + repeat(150, (i) => `a${i} & `) + 'a --> b' + repeat(150, (i) => ` & b${i}`) + '\n', big),
  ],
  [
    '2,000 parallel edges and 2,000 self-loops',
    () => render(H + 'A --> B\n'.repeat(2000) + 'A --> A\n'.repeat(2000), big),
  ],
  [
    'complete graph of 50',
    () => {
      let s = H;
      for (let i = 0; i < 50; i++) for (let j = i + 1; j < 50; j++) s += `n${i} --> n${j}\n`;
      return render(s, big);
    },
  ],
  [
    '1,200 nodes that each point at their own flow',
    () => render(H + 'flow f\n' + repeat(1200, (i) => `n${i} --> f\n`) + 'end\n', big),
  ],
  [
    '5,000 edges between a flow and a node inside it',
    () => render(H + 'flow f\na --> b\n' + 'a --> f\nf --> b\n'.repeat(2500) + 'end\n', big),
  ],
  [
    '2,000 edges between flows',
    () => render(H + 'flow a\nA\nend\nflow b\nB\nend\n' + 'a --> b\nA -.- b\n'.repeat(1000), big),
  ],
];

describe('agentflow worst cases', () => {
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
    expect(() => render(H + 'A --> B\n'.repeat(7000))).toThrow(/limit of 50000/);
  });
});
