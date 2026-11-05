import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/usecase/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const U = 'usecase-beta\n';
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');

function lex(src: string): unknown {
  return tokenize(src);
}

const CASES: [string, () => unknown][] = [
  ['one run of 50,000 dashes', () => lex(U + 'A ' + '-'.repeat(N) + ' B')],
  ['runs of dashes that never reach an arrowhead', () => lex(U + '-- '.repeat(N / 3))],
  ['unclosed stereotypes, one a line', () => lex(U + 'A <<x\n'.repeat(N / 6))],
  ['one stereotype opened 25,000 times on a line', () => lex(U + 'A ' + '<<'.repeat(N / 2))],
  ['stereotypes closed only at the very end', () => lex(U + 'A <<x\n'.repeat(N / 6) + '>>')],
  ['accDescr blocks that never close', () => lex(U + 'accDescr {\n'.repeat(N / 11))],
  ['accDescr blocks closed only at the very end', () => lex(U + 'accDescr {\n'.repeat(N / 11) + '}')],
  ['accTitle with no colon, many times', () => lex(U + ('accTitle' + ' '.repeat(40) + '\n').repeat(N / 49))],
  ['unclosed markdown strings', () => lex(U + '"`x'.repeat(N / 3))],
  [
    'unclosed plain strings',
    () => {
      try {
        return lex(U + 'A "x'.repeat(N / 4));
      } catch (error) {
        if (error instanceof PeleError) return undefined;
        throw error;
      }
    },
  ],
  ['quotes that cannot start a token, one after another', () => parse(U + 'A ' + '"\''.repeat(N / 2), big)],
  ['unclosed JSON of braces', () => lex(U + 'json P@' + '{'.repeat(N))],
  ['JSON declarations that never close', () => lex(U + 'json P@{ "a": \n'.repeat(N / 15))],
  ['comment markers after text', () => lex(U + ('A' + ' '.repeat(40) + '%%\n').repeat(N / 44))],
  ['long indented lines of percent signs', () => lex(U + (' '.repeat(100) + '%'.repeat(100) + '\n').repeat(N / 201))],
  ['keyword prefixes', () => lex(U + 'endx forx notex truex classDefx jsonx '.repeat(N / 38))],
  ['50,000 characters of symbols', () => lex(U + 'é日'.repeat(N / 2))],
  ['one label of 20,000 words', () => parse(U + 'A(' + 'word '.repeat(20000) + ')', big)],
  ['a label that is never closed', () => parse(U + 'A -- ' + 'word '.repeat(N / 5), big)],
  ['10,000 statements that each look for a label', () => parse(U + repeat(10000, (i) => `A${i} -- x y z\n`), big)],
  ['metadata of 10,000 properties', () => parse(U + 'actor A\nA@{\n' + 'business: true,\n'.repeat(10000) + '}', big)],
  ['metadata of line breaks', () => parse(U + 'A@{' + '\n'.repeat(N) + '}\nA', big)],
  [
    'class statement with 2,500 ids and 4,000 classes',
    () =>
      parse(
        U +
          repeat(2500, (i) => `e${i}\n`) +
          'class ' +
          repeat(2500, (i) => `e${i},`) +
          'e0 ' +
          repeat(4000, (i) => `c${i},`) +
          'z\n',
        big
      ),
  ],
  ['8,000 classes on one element', () => render(U + 'A:::' + repeat(8000, (i) => `c${i},`) + 'z\n', big)],
  [
    'classDef of 8,000 names and a style list of 8,000 declarations',
    () =>
      render(
        U +
          'A\nclassDef ' +
          repeat(8000, (i) => `c${i},`) +
          'z ' +
          repeat(8000, (i) => `x${i}:1,`) +
          'fill:red\nclass A z\n',
        big
      ),
  ],
  ['style of colons and hashes', () => parse(U + 'A\nstyle A fill:' + ':#'.repeat(N / 2), big)],
  ['10,000 declarations of one element', () => render(U + 'A(A) <<S>>:::c\n'.repeat(10000), big)],
  [
    '10,000 members of one boundary',
    () => parse(U + 'systemBoundary S\n' + repeat(10000, (i) => `u${i}\n`) + 'end', big),
  ],
  ['3,000 boundaries', () => render(U + repeat(3000, (i) => `systemBoundary s${i}\n u${i}\nend\n`), big)],
  ['5,000 metadata statements for one actor', () => parse(U + 'actor A\n' + 'A@{ type: hollow }\n'.repeat(5000), big)],
  ['5,000 notes on one use case', () => render(U + 'A\n' + 'note for A "n"\n'.repeat(5000), big)],
  [
    'JSON nested 12,000 objects deep',
    () => render(U + 'json P@' + '{"a":'.repeat(12000) + '1' + '}'.repeat(12000), big),
  ],
  [
    'JSON nested 20,000 arrays deep',
    () => render(U + 'json P@{"a":' + '['.repeat(20000) + ']'.repeat(20000) + '}', big),
  ],
  ['JSON with 8,000 keys', () => render(U + 'json P@{' + repeat(8000, (i) => `"k${i}":${i},`) + '"z":0}', big)],
  [
    'JSON with one key written 8,000 times',
    () => render(U + 'json P@{' + '"k":{"a":{}},'.repeat(8000) + '"z":0}', big),
  ],
  [
    'JSON with 5,000 nested objects and 5,000 repeats of a key',
    () => render(U + 'json P@{' + repeat(5000, (i) => `"k${i}":{},`) + '"d":1,'.repeat(5000) + '"z":0}', big),
  ],
  ['JSON array of 20,000 scalars', () => render(U + 'json P@{"a":[' + '1,'.repeat(20000) + '1]}', big)],
  ['invalid JSON at the end of 50,000 characters', () => parse(U + 'json P@{"a":"' + 'x'.repeat(N) + '",}', big)],
  ['one 50,000 character plain label', () => render(U + 'A("' + 'word '.repeat(N / 5) + '")', big)],
  ['one 50,000 character word', () => render(U + 'actor A("' + 'a'.repeat(N) + '")', big)],
  ['label of entities and markup', () => render(U + 'A("' + '#35;&amp;<br>'.repeat(N / 13) + '")', big)],
  ['markdown label of emphasis markers', () => render(U + 'A("`' + '*a _b '.repeat(N / 6) + '`")', big)],
  ['one 50,000 character relationship label', () => render(U + 'A -- "' + 'word '.repeat(N / 5) + '" --> B', big)],
  ['3,000 actors', () => render(U + repeat(3000, (i) => `actor a${i}\n`), big)],
  ['chain of 3,000', () => render(U + repeat(3000, (i) => `n${i} --> n${i + 1}\n`), big)],
  ['star of 3,000 into one use case', () => render(U + repeat(3000, (i) => `actor a${i}\na${i} --> hub\n`), big)],
  [
    '2,000 parallel relationships and 1,000 loops',
    () => render(U + 'A --> B\n'.repeat(2000) + 'A -- l --> A\n'.repeat(1000), big),
  ],
  [
    'complete graph of 40',
    () => {
      let s = U;
      for (let i = 0; i < 40; i++) for (let j = i + 1; j < 40; j++) s += `n${i} --o n${j}\n`;
      return render(s, big);
    },
  ],
  ['a relationship of 10,000 dashes', () => render(U + 'A ' + '-'.repeat(10000) + '> B', big)],
  [
    '2,000 relationships into one boundary',
    () =>
      render(
        U + 'systemBoundary S\n U\n V\nend\n' + repeat(1000, (i) => `actor a${i}\na${i} --> U\nV --> a${i}\n`),
        big
      ),
  ],
];

describe('use case worst cases', () => {
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
    expect(() => render(U + 'A --> B\n'.repeat(8000))).toThrow(/limit of 50000/);
  });
});
