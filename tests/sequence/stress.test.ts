import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { tokenize } from '../../src/diagrams/sequence/lexer.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

// Inputs of about 50,000 characters built to hit the sequence diagram's worst cases: lexer rules
// with lookahead, deep nesting, huge counts, and wide layouts. Each must finish quickly and either
// succeed or fail with a PeleError. The bounds are loose; a regression is seconds or a hang.

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const head = 'sequenceDiagram\n';

const CASES: [string, () => unknown][] = [
  ['one 50,000 character message', () => render(head + 'A->>B: ' + 'word '.repeat(N / 5), big)],
  ['one wrapped 50,000 character message', () => render(head + 'A->>B: wrap: ' + 'word '.repeat(N / 5), big)],
  ['one 50,000 character note without spaces', () => render(head + 'Note over A: wrap: ' + 'x'.repeat(N), big)],
  ['message of line breaks and entities', () => render(head + 'A->>B: ' + '#35;<br/>'.repeat(N / 9), big)],
  ['participant name of 50,000 characters', () => render(head + 'participant ' + 'n'.repeat(N) + '\nA->>B: x', big)],
  ['participant name followed by spaces, no alias', () => tokenize(head + 'participant A' + ' '.repeat(N) + 'x')],
  [
    'participant line of comment markers before a colon',
    () => tokenize(head + 'participant a' + ' #b'.repeat(N / 3) + ':'),
  ],
  ['many participant lines that end in a colon', () => tokenize(head + 'participant a b c d e f:\n'.repeat(N / 24))],
  ['alias marker repeated', () => tokenize(head + 'participant a' + ' as'.repeat(N / 3))],
  ['config object never closed', () => parse(head + 'participant A@{ ' + '"a": 1, '.repeat(N / 8), big)],
  [
    'config object of nested brackets',
    () => parse(head + 'participant A@{ "a": ' + '['.repeat(N) + ' }\nA->>A: x', big),
  ],
  ['config braces repeated', () => tokenize(head + 'participant A' + '@{}'.repeat(N / 3))],
  ['actor name of dashes', () => tokenize(head + 'A' + '-'.repeat(N) + '>>B: x')],
  ['actor name of dashes and letters', () => tokenize(head + 'A' + '-b'.repeat(N / 2) + '->>B: x')],
  ['actor name of slashes and parentheses', () => tokenize(head + 'A' + '/(|)\\'.repeat(N / 5) + '->>B: x')],
  ['digits without a following space', () => tokenize(head + '1'.repeat(N) + 'x')],
  ['dotted numbers', () => tokenize(head + '1.2.'.repeat(N / 4))],
  ['menu keywords followed by spaces', () => tokenize(head + ('links' + ' '.repeat(40) + '\n').repeat(N / 46))],
  ['title keyword repeated', () => tokenize(head + 'title'.repeat(N / 5))],
  ['accessible description never closed', () => tokenize(head + 'accDescr {' + ' x'.repeat(N / 2))],
  ['accessible description opened and closed', () => parse(head + 'accDescr{}'.repeat(N / 10), big)],
  ['comment markers', () => tokenize(head + 'a%%'.repeat(N / 3))],
  ['whitespace', () => tokenize(head + ' \n\t'.repeat(N / 3) + 'A->>B: x')],
  [
    'box with a long color function',
    () => parse(head + 'box rgb(' + '1, '.repeat(N / 3) + ') T\nparticipant A\nend', big),
  ],
  [
    'box of parentheses',
    () => parse(head + 'box rgb' + ' '.repeat(N / 2) + '('.repeat(N / 2) + ')\nparticipant A\nend', big),
  ],
  [
    'links with a huge JSON object',
    () => parse(head + 'participant A\nlinks A: {' + repeat(4000, (i) => `"k${i}": "v", `) + '"z": 1}', big),
  ],
  [
    'links with deeply nested JSON',
    () => parse(head + 'participant A\nlinks A: ' + '['.repeat(N / 2) + ']'.repeat(N / 2), big),
  ],
  [
    'autonumber with enormous numbers',
    () => render(head + 'autonumber ' + '9'.repeat(N / 2) + ' ' + '9'.repeat(N / 2) + '\nA->>B: x\nB->>A: y', big),
  ],
  ['8,000 messages between two participants', () => render(head + 'A->>B: x\n'.repeat(8000), big)],
  ['4,000 participants', () => render(head + repeat(4000, (i) => `participant p${i}\n`), big)],
  [
    '2,500 participants and messages across all of them',
    () => render(head + repeat(2500, (i) => `p${i}->>p${2499 - i}: m\n`), big),
  ],
  [
    '1,500 participants with a long message between each pair of ends',
    () => render(head + repeat(1500, (i) => `p${i}->>p${(i * 7) % 1500}: a long message ${i}\n`), big),
  ],
  ['10,000 nested loops', () => parse(head + 'loop\n'.repeat(10000) + 'A->>B: x\n' + 'end\n'.repeat(10000), big)],
  [
    '3,000 nested blocks, rendered',
    () => render(head + repeat(3000, (i) => (i % 2 ? 'alt a\n' : 'rect\n')) + 'A->>B: x\n' + 'end\n'.repeat(3000), big),
  ],
  ['alt with 10,000 else sections', () => render(head + 'alt a\n' + 'else b\n'.repeat(10000) + 'end\n', big)],
  [
    'par with 5,000 sections of messages',
    () => render(head + 'par a\nA->>B: x\n' + 'and b\nB->>A: y\n'.repeat(5000) + 'end\n', big),
  ],
  ['5,000 nested activations', () => render(head + 'A->>+B: x\n'.repeat(5000) + 'B-->>-A: y\n'.repeat(5000), big)],
  ['8,000 explicit activations never closed', () => render(head + 'activate A\n'.repeat(4500), big)],
  [
    '4,000 notes over a span',
    () => render(head + 'participant A\nparticipant B\nparticipant C\n' + 'Note over A,C: n\n'.repeat(3000), big),
  ],
  ['5,000 self messages', () => render(head + 'A->>A: x\n'.repeat(5000), big)],
  [
    '2,000 creations without a message',
    () => render(head + repeat(2000, (i) => `create participant c${i}\n`) + 'Note over c0: n\n'.repeat(500), big),
  ],
  [
    '1,000 created and destroyed participants',
    () =>
      render(head + repeat(1000, (i) => `create participant c${i}\nA->>c${i}: x\ndestroy c${i}\nA-xc${i}: y\n`), big),
  ],
  ['1,500 boxes', () => render(head + repeat(1500, (i) => `box Aqua Group ${i}\nparticipant b${i}\nend\n`), big)],
  ['3,000 central connections with numbers', () => render(head + 'autonumber\n' + 'A()->>()B: x\n'.repeat(3000), big)],
  ['3,000 deactivations of an idle participant', () => parse(head + 'A->>B: x\n' + 'deactivate A\n'.repeat(3000), big)],
  ['invalid characters', () => parse(head + '>'.repeat(N), big)],
];

describe('sequence worst cases', () => {
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
    expect(() => render(head + 'A->>B: hello\n'.repeat(5000))).toThrow(/limit of 50000/);
    expect(render(head + 'A->>B: hello\n'.repeat(5000), big).svg).toContain('<svg');
  });
});
