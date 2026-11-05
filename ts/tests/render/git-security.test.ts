import { describe, expect, it } from 'vitest';
import { PeleError, render } from '../../src/index.js';
import { metricsMeasurer } from '../../src/text/measurer.js';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { PAYLOADS, assertInert } from '../support/inert.js';

const options = { measurer: metricsMeasurer };

function check(src: string): boolean {
  let svg: string;
  try {
    svg = render(src, options).svg;
  } catch (error) {
    if (error instanceof PeleError) return false;
    throw new Error(
      `Unexpected ${(error as Error).name} for ${JSON.stringify(src).slice(0, 200)}: ${(error as Error).message}`
    );
  }
  assertInert(svg, JSON.stringify(src).slice(0, 160));
  return true;
}

// A string in double quotes cannot hold a double quote or a line break, so the payload is tried
// as written, in single quotes, and with its quotes escaped.
const quoted = (p: string): string[] => [`"${p}"`, `'${p}'`, `"${p.replace(/["\\]/g, '\\$&')}"`];

const TEMPLATES: ((q: string, p: string) => string)[] = [
  (q) => `gitGraph\n commit id: ${q}`,
  (q) => `gitGraph\n commit id: ${q} msg: ${q} tag: ${q} tag: ${q} type: HIGHLIGHT`,
  (q) => `gitGraph\n commit ${q}`,
  (q) => `gitGraph\n commit\n branch ${q}\n commit\n checkout ${q}\n commit`,
  (q) =>
    `gitGraph\n commit\n branch ${q} order: 2\n commit\n switch main\n commit\n merge ${q} id: ${q} tag: ${q} type: REVERSE`,
  (q) => `gitGraph\n commit id: ${q}\n branch b\n commit\n cherry-pick id: ${q}`,
  (q) =>
    `gitGraph\n commit id: "a"\n branch b\n commit id: "c"\n checkout main\n commit\n cherry-pick id: "c" tag: ${q}`,
  (q) =>
    `gitGraph\n commit id:"z"\n branch b\n commit id:"a"\n checkout main\n commit\n merge b id:"m"\n checkout b\n cherry-pick id:"m" parent:"a" tag: ${q}`,
  (q) => `gitGraph TB:\n commit id: ${q} tag: ${q}\n branch ${q}\n commit`,
  (q) => `gitGraph BT:\n commit id: ${q} tag: ${q}\n branch ${q}\n commit`,
  (_q, p) => `gitGraph\n commit\n branch ${p}\n commit`,
  (_q, p) => `gitGraph\n title ${p}\n commit`,
  (_q, p) => `gitGraph\n accTitle: ${p}\n accDescr: ${p}\n commit`,
  (_q, p) => `gitGraph\n accDescr {\n ${p}\n }\n commit`,
  (_q, p) => `gitGraph ${p}:\n commit`,
  (_q, p) => `gitGraph\n commit type: ${p}`,
  (_q, p) => `gitGraph\n commit\n branch b order: ${p}`,
  (q, p) =>
    `---\ntitle: ${q}\nconfig:\n  gitGraph:\n    mainBranchName: ${q}\n    mainBranchOrder: ${q}\n    showBranches: ${q}\n    rotateCommitLabel: ${q}\n---\ngitGraph\n commit id: "${p.replace(/["\\\n]/g, '')}"`,
  (q) =>
    `%%{init: {"gitGraph": {"mainBranchName": ${q}, "showCommitLabel": ${q}, "parallelCommits": ${q}}}}%%\ngitGraph\n commit\n branch b\n commit`,
];

describe('git graph output is inert', () => {
  it('escapes hostile text in every position', () => {
    let rendered = 0;
    let turn = 0;
    for (const payload of PAYLOADS) {
      const forms = quoted(payload);
      for (const template of TEMPLATES) {
        if (check(template(forms[turn++ % forms.length], payload))) rendered++;
      }
    }
    expect(rendered).toBeGreaterThan(300);
  }, 120_000);

  it('keeps payload text out of markup', () => {
    const { svg } = render(
      'gitGraph\n commit id: "<script>alert(1)</script>" tag: "\\"><img src=x onerror=alert(1)>"\n branch "</text><script>"\n commit',
      options
    );
    expect(svg).not.toContain('<script');
    expect(svg).not.toContain('<img');
    expect(svg).toContain('&lt;script&gt;alert(1)&lt;/script&gt;');
  });

  it('stays inert for corpus and mutated inputs', () => {
    const corpus = loadCorpus('git', /gitGraph/);
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    expect(rendered).toBeGreaterThan(60);
    const next = mutator(
      corpus,
      [
        ...PAYLOADS,
        '"',
        "'",
        '\n',
        'commit ',
        'branch ',
        'merge ',
        'id: "',
        'tag: "',
        'title ',
        'TB:',
        'cherry-pick id: "',
      ],
      random(11)
    );
    const count = Number(process.env.FUZZ ?? 2000);
    for (let i = 0; i < count; i++) check(next());
  }, 600_000);
});
