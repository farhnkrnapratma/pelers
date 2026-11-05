import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/class/lexer.js';
import { ClassMember } from '../../src/diagrams/class/members.js';
import { parseGenericTypes } from '../../src/diagrams/common/generics.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const diagram = (body: string): string => 'classDiagram\n' + body;

const CASES: [string, () => unknown][] = [
  ['long line that mentions direction', () => tokenize(diagram('A --> B ' + 'A '.repeat(N / 2) + 'direction XX'))],
  ['direction on every line, never valid', () => tokenize(diagram('direction\n'.repeat(N / 10)))],
  [
    'direction followed by a run of blank lines',
    () => tokenize(diagram('direction' + '\n '.repeat(N / 2) + 'XX\nclass A')),
  ],
  ['many directions on one line', () => tokenize(diagram('direction X '.repeat(N / 12)))],
  ['one line of spaced words', () => tokenize(diagram('a '.repeat(N / 2)))],
  ['whitespace run with no line break', () => tokenize(diagram('class A' + ' '.repeat(N) + 'x'))],
  [
    'blank lines between statements',
    () => parse(diagram(repeat(2000, (i) => `class C${i}` + '\n \t'.repeat(8) + '\n')), big),
  ],
  ['accTitle with no colon, many times', () => tokenize(diagram('accTitle '.repeat(N / 9)))],
  ['accDescr followed by spaces', () => tokenize(diagram('accDescr' + ' '.repeat(N)))],
  ['unclosed accDescr block', () => parse(diagram('accDescr {' + 'x\n'.repeat(N / 2)), big)],
  ['call followed by spaces', () => tokenize(diagram('click A call' + ' '.repeat(N) + 'f()'))],
  ['callback arguments with no closing parenthesis', () => parse(diagram('click A call f(' + 'a,'.repeat(N / 2)), big)],
  ['empty call parentheses over spaces', () => tokenize(diagram('click A call f(' + ' '.repeat(N) + 'x)'))],
  ['unclosed string', () => parse(diagram('note "' + 'x'.repeat(N)), big)],
  ['unclosed generic', () => parse(diagram('class A~' + 'T'.repeat(N)), big)],
  ['unclosed backtick name', () => parse(diagram('class `' + 'a'.repeat(N)), big)],
  ['label of colons', () => tokenize(diagram('A ' + ':'.repeat(N)))],
  ['label of semicolons and colons', () => tokenize(diagram('A ' + ':;'.repeat(N / 2)))],
  ['runs of dashes and dots', () => tokenize(diagram('A ' + '-'.repeat(N / 2) + '.'.repeat(N / 2) + ' B'))],
  ['runs of relation ends', () => tokenize(diagram('A ' + '<|'.repeat(N / 4) + '()'.repeat(N / 4)))],
  ['class name of 25,000 parts', () => parse(diagram('class ' + 'A.'.repeat(N / 2) + 'B'), big)],
  ['class name of 25,000 words', () => parse(diagram('A '.repeat(N / 2) + '--> B'), big)],
  ['namespace name of 12,000 parts', () => parse(diagram('namespace ' + 'a.'.repeat(12000) + 'b {\nclass X\n}'), big)],
  [
    'namespace name of 3,000 parts, rendered',
    () => render(diagram('namespace ' + 'a.'.repeat(3000) + 'b {\nclass X\n}'), big),
  ],
  [
    '4,000 nested namespaces',
    () => parse(diagram('namespace n {\n'.repeat(4000) + 'class X\n' + '}\n'.repeat(4000)), big),
  ],
  [
    '2,000 nested namespaces, rendered',
    () => render(diagram(repeat(2000, (i) => `namespace n${i} {\n`) + 'class X\n' + '}\n'.repeat(2000)), big),
  ],
  ['unclosed nested namespaces', () => parse(diagram('namespace n {\n'.repeat(4000) + 'class X\n'), big)],
  ['closing braces with nothing open', () => parse(diagram('namespace n {\nclass X\n' + '}'.repeat(N)), big)],
  ['namespace keyword repeated', () => parse(diagram('namespace '.repeat(N / 10) + 'x {\nclass X\n}'), big)],
  ['5,000 sibling namespaces', () => render(diagram(repeat(5000, (i) => `namespace n${i} {\nclass C${i}\n}\n`)), big)],
  ['class with 10,000 members', () => render(diagram('class A {\n' + repeat(10000, (i) => `+m${i}()\n`) + '}\n'), big)],
  ['member of opening parentheses', () => render(diagram('class A {\n+a)' + '('.repeat(N) + '\n}\n'), big)],
  ['member of nested parentheses', () => render(diagram('class A {\n' + '(a)'.repeat(N / 3) + '\n}\n'), big)],
  ['member of tildes', () => render(diagram('class A {\n+x ' + '~'.repeat(N) + '\n}\n'), big)],
  ['member of tildes and commas', () => render(diagram('class A {\n+x(' + '~a,'.repeat(N / 3) + ')\n}\n'), big)],
  [
    'method parser on parentheses and line breaks',
    () => new ClassMember(('(' + '\r').repeat(N / 2) + 'a(b)', 'method'),
  ],
  ['generic types of commas', () => parseGenericTypes('~' + ','.repeat(N) + '~')],
  ['one 50,000 character member', () => render(diagram('class A {\n+' + 'word '.repeat(N / 5) + '\n}\n'), big)],
  ['one 50,000 character note', () => render(diagram('note "' + 'word '.repeat(N / 5) + '"\n'), big)],
  ['note of emphasis markers and breaks', () => render(diagram('note "' + '*a _b <br>'.repeat(N / 10) + '"\n'), big)],
  ['label of entities', () => render(diagram('class A["' + '#35;&amp;'.repeat(N / 9) + '"]\n'), big)],
  ['5,000 classes', () => render(diagram(repeat(5000, (i) => `class C${i}\n`)), big)],
  ['chain of 4,000', () => render(diagram(repeat(4000, (i) => `C${i} <|-- C${i + 1}\n`)), big)],
  ['star of 4,000 with cardinalities', () => render(diagram(repeat(4000, (i) => `Hub "1" --> "${i}" C${i}\n`)), big)],
  [
    '2,000 parallel relations and 2,000 self relations',
    () => render(diagram('A "1" --> "2" B : x\n'.repeat(2000) + 'A "1" --> "2" A : y\n'.repeat(2000)), big),
  ],
  [
    'complete graph of 50',
    () => {
      let s = '';
      for (let i = 0; i < 50; i++) for (let j = i + 1; j < 50; j++) s += `C${i} --> C${j}\n`;
      return render(diagram(s), big);
    },
  ],
  ['3,000 lollipop interfaces on one class', () => render(diagram(repeat(3000, (i) => `i${i} ()-- A\n`)), big)],
  ['3,000 notes for one class', () => render(diagram('class A\n' + repeat(3000, (i) => `note for A "n${i}"\n`)), big)],
  [
    'style with 12,000 declarations',
    () => render(diagram('class A\nstyle A fill:#fff' + ',stroke:#000'.repeat(4000) + '\n'), big),
  ],
  [
    '3,000 classDefs applied to 100 classes',
    () => render(diagram(repeat(100, (i) => `class C${i}:::k\n`) + 'classDef k fill:#fff\n'.repeat(3000)), big),
  ],
  [
    '1,600 classDefs applied to 1,600 classes',
    () => render(diagram(repeat(1600, (i) => `class C${i}:::k\n`) + 'classDef k fill:#fff\n'.repeat(1600)), big),
  ],
  [
    'a classDef of 5,000 declarations on 800 classes, each with a style of its own',
    () =>
      render(
        diagram(
          repeat(800, (i) => `class C${i}:::k\nstyle C${i} x:${i}\n`) +
            'classDef k x:y' +
            repeat(5000, (i) => `,a${i}:b`) +
            '\n'
        ),
        big
      ),
  ],
  ['cssClass with 12,000 ids', () => render(diagram('class A\ncssClass "' + 'A,'.repeat(12000) + 'A" k\n'), big)],
  [
    'link wrapped in layers of entity encoding',
    () => render(diagram('class A\nlink A "&' + '#38;'.repeat(N / 4) + 'x"\n'), big),
  ],
  ['annotation statements for one class', () => render(diagram(repeat(4000, () => '<<a>> A\n')), big)],
];

describe('class diagram worst cases', () => {
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
    expect(() => render(diagram('class A\n'.repeat(10000)))).toThrow(/limit of 50000/);
  });
});
