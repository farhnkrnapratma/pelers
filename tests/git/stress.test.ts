import { describe, expect, it } from 'vitest';
import { PeleError, parse, render } from '../../src/index.js';
import { parseGit } from '../../src/diagrams/git/parser.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

// Inputs of about 50,000 characters built to trigger worst cases: patterns that rescan, lookups
// that could go quadratic, and oversized layouts. Each must finish quickly and either succeed or
// fail with a PeleError. The bounds are loose; a regression here is seconds or a hang.

const N = 50000;
const big = { measurer: metricsMeasurer, limit: Infinity };
const repeat = (count: number, line: (i: number) => string): string =>
  Array.from({ length: count }, (_, i) => line(i)).join('');
const config = (yaml: string): string => `---\nconfig:\n  gitGraph:\n${yaml}\n---\n`;

const ZIGZAG = repeat(1200, (i) => `checkout ${i % 2 ? 'main' : 'b'}\ncommit\nmerge ${i % 2 ? 'b' : 'main'}\n`);
const PICKS =
  'gitGraph\ncommit id:"s"\nbranch b\ncommit\n' +
  repeat(1500, (i) => `checkout main\ncommit id:"m${i}"\ncheckout b\ncommit\ncherry-pick id:"s"\n`);

const CASES: [string, () => unknown][] = [
  ['7,000 commits', () => render('gitGraph\n' + 'commit\n'.repeat(7000), big)],
  [
    '7,000 commits, top to bottom, ids level',
    () => render(config('    rotateCommitLabel: false') + 'gitGraph TB:\n' + 'commit\n'.repeat(7000), big),
  ],
  [
    '3,000 branches of one commit',
    () => render('gitGraph\ncommit\n' + repeat(3000, (i) => `branch b${i}\ncommit\n`), big),
  ],
  [
    '3,000 branches, bottom to top, parallel',
    () =>
      render(
        config('    parallelCommits: true') + 'gitGraph BT:\ncommit\n' + repeat(3000, (i) => `branch b${i}\ncommit\n`),
        big
      ),
  ],
  [
    '1,500 branches merged back',
    () => render('gitGraph\ncommit\n' + repeat(1500, (i) => `branch b${i}\ncommit\ncheckout main\nmerge b${i}\n`), big),
  ],
  [
    'two branches merging into each other 1,200 times',
    () => render('gitGraph\ncommit\nbranch b\ncommit\n' + ZIGZAG, big),
  ],
  [
    'the same, parallel',
    () => render(config('    parallelCommits: true') + 'gitGraph\ncommit\nbranch b\ncommit\n' + ZIGZAG, big),
  ],
  ['1,500 cherry-picks of one old commit', () => render(PICKS, big)],
  [
    'the same, parallel and top to bottom',
    () => render(config('    parallelCommits: true') + PICKS.replace('gitGraph', 'gitGraph TB:'), big),
  ],
  [
    '4,000 tags on one commit',
    () => render('gitGraph\ncommit' + ' tag:"t"'.repeat(4000) + '\nbranch b\ncommit' + ' tag:"u"'.repeat(2000), big),
  ],
  ['3,000 commits with one id', () => render('gitGraph\n' + 'commit id:"same"\n'.repeat(3000), big)],
  [
    '1,500 commits that already have the ids Pele would generate next',
    () => {
      const model = parse('gitGraph\n' + 'commit\n'.repeat(3000), big);
      if (model.type !== 'gitGraph') throw new Error('not a git graph');
      const later = [...model.commits.keys()].slice(1500);
      return render('gitGraph\n' + later.map((id) => `commit id:"${id}"\n`).join('') + 'commit\n'.repeat(1500), big);
    },
  ],
  ['one 50,000 character id', () => render('gitGraph\ncommit id:"' + 'word '.repeat(N / 5) + '"', big)],
  ['one 50,000 character branch name', () => render('gitGraph\ncommit\nbranch ' + 'x'.repeat(N) + '\ncommit', big)],
  ['a name of dots with no end', () => parseGit('gitGraph\nbranch a' + '.'.repeat(N))],
  ['names that end in runs of dots', () => parseGit('gitGraph\n' + ('branch a' + '.'.repeat(20) + 'b\n').repeat(2000))],
  ['digits before a dot', () => parseGit('gitGraph\nbranch b order: ' + '9'.repeat(N) + '.')],
  ['an order of 50,000 digits', () => render('gitGraph\ncommit\nbranch b order: ' + '9'.repeat(N) + '\ncommit', big)],
  ['unclosed strings', () => parseGit('gitGraph\ncommit id:' + '"a'.repeat(N / 2))],
  ['one unclosed string of escapes', () => parseGit('gitGraph\ncommit id:"' + '\\"'.repeat(N / 2))],
  ['unclosed accDescr blocks', () => parseGit('gitGraph\n' + 'accDescr {\n'.repeat(N / 11))],
  ['accDescr over blank lines', () => parseGit('gitGraph\n' + ('accDescr' + '\n'.repeat(50)).repeat(800))],
  ['unclosed directives', () => parseGit('gitGraph\n' + '%%{\n'.repeat(N / 4))],
  ['front matter fences that never close', () => parseGit('gitGraph\n' + '---\n'.repeat(N / 4))],
  ['a fence over lines that look like closers', () => parseGit('---\n' + '--- x\n'.repeat(N / 6) + 'gitGraph')],
  ['title lines with comment marks', () => parseGit('gitGraph\n' + ('title ' + 'a%'.repeat(40) + '\n').repeat(550))],
  ['one 50,000 character title', () => render('gitGraph\ntitle ' + 'x'.repeat(N) + '\ncommit', big)],
  ['50,000 spaces before a statement', () => parseGit('gitGraph\n' + ' '.repeat(N) + 'commit')],
  [
    'spaces between every token',
    () => parseGit('gitGraph\n' + ('commit' + ' '.repeat(40) + 'id:' + ' '.repeat(40) + '"a"\n').repeat(500)),
  ],
  ['keywords run together', () => parseGit('gitGraph\n' + 'commit'.repeat(N / 6))],
  ['a header repeated', () => parseGit('gitGraph:'.repeat(N / 9))],
];

describe('git graph worst cases', () => {
  for (const [name, run] of CASES) {
    it(name, () => {
      const started = performance.now();
      try {
        run();
      } catch (error) {
        expect(error, (error as Error).message).toBeInstanceOf(PeleError);
      }
      expect(performance.now() - started).toBeLessThan(2000);
    });
  }
});
