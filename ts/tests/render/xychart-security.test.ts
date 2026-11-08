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
  (p) => `xychart\n  title "${p}"\n  line [1, 2]`,
  (p) => `xychart\n  title ${p}\n  line [1, 2]`,
  (p) => `xychart\n  x-axis "${p}" ["${p}", "b"]\n  bar [1, 2]`,
  (p) => `xychart\n  x-axis ${p} [${p}, b]\n  bar [1, 2]`,
  (p) => `xychart\n  x-axis "${p}" 1 --> 5\n  line [1, 2]`,
  (p) => `xychart\n  x-axis ${p} --> ${p}\n  line [1, 2]`,
  (p) => `xychart\n  y-axis "${p}"\n  line [1, 2]`,
  (p) => `xychart\n  y-axis "${p}" 0 --> 10\n  bar [1, 2]`,
  (p) => `xychart\n  y-axis ${p} --> ${p}\n  bar [1, 2]`,
  (p) => `xychart\n  line "${p}" [1, 2]\n  bar "${p}" [2, 1]`,
  (p) => `xychart\n  line ${p} [1, 2]`,
  (p) => `xychart\n  line [1 "${p}", 2 "${p}"]`,
  (p) => `xychart horizontal\n  x-axis ["${p}", "b"]\n  line "${p}" [1 "${p}", 2]\n  bar [3, 4]`,
  (p) => `xychart\n  line [${p}, 2]`,
  (p) => `xychart\n  bar [1, ${p}]`,
  (p) => `xychart ${p}\n  line [1, 2]`,
  (p) => `xychart\n  accTitle: ${p}\n  accDescr: ${p}\n  line [1, 2]`,
  (p) => `xychart\n  accDescr {\n ${p}\n }\n  line [1, 2]`,
  (p) => `---\ntitle: "${p}"\n---\nxychart\n  line [1, 2]`,
  (p) =>
    `---\nconfig:\n  xyChart:\n    width: "${p}"\n    height: "${p}"\n    chartOrientation: "${p}"\n    showDataLabel: "${p}"\n    showTitle: "${p}"\n    xAxis:\n      showLabel: "${p}"\n      showTick: "${p}"\n    yAxis: "${p}"\n---\nxychart\n  title t\n  x-axis x [a, b]\n  y-axis y\n  bar "s" [1, 2]`,
  (p) => `---\nconfig:\n  xyChart: "${p}"\n---\nxychart\n  line [1, 2]`,
  (p) => `---\nconfig:\n  xyChart:\n    xAxis: ["${p}"]\n    yAxis:\n      "${p}": true\n---\nxychart\n  line [1, 2]`,
  (p) =>
    `%%{init: {"xyChart": {"width": "${p}", "showDataLabel": true}, "themeVariables": {"xyChart": {"plotColorPalette": "${p}"}}}}%%\nxychart\n  bar [1, 2]`,
  (p) => `xychart\n  %% ${p}\n  line [1, 2]`,
];

describe('inert XY chart output', () => {
  const corpus = loadCorpus('xychart', /xychart/i);

  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(25);
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
      expect(check('xychart\n  accTitle: t\n  line "s" [1, 2]', { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  }, 60_000);

  it('holds for extreme numbers', () => {
    const huge = '9'.repeat(400);
    const charts = [
      `xychart\n  line [${huge}, 1]`,
      `xychart\n  bar [-${huge}, ${huge}]\n  line [1, 2]`,
      `xychart\n  y-axis ${huge} --> -${huge}\n  bar [1, 2]`,
      `xychart\n  y-axis 0 --> ${huge}\n  bar [1, 2]`,
      `xychart\n  x-axis ${huge} --> ${huge}\n  line [1, 2]\n  bar [1, 2]`,
      `xychart\n  x-axis -${huge} --> 3\n  line [1, 2]`,
      'xychart\n  y-axis 5 --> 5\n  bar [5, 5]',
      'xychart\n  y-axis 9 --> 1\n  x-axis 9 --> 1\n  bar [5, 3]\n  line [2, 8]',
      'xychart\n  bar [0, 0, 0]',
      'xychart\n  line [7]',
      'xychart\n  line [0.0000001, 0.0000002]',
      'xychart\n  bar [99999999999999999999, 1]\n  line [-99999999999999999999]',
      'xychart horizontal\n  x-axis 5 --> 5\n  bar [1]',
    ];
    for (const chart of charts) expect(check(chart), chart.slice(0, 60)).toBe(true);
    for (const value of ['1e999', '-1', '0', '0.0001', '99999999', 'NaN', '.inf', '-.inf', '[1]', '{a: 1}', 'null']) {
      const src = `---\nconfig:\n  xyChart:\n    width: ${value}\n    height: ${value}\n    showDataLabel: true\n---\nxychart\n  x-axis x [a, b]\n  bar "s" [1, 2]`;
      expect(check(src), value).toBe(true);
    }
  }, 60_000);

  it('holds for mutated inputs', () => {
    const fragments = [
      ...PAYLOADS,
      '"',
      '[',
      ']',
      ',',
      '\n',
      ' --> ',
      'line ',
      'bar ',
      'x-axis ',
      'y-axis ',
      'title ',
      '-',
      '.',
      '9999999999',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('does not pollute prototypes', () => {
    render(
      'xychart\n  x-axis "__proto__" ["__proto__", "constructor", "toString"]\n  bar "__proto__" [1, 2, 3]\n  line "constructor" [3 "__proto__", 2, 1]',
      options
    );
    render(
      '---\nconfig:\n  xyChart:\n    __proto__:\n      polluted: true\n    constructor: 1\n---\nxychart\n  line [1, 2]',
      options
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
  });
});
