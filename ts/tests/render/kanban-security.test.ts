import { describe, expect, it } from 'vitest';
import { PelersError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

// A host with an icon for every name, so that each name a diagram gives reaches the output.
const options = { measurer: metricsMeasurer, icons: () => '<path d="M0,0"/>' };
const BASE = "---\nconfig:\n  kanban:\n    ticketBaseUrl: 'https://example.com/browse/#TICKET#'\n---\n";

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

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `kanban\n  ${p}\n    card`,
  (p) => `kanban\n  todo\n    ${p}`,
  (p) => `kanban\n  todo[${p}]\n    a[${p}]`,
  (p) => `kanban\n  ${p}[Todo]\n    ${p}[Card]`,
  (p) => `kanban\n  todo(${p})\n    a((${p}))\n    b{{${p}}}\n    c))${p}((`,
  (p) => `kanban\n  todo["${p}"]\n    a["${p}"]`,
  (p) => `kanban\n  todo["\`${p}\`"]\n    a["\`**${p}**\n${p}\`"]`,
  (p) => `kanban\n  todo\n    a@{ label: "${p}" }`,
  (p) => `kanban\n  todo\n    a@{ label: '${p}' }`,
  (p) => `kanban\n  todo\n    a@{ assigned: "${p}" }`,
  (p) => `kanban\n  todo\n    a@{ ticket: "${p}" }`,
  (p) => `kanban\n  todo\n    a@{ priority: "${p}" }`,
  (p) => `kanban\n  todo\n    a@{ icon: "${p}" }`,
  (p) => `kanban\n  todo\n    a@{ shape: "${p}" }`,
  (p) => `kanban\n  todo\n    a@{ ${p}: 1 }`,
  (p) => `kanban\n  todo\n    a@{ ${p} }`,
  (p) => `kanban\n  todo\n    a@{\n      assigned: ${p}\n      ticket: ${p}\n      priority: ${p}\n    }`,
  (p) => `kanban\n  todo@{ label: "${p}", ticket: "${p}", assigned: "${p}", priority: "${p}", icon: "${p}" }\n    a`,
  (p) => `${BASE}kanban\n  todo\n    a@{ ticket: "${p}" }`,
  (p) => `---\nconfig:\n  kanban:\n    ticketBaseUrl: "${p}"\n---\nkanban\n  todo\n    a@{ ticket: T-1 }`,
  (p) => `---\nconfig:\n  kanban:\n    ticketBaseUrl: "${p}#TICKET#"\n---\nkanban\n  todo\n    a@{ ticket: "${p}" }`,
  (p) => `---\nconfig:\n  kanban:\n    ticketBaseUrl: "#TICKET#"\n---\nkanban\n  todo\n    a@{ ticket: "${p}" }`,
  (p) =>
    `%%{init: {"kanban": {"ticketBaseUrl": "${p}", "sectionWidth": "${p}"}}}%%\nkanban\n  todo\n    a@{ ticket: T-1 }`,
  (p) => `kanban\n  todo\n  ::icon(${p})\n    a\n    ::icon(${p})`,
  (p) => `kanban\n  todo\n  :::${p}\n    a\n    :::${p} ${p}`,
  (p) => `kanban\n  todo\n    a %% ${p}\n    b[c] %% ${p}`,
  (p) => `kanban\n  todo\n    fa:fa-${p} x`,
  (p) => `kanban ${p}\n  todo`,
  (p) => `---\ntitle: "${p}"\n---\nkanban\n  todo\n    a`,
];

describe('inert kanban output', () => {
  it('holds for every corpus input', () => {
    let rendered = 0;
    for (const src of loadCorpus('kanban', /kanban/)) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(30);
  });

  it('holds with hostile text in every position', () => {
    let rendered = 0;
    for (const template of TEMPLATES) {
      for (const payload of PAYLOADS) {
        if (check(template(payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(400);
  });

  it('never links a ticket to a script URL', () => {
    for (const payload of PAYLOADS) {
      for (const base of [payload, payload + '#TICKET#', '#TICKET#']) {
        const config = { kanban: { ticketBaseUrl: base } };
        for (const ticket of ['T-1', 'javascript:alert(1)', ' JaVaScRiPt:alert(1)', 'data:text/html,x', '$&', "$'"]) {
          const { svg, links } = render(`kanban\n  todo\n    a@{ ticket: "${ticket}" }`, { ...options, config });
          assertInert(svg, JSON.stringify([base, ticket]));
          for (const link of links) expect(link.href).not.toMatch(/^\s*(?:javascript|vbscript|data)\s*:/i);
        }
      }
    }
  });

  it('escapes text, ids, metadata and classes', () => {
    const src = `${BASE}kanban\n  a<b>"&[<i onload=x>"&]\n  :::a"b <c> d=e\n    c@{ ticket: '"><x>&', assigned: 'x < y "&' }`;
    const { svg, links } = render(src, options);
    expect(svg).toContain('data-id="a&lt;b&gt;&quot;&amp;"');
    expect(svg).toContain('class="pelers-cluster pelers-column a b c d e"');
    expect(svg).toContain('>x &lt; y &quot;&amp;</text>');
    expect(svg).not.toContain('<i');
    expect(svg).not.toContain('<x>');
    expect(links[0].href).toBe('https://example.com/browse/%22%3E%3Cx%3E&');
  });

  it('holds for hostile option values', () => {
    for (const payload of PAYLOADS) {
      expect(check('kanban\n  todo\n    a@{ icon: x, ticket: y }', { idPrefix: payload, fontFamily: payload })).toBe(
        true
      );
    }
  });

  it('holds for mutated inputs', () => {
    const corpus = loadCorpus('kanban', /kanban/);
    const fragments = [
      ...PAYLOADS,
      '"',
      '[',
      ']',
      '(',
      ')',
      '\n',
      '\n    ',
      ':::',
      '::icon(',
      '@{ ',
      ' }',
      '@{ ticket: "',
      '@{ label: "',
      'assigned: ',
      'priority: High',
      ', ',
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
      'kanban\n  __proto__\n    constructor@{ __proto__: [polluted], constructor: 1, label: x }\n    :::__proto__\n    toString[hasOwnProperty]',
      options
    );
    render('kanban\n  todo\n    a@{\n      __proto__:\n        polluted: 1\n    }', options);
    expect(({} as Record<string, unknown>).polluted).toBeUndefined();
    expect(Object.keys(Object.prototype)).toEqual([]);
  });
});
