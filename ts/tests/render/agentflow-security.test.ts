import { describe, expect, it } from 'vitest';
import { PelersError, parse, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };
const H = 'agentflow-beta TB\n';

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
  expect(svg, JSON.stringify(src).slice(0, 160)).not.toMatch(
    / (?:d|transform|points|x|y|width|height|viewBox)="[^"]*(?:NaN|Infinity|undefined)/
  );
  return true;
}

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `${H}  ${p}`,
  (p) => `${H}  ${p} --> b`,
  (p) => `${H}  a --> ${p}`,
  (p) => `${H}  a[${p}]`,
  (p) => `${H}  a["${p}"]`,
  (p) => `${H}  a["\`${p}\`"]`,
  (p) => `${H}  a(${p})`,
  (p) => `${H}  a{${p}}`,
  (p) => `${H}  a{"${p}"}`,
  (p) => `${H}  a{{${p}}}`,
  (p) => `${H}  a[[${p}]]`,
  (p) => `${H}  a[/${p}/]`,
  (p) => `${H}  a[\\${p}\\]`,
  (p) => `${H}  a(-${p}-)`,
  (p) => `${H}  a>${p}]`,
  (p) => `${H}  a((${p}))`,
  (p) => `${H}  a([${p}])`,
  (p) => `${H}  a[|${p}:${p}|x]`,
  (p) => `${H}  a -- ${p} --> b`,
  (p) => `${H}  a -- "${p}" --> b`,
  (p) => `${H}  a -->|${p}| b`,
  (p) => `${H}  a -->|"${p}"| b`,
  (p) => `${H}  a --x|"${p}"| a`,
  (p) => `${H}  a ${p}@--> b`,
  (p) => `${H}  a e1@--> b\n  e1@{ curve: "${p}", animate: "${p}", animation: "${p}", instruction: "${p}" }`,
  (p) => `${H}  a@{ label: "${p}" }`,
  (p) => `${H}  a@{ label: "${p}", labelType: "${p}" }`,
  (p) => `${H}  a@{ shape: ${p} }`,
  (p) => `${H}  a@{ shape: "${p}" }`,
  (p) => `${H}  a@{ "${p}": "${p}" }`,
  (p) => `${H}  a@{ ${p} }`,
  (p) => `${H}  a@{\n    ${p}\n  }`,
  (p) => `${H}  a@{\n    label: |\n      ${p}\n  }`,
  (p) => `${H}  a["A"]@{ shape: tool, icon: "${p}", img: "${p}", form: "${p}", pos: "${p}", w: "${p}", h: "${p}" }`,
  (p) => `${H}  a@{ shape: tool, class: "${p}", style: "${p}", view: "${p}" }`,
  (p) => `${H}  flow ${p}\n  a\n  end`,
  (p) => `${H}  flow "${p}"\n  a\n  end`,
  (p) => `${H}  flow f[${p}]\n  a\n  end`,
  (p) => `${H}  flow f["${p}"]\n  a --> f\n  end\n  f --> b`,
  (p) => `${H}  flow f["${p}"]@{ view: collapsed, model: "${p}" }\n  a\n  end\n  a --> b`,
  (p) => `${H}  flow f\n  a\n  end\n  f@{ view: "${p}", "${p}": 1 }`,
  (p) => `${H}  flow f\n  a\n  end\n  style f fill:${p}\n  class f ${p}`,
  (p) => `${H}  flow f\n  direction ${p}\n  a\n  end`,
  (p) => `${H}  connector ${p}`,
  (p) => `${H}  connector "${p}"`,
  (p) => `${H}  connector c[${p}]`,
  (p) => `${H}  connector c["${p}"]@{ protocol: "${p}", "${p}": "${p}" }\n  c --> a`,
  (p) => `${H}  global\n  ${p}\n  a["${p}"]\n  end\n  flow f\n  a\n  end`,
  (p) => `${H}  a:::${p}`,
  (p) => `${H}  a:::${p} --> b:::${p}`,
  (p) => `${H}  a\n  class a ${p}`,
  (p) => `${H}  a\n  class ${p} x`,
  (p) => `${H}  a\n  classDef ${p} fill:red\n  class a ${p}`,
  (p) => `${H}  a\n  classDef c fill:${p}\n  class a c`,
  (p) => `${H}  a\n  classDef c ${p}:red\n  class a c`,
  (p) => `${H}  a\n  classDef default fill:${p},color:${p},stroke:${p}\n`,
  (p) => `${H}  a\n  style a fill:${p}\n`,
  (p) => `${H}  a\n  style a ${p}:red\n`,
  (p) => `${H}  a\n  style a color:${p},stroke:${p},font-family:${p},font-size:${p}\n`,
  (p) => `${H}  a\n  style ${p} fill:red\n`,
  (p) => `${H}  a --> b\n  linkStyle 0 stroke:${p},fill:${p}\n`,
  (p) => `${H}  a --> b\n  linkStyle default ${p}:${p}\n`,
  (p) => `${H}  a --> b\n  linkStyle 0 interpolate ${p}\n`,
  (p) => `${H}  a --> b\n  linkStyle default interpolate ${p} stroke:${p}\n`,
  (p) => `${H}  a\n  click a "${p}"\n`,
  (p) => `${H}  a\n  click a href "${p}"\n`,
  (p) => `${H}  a\n  click a href "${p}" "${p}" _blank\n`,
  (p) => `${H}  a\n  click a "https://example.com" "${p}"\n`,
  (p) => `${H}  a\n  click a call ${p}()\n`,
  (p) => `${H}  a\n  click a call cb("${p}") "${p}"\n`,
  (p) => `${H}  a\n  click a ${p}\n`,
  (p) => `${H}  a\n  click ${p} href "https://example.com"\n`,
  (p) => `${H}  a["${p}"]:::internal-link`,
  (p) => `${H}  ${p}:::internal-link`,
  (p) => `${H}  accTitle: ${p}\n  accDescr: ${p}\n  a`,
  (p) => `${H}  accDescr {\n ${p}\n }\n  a`,
  (p) => `agentflow-beta ${p}\n  a`,
  (p) => `${H}  direction ${p}\n  a`,
  (p) => `${H}  %% ${p}\n  a %% ${p}\n  b[x %% ${p}\n y]`,
  (p) =>
    `---\ntitle: "${p}"\nconfig:\n  agentflow:\n    nodeSpacing: "${p}"\n    rankSpacing: "${p}"\n    wrappingWidth: "${p}"\n  flowchart:\n    curve: "${p}"\n    inheritDir: "${p}"\n---\n${H}  a --> b`,
  (p) => `---\ntitle: ${p}\n---\n${H}  a`,
  (p) =>
    `%%{init: {"agentflow": {"nodeSpacing": "${p}"}, "flowchart": {"curve": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\n${H}  a --> b`,
];

