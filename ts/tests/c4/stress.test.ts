import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/c4/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const H = 'C4Context\n';

const CASES: [string, () => unknown][] = [
  [
    'one long line of statements that mentions direction',
    () => tokenize(H + 'Rel(a,b,"x")'.repeat(N / 12) + ' direction XX'),
  ],
  [
    'a line made of the word direction',
    () => tokenize(H + 'direction '.repeat(N / 10) + '\n' + 'direction\n'.repeat(N / 10)),
  ],
  [
    'direction followed by blank lines, many times',
    () => tokenize(H + ('x direction' + '\n'.repeat(50)).repeat(N / 60)),
  ],
  ['spaces after an opening parenthesis', () => tokenize(H + 'Person(' + ' '.repeat(N))],
  [
    'spaces before each quoted argument',
    () => tokenize(H + 'Person(' + (' '.repeat(100) + '"a",').repeat(N / 104) + ')'),
  ],
  ['an unclosed string', () => tokenize(H + 'Person(a, "' + 'x'.repeat(N))],
  ['a named argument with no equals sign', () => tokenize(H + 'Person(a, $' + 'k'.repeat(N))],
  ['spaces after an equals sign', () => tokenize(H + 'Person(a, $k=' + ' '.repeat(N) + 'x')],
  ['an unclosed multi-line description', () => tokenize(H + 'accDescr {' + ' x\n'.repeat(N / 3))],
  ['accTitle followed by spaces and no colon', () => tokenize(H + ('accTitle' + ' '.repeat(100)).repeat(N / 108))],
  ['a comment followed by line breaks', () => tokenize(H + '%% c' + '\r\n'.repeat(N / 2) + 'Person(a, "b")')],
  ['blank lines and trailing spaces', () => tokenize(H + ' \n\t'.repeat(N / 3) + 'Person(a, "b")')],
  ['a word of 50,000 letters', () => tokenize(H + 'Person'.repeat(N / 6))],
  ['a title of 50,000 characters', () => render(H + 'title ' + 'word '.repeat(N / 5) + '\nPerson(a, "b")', big)],
  ['50,000 commas', () => parse(H + 'Person(' + ','.repeat(N) + ')', big)],
  ['25,000 unquoted arguments', () => render(H + 'Person(' + 'a,'.repeat(N / 2) + '"z")', big)],
  [
    '5,000 named arguments on one shape',
    () => render(H + 'System(a, "A"' + repeat(5000, (i) => `, $k${i}="v"`) + ')', big),
  ],
  [
    '2,000 nested boundaries',
    () =>
      render(H + repeat(2000, (i) => `Boundary(b${i}, "B${i}") {\n`) + 'System(s, "S")\n' + '}\n'.repeat(2000), big),
  ],
  [
    '3,000 boundaries nested under one alias',
    () => render(H + 'Boundary(b, "B") {\n'.repeat(3000) + 'System(s, "S")\n' + '}\n'.repeat(3000), big),
  ],
  [
    '1,200 sibling boundaries',
    () => render(H + repeat(1200, (i) => `Boundary(b${i}, "B") {\nSystem(s${i}, "S")\n}\n`), big),
  ],
  ['2,500 shapes', () => render(H + repeat(2500, (i) => `System(s${i}, "S${i}")\n`), big)],
  [
    '2,500 shapes in one row',
    () => render(H + 'UpdateLayoutConfig("99999999", "1")\n' + repeat(2500, (i) => `System(s${i}, "S${i}")\n`), big),
  ],
  ['3,000 definitions of one shape', () => render(H + 'System(a, "A", "d")\n'.repeat(3000), big)],
  [
    '2,450 relations among 50 shapes',
    () => {
      let src = H + repeat(50, (i) => `System(s${i},"S")\n`);
      for (let i = 0; i < 50; i++) for (let j = 0; j < 50; j++) if (i !== j) src += `Rel(s${i},s${j},"x")\n`;
      return render(src, big);
    },
  ],
  [
    'as many relations as get careful placement',
    () => {
      let src = H + repeat(700, (i) => `System(s${i},"S")\n`);
      for (let k = 0; k < 700; k++) src += `Rel(s${k},s${(k * 37 + 11) % 700},"a label to place ${k}", "tech")\n`;
      return render(src, big);
    },
  ],
  [
    '600 relations among 600 boundaries',
    () => {
      let src = H + repeat(600, (i) => `Boundary(b${i}, "A heading ${i}") {\nSystem(s${i},"S")\n}\n`);
      for (let k = 0; k < 600; k++) src += `Rel(b${k},s${(k * 11 + 5) % 600},"x")\n`;
      return render(src, big);
    },
  ],
  [
    '480 detours along one row of 1,000',
    () => {
      let src = H + 'UpdateLayoutConfig("1000", "1")\n' + repeat(1000, (i) => `System(s${i},"S")\n`);
      for (let k = 0; k < 480; k++) src += `Rel(s0,s${k + 2},"x")\n`;
      return render(src, big);
    },
  ],
  [
    '2,000 self relations and reversed pairs',
    () => render(H + 'System(a,"A")\nSystem(b,"B")\n' + 'Rel(a,a,"x")\nRel(a,b,"y")\nRel(b,a,"z")\n'.repeat(700), big),
  ],
  [
    'a relation between the top and the bottom of 1,000 nested boundaries',
    () =>
      render(
        H +
          'System(top, "T")\n' +
          repeat(1000, (i) => `Node(n${i}, "N") {\n`) +
          'System(deep, "D")\n' +
          '}\n'.repeat(1000) +
          'Rel(top, deep, "x")\nRel(n0, deep, "y")\nRel(deep, n999, "z")',
        big
      ),
  ],
  ['one 50,000 character label', () => render(H + 'System(a, "' + 'word '.repeat(N / 5) + '")', big)],
  ['one unbreakable 50,000 character label', () => render(H + 'System(a, "' + 'w'.repeat(N) + '")', big)],
  [
    'a description of entities and line breaks',
    () => render(H + 'System(a, "A", "' + '#35;&amp;<br>'.repeat(N / 13) + '")', big),
  ],
  [
    'a relation label of 50,000 characters',
    () =>
      render(
        H + 'System(a,"A")\nSystem(b,"B")\nRel(a, b, "' + 'word '.repeat(N / 5) + '", "' + 'tech '.repeat(100) + '")',
        big
      ),
  ],
  [
    'a boundary heading of 50,000 characters',
    () => render(H + 'Boundary(b, "' + 'word '.repeat(N / 5) + '") {\nSystem(a,"A")\n}', big),
  ],
  ['25,000 tags', () => render(H + 'System(a, "A", $tags="' + 'a,'.repeat(N / 2) + '")', big)],
  [
    'a link wrapped in layers of entity encoding',
    () => render(H + 'System(a, "A", $link="&' + '#38;'.repeat(N / 4) + 'x")', big),
  ],
  [
    'colours of 50,000 characters',
    () =>
      render(
        H + 'System(a, "A")\nUpdateElementStyle(a, "' + 'url( '.repeat(N / 5) + '", "' + 'a'.repeat(N) + '")',
        big
      ),
  ],
  [
    '3,000 style updates for things that do not exist',
    () =>
      render(
        H + 'System(a,"A")\n' + repeat(1500, (i) => `UpdateElementStyle(x${i},"red")\nUpdateRelStyle(a,x${i},"red")\n`),
        big
      ),
  ],
  [
    '3,000 other statements',
    () => parse(H + 'title t\naccDescr: d\naccTitle: t\n'.repeat(1000) + 'System(a,"A")', big),
  ],
  [
    'a multi-line description full of line breaks',
    () => parse(H + 'accDescr {' + '\n   '.repeat(N / 4) + '}\nSystem(a,"A")', big),
  ],
];

describe('C4 worst cases', () => {
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

  // One optimized version of the lexer once repeated a whole-text search for every statement.
  // It was only ever one call in a run, so every call is timed.
  it('stays fast on every call while the lexer is being optimized', () => {
    let worst = 0;
    for (let i = 0; i < 30; i++) {
      const src = H + 'System(a, "A", "d")\n'.repeat(3000) + '\n';
      const started = performance.now();
      tokenize(src);
      worst = Math.max(worst, performance.now() - started);
    }
    expect(worst).toBeLessThan(1000);
  });

  it('renders what it can of the large cases', () => {
    expect(
      render(H + repeat(2500, (i) => `System(s${i}, "S${i}")\n`), big).svg.split('class="pelers-node ').length - 1
    ).toBe(2500);
    expect(
      render(H + repeat(2000, (i) => `Boundary(b${i}, "B${i}") {\n`) + 'System(s, "S")\n' + '}\n'.repeat(2000), big).svg
    ).toContain('data-id="b1999"');
    expect(() => render(H + 'System(a, "A")\n'.repeat(5000))).toThrow(/limit of 50000/);
  });
});
