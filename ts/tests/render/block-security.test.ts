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
  expect(svg, JSON.stringify(src).slice(0, 160)).not.toMatch(/="[^"]*(?:NaN|Infinity)/);
  return true;
}

// One template for each place the block grammar accepts text.
const TEMPLATES: ((p: string) => string)[] = [
  (p) => `block\n  ${p}`,
  (p) => `block\n  a ${p} b`,
  (p) => `block\n  a["${p}"]`,
  (p) => `block\n  a("${p}") b(["${p}"]) c[["${p}"]] d[("${p}")]`,
  (p) => `block\n  a(("${p}")) b((("${p}"))) c>"${p}"] d{"${p}"} e{{"${p}"}}`,
  (p) => `block\n  a[/"${p}"/] b[\\"${p}"\\] c[/"${p}"\\] d[\\"${p}"/]`,
  (p) => `block\n  a<["${p}"]>(right) b<["${p}"]>(x, y)`,
  (p) => `block\n  a<["x"]>(${p})`,
  (p) => `block\n  a["\`${p}\`"]`,
  (p) => `block\n  block:g["${p}"]\n    a\n  end`,
  (p) => `block\n  block:${p}\n    a\n  end`,
  (p) => `block\n  block:g:${p}\n    a\n  end`,
  (p) => `block\n  a space b\n  a -- "${p}" --> b`,
  (p) => `block\n  a space b\n  a == "${p}" ==> b\n  b -. "${p}" .-> a\n  a -- "${p}" --> a`,
  (p) => `block\n  a space b\n  a ${p} b`,
  (p) => `block\n  a:${p} b`,
  (p) => `block\n  a space:${p} b`,
  (p) => `block\n  columns ${p}\n  a b`,
  (p) => `block\n  a b\n  class a ${p}`,
  (p) => `block\n  a b\n  class ${p} hot`,
  (p) => `block\n  a b\n  classDef ${p} fill:red\n  class a ${p}`,
  (p) => `block\n  a b\n  classDef hot fill:${p}\n  class a hot`,
  (p) => `block\n  a b\n  classDef hot ${p}\n  class a,b hot`,
  (p) => `block\n  a b\n  classDef hot color:${p},stroke:${p},font-family:${p},font-size:${p}\n  class a hot`,
  (p) => `block\n  a b\n  classDef default ${p}:red,fill:${p}`,
  (p) => `block\n  a b\n  style a fill:${p}`,
  (p) => `block\n  a b\n  style a ${p}`,
  (p) => `block\n  a b\n  style a ${p}:red;${p}`,
  (p) => `block\n  a b\n  style a color:${p},stroke:${p},stroke-dasharray:${p}`,
  (p) => `block\n  a b\n  style ${p} fill:red`,
  (p) => `block\n  block:g\n    a\n  end\n  style g fill:${p},color:${p}\n  class g ${p}`,
  (p) => `block\n  a["fa:fa-${p} x"]`,
  (p) => `block\n  a\n  accDescr {\n ${p}\n }`,
  (p) => `block\n  accTitle: ${p}\n  a`,
  (p) => `block\n  %% ${p}\n  a`,
  (p) => `---\ntitle: "${p}"\nconfig:\n  block:\n    padding: "${p}"\n---\nblock\n  a b`,
  (p) => `%%{init: {"block": {"padding": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\nblock\n  a b`,
  (p) => `block ${p}\n  a`,
  (p) => `block-beta\n  ${p}["${p}"] --> ${p}`,
];

describe('inert block output', () => {
  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of loadCorpus('block', /block/)) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(60);
  });

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    const never: string[] = [];
    for (const template of TEMPLATES) {
      const before = rendered;
      for (const payload of PAYLOADS) if (check(template(payload))) rendered++;
      if (rendered === before) never.push(template('#'));
    }
    expect(rendered).toBeGreaterThan(700);
    // These positions take only digits or direction words, or hold text the grammar always rejects.
    expect(never).toEqual([
      'block\n  a<["x"]>(#)',
      'block\n  a["`#`"]',
      'block\n  block:g:#\n    a\n  end',
      'block\n  a:# b',
      'block\n  a space:# b',
      'block\n  a\n  accDescr {\n #\n }',
      'block\n  accTitle: #\n  a',
    ]);
  }, 120_000);

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(check('block\n  a space b\n  a -- "x" --> b', { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('block', /block/);
    const fragments = [
      ...PAYLOADS,
      '"',
      '["',
      '"]',
      '<["',
      '"]>(right)',
      '\n',
      ':',
      ' --> ',
      ' -- "',
      '" --> ',
      'style a fill:',
      'classDef a ',
      'class a ',
      'block:',
      'end',
      'space:9',
      'columns 2',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    let rendered = 0;
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000 && check(src)) rendered++;
    }
    expect(rendered).toBeGreaterThan(count / 20);
  }, 600_000);

  it('keeps coordinates finite when blocks have no size', () => {
    const zero = '---\nconfig:\n  block:\n    padding: 0\n---\n';
    for (const body of [
      'block\n  columns 1\n  block:g\n    classDef x fill:red\n  end\n  a\n  g --> a\n  a --> g\n  g --> g',
      'block\n  block:g\n    space\n  end\n  block:h\n    space\n  end\n  g --> h\n  h -- "x" --> g',
      'block\n  columns 1\n  block:g\n    classDef x fill:red\n  end\n  block:h\n    classDef x fill:red\n  end\n  g --> h',
      'block\n  space\n  block:g\n    space:0\n  end\n  g --> g',
    ]) {
      expect(check(zero + body), body).toBe(true);
      expect(check(body), body).toBe(true);
    }
  });

  it('keeps hostile text out of markup', () => {
    const { svg } = render(
      'block\n  a["</text><script>alert(1)</script>"]\n  classDef x fill:url(javascript:alert(1)),stroke:red\n  class a x',
      options
    );
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('url(');
    expect(svg).toContain('stroke:red');
  });

  it('does not pollute prototypes', () => {
    render(
      'block\n  __proto__ constructor toString\n  classDef __proto__ fill:red\n  class constructor,__proto__ __proto__\n  style __proto__ fill:blue\n  __proto__ --> constructor',
      options
    );
    render('block\n  class __proto__ polluted\n  style constructor fill:red\n  a', options);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(({} as Record<string, unknown>).classes).toBeUndefined();
    expect(({} as Record<string, unknown>).styles).toBeUndefined();
  });
});
