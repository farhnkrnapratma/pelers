import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };

function check(src: string, extra: object = {}): boolean {
  let svg: string;
  try {
    svg = render(src, { ...options, ...extra }).svg;
  } catch (error) {
    if (error instanceof PeleError) return false;
    throw new Error(
      `Unexpected ${(error as Error).name} for ${JSON.stringify(src).slice(0, 200)}: ${(error as Error).message}`
    );
  }
  assertInert(svg, JSON.stringify(src).slice(0, 160));
  return true;
}

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `radar-beta\n axis a["${p}"], b, c\n curve x{1,2,3}`,
  (p) => `radar-beta\n axis a['${p}'], b, c\n curve x{1,2,3}`,
  (p) => `radar-beta\n axis a, b, c\n curve x["${p}"]{1,2,3}`,
  (p) => `radar-beta\n axis a, b, c\n curve x['${p}']{ a: 1, b: 2, c: 3 }`,
  (p) => `radar-beta\n axis ${p}, b\n curve x{1,2}`,
  (p) => `radar-beta\n axis a, b\n curve ${p}{1,2}`,
  (p) => `radar-beta\n axis a, b\n curve x{ ${p}: 1, a: 1, b: 2 }`,
  (p) => `radar-beta\n axis a, b\n curve x{ ${p}, 2 }`,
  (p) => `radar-beta\n title ${p}\n axis a, b\n curve x{1,2}`,
  (p) => `radar-beta title ${p}`,
  (p) => `radar-beta\n accTitle: ${p}\n accDescr: ${p}\n axis a, b\n curve x{1,2}`,
  (p) => `radar-beta\n accDescr {\n ${p}\n }\n axis a, b\n curve x{1,2}`,
  (p) => `radar-beta\n axis a, b\n curve x{1,2}\n graticule ${p}`,
  (p) => `radar-beta\n axis a, b\n curve x{1,2}\n max ${p}\n ticks ${p}`,
  (p) => `radar-beta\n axis a, b %% ${p}\n curve x{1,2}`,
  (p) => `radar-beta: ${p}\n axis a, b`,
  (p) => `---\ntitle: "${p}"\n---\nradar-beta\n axis a, b\n curve x{1,2}`,
  (p) =>
    `---\nconfig:\n  radar:\n    width: "${p}"\n    curveTension: "${p}"\n    marginLeft: "${p}"\n---\nradar-beta\n axis a, b\n curve x{1,2}`,
  (p) =>
    `%%{init: {"radar": {"axisLabelFactor": "${p}"}, "themeVariables": {"cScale0": "${p}", "radar": {"axisColor": "${p}"}}}}%%\nradar-beta\n axis a, b\n curve x{1,2}`,
];

describe('inert radar output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('radar', /radar/)) check(src);
  });

  it('holds with hostile text in every position', { timeout: 60_000 }, () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(200);
  });

  it('escapes names and labels', () => {
    const { svg } = render(
      'radar-beta\n axis a["<script>x</script>"], b["\\"q\\" & co"]\n curve c["<img src=x>"]{1,2}',
      options
    );
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('<img');
    expect(svg).toContain('&quot;q&quot; &amp; co');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(
        check('radar-beta\n accTitle: t\n axis a, b\n curve x{1,2}', { idPrefix: payload, fontFamily: payload })
      ).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('radar', /radar/);
    const fragments = [
      ...PAYLOADS,
      '"',
      "'",
      '[',
      ']',
      '{',
      '}',
      ':',
      ',',
      '\n',
      'title ',
      'axis ',
      'curve c{1,2}',
      'max 3',
      'graticule polygon',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);
});
