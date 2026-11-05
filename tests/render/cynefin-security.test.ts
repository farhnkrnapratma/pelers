import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };

function check(src: string): boolean {
  let svg: string;
  try {
    svg = render(src, options).svg;
  } catch (error) {
    if (error instanceof PeleError) return false;
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
  (p) => `cynefin-beta\ncomplex\n"${quoted(p)}"`,
  (p) => `cynefin-beta\ncomplex\n'${single(p)}'`,
  (p) => `cynefin-beta\nconfusion\n"${quoted(p)}"\n"${quoted(p)}"\n"${quoted(p)}"\n"${quoted(p)}"`,
  (p) => `cynefin-beta\nclear\n"a \\"${quoted(p)}\\" \\\\ b"`,
  (p) => `cynefin-beta\ncomplex --> clear : "${quoted(p)}"`,
  (p) => `cynefin-beta\nconfusion --> chaotic : '${single(p)}'`,
  (p) => `cynefin-beta\ntitle ${p}\ncomplex`,
  (p) => `cynefin-beta\naccTitle: ${p}\naccDescr: ${p}\ncomplex`,
  (p) => `cynefin-beta\naccDescr {\n ${p}\n}\ncomplex`,
  (p) => `cynefin-beta\n${p}`,
  (p) => `cynefin-beta\ncomplex --> ${p}`,
  (p) => `---\ntitle: "${quoted(p)}"\n---\ncynefin-beta\ncomplex\n"a"`,
  (p) =>
    `---\nconfig:\n  cynefin:\n    width: '${single(p)}'\n    height: 1e999\n    seed: .nan\n    boundaryAmplitude: '${single(p)}'\n    showDomainDescriptions: '${single(p)}'\n---\ncynefin-beta\ncomplex\n"a"\nclear --> chaotic : "b"`,
];

describe('cynefin output is inert', () => {
  it('for hostile text in every position', () => {
    let rendered = 0;
    for (const payload of PAYLOADS) for (const template of TEMPLATES) if (check(template(payload))) rendered++;
    expect(rendered).toBeGreaterThan(PAYLOADS.length * 8);
  }, 60_000);

  it('for extreme options', () => {
    for (const body of [
      'width: 1\n    height: 1',
      'width: 10000\n    height: 10000',
      'seed: 1e300',
      'seed: -7',
      'boundaryAmplitude: 50',
      'boundaryAmplitude: -1',
    ]) {
      expect(check(`---\nconfig:\n  cynefin:\n    ${body}\n---\ncynefin-beta\ncomplex\n"a"\ncomplex --> clear\n`)).toBe(
        true
      );
    }
  });

  it('for mutated corpus inputs', () => {
    const corpus = loadCorpus('cynefin', /cynefin-beta/);
    const next = mutator(
      corpus,
      [...PAYLOADS, '"', "'", '\n', ' --> ', ' : ', 'complex', 'confusion', 'title '],
      random(13)
    );
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    for (let i = 0; i < 2000; i++) if (check(next())) rendered++;
    expect(rendered).toBeGreaterThan(100);
  }, 60_000);
});
