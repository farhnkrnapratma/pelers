import { readFileSync, readdirSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { loadCorpus, mutator, random } from '../support/corpus.js';
import { oracleParse, oracleTokens, peleParse, peleTokens } from '../support/agentflow-oracle.js';

// Compares Pele's agentflow lexer and parser with the parser Mermaid generates from agentflow.jison:
// the same tokens at the same source positions, the same accept or reject decision, and the same
// calls into the model, positions included.
// Set FUZZ to raise the number of generated inputs and SEED to change them.

const FRAGMENTS = [
  ' ',
  '  ',
  '\n',
  ';',
  '-->',
  '--->',
  '--x',
  '---x',
  '--',
  '---',
  '-.-',
  '-..-',
  '-.',
  '.-',
  '-.->',
  '==>',
  '~~~',
  '--o',
  '<--',
  'x--',
  '|',
  '[',
  ']',
  '(',
  ')',
  '{',
  '}',
  '((',
  '))',
  '[[',
  ']]',
  '([',
  '])',
  '[(',
  ')]',
  '(((',
  ')))',
  '{{',
  '}}',
  '[/',
  '/]',
  '[\\',
  '\\]',
  '>',
  '(-',
  '-)',
  '"',
  '`',
  '"`',
  '`"',
  '@',
  '@{',
  '@{ ',
  '}\n',
  'e1@',
  ':::',
  ':',
  '&',
  ' & ',
  ',',
  '*',
  '#',
  '^',
  'v',
  '-',
  '=',
  '.',
  '~',
  '<',
  '\\|',
  '[|',
  'x',
  'o',
  'A',
  'B',
  'id1',
  '1',
  '23',
  'text',
  'é',
  'ö',
  'style ',
  'classDef ',
  'class ',
  'click ',
  'href ',
  'call ',
  'linkStyle ',
  'interpolate ',
  'default',
  'flow ',
  'flow',
  'flowx',
  'connector ',
  'connector',
  'global',
  'global ',
  'global\n',
  'end',
  'end\n',
  'end ',
  'direction TB',
  'direction LR',
  'direction',
  'agentflow-beta ',
  'agentflow-beta',
  'TD',
  'LR',
  'TB',
  'BT',
  'RL',
  '_blank',
  '_self',
  'accTitle: ',
  'accDescr: ',
  'accDescr { ',
  'fill:#f9f',
  'stroke-width:2px',
  'shape: tool',
  'label: "x"',
  'view: collapsed',
  '%%',
  '%% c',
  ' %% c\n',
  '%',
  '%%{',
  '%%x@y',
  '\t',
  '\r\n',
  '\r',
  ' \n',
  '\u2028',
  '\u00a0',
  'a()',
  '()',
  'foo',
  '["F"]',
  '@{ shape: tool }',
  '@{\n a: 1\n}',
];

// The agentflow specs are thin on the syntax it shares with flowcharts, so the flowchart corpus is reused
// with its keywords swapped.
const fromFlowchart = loadCorpus('flowchart', /graph|flowchart/).map((src) =>
  src.replace(/\b(?:flowchart-elk|flowchart|graph)\b/g, 'agentflow-beta').replace(/\bsubgraph\b/g, 'flow')
);

const fixtures = readdirSync('tests/compat/agentflow/upstream')
  .filter((name) => name.endsWith('.mmd'))
  .map((name) => readFileSync(`tests/compat/agentflow/upstream/${name}`, 'utf8'));

const SEEDS = [
  'agentflow-beta TB\n  a["A"] --> b{"B?"} -- yes --> c[["tool"]]\n  b -- "no" --x d\n  c -.- e[/in/]\n',
  'agentflow-beta LR\n  flow f["F"]@{ model: "m" }\n    a e1@--> b & c\n    e1@{ instruction: "go" }\n  end\n  f@{ view: collapsed }\n',
  'agentflow-beta\nglobal\n  A\n  B --> C\nend\nflow\n A --> D\nend\nflow x y\n direction LR\n q\nend\n',
  'agentflow-beta TB\n  connector gh["GitHub"]@{ protocol: "mcp" }\n  connector s\n  connector t@{\n    a: 1,\n    b: 2\n  }\n  gh --> s\n',
  '%% one\n\n  %% two\nagentflow-beta TB %% three\n  a --> b %% four\n  %% five\n  b[one %% six\n two] --> c(-el %% seven\n lipse-)\n  d[/tr %% eight\n ap/]\n',
  'agentflow-beta TB\n  a:::big --> b:::small\n  classDef big fill:#f9f,stroke:#333\n  class a,b big\n  style a fill:#bbf\n  linkStyle 0 stroke:red\n  linkStyle default interpolate basis stroke:blue\n  click a href "https://example.com" "tip" _blank\n  click b call cb("x") "tip"\n',
  'agentflow-beta TB\n  accTitle: A title\n  accDescr: A description\n  accDescr {\n   many\n   lines\n  }\n  a(round) --> b((circle)) --> c([stadium]) --> d[(db)] --> e>odd] --> f{{hex}} --> g(((dbl)))\n',
  'agentflow-beta TB\n  flow outer["Outer"]\n    flow inner["Inner"]\n      a --> inner\n    end\n    a --> outer\n  end\n  flow "quoted id"\n   b\n  end\n  flow end x\n  c\n  end\n',
  'agentflow-beta TB\n  a@{ shape: input, value: "x\n  y" }\n  b@{\n    label: "multi"\n    params:\n      - one\n  }\n  a --> b\n',
  'agentflow-beta TB;a-->b;b--xc;c-.-d;flow f;e;end;connector k;global;g;end\n',
];

const corpus = [...new Set([...loadCorpus('agentflow', /agentflow-beta/), ...fixtures, ...SEEDS, ...fromFlowchart])];

function compare(src: string): void {
  expect(peleTokens(src), `tokens of ${JSON.stringify(src)}`).toEqual(oracleTokens(src));
  const expected = oracleParse(src);
  const actual = peleParse(src);
  expect(actual.ok, `acceptance of ${JSON.stringify(src)}: ${expected.error ?? actual.error}`).toBe(expected.ok);
  if (expected.ok) expect(actual.calls, `model calls of ${JSON.stringify(src)}`).toEqual(expected.calls);
}

describe('agentflow parser against the Mermaid grammar', () => {
  it('has a corpus to work from', () => {
    expect(corpus.length).toBeGreaterThan(400);
  });

  it('agrees on every spec, fixture and documentation input', () => {
    for (const src of corpus) compare(src);
  });

  it('agrees on mutated inputs', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const next = mutator(corpus, FRAGMENTS, random(Number(process.env.SEED ?? 7)));
    for (let i = 0; i < count; i++) {
      const src = next();
      if (src.length <= 4000) compare(src);
    }
  }, 1_200_000);

  // Mutation rarely joins two tokens without a space, which is where keyword boundaries and
  // lookahead matter. This strings random pieces together, in each lexer state.
  it('agrees on random sequences of tokens', () => {
    const count = Number(process.env.FUZZ ?? 5000);
    const rnd = random(Number(process.env.SEED ?? 7) + 1);
    const pieces = [...new Set([...FRAGMENTS, ...FRAGMENTS.map((f) => f.trim()).filter((f) => f !== '')])];
    const pick = (): string => pieces[Math.floor(rnd() * pieces.length)];
    const openers = [
      'agentflow-beta\n',
      'agentflow-beta TB\na',
      'agentflow-beta TB\na --> ',
      'agentflow-beta TB\na -- ',
      'agentflow-beta TB\nflow f\n',
      'agentflow-beta TB\nflow ',
      'agentflow-beta TB\nconnector ',
      'agentflow-beta TB\na[',
      'agentflow-beta TB\na(-',
      'agentflow-beta TB\na[/',
      'agentflow-beta TB\na@{',
      'agentflow-beta TB\nstyle a ',
      'agentflow-beta TB\nclick a ',
      'agentflow-beta',
      '',
    ];
    for (let i = 0; i < count; i++) {
      let src = openers[Math.floor(rnd() * openers.length)];
      for (let k = 1 + Math.floor(rnd() * 10); k > 0; k--) src += pick() + (rnd() < 0.3 ? ' ' : '');
      compare(src);
    }
  }, 1_200_000);
});
