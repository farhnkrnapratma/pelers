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
  (p) => `packet\n0-31: "${p}"`,
  (p) => `packet\n0-31: '${p}'`,
  (p) => `packet\n0: "${p}"\n1: "${p}"\n2-31: "ok"`,
  (p) => `packet\n+8: "${p}"\n+200: "${p}"`,
  (p) => `packet\ntitle ${p}\n0-31: "a"`,
  (p) => `packet title ${p}`,
  (p) => `packet\naccTitle: ${p}\naccDescr: ${p}\n0-31: "a"`,
  (p) => `packet\naccDescr {\n ${p}\n }\n0-31: "a"`,
  (p) => `packet\n0-31: "a" %% ${p}`,
  (p) => `packet\n${p}: "a"`,
  (p) => `packet\n0-${p}: "a"`,
  (p) => `packet ${p}\n0-31: "a"`,
  (p) => `---\ntitle: "${p}"\n---\npacket\n0-31: "a"`,
  (p) =>
    `---\nconfig:\n  packet:\n    bitOrder: "${p}"\n    bitsPerRow: "${p}"\n    showBits: "${p}"\n    rowHeight: "${p}"\n---\npacket\n0-31: "a"`,
  (p) =>
    `%%{init: {"packet": {"bitWidth": "${p}", "paddingX": "${p}"}, "themeVariables": {"packet": {"blockFillColor": "${p}"}}}}%%\npacket\n0-31: "a"`,
];

describe('inert packet output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('packet', /packet/)) check(src);
  });

  it('holds with hostile text in every position', { timeout: 60_000 }, () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(150);
  });

  it('escapes a label instead of reading it as markup', () => {
    const { svg } = render('packet\n0-31: "<b>&amp;</b> <script>x</script>"', options);
    expect(svg).toContain('&lt;b&gt;&amp;&lt;/b&gt; &lt;script&gt;x&lt;/script&gt;');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(check('packet\naccTitle: t\n0-31: "a"', { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('packet', /packet/);
    const next = mutator(corpus, [...PAYLOADS, '"', "'", ':', '-', '+', '\n', 'title ', '0-7: "', '+8: "'], random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);
});
