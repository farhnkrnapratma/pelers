import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer, now: new Date(2014, 0, 16, 12) };

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
  expect(svg, JSON.stringify(src).slice(0, 160)).not.toMatch(/NaN|Infinity/);
  return true;
}

const HEAD = 'gantt\n  dateFormat YYYY-MM-DD\n';
const TASKS = '  section S\n  A task :a1, 2014-01-01, 30d\n  Another :after a1, 20d\n';

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `${HEAD}  title ${p}\n${TASKS}`,
  (p) => `${HEAD}  section ${p}\n  A task :a1, 2014-01-01, 30d\n`,
  (p) => `${HEAD}  ${p} :a1, 2014-01-01, 30d\n`,
  (p) => `${HEAD}  ${p} :a1, 2014-01-01, 1h\n  B :b, 2014-01-01, 30d\n`,
  (p) => `${HEAD}  A task :${p}, 2014-01-01, 30d\n`,
  (p) => `${HEAD}  A task :a1, ${p}, 30d\n`,
  (p) => `${HEAD}  A task :a1, 2014-01-01, ${p}\n`,
  (p) => `${HEAD}  A task :${p}\n`,
  (p) => `${HEAD}  A task :a1, after ${p}, 3d\n  B :b, 2014-01-01, until ${p}\n`,
  (p) => `${HEAD}  ${p} :milestone, m1, 2014-01-01, 0d\n  B :b, 2014-01-01, 30d\n`,
  (p) => `${HEAD}  ${p} :vert, v1, 2014-01-05, 0d\n  B :b, 2014-01-01, 30d\n`,
  (p) => `${HEAD}  A :${p}, a1, 2014-01-01, 30d\n`,
  (p) => `gantt\n  dateFormat ${p}\n${TASKS}`,
  (p) => `gantt\n  dateFormat [${p}]YYYY-MM-DD\n  A :a, ${p}2014-01-01, 3d\n`,
  (p) => `${HEAD}  axisFormat ${p}\n${TASKS}`,
  (p) => `${HEAD}  axisFormat %Y ${p} %c %%\n${TASKS}`,
  (p) => `${HEAD}  tickInterval ${p}\n${TASKS}`,
  (p) => `${HEAD}  excludes ${p}\n${TASKS}`,
  (p) => `${HEAD}  excludes weekends\n  includes ${p}\n${TASKS}`,
  (p) => `${HEAD}  todayMarker ${p}\n${TASKS}`,
  (p) => `${HEAD}  todayMarker stroke:${p},stroke-width:${p},opacity:${p}\n${TASKS}`,
  (p) => `${HEAD}  todayMarker ${p}:red\n${TASKS}`,
  (p) => `${HEAD}  weekday ${p}\n${TASKS}`,
  (p) => `${HEAD}  weekend ${p}\n${TASKS}`,
  (p) => `${HEAD}${TASKS}  click a1 href "${p}"\n`,
  (p) => `${HEAD}${TASKS}  click a1 call ${p}()\n`,
  (p) => `${HEAD}${TASKS}  click a1 call f(${p})\n`,
  (p) => `${HEAD}${TASKS}  click a1 call f("${p}") href "${p}"\n`,
  (p) => `${HEAD}${TASKS}  click ${p} href "https://example.com"\n`,
  (p) => `${HEAD}  A :${p}, 2014-01-01, 3d\n  click ${p} href "https://example.com"\n`,
  (p) => `${HEAD}  accTitle: ${p}\n  accDescr: ${p}\n${TASKS}`,
  (p) => `${HEAD}  accDescr {\n ${p}\n }\n${TASKS}`,
  (p) => `---\ntitle: "${p}"\ndisplayMode: "${p}"\n---\n${HEAD}${TASKS}`,
  (p) =>
    `---\nconfig:\n  gantt:\n    axisFormat: "${p}"\n    tickInterval: "${p}"\n    weekday: "${p}"\n    displayMode: "${p}"\n---\n${HEAD}${TASKS}`,
  (p) =>
    `---\nconfig:\n  gantt:\n    barHeight: "${p}"\n    barGap: "${p}"\n    fontSize: "${p}"\n    sectionFontSize: "${p}"\n    leftPadding: "${p}"\n    useWidth: "${p}"\n    numberSectionStyles: "${p}"\n---\n${HEAD}${TASKS}`,
  (p) =>
    `%%{init: {"gantt": {"axisFormat": "${p}", "topAxis": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\n${HEAD}${TASKS}`,
  (p) => `gantt ${p}\n${TASKS}`,
  (p) => `${HEAD}  %% ${p}\n${TASKS}`,
];

describe('inert gantt output', () => {
  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of loadCorpus('gantt', /gantt/)) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(20);
  });

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(500);
  }, 120_000);

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(check(`${HEAD}  accTitle: t\n${TASKS}`, { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('holds for hostile times', () => {
    for (const now of [NaN, Infinity, -Infinity, 8.64e15, -8.64e15, 0, new Date(NaN)]) {
      check(`${HEAD}${TASKS}`, { now });
      check('gantt\n  dateFormat HH:mm\n  A :a, 17:49, 2m\n  B :after nothing, 3m\n  C :until nothing\n', { now });
      check('gantt\n  A :3d\n', { now });
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('gantt', /gantt/);
    const fragments = [
      ...PAYLOADS,
      '"',
      ':',
      ',',
      ';',
      '#',
      '\n',
      '%',
      ' :a, 2014-01-01, 3d\n',
      'click a1 href "',
      'click a1 call ',
      'todayMarker ',
      'axisFormat ',
      'dateFormat ',
      'excludes ',
      'section ',
      'title ',
      'after ',
      'until ',
      'milestone, ',
      'vert, ',
      'crit, ',
      '99999999d',
      '0001-01-01',
      '275760-09-13',
      '-1',
      '1e400',
      'tickInterval 1millisecond\n',
      'topAxis\n',
      'x',
      'X',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('never links to a script URL', () => {
    for (const url of [
      'javascript:alert(1)',
      'JaVaScRiPt:alert(1)',
      ' javascript:alert(1)',
      'data:text/html,x',
      'vbscript:x',
    ]) {
      const { svg, links } = render(`${HEAD}${TASKS}  click a1 href "${url}"\n`, options);
      expect(svg).not.toMatch(/<a\b/);
      expect(links).toEqual([]);
    }
  });

  it('does not pollute prototypes', () => {
    render(
      `${HEAD}  A :__proto__, 2014-01-01, 3d\n  B :constructor, after __proto__ toString, 2d\n  section __proto__\n  C :hasOwnProperty, 2014-01-02, until constructor\n  click __proto__,constructor href "https://example.com"\n  click toString call __proto__(constructor)\n`,
      options
    );
    render(
      '---\nconfig:\n  gantt:\n    __proto__:\n      polluted: 1\n    constructor: 2\n---\n' + HEAD + TASKS,
      options
    );
    render(`${HEAD}  excludes __proto__ constructor\n  includes toString\n${TASKS}`, options);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, 'a1')).toBe(false);
  });
});
