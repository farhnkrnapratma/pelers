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

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `treeView-beta\n  ${p}`,
  (p) => `treeView-beta\n  ${p}/\n    child`,
  (p) => `treeView-beta\n  "${p}"`,
  (p) => `treeView-beta\n  '${p}'`,
  (p) => `treeView-beta\n  a ## ${p}`,
  (p) => `treeView-beta\n  a :::${p}`,
  (p) => `treeView-beta\n  a :::highlight ## ${p}`,
  (p) => `treeView-beta\n  a icon(${p})`,
  (p) => `treeView-beta\n  a icon(x:${p})`,
  (p) => `treeView-beta\ntitle ${p}\n  a`,
  (p) => `treeView-beta\naccTitle: ${p}\naccDescr: ${p}\n  a`,
  (p) => `treeView-beta\naccDescr {\n ${p}\n}\n  a`,
  (p) => `treeView-beta\n├── ${p}\n│   └── ${p} ## ${p}\n└── x`,
  (p) => `---\ntitle: "${p.replace(/["\\]/g, '')}"\n---\ntreeView-beta\n  a`,
  (p) =>
    `---\nconfig:\n  treeView:\n    showIcons: true\n    defaultIconPack: '${p.replace(/'/g, '')}'\n    filenameIcons:\n      a: '${p.replace(/'/g, '')}'\n    extensionIcons:\n      .b: '${p.replace(/'/g, '')}'\n---\ntreeView-beta\n  a\n  c.b\n  d icon(e)\n  ${p}`,
  (p) =>
    `---\nconfig:\n  treeView:\n    rowIndent: '${p.replace(/'/g, '')}'\n    paddingX: 1e999\n    paddingY: -5\n    lineThickness: .nan\n---\ntreeView-beta\n  a\n    b`,
];

describe('treeView output is inert', () => {
  it('for hostile text in every position', () => {
    let rendered = 0;
    for (const payload of PAYLOADS) for (const template of TEMPLATES) if (check(template(payload))) rendered++;
    expect(rendered).toBeGreaterThan(PAYLOADS.length * 8);
  }, 60_000);

  it('for names that are object keys', () => {
    for (const key of ['__proto__', 'constructor', 'toString', 'hasOwnProperty', 'valueOf']) {
      const src = `---\nconfig:\n  treeView:\n    showIcons: true\n    defaultIconPack: p\n---\ntreeView-beta\n  ${key}\n  x.${key}\n  y icon(${key})\n`;
      const svg = render(src, options).svg;
      assertInert(svg, key);
      expect(svg).not.toContain('function');
      expect(svg).not.toContain('[object');
    }
  });

  it('for mutated corpus inputs', () => {
    const corpus = loadCorpus('treeview', /treeView-beta/);
    const next = mutator(
      corpus,
      [...PAYLOADS, ' :::', ' icon(', ' ## ', '"', "'", '\n', '    ', '├── ', '/'],
      random(11)
    );
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    for (let i = 0; i < 2000; i++) if (check(next())) rendered++;
    expect(rendered).toBeGreaterThan(300);
  }, 60_000);
});