describe('inert agentflow output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('agentflow', /agentflow-beta/)) check(src);
  });

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(900);
  });

  it('escapes what it shows', () => {
    const { svg } = render(
      `${H}  a["<img src=x onerror=alert(1)>"] -- "</text><script>alert(1)</script>" --> b["a&b"]\n  flow f["<svg onload=alert(1)>"]\n   c\n  end`,
      options
    );
    expect(svg).not.toContain('<img');
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('<svg onload');
    expect(svg).toContain('a&amp;b');
  });

  it('escapes ids in attributes', () => {
    const { svg } = render(`${H}  flow q&r\n  a\n  end\n  connector 'x&y'`, options);
    expect(svg).toContain('data-id="q&amp;r"');
    expect(svg).toContain(`data-id="'x&amp;y'"`);
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(
        check(`${H}  accTitle: t\n  flow f["F"]\n  a --> b\n  end`, { idPrefix: payload, fontFamily: payload })
      ).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('agentflow', /agentflow-beta/);
    const fragments = [
      ...PAYLOADS,
      '"',
      '`',
      '{',
      '}',
      '[',
      ']',
      '(',
      ')',
      '|',
      '\n',
      ':::',
      '-->',
      '--x',
      '-.-',
      ' -- ',
      '@{ ',
      'label: ',
      'shape: ',
      'view: collapsed',
      'style a fill:',
      'classDef c ',
      'click a href "',
      'flow ',
      'connector ',
      'global\n',
      'end\n',
      '%% ',
    ];
    const next = mutator(corpus, fragments, random(3));
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('never links to a script', () => {
    for (const url of [
      'javascript:alert(1)',
      ' JaVaScRiPt:alert(1)',
      'data:text/html,<script>alert(1)</script>',
      'vbscript:x',
    ]) {
      const { svg, links } = render(`${H}  a\n  click a href "${url}"`, options);
      expect(svg).not.toMatch(/href="(?:\s*javascript|data|vbscript)/i);
      for (const link of links) expect(link.href).not.toMatch(/^\s*(?:javascript|data|vbscript):/i);
    }
  });

  it('never gives an unsafe href to an internal link', () => {
    const { svg, links } = render(`${H}  a["javascript:alert(1)"]:::internal-link`, options);
    expect(svg).toContain('<a class="internal-link" data-href="javascript:alert(1)">');
    expect(links).toEqual([{ id: 'a', href: 'javascript:alert(1)', internal: true }]);
  });

  it('does not pollute prototypes', () => {
    render(
      `${H}  __proto__ --> constructor --> toString\n  flow __proto__[constructor]\n  hasOwnProperty\n  end\n  connector prototype["__proto__"]`,
      options
    );
    render(
      `${H}  a\n  classDef __proto__ fill:red\n  class a __proto__\n  class __proto__ constructor\n  style __proto__ fill:red\n  a:::__proto__`,
      options
    );
    render(
      `${H}  a@{ __proto__: 1, constructor: 2, prototype: 3 }\n  a@{\n    __proto__:\n      polluted: true\n    constructor:\n      prototype:\n        polluted: true\n  }`,
      options
    );
    render(
      `${H}  a[|__proto__:polluted|x]\n  global\n  __proto__\n  end\n  flow f@{ __proto__: 1 }\n  __proto__\n  end`,
      options
    );
    const model = parse(`${H}  a@{ shape: constructor }`);
    expect(model.type === 'agentflow' && model.diagnostics.map((d) => d.id)).toEqual(['SHAPE_UNSUPPORTED']);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.keys(Object.prototype)).toEqual([]);
  });
});
