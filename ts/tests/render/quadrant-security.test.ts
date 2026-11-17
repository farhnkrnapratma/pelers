import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };

function check(src: string, extra: object = {}): boolean {
  let svg: string;
  try {
    svg = render(src, { ...options, ...extra }).svg;
  } catch (error) {
    if (error instanceof PelersError) return false;
    throw new Error(
      `Unexpected ${(error as Error).name} for ${JSON.stringify(src).slice(0, 200)}: ${(error as Error).message}`
    );
  }
  assertInert(svg, JSON.stringify(src).slice(0, 160));
  return true;
}

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `quadrantChart\n  title ${p}\n  A: [0.5, 0.5]`,
  (p) => `quadrantChart\n  x-axis ${p} --> ${p}`,
  (p) => `quadrantChart\n  x-axis "${p}" --> "${p}"\n  A: [0.5, 0.5]`,
  (p) => `quadrantChart\n  y-axis "${p}" --> "${p}"`,
  (p) => `quadrantChart\n  y-axis ${p}\n  A: [0.5, 0.5]`,
  (p) => `quadrantChart\n  x-axis "${p}" -->`,
  (p) => `quadrantChart\n  quadrant-1 ${p}\n  quadrant-2 "${p}"`,
  (p) => `quadrantChart\n  quadrant-3 "\`${p}\`"\n  quadrant-4 "\`**${p}**\`"\n  A: [0.1, 0.9]`,
  (p) => `quadrantChart\n  ${p}: [0.5, 0.5]`,
  (p) => `quadrantChart\n  "${p}": [0.5, 0.5]`,
  (p) => `quadrantChart\n  "\`${p}\`": [0.5, 0.5]`,
  (p) => `quadrantChart\n  A:::${p}: [0.5, 0.5]`,
  (p) => `quadrantChart\n  A:::k: [0.5, 0.5]\n  classDef ${p} color: #f00`,
  (p) => `quadrantChart\n  A:::k: [0.5, 0.5]\n  classDef k color: ${p}`,
  (p) => `quadrantChart\n  A:::k: [0.5, 0.5]\n  classDef k ${p}: 3`,
  (p) => `quadrantChart\n  A: [0.5, 0.5] color: ${p}`,
  (p) => `quadrantChart\n  A: [0.5, 0.5] radius: ${p}, stroke-width: ${p}, stroke-color: ${p}`,
  (p) => `quadrantChart\n  A: [0.5, 0.5] ${p}`,
  (p) => `quadrantChart\n  A: [${p}, 0.5]`,
  (p) => `quadrantChart\n  A: [0.5, ${p}]`,
  (p) => `quadrantChart\n  accTitle: ${p}\n  accDescr: ${p}\n  A: [0.5, 0.5]`,
  (p) => `quadrantChart\n  accDescr {\n ${p}\n }\n  A: [0.5, 0.5]`,
  (p) => `---\ntitle: "${p}"\n---\nquadrantChart\n  A: [0.5, 0.5]`,
  (p) =>
    `---\nconfig:\n  quadrantChart:\n    chartWidth: "${p}"\n    chartHeight: "${p}"\n    pointRadius: "${p}"\n    quadrantPadding: "${p}"\n    xAxisPosition: "${p}"\n    yAxisPosition: "${p}"\n---\nquadrantChart\n  x-axis a --> b\n  y-axis c --> d\n  A: [0.5, 0.5]`,
  (p) => `---\nconfig:\n  quadrantChart: "${p}"\n---\nquadrantChart\n  A: [0.5, 0.5]`,
  (p) =>
    `%%{init: {"quadrantChart": {"chartWidth": "${p}", "titlePadding": "${p}"}, "themeVariables": {"quadrant1Fill": "${p}"}}}%%\nquadrantChart\n  title t\n  A: [0.5, 0.5]`,
  (p) => `quadrantChart ${p}\n  A: [0.5, 0.5]`,
  (p) => `quadrantChart\n  %% ${p}\n  A: [0.5, 0.5]`,
];

describe('inert quadrant chart output', () => {
  const corpus = loadCorpus('quadrant', /quadrant/i);

  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(30);
  }, 60_000);

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(250);
  }, 60_000);

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(check('quadrantChart\n  accTitle: t\n  A: [0.5, 0.5]', { idPrefix: payload, fontFamily: payload })).toBe(
        true
      );
    }
  }, 60_000);

  it('holds for extreme numbers', () => {
    for (const value of [
      '1e999',
      '-1',
      '0',
      '0.0001',
      '99999999',
      'NaN',
      '.inf',
      '-.inf',
      '[1]',
      '{a: 1}',
      'true',
      'null',
    ]) {
      const keys = [
        'chartWidth',
        'chartHeight',
        'titlePadding',
        'quadrantPadding',
        'xAxisLabelPadding',
        'yAxisLabelPadding',
      ];
      const more = ['quadrantTextTopPadding', 'pointTextPadding', 'pointRadius'];
      const config = [...keys, ...more].map((key) => `    ${key}: ${value}`).join('\n');
      const src = `---\nconfig:\n  quadrantChart:\n${config}\n---\nquadrantChart\n  title t\n  x-axis a --> b\n  y-axis c --> d\n  quadrant-1 q\n  A: [0.5, 0.5]`;
      expect(check(src), value).toBe(true);
    }
    expect(check(`quadrantChart\n  A: [0.5, 0.5] radius: ${'9'.repeat(400)}, stroke-width: ${'9'.repeat(400)}px`)).toBe(
      true
    );
    expect(check('quadrantChart\n  A: [0x5, 0e9]\n  B: [0,5, 0 5]\n  C: [055, 0b1]')).toBe(true);
  }, 60_000);

  it('holds for mutated inputs', () => {
    const fragments = [
      ...PAYLOADS,
      '"',
      '`',
      ':',
      ':::',
      ': [',
      ']',
      '\n',
      ', ',
      'classDef ',
      'color: #',
      'radius: ',
      '-->',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('does not pollute prototypes', () => {
    const src =
      'quadrantChart\n  __proto__:::__proto__: [0.5, 0.5]\n  constructor:::constructor: [0.1, 0.1]\n  toString:::toString: [0.2, 0.2]\n' +
      '  classDef constructor radius: 3\n  classDef hasOwnProperty color: #f00';
    const { svg } = render(src, options);
    expect(svg.match(/<circle/g)?.length).toBe(3);
    expect(svg.match(/ r="3"/g)?.length).toBe(1);
    expect(({} as Record<string, unknown>).color).toBeUndefined();
    expect(({} as Record<string, unknown>).radius).toBeUndefined();
  });
});
