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

const U = 'usecase-beta\n';

// One template for every place the syntax takes text. A payload that holds the delimiter of its
// position ends the token early and is rejected, which is fine: it must not get through as markup.
const TEMPLATES: ((p: string) => string)[] = [
  (p) => `${U}${p}`,
  (p) => `${U}actor ${p}`,
  (p) => `${U}actor A(${p})`,
  (p) => `${U}actor A("${p}")`,
  (p) => `${U}actor A('${p}')`,
  (p) => `${U}actor A("\`${p}\`")`,
  (p) => `${U}actor "${p}"`,
  (p) => `${U}actor "\`${p}\`"`,
  (p) => `${U}actor A <<${p}>>`,
  (p) => `${U}actor A:::${p}`,
  (p) => `${U}actor A@{ icon: "${p}" }`,
  (p) => `${U}actor A@{ type: "${p}" }`,
  (p) => `${U}actor A@{ "${p}": true }`,
  (p) => `${U}actor A, "${p}"`,
  (p) => `${U}U(${p})`,
  (p) => `${U}U[${p}]`,
  (p) => `${U}U("${p}")`,
  (p) => `${U}U["\`${p}\`"]`,
  (p) => `${U}"${p}"`,
  (p) => `${U}'${p}'`,
  (p) => `${U}"\`${p}\`"`,
  (p) => `${U}U(x) <<${p}>>:::${p}`,
  (p) => `${U}U(x)@{ business: true } <<${p}>>`,
  (p) => `${U}A --> "${p}"`,
  (p) => `${U}"${p}" --> "${p} "`,
  (p) => `${U}A -- ${p} --> B`,
  (p) => `${U}A -- "${p}" --> B`,
  (p) => `${U}A -- "\`${p}\`" --> B`,
  (p) => `${U}A <-- "${p}" -- B`,
  (p) => `${U}A o-- "${p}" -- B\nA x-- '${p}' -- B`,
  (p) => `${U}A -- "${p}" --o B\nA -- "${p}" --x B`,
  (p) => `${U}A -- "${p}" --> A`,
  (p) => `${U}A ${p}@--> B`,
  (p) => `${U}A e@--> B\ne@{ animation: "${p}" }`,
  (p) => `${U}A ..> : include B\nA ..> : ${p} B`,
  (p) => `${U}A ${p} B`,
  (p) => `${U}systemBoundary ${p}\nend`,
  (p) => `${U}systemBoundary "${p}"\n  U\nend`,
  (p) => `${U}systemBoundary S[${p}]\n  U\nend`,
  (p) => `${U}systemBoundary S("\`${p}\`")@{ type: package }\n  U\nend`,
  (p) => `${U}systemBoundary S["${p}"]@{ type: package }:::${p}\n  actor "${p}"\n  "${p} "\nend`,
  (p) => `${U}systemBoundary S@{ type: "${p}" }\nend`,
  (p) => `${U}U\nnote for U ${p}`,
  (p) => `${U}U\nnote for U "${p}"`,
  (p) => `${U}U\nnote for U "\`${p}\`"`,
  (p) => `${U}U\nnote for ${p} "x"`,
  (p) => `${U}json P@{"${p}": "${p}"}`,
  (p) => `${U}json P@{"a": ["${p}", {"${p}": null}], "b": {"${p}": 1}}`,
  (p) => `${U}json P@{"k": ${JSON.stringify(p)}, ${JSON.stringify(p)}: [${JSON.stringify(p)}]}`,
  (p) => `${U}json ${p}@{}`,
  (p) => `${U}json P@{}:::${p}`,
  (p) => `${U}json P@${p}`,
  (p) => `${U}A\nclass A ${p}`,
  (p) => `${U}A\nclass ${p} c`,
  (p) => `${U}A\nclassDef ${p} fill:red\nclass A ${p}`,
  (p) => `${U}A\nclassDef c fill:${p}\nclass A c`,
  (p) => `${U}A\nclassDef c ${p}:red\nclass A c`,
  (p) => `${U}A\nclassDef default fill:${p},color:${p},stroke:${p}`,
  (p) => `${U}A\nstyle A fill:${p}`,
  (p) => `${U}A\nstyle A ${p}:red`,
  (p) => `${U}actor A\nstyle A color:${p},stroke:${p},font-family:${p},font-size:${p}`,
  (p) => `${U}A e@--> B\nstyle e stroke:${p},stroke-width:${p}\nclass e ${p}`,
  (p) => `${U}systemBoundary S\nend\nstyle S fill:${p}\nclass S ${p}`,
  (p) => `${U}json P@{}\nstyle P fill:${p},stroke-width:${p}`,
  (p) => `${U}A\nstyle ${p} fill:red`,
  (p) => `${U}accTitle: ${p}\naccDescr: ${p}\nA`,
  (p) => `${U}accDescr {\n ${p}\n}\nA`,
  (p) => `${U}direction ${p}\nA`,
  (p) => `${U}direction TB ${p}\nA`,
  (p) => `${U}%% ${p}\nA`,
  (p) => `${U}A@{ ${p}: true }\nA`,
  (p) => `${U}A\nA@{ business: "${p}" }`,
  (p) =>
    `---\ntitle: "${p}"\nconfig:\n  usecase:\n    nodeSpacing: "${p}"\n    rankSpacing: "${p}"\n---\n${U}actor A\nA --> B`,
  (p) => `---\ntitle: ${p}\n---\n${U}A`,
  (p) => `%%{init: {"usecase": {"nodeSpacing": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\n${U}A`,
  (p) => `usecase-beta ${p}\nA`,
];

