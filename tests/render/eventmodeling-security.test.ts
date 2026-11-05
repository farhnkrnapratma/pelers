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

const line = (p: string): string => p.replace(/[\n\r]/g, ' ');

const TEMPLATES: ((p: string) => string)[] = [
  (p) => `eventmodeling\ntf 01 cmd A { ${line(p)} }`,
  (p) => `eventmodeling\ntf 01 evt A "${line(p)}"`,
  (p) => `eventmodeling\ntf 01 evt A '${line(p)}'`,
  (p) => `eventmodeling\ntf 01 rmo A \`json\`{ "k": "${line(p)}" }`,
  (p) => `eventmodeling\ntf 01 evt A [[D]]\ndata D {\n  ${p}\n}\n`,
  (p) => `eventmodeling\ntf 01 evt A\nnote 01 {\n  ${p}\n}\n`,
  (p) => `eventmodeling\ntf 01 evt A\nnote 01 \`html\` {\n  <div>${p}</div>\n}\n`,
  (p) => `eventmodeling title ${line(p)}\ntf 01 ui A`,
  (p) => `eventmodeling accTitle: ${line(p)}\ntf 01 ui A accDescr: ${line(p)}`,
  (p) => `eventmodeling accDescr {\n ${p}\n}\ntf 01 ui A`,
  (p) => `eventmodeling\ntf 01 ui ${p}`,
  (p) => `eventmodeling\ntf 01 ui A.${p}`,
  (p) => `eventmodeling\ntf 01 evt A\ngwt 01 given evt ${p} then evt B`,
  (p) => `eventmodeling\n/* ${p} */\ntf 01 ui A // ${line(p)}\n`,
  (p) => `---\ntitle: "${p.replace(/["\\\n]/g, '')}"\n---\neventmodeling\ntf 01 ui A`,
];

describe('eventmodeling output is inert', () => {
  it('for hostile text in every position', () => {
    let rendered = 0;
    for (const payload of PAYLOADS) for (const template of TEMPLATES) if (check(template(payload))) rendered++;
    expect(rendered).toBeGreaterThan(PAYLOADS.length * 8);
  }, 60_000);

  it('for names that are object keys', () => {
    for (const key of ['__proto__', 'constructor', 'toString', 'hasOwnProperty', 'valueOf']) {
      const src = `eventmodeling\ntf 01 cmd ${key}.${key} [[${key}]]\ntf 02 evt ${key}\ndata ${key} {\n x\n}\nentity ${key}\ngwt 01 given evt ${key} then evt ${key}\n`;
      expect(check(src), key).toBe(true);
    }
  });

  it('for mutated corpus inputs', () => {
    const corpus = loadCorpus('eventmodeling', /eventmodeling/);
    const next = mutator(
      corpus,
      [...PAYLOADS, '"', "'", '\n', '{', '}', ' ->> 01', '[[', ']]', 'tf 09 evt Z', 'note 01 {\n', '\n}\n'],
      random(19)
    );
    let rendered = 0;
    for (const src of corpus) if (check(src)) rendered++;
    for (let i = 0; i < 2000; i++) if (check(next())) rendered++;
    expect(rendered).toBeGreaterThan(100);
  }, 60_000);
});
