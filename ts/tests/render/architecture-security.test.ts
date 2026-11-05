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

const head = 'architecture-beta\n';
const TEMPLATES: ((p: string) => string)[] = [
  (p) => `${head}service a(server)[${p}]`,
  (p) => `${head}service a(server)["${p}"]`,
  (p) => `${head}service a(server)['${p}']`,
  (p) => `${head}service a "${p}"[A]`,
  (p) => `${head}service a '${p}'`,
  (p) => `${head}service a(${p})[A]`,
  (p) => `${head}service ${p}`,
  (p) => `${head}group g(cloud)[${p}]\nservice a in g`,
  (p) => `${head}group g(cloud)["${p}"]\nservice a[${p}] in g`,
  (p) => `${head}group g(${p})[G]`,
  (p) => `${head}group ${p}`,
  (p) => `${head}group g\nservice a in ${p}`,
  (p) => `${head}junction ${p}`,
  (p) => `${head}service a\nservice b\na:R -[${p}]-> L:b`,
  (p) => `${head}service a\nservice b\na:R -["${p}"]- L:b`,
  (p) => `${head}service a\nservice b\na:B <-['${p}']-> T:b`,
  (p) => `${head}service a\nservice b\na:${p} -- L:b`,
  (p) => `${head}service a\nservice b\n${p}:R -- L:b`,
  (p) => `${head}service a\nservice b\na${p}:R -- L:b{group}`,
  (p) => `${head}service a\nservice b\nalign row a ${p}`,
  (p) => `${head}service a\nservice b\nalign ${p} a b`,
  (p) => `${head}title ${p}\nservice a`,
  (p) => `${head}accTitle: ${p}\naccDescr: ${p}\nservice a`,
  (p) => `${head}accDescr {\n ${p}\n }\nservice a`,
  (p) => `architecture-beta ${p}\nservice a`,
  (p) => `${head}%% ${p}\nservice a`,
  (p) =>
    `---\ntitle: "${p}"\nconfig:\n  architecture:\n    iconSize: "${p}"\n    fontSize: "${p}"\n    padding: "${p}"\n---\n${head}group g[G]\nservice a[A] in g`,
  (p) =>
    `%%{init: {"architecture": {"iconSize": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\n${head}service a[A]`,
  (p) => `---\nconfig:\n  architecture: "${p}"\n---\n${head}service a[A]`,
];

describe('inert architecture output', () => {
  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of loadCorpus('architecture', /architecture/)) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(15);
  });

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(300);
  });

  it('holds for hostile option values', () => {
    const src = `${head}accTitle: t\ngroup g(cloud)[G]\nservice a(server)[A] in g\nservice b "x"[B]\na:R -[l]-> L:b`;
    for (const payload of PAYLOADS) {
      expect(check(src, { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('holds for extreme numeric settings', () => {
    for (const value of ['0', '-1', '1e308', '.inf', '.nan', '1e-9', '99999999', '[1]', '{a: 1}', 'null', 'true']) {
      const src = `---\nconfig:\n  architecture:\n    iconSize: ${value}\n    fontSize: ${value}\n    padding: ${value}\n---\n${head}group g[G]\nservice a "t"[A] in g\nservice b[B]\na:R -[l]-> L:b`;
      expect(check(src)).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('architecture', /architecture/);
    const fragments = [
      ...PAYLOADS,
      '"',
      '[',
      ']',
      '(',
      ')',
      '\n',
      ' in ',
      '{group}',
      ':R --> L:',
      'service ',
      'group ',
      'junction ',
      'title ',
      '-[',
      ']-',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('treats ids that name object internals as plain ids', () => {
    const src = `${head}group __proto__(cloud)[P]\ngroup constructor[C] in __proto__\nservice toString(server)[A] in __proto__\nservice hasOwnProperty[B] in constructor\njunction valueOf\ntoString:R -- L:hasOwnProperty\nhasOwnProperty:B -- T:valueOf\nalign row toString hasOwnProperty`;
    const { svg } = render(src, options);
    expect(svg.match(/class="pele-cluster"/g)?.length).toBe(2);
    expect(svg.match(/class="pele-node /g)?.length).toBe(3);
    expect(svg).toContain('data-id="__proto__"');
    expect(Object.prototype).not.toHaveProperty('polluted');
    expect(Object.keys(Object.prototype)).toHaveLength(0);
  });
});
