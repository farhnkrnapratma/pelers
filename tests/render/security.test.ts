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
  (p) => `flowchart TD\n  A[${p}] --> B`,
  (p) => `flowchart TD\n  A["${p}"] --> B`,
  (p) => `flowchart TD\n  A["\`${p}\`"] --> B`,
  (p) => `flowchart TD\n  A(${p}) --> B{${p}}`,
  (p) => `flowchart TD\n  ${p} --> B`,
  (p) => `flowchart TD\n  A -->|${p}| B`,
  (p) => `flowchart TD\n  A -- ${p} --> B`,
  (p) => `flowchart TD\n  A -- "${p}" --> B`,
  (p) => `flowchart TD\n  subgraph ${p}\n  A\n  end`,
  (p) => `flowchart TD\n  subgraph s ["${p}"]\n  A\n  end`,
  (p) => `flowchart TD\n  A:::${p} --> B`,
  (p) => `flowchart TD\n  A --> B\n  class A ${p}`,
  (p) => `flowchart TD\n  A --> B\n  classDef ${p} fill:red\n  class A ${p}`,
  (p) => `flowchart TD\n  A --> B\n  classDef c fill:${p}\n  class A c`,
  (p) => `flowchart TD\n  A --> B\n  style A fill:${p}`,
  (p) => `flowchart TD\n  A --> B\n  style A ${p}:red`,
  (p) => `flowchart TD\n  A --> B\n  style A color:${p},stroke:${p},font-family:${p}`,
  (p) => `flowchart TD\n  A --> B\n  linkStyle 0 stroke:${p}`,
  (p) => `flowchart TD\n  A --> B\n  click A "${p}"`,
  (p) => `flowchart TD\n  A --> B\n  click A href "${p}" "${p}" _blank`,
  (p) => `flowchart TD\n  A --> B\n  click A "x" "${p}"`,
  (p) => `flowchart TD\n  A --> B\n  click A call ${p}()`,
  (p) => `flowchart TD\n  A --> B\n  click A "https://example.com" ${p}`,
  (p) => `flowchart TD\n  A["${p}"] --> B\n  class A internal-link`,
  (p) => `flowchart TD\n  A[${p}]:::internal-link --> B`,
  (p) => `flowchart TD\n  accTitle: ${p}\n  accDescr: ${p}\n  A --> B`,
  (p) => `flowchart TD\n  accDescr {\n ${p}\n }\n  A --> B`,
  (p) => `flowchart TD\n  A@{ label: "${p}" } --> B`,
  (p) => `flowchart TD\n  A@{ shape: ${p} } --> B`,
  (p) => `flowchart TD\n  A@{ icon: "${p}", form: "${p}", pos: "${p}", label: "x" } --> B`,
  (p) => `flowchart TD\n  A@{ img: "${p}", label: "x", w: "${p}", h: "${p}", constraint: "${p}" } --> B`,
  (p) => `flowchart TD\n  A@{ ${p}: 1 } --> B`,
  (p) => `flowchart TD\n  A e1@--> B\n  e1@{ animate: "${p}", curve: "${p}" }`,
  (p) => `flowchart TD\n  A[fa:fa-${p} x] --> B`,
  (p) => `---\ntitle: "${p}"\nconfig:\n  flowchart:\n    curve: "${p}"\n---\nflowchart TD\n  A --> B`,
  (p) =>
    `%%{init: {"flowchart": {"curve": "${p}"}, "themeVariables": {"fontFamily": "${p}"}}}%%\nflowchart TD\n  A --> B`,
  (p) => `flowchart ${p}\n  A --> B`,
  (p) => `flowchart TD\n  %% ${p}\n  A --> B`,
];

describe('inert output', () => {
  it('holds for every corpus input', () => {
    for (const src of loadCorpus('flowchart', /graph|flowchart/)) check(src);
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
    for (const payload of PAYLOADS) {
      expect(check('flowchart TD\n  accTitle: t\n  A --> B', { idPrefix: payload, fontFamily: payload })).toBe(true);
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('flowchart', /graph|flowchart/);
    const next = mutator(
      corpus,
      [...PAYLOADS, '"', '[', ']', '|', '\n', ':::', 'click A "', 'style A fill:', '@{ label: "', ' }'],
      random(3)
    );
    const count = Number(process.env.FUZZ ?? 4000);
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) check(src);
    }
  }, 600_000);

  it('never gives an unsafe href to an internal link', () => {
    const { svg } = render('flowchart TD\n  A["javascript:alert(1)"]:::internal-link', options);
    expect(svg).toContain('<a class="internal-link" data-href="javascript:alert(1)">');
    for (const type of ['flowchart TD', 'agentflow-beta TB', 'erDiagram']) {
      const allowed = render(`${type}\n  A["javascript:alert(1)"]:::internal-link`, {
        ...options,
        linkSchemes: ['about'],
      }).svg;
      expect(allowed).not.toContain(' href=');
    }
  });

  it('does not pollute prototypes', () => {
    render('flowchart TD\n  A@{ __proto__: [polluted], constructor: 1, label: x } --> B', options);
    render('flowchart TD\n  __proto__ --> constructor --> toString\n  class __proto__ constructor', options);
    render('flowchart TD\n  A[|__proto__:x|text] --> B', options);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, 'x')).toBe(false);
  });
});
