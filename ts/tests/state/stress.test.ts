import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/state/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

// Inputs of about 50,000 characters built to trigger the state diagram's worst cases: lookahead
// that could rescan, deep nesting, and oversized layouts. Each must finish quickly and either
// succeed or fail with a PelersError. The bounds are loose; a regression here is seconds or a hang.

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const head = 'stateDiagram-v2\n';

const CASES: [string, () => unknown][] = [
  [
    'the same class statement and transitions 1,500 times',
    () =>
      render(
        head +
          'classDef hot fill:#f00,color:white,font-weight:bold,stroke-width:2px,stroke:yellow\n' +
          repeat(1500, () => 'A --> B\nB --> A\nclass A hot\n'),
        big
      ),
  ],
  ['one line that mentions direction 3,000 times', () => tokenize(head + 'a direction XX '.repeat(N / 15))],
  ['one line of valid directions', () => tokenize(head + 'direction TB direction lr '.repeat(N / 26))],
  [
    'direction followed by blank lines, 2,000 times',
    () => tokenize(head + ('direction' + '\n'.repeat(24)).repeat(N / 33)),
  ],
  ['directions at the end of a long line of tokens', () => tokenize(head + 'a b '.repeat(N / 4) + 'direction TB')],
  ['unterminated string after many tokens', () => tokenize(head + 'a '.repeat(N / 2) + '"' + 'b '.repeat(N / 2))],
  ['stereotype look-alikes on one state line', () => tokenize(head + 'state ' + '<<fork> [[join] '.repeat(N / 16))],
  ['stereotypes on one state line', () => tokenize(head + 'state ' + 'x <<choice>> '.repeat(N / 13))],
  ['state keyword 8,000 times', () => tokenize(head + 'state '.repeat(N / 6))],
  ['state name followed by spaces', () => tokenize(head + 'state a' + ' '.repeat(N) + 'b')],
  ['state names with no brace on the line', () => tokenize(head + 'state ' + 'w '.repeat(N / 2))],
  [
    'state names with a brace far along the line',
    () => tokenize(head + repeat(2000, () => 'state a\n') + 'state ' + 'w'.repeat(N / 2) + ' x {'),
  ],
  ['ids made of comment markers', () => tokenize(head + 'a%%{'.repeat(N / 4))],
  ['alias made of comment markers', () => tokenize(head + 'state "x" as ' + '%%{'.repeat(N / 3))],
  ['note that never ends, over blank lines', () => tokenize(head + 'note right of a\n' + '\n '.repeat(N / 2))],
  ['note with near-miss endings', () => tokenize(head + 'note right of a\n' + ' end not\n'.repeat(N / 9))],
  ['note id after a run of spaces', () => tokenize(head + 'note left of' + ' '.repeat(N) + ': x')],
  ['floating note of spaces', () => tokenize(head + 'note "' + ' '.repeat(N) + 'x" as n')],
  ['class statement with dangling commas', () => tokenize(head + 'class a' + ', '.repeat(N / 2))],
  ['description of colons', () => tokenize(head + 'a ' + ':x'.repeat(N / 2))],
  ['runs of dashes and colons', () => tokenize(head + '-'.repeat(N / 2) + ':'.repeat(N / 2))],
  ['unclosed accDescr block', () => parse(head + 'accDescr {' + 'x '.repeat(N / 2), big)],
  ['accTitle followed by spaces', () => tokenize(head + 'accTitle' + ' '.repeat(N) + 'x')],
  ['scale followed by spaces', () => tokenize(head + 'scale 1' + ' '.repeat(N) + 'x')],
  ['6,000 nested composites', () => parse(head + repeat(6000, () => 'state s{\n') + '}\n'.repeat(6000), big)],
  [
    '3,000 nested composites, rendered',
    () => render(head + repeat(3000, (i) => `state s${i}{\n`) + 'a-->b\n' + '}\n'.repeat(3000), big),
  ],
  [
    '2,500 nested composites, each split in two',
    () => render(head + repeat(2500, (i) => `state s${i}{\nx${i}\n--\n`) + 'a\n' + '}\n'.repeat(2500), big),
  ],
  ['5,000 sibling composites', () => render(head + repeat(5000, (i) => `state s${i}{\n}\n`), big)],
  [
    'one composite with 8,000 regions',
    () => render(head + 'state s{\n' + repeat(8000, (i) => `a${i}\n--\n`) + 'z\n}\n', big),
  ],
  ['chain of 5,000', () => render(head + repeat(5000, (i) => `n${i}-->n${i + 1}\n`), big)],
  ['star of 5,000', () => render(head + repeat(5000, (i) => `c-->n${i}\n`), big)],
  ['fork with 5,000 branches', () => render(head + 'state f <<fork>>\n' + repeat(5000, (i) => `f-->n${i}\n`), big)],
  [
    'complete graph of 60',
    () => {
      let s = head;
      for (let i = 0; i < 60; i++) for (let j = i + 1; j < 60; j++) s += `n${i}-->n${j}\n`;
      return render(s, big);
    },
  ],
  [
    '3,000 parallel transitions and 3,000 self-transitions',
    () => render(head + 'a-->b\n'.repeat(3000) + 'a-->a\n'.repeat(3000), big),
  ],
  ['4,000 start and end states', () => render(head + '[*]-->[*]\n'.repeat(N / 12), big)],
  [
    '2,500 notes on one state',
    () => render(head + repeat(2500, (i) => `note ${i % 2 ? 'left' : 'right'} of a:n\n`), big),
  ],
  ['2,000 states with a note each', () => render(head + repeat(2000, (i) => `note right of s${i}:n\n`), big)],
  ['8,000 description lines on one state', () => render(head + 'a:x\n'.repeat(N / 4), big)],
  ['one 50,000 character description', () => render(head + 'a : ' + 'word '.repeat(N / 5), big)],
  [
    'one 50,000 character note',
    () => render(head + 'note right of a\n' + 'word *b _c\n'.repeat(N / 11) + 'end note\n', big),
  ],
  ['one 50,000 character transition label', () => render(head + 'a --> b : ' + '#35;<br>x '.repeat(N / 10), big)],
  [
    'one 50,000 character composite title',
    () => render(head + 'state "' + 'w '.repeat(N / 2) + '" as s {\na\n}\n', big),
  ],
  ['5,000 class statements for one state', () => render(head + 'a\n' + repeat(5000, (i) => `class a c${i}\n`), big)],
  ['class statement naming 8,000 states', () => render(head + 'class ' + repeat(8000, (i) => `s${i},`) + 'z c\n', big)],
  [
    'classDef with 8,000 declarations',
    () => render(head + 'a\nclassDef c ' + 'fill:red,'.repeat(N / 9) + 'x\nclass a c\n', big),
  ],
  ['3,000 style statements', () => render(head + 'a-->b\n' + 'style a,b fill:red\n'.repeat(3000), big)],
  ['3,000 click statements', () => render(head + 'a-->b\n' + 'click a "u" "t"\n'.repeat(3000), big)],
  ['link of nested entity codes', () => render(head + 'a\nclick a "&' + '#38;'.repeat(N / 4) + 'x" "t"\n', big)],
  [
    'transitions from 1,500 levels out to the innermost state',
    () =>
      render(
        head + repeat(1500, (i) => `state s${i}{\n`) + 'z\n' + '}\n'.repeat(1500) + repeat(1500, (i) => `s${i}-->z\n`),
        big
      ),
  ],
  [
    '3,000 transitions into a state 1,500 levels deep',
    () =>
      render(
        head + repeat(1500, (i) => `state s${i}{\n`) + 'z\n' + '}\n'.repeat(1500) + repeat(3000, (i) => `o${i}-->z\n`),
        big
      ),
  ],
  [
    'composites that contain each other',
    () => render(head + repeat(2000, (i) => `state a${i}{\nstate a${(i + 1) % 2000}{\n}\n}\n`), big),
  ],
  [
    '3,000 transitions into a ring of 1,200 composites',
    () =>
      render(
        head +
          repeat(1200, (i) => `state a${i}{\nstate a${(i + 1) % 1200}{\nz${i}\n}\n}\n`) +
          repeat(3000, (i) => `o${i}-->z${i % 1200}\n`),
        big
      ),
  ],
  ['a composite named root', () => render(head + 'state root {\n a --> b\n}\nroot --> root\n', big)],
];

describe('state diagram worst cases', () => {
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
      60_000
    );
  }
});
