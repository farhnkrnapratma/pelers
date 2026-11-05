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
  (p) => `treemap\n"${p}"\n  "a": 1`,
  (p) => `treemap\n'${p}'\n  "a": 1`,
  (p) => `treemap\n"s"\n  "${p}": 1\n  "b": 2`,
  (p) => `treemap\n"${p}": 5`,
  (p) => `treemap\n"s":::${p}\n  "a": 1`,
  (p) => `treemap\n"s"\n  "a": 1:::${p}`,
  (p) => `treemap\n"s":::c1\n  "a": 1:::c1\nclassDef c1 fill:${p}`,
  (p) => `treemap\n"s":::c1\n  "a": 1:::c1\nclassDef c1 ${p}:red`,
  (p) => `treemap\n"s":::c1\n  "a": 1:::c1\nclassDef c1 color:${p},stroke:${p},font-family:${p},font-size:${p}`,
  (p) => `treemap\n"s":::c1\n  "a": 1:::c1\nclassDef c1 ${p}`,
  (p) => `treemap\n"s"\n  "a": 1\nclassDef ${p} fill:red`,
  (p) => `treemap\n"s"\n  "a": ${p}`,
  (p) => `treemap\ntitle ${p}\n"a": 1`,
  (p) => `treemap title ${p}`,
  (p) => `treemap\naccTitle: ${p}\naccDescr: ${p}\n"a": 1`,
  (p) => `treemap\naccDescr {\n ${p}\n }\n"a": 1`,
  (p) => `treemap\n"a": 1 %% ${p}`,
  (p) => `treemap-beta ${p}\n"a": 1`,
  (p) => `---\ntitle: "${p}"\n---\ntreemap\n"a": 1`,
  (p) =>
    `---\nconfig:\n  treemap:\n    valueFormat: "${p}"\n    padding: "${p}"\n    showValues: "${p}"\n---\ntreemap\n"s"\n  "a": 1`,
  (p) =>
    `%%{init: {"treemap": {"nodeWidth": "${p}", "valueFormat": "${p}"}, "themeVariables": {"cScale0": "${p}"}}}%%\ntreemap\n"a": 1`,
];

describe('inert treemap output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('treemap', /treemap/)) check(src);
  });

  it('holds with hostile text in every position', { timeout: 60_000 }, () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(250);
  });

  it('escapes names and drops unsafe style values', () => {
    const { svg } = render(
      'treemap\n"<script>x</script>":::bad\n  "<img src=x onerror=alert(1)>": 1:::bad\nclassDef bad fill:url(javascript:alert(1)),stroke:red',
      options
    );
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('<img');
    expect(svg).not.toContain('url(');
    expect(svg).toContain('style="rx:var(--_r);stroke:red;"');
  });

  it('keeps a value format from writing markup', () => {
    const { svg } = render('---\nconfig:\n  treemap:\n    valueFormat: "<b>"\n---\ntreemap\n"a": 1234', options);
    expect(svg).toContain('>1,234<');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(check('treemap\naccTitle: t\n"a": 1', { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('does not pollute prototypes', () => {
    render(
      'treemap\n"__proto__":::__proto__\n  "constructor": 1:::constructor\nclassDef __proto__ fill:red\nclassDef constructor fill:blue\nclassDef toString polluted:1',
      options
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, 'fill')).toBe(false);
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('treemap', /treemap/);
    const fragments = [
      ...PAYLOADS,
      '"',
      "'",
      ':',
      ':::',
      ',',
      '\n',
      '\n  ',
      'title ',
      '"a": 1',
      'classDef c1 fill:',
      ':::c1',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);
});
