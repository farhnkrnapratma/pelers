import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/requirement/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const HEAD = 'requirementDiagram\n';

const CASES: [string, () => unknown][] = [
  [
    'one line that mentions direction without ending it',
    () => tokenize(HEAD + 'a - contains -> b '.repeat(N / 18) + 'direction XX'),
  ],
  ['one line of many tokens ending in a direction', () => tokenize(HEAD + ': '.repeat(N / 2) + 'direction LR')],
  ['direction followed by blank lines', () => tokenize(HEAD + ('direction' + '\n'.repeat(40)).repeat(N / 50) + 'TB')],
  ['many direction words on one line', () => tokenize(HEAD + 'direction '.repeat(N / 10) + 'TB')],
  ['many direction statements on one line', () => tokenize(HEAD + 'direction TB '.repeat(N / 13))],
  [
    'accTitle with no colon before a wall of spaces',
    () => tokenize(HEAD + ('accTitle' + ' '.repeat(100) + '\n').repeat(N / 110)),
  ],
  ['accDescr followed by whitespace only', () => tokenize(HEAD + 'accDescr' + ' \n'.repeat(N / 2))],
  ['unclosed accDescr block', () => tokenize(HEAD + 'accDescr {' + ' a\n'.repeat(N / 3))],
  ['title word on every line', () => tokenize(HEAD + 'title\n'.repeat(N / 6))],
  ['title followed by hashes', () => tokenize(HEAD + 'title #'.repeat(N / 7))],
  ['one unquoted string of spaces', () => tokenize(HEAD + 'a' + ' '.repeat(N) + 'b')],
  ['one unclosed quoted string', () => tokenize(HEAD + 'element "' + 'x'.repeat(N))],
  ['quotes and nothing else', () => tokenize(HEAD + '"'.repeat(N))],
  ['runs of whitespace between tokens', () => tokenize(HEAD + ('a' + ' \n\t'.repeat(30) + ':').repeat(N / 100))],
  ['spaces in a style statement', () => tokenize(HEAD + 'style a' + ' '.repeat(N) + 'fill:red\n')],
  ['a style statement of 25,000 parts', () => parse(HEAD + 'style a ' + 'f:#-;'.repeat(N / 5) + '\n', big)],
  [
    'a class list of 25,000 names',
    () => parse(HEAD + 'requirement a {\n}\nclass a ' + 'c,'.repeat(N / 2) + 'c\n', big),
  ],
  [
    'a class defined 1,600 times and applied 1,600 times',
    () => render(HEAD + 'requirement a {\n}\n' + 'classDef c fill:red\nclass a c\n'.repeat(1600), big),
  ],
  [
    'a class of 6,000 declarations applied 2,500 times',
    () =>
      render(
        HEAD + 'requirement a {\n}\nclassDef c ' + 'a:b,'.repeat(6000) + 'a:b\n' + 'class a c\n'.repeat(2500),
        big
      ),
  ],
  [
    'a class of 3,000 declarations applied to 1,000 nodes',
    () =>
      render(
        HEAD +
          repeat(1000, (i) => `element e${i} {\n}\n`) +
          'classDef c ' +
          'a:b,'.repeat(3000) +
          'a:b\n' +
          ('class ' + Array.from({ length: 1000 }, (_, i) => `e${i}`).join(',') + ' c\n').repeat(3),
        big
      ),
  ],
  [
    '2,000 nodes and 600 class definitions',
    () => render(HEAD + repeat(2000, (i) => `element e${i}:::c {\n}\n`) + 'classDef c a:b\n'.repeat(600), big),
  ],
  ['a body of 10,000 repeated fields', () => parse(HEAD + 'requirement a {\n' + 'id: 1\n'.repeat(10000) + '}\n', big)],
  ['a body that never closes', () => parse(HEAD + 'requirement a {\n' + 'text: x\n'.repeat(6000), big)],
  ['25,000 blank lines', () => parse(HEAD + '\n \n'.repeat(N / 3) + 'requirement a {\n}\n', big)],
  ['comment characters on every line', () => parse(HEAD + '# c\n% d\n'.repeat(N / 8), big)],
  [
    '2,000 requirements with full bodies',
    () => render(HEAD + repeat(2000, (i) => `requirement r${i} {\nid: ${i}\nrisk: low\n}\n`), big),
  ],
  [
    'chain of 1,500',
    () =>
      render(
        HEAD + repeat(1500, (i) => `element e${i} {\n}\n`) + repeat(1499, (i) => `e${i} - traces -> e${i + 1}\n`),
        big
      ),
  ],
  [
    'star of 1,500',
    () =>
      render(
        HEAD + repeat(1500, (i) => `element e${i} {\n}\n`) + repeat(1499, (i) => `e0 - contains -> e${i + 1}\n`),
        big
      ),
  ],
  [
    '2,500 parallel relationships and 500 self relationships',
    () =>
      render(
        HEAD +
          'element a {\n}\nelement b {\n}\n' +
          'a - copies -> b\n'.repeat(2500) +
          'a - contains -> a\n'.repeat(500),
        big
      ),
  ],
  [
    '2,500 relationships to names that do not exist',
    () => render(HEAD + 'element a {\n}\n' + repeat(2500, (i) => `a - refines -> x${i}\n`), big),
  ],
  [
    'complete graph of 40',
    () => {
      let s = HEAD + repeat(40, (i) => `element e${i} {\n}\n`);
      for (let i = 0; i < 40; i++) for (let j = i + 1; j < 40; j++) s += `e${i} - derives -> e${j}\n`;
      return render(s, big);
    },
  ],
  [
    'one 50,000 character text',
    () => render(HEAD + 'requirement a {\ntext: "' + 'word '.repeat(N / 5) + '"\n}\n', big),
  ],
  [
    'text of emphasis markers',
    () => render(HEAD + 'requirement a {\ntext: "' + '*a _b **c '.repeat(N / 10) + '"\n}\n', big),
  ],
  ['a name of 50,000 characters without spaces', () => render(HEAD + 'requirement ' + 'x'.repeat(N) + ' {\n}\n', big)],
  [
    'text of entities and line breaks',
    () => render(HEAD + 'requirement a {\ntext: "' + '#35;&amp;<br>'.repeat(N / 13) + '"\n}\n', big),
  ],
];

describe('requirement worst cases', () => {
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

  it('refuses source over the default limit', () => {
    expect(() => render(HEAD + 'element a {\n}\n'.repeat(5000))).toThrow(/limit of 50000/);
  });
});