describe('inert use case output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('usecase', /usecase-beta/)) check(src);
  });

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(600);
  });

  it('escapes what it shows', () => {
    const { svg } = render(
      `${U}actor "<img src=x onerror=alert(1)>" <<</text><script>alert(1)</script >>\n'<script>a&b</script>'\njson P@{"<b>": "</text><script>alert(1)</script>"}\nnote for P2 "<svg onload=alert(1)>"\nP2`,
      options
    );
    expect(svg).not.toContain('<img');
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('<svg onload');
    expect(svg).toContain('&lt;script&gt;a&amp;b&lt;/script&gt;');
    expect(svg).toContain('data-id="_img_src_x_onerror_alert_1__"');
    expect(svg).toContain('>&lt;b&gt;</text>');
    expect(svg).toContain('«&lt;/text&gt;&lt;script&gt;alert(1)&lt;/script»');
  });

  it('never runs or emits the icon name as markup', () => {
    const { svg } = render(`${U}actor A@{ icon: '"><script>alert(1)</script>' }`, options);
    expect(svg).toContain('data-icon="&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;"');
    expect(svg).not.toContain('<script');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(
        check(`${U}accTitle: t\nactor A\nA -- "l" --> B\njson P@{"a":1}`, { idPrefix: payload, fontFamily: payload })
      ).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('usecase', /usecase-beta/);
    const fragments = [
      ...PAYLOADS,
      '"',
      "'",
      '`',
      '"`',
      '`"',
      '{',
      '}',
      '[',
      ']',
      '(',
      ')',
      '\n',
      ':::',
      '<<',
      '>>',
      '@{',
      ' --> ',
      ' -- ',
      'style A fill:',
      'classDef c ',
      'systemBoundary ',
      'end\n',
      'json P@',
      'note for A ',
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
      `${U}actor __proto__\n__proto__ constructor@--> toString\nconstructor@{ animate: true }\nnote for toString "__proto__"`,
      options
    );
    render(`${U}A\nclassDef __proto__ fill:red\nclass A __proto__\nstyle A fill:red\nA@{ business: true }`, options);
    render(
      `${U}systemBoundary __proto__[constructor]\n  hasOwnProperty\n  actor valueOf\nend\n__proto__@{ type: package }`,
      options
    );
    render(
      `${U}json __proto__@{"__proto__": {"polluted": true}, "constructor": {"prototype": {"polluted": true}}}\nA --> __proto__`,
      options
    );
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.keys(Object.prototype)).toEqual([]);
  });

  it('shows a JSON key named __proto__ like any other', () => {
    const { svg } = render(`${U}json P@{"__proto__": {"x": 1}, "constructor": 2}`, options);
    expect(svg).toContain('>__proto__.x</text>');
    expect(svg).toContain('>constructor</text>');
  });
});
