import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };

function check(src: string): boolean {
  let svg: string;
  try {
    svg = render(src, options).svg;
  } catch (error) {
    if (error instanceof PelersError) return false;
    throw new Error(
      `Unexpected ${(error as Error).name} for ${JSON.stringify(src).slice(0, 200)}: ${(error as Error).message}`
    );
  }
  assertInert(svg, JSON.stringify(src).slice(0, 160));
  return true;
}

const quoted = (p: string): string => p.replace(/["\\\n]/g, '');
const single = (p: string): string => p.replace(/['\\\n]/g, '');

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `wardley-beta\ncomponent "${quoted(p)}" [0.5, 0.5]`,
  (p) => `wardley-beta\ncomponent '${single(p)}' [0.5, 0.5] (buy) (inertia)`,
  (p) => `wardley-beta\ncomponent ${p} [0.5, 0.5]`,
  (p) => `wardley-beta\nanchor "${quoted(p)}" [0.9, 0.5]\ncomponent B [0.5, 0.5]\n"${quoted(p)}" -> B`,
  (p) => `wardley-beta\ncomponent A [0.9, 0.1]\ncomponent B [0.5, 0.5]\nA -> B; ${p}`,
  (p) => `wardley-beta\ncomponent A [0.9, 0.1]\ncomponent B [0.5, 0.5]\nA +'${single(p)}'> B`,
  (p) => `wardley-beta\ncomponent A [0.9, 0.1]\n"${quoted(p)}" -> A\nA -> "${quoted(p)}"`,
  (p) => `wardley-beta\nnote "${quoted(p)}" [0.5, 0.5]`,
  (p) => `wardley-beta\nannotations [0.9, 0.1]\nannotation 1,[0.5, 0.5] "${quoted(p)}"`,
  (p) => `wardley-beta\nannotation 7,[0.5, 0.5] "${quoted(p)}"`,
  (p) => `wardley-beta\naccelerator "${quoted(p)}" [0.5, 0.5]\ndeaccelerator "${quoted(p)}" [0.2, 0.2]`,
  (p) => `wardley-beta\nevolution "${quoted(p)}" -> "${quoted(p)}"@0.5 / "${quoted(p)}" -> C\ncomponent A [0.5, 0.5]`,
  (p) => `wardley-beta\ncomponent "${quoted(p)}" [0.5, 0.5]\nevolve "${quoted(p)}" 0.9`,
  (p) =>
    `wardley-beta\ncomponent "${quoted(p)}" [0.5, 0.5]\npipeline "${quoted(p)}" {\n component "${quoted(p)}" [0.2]\n component B [0.8]\n}`,
  (p) => `wardley-beta\ntitle ${p}\ncomponent A [0.5, 0.5]`,
  (p) => `wardley-beta\naccTitle: ${p}\naccDescr: ${p}\ncomponent A [0.5, 0.5]`,
  (p) => `wardley-beta\naccDescr {\n ${p}\n}\ncomponent A [0.5, 0.5]`,
  (p) =>
    `---\ntitle: "${quoted(p)}"\nconfig:\n  wardley-beta:\n    width: '${single(p)}'\n    height: 1e999\n    showGrid: '${single(p)}'\n---\nwardley-beta\ncomponent A [0.5, 0.5]`,
];

describe('wardley output is inert', () => {
  it('for hostile text in every position', () => {
    let rendered = 0;
    for (const payload of PAYLOADS) for (const template of TEMPLATES) if (check(template(payload))) rendered++;
    expect(rendered).toBeGreaterThan(PAYLOADS.length * 12);
  }, 60_000);

  it('for extreme numbers', () => {
    const nines = '9'.repeat(400);
    for (const src of [
      `wardley-beta\nsize [${nines}, ${nines}]\ncomponent A [0.5, 0.5]`,
      'wardley-beta\nsize [0, 0]\ncomponent A [0.5, 0.5]',
      `wardley-beta\ncomponent A [0.5, 0.5] label [-${nines}, ${nines}]`,
      `wardley-beta\nannotation ${nines},[0.5, 0.5] "big"\nannotations [1, 1]`,
      `wardley-beta\nevolution A@${nines}.0 -> B@0.0\ncomponent A [0.0, 1.0]`,
      'wardley-beta\ncomponent A [0.5, 0.5]\ncomponent B [0.5, 0.5]\nA -> B\nA +<> B\nevolve A 0.5',
      'wardley-beta\ncomponent A [0.5, 0.5]\npipeline A {\n component B [0.5]\n component C [0.5]\n}\nevolve A 0.50',
      '---\nconfig:\n  wardley-beta:\n    width: 1\n    height: 100000\n---\nwardley-beta\ncomponent A [0.5, 0.5]',
    ]) {
      expect(check(src), src.slice(0, 60)).toBe(true);
    }
  });

  it('for mutated corpus inputs', () => {
    const corpus = loadCorpus('wardley', /wardley-beta/);
    const next = mutator(
      corpus,
      [...PAYLOADS, '"', "'", '\n', ' -> ', '; ', "+'", 'component ', ' [0.5, 0.5]', 'note ', '(buy)'],
      random(17)
    );
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    for (let i = 0; i < 2000; i++) if (check(next())) rendered++;
    expect(rendered).toBeGreaterThan(100);
  }, 60_000);
});
