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

const I = 'ishikawa-beta\n';

// The syntax has one kind of text, at any depth. These put a payload at each depth, alone on its
// line and beside other text, and in the places around the diagram that take text.
const TEMPLATES: ((p: string) => string)[] = [
  (p) => `${I}${p}`,
  (p) => `${I}  ${p}`,
  (p) => `${I}  Effect\n    ${p}`,
  (p) => `${I}  Effect\n    Category\n      ${p}`,
  (p) => `${I}  Effect\n    Category\n      Cause\n        ${p}`,
  (p) => `${I}  Effect\n    Category\n      Cause\n        Sub\n          ${p}\n            ${p}`,
  (p) => `${I}  ${p}\n    ${p}\n      ${p}\n    ${p}\n      ${p}\n        ${p}`,
  (p) => `${I}  Effect ${p}\n    Category ${p}\n      Cause ${p} more`,
  (p) => `${I}  Effect\n    A\n    B\n    ${p}\n    C`,
  (p) => `${I}  Effect\n%% ${p}\n    Category`,
  (p) => `${I}  Effect\n    Category %% ${p}`,
  (p) => `ishikawa ${p}`,
  (p) => `ishikawa-beta ${p}\n  Cause`,
  (p) => `---\ntitle: "${p}"\n---\n${I}  Effect\n    Cause`,
  (p) => `---\ntitle: ${p}\nconfig:\n  ishikawa:\n    diagramPadding: "${p}"\n---\n${I}  Effect`,
  (p) =>
    `%%{init: {"ishikawa": {"useMaxWidth": "${p}"}, "themeVariables": {"lineColor": "${p}"}}}%%\n${I}  Effect\n    Cause`,
];

describe('inert Ishikawa output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('ishikawa', /ishikawa/i)) check(src);
  });

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(450);
  });

  it('escapes what it shows', () => {
    const { svg } = render(
      `${I}  <img src=x onerror=alert(1)>\n    <script>a&b</script>\n      </text></g></svg><script>alert(1)</script>\n        " onload="alert(1)\n`,
      options
    );
    expect(svg).not.toContain('<img');
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('onload="alert');
    expect(svg).toContain('data-id="&lt;img src=x onerror=alert(1)&gt;"');
    expect(svg).toContain('data-id="&lt;script&gt;a&amp;b&lt;/script&gt;"');
    expect(svg).toContain('&quot; onload=&quot;alert(1)');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(check(`${I}  Effect\n    Category\n      Cause`, { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('ishikawa', /ishikawa/i).filter((src) => /^\s*ishikawa/i.test(src));
    const fragments = [...PAYLOADS, '\n', '\n  ', '\n      ', '\t', ' ', '%%', '<br>', '#35;', 'ishikawa'];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('does not pollute prototypes', () => {
    render(`${I}  __proto__\n    constructor\n      toString\n        hasOwnProperty\n    __proto__\n`, options);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.keys(Object.prototype)).toEqual([]);
  });
});
