import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';

const options = { measurer: metricsMeasurer, limit: Infinity };
const LIMIT = 3000;

function timed(name: string, src: string): void {
  it(name, { timeout: 120000 }, () => {
    const started = performance.now();
    try {
      const { svg } = render(src, options);
      expect(svg).not.toMatch(/NaN|Infinity/);
    } catch (error) {
      if (!(error instanceof PeleError)) throw error;
    }
    expect(performance.now() - started).toBeLessThan(LIMIT);
  });
}

const lanes = (count: number, body: (k: number) => string): string =>
  Array.from({ length: count }, (_, k) => `  subgraph L${k}\n${body(k)}\n  end`).join('\n');

describe('swimlane worst cases', () => {
  timed(
    '2,000 nodes in 20 lanes with 3,000 edges',
    (() => {
      let src =
        'swimlane-beta TD\n' +
        lanes(20, (k) => Array.from({ length: 100 }, (_, i) => `    n${k}_${i}`).join('\n')) +
        '\n';
      let seed = 7;
      const next = (n: number): number => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) % n;
      for (let e = 0; e < 3000; e++) src += `  n${next(20)}_${next(100)} --> n${next(20)}_${next(100)}\n`;
      return src;
    })()
  );

  timed(
    'one long chain that changes lane at every step',
    (() => {
      let src =
        'swimlane-beta LR\n' +
        lanes(8, (k) => Array.from({ length: 250 }, (_, i) => `    n${i * 8 + k}`).join('\n')) +
        '\n';
      for (let i = 0; i < 1999; i++) src += `  n${i} --> n${i + 1}\n`;
      return src;
    })()
  );

  timed('500 lanes', 'swimlane-beta TD\n' + lanes(500, (k) => `    a${k} --> b${k}`) + '\n');

  timed(
    'edges that span 1,500 ranks',
    (() => {
      let src = 'swimlane-beta TD\n  subgraph A\n';
      for (let i = 0; i < 1500; i++) src += `    a${i} --> a${i + 1}\n`;
      src += '  end\n  subgraph B\n    b\n  end\n';
      for (let i = 0; i < 40; i++) src += `  a0 --> a${1500 - i}\n  b --> a${1400 - i}\n`;
      return src;
    })()
  );

  timed(
    '3,000 labelled edges between two nodes',
    'swimlane-beta TD\n  subgraph A\n    a\n  end\n  subgraph B\n    b\n  end\n' + '  a -- label --> b\n'.repeat(3000)
  );

  timed('3,000 self-loops', 'swimlane-beta TD\n  subgraph A\n    a\n  end\n' + '  a --> a\n'.repeat(3000));

  timed(
    'groups nested 2,000 deep in a lane',
    'swimlane-beta TD\n' +
      '  subgraph g\n'.repeat(1).replace('g', 'L') +
      Array.from({ length: 2000 }, (_, i) => `subgraph g${i}\n`).join('') +
      'a --> b\n' +
      'end\n'.repeat(2001)
  );

  timed(
    '1,000 edges between lanes themselves',
    'swimlane-beta TD\n' +
      lanes(20, (k) => `    a${k}`) +
      '\n' +
      Array.from({ length: 1000 }, (_, i) => `  L${i % 20} --> L${(i * 7 + 3) % 20}`).join('\n') +
      '\n'
  );

  timed(
    'a cycle through every node',
    (() => {
      let src =
        'swimlane-beta RL\n' +
        lanes(10, (k) => Array.from({ length: 100 }, (_, i) => `    n${k * 100 + i}`).join('\n')) +
        '\n';
      for (let i = 0; i < 1000; i++)
        src += `  n${i} --> n${(i * 37 + 11) % 1000}\n  n${(i * 37 + 11) % 1000} --> n${i}\n`;
      return src;
    })()
  );
});
