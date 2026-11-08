import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

// A host with an icon for every name, so that each name a diagram gives reaches the output.
const options = { measurer: metricsMeasurer, icons: () => '<path d="M0,0"/>' };

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
  (p) => `mindmap\n  ${p}\n    child`,
  (p) => `mindmap\n  root\n    ${p}`,
  (p) => `mindmap\n  root[${p}]`,
  (p) => `mindmap\n  root\n    id(${p})`,
  (p) => `mindmap\n  root\n    id((${p}))`,
  (p) => `mindmap\n  root\n    id))${p}((`,
  (p) => `mindmap\n  root\n    id)${p}(`,
  (p) => `mindmap\n  root\n    id{{${p}}}`,
  (p) => `mindmap\n  root\n    ${p}[text]`,
  (p) => `mindmap\n  root\n    ${p}((text))`,
  (p) => `mindmap\n  root["${p}"]\n    ["${p}"]`,
  (p) => `mindmap\n  root["\`${p}\`"]\n    a["\`**${p}**\n${p}\`"]`,
  (p) => `mindmap\n  root\n    a\n    ::icon(${p})`,
  (p) => `mindmap\n  root\n  ::icon(${p})\n    a(b)\n    ::icon(${p})`,
  (p) => `mindmap\n  root\n    a\n    :::${p}`,
  (p) => `mindmap\n  root\n  :::${p}\n    a[b]\n    :::${p} ${p}`,
  (p) => `mindmap\n  root\n    a %% ${p}\n    b(c) %% ${p}`,
  (p) => `mindmap\n  root\n    fa:fa-${p} x`,
  (p) => `mindmap ${p}\n  root`,
  (p) => `---\ntitle: "${p}"\n---\nmindmap\n  root\n    a`,
  (p) => `---\nconfig:\n  mindmap:\n    maxNodeWidth: "${p}"\n    padding: "${p}"\n---\nmindmap\n  root\n    a`,
  (p) => `%%{init: {"mindmap": {"maxNodeWidth": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\nmindmap\n  root`,
];

describe('inert mindmap output', () => {
  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of loadCorpus('mindmap', /mindmap/)) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(30);
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

  it('escapes text, ids, icons and classes', () => {
    const { svg } = render('mindmap\n  a<b>"&[<i onload=x>"&]\n  ::icon(x" onload="y)\n  :::a"b <c> d=e', options);
    expect(svg).toContain('data-id="a&lt;b&gt;&quot;&amp;"');
    expect(svg).toContain('data-icon="x&quot; onload=&quot;y"');
    expect(svg).toContain('class="pele-node pele-shape-rect pele-root a b c d e"');
    expect(svg).not.toContain('<i');
  });

  it('holds for hostile option values and icon names', () => {
    for (const payload of PAYLOADS) {
      expect(check('mindmap\n  root\n    a\n    ::icon(x)', { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('mindmap', /mindmap/);
    const fragments = [
      ...PAYLOADS,
      '"',
      '[',
      ']',
      '(',
      ')',
      '((',
      '))',
      '{{',
      '}}',
      '\n',
      '\n    ',
      ':::',
      '::icon(',
      '"`',
      '`"',
      '%%',
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
      'mindmap\n  __proto__\n    constructor\n    :::__proto__ constructor\n    ::icon(__proto__)\n    toString[hasOwnProperty]',
      options
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.keys(Object.prototype)).toEqual([]);
  });
});
