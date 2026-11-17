import { resolve } from 'node:path';
import { tokenize } from '../../src/diagrams/flowchart/lexer.js';
import { parseFlowchart } from '../../src/diagrams/flowchart/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/flowchart/tokens.js';
import type { FlowDb } from '../../src/diagrams/flowchart/db.js';
import { createOracle, mergeTokens, recordingYy, type Outcome, type Pair } from './oracle.js';

const oracle = createOracle(resolve('tests/compat/flowchart/upstream/flow.jison'));
const MERGED = ['TEXT', 'EDGE_TEXT'];

function recorder() {
  let first = true;
  let subgraphs = 0;
  const rec = recordingYy(oracle.methods, {
    destructLink: (end: string, start?: string) => ({ type: `${start ?? ''}|${end}`, stroke: 's', length: 1 }),
    addSubGraph: () => 'SG' + ++subgraphs,
  });
  rec.yy.lex = {
    firstGraph: () => {
      if (first) {
        first = false;
        return true;
      }
      return false;
    },
  };
  return rec;
}

// Mermaid's flowParser.ts applies this before handing text to the generated parser.
export function prepare(src: string): string {
  return src.replace(/}\s*\n/g, '}\n');
}

export function oracleTokens(src: string): Pair[] {
  return mergeTokens(oracle.tokens(prepare(src), recorder().yy), MERGED);
}

export function pelersTokens(src: string): Pair[] {
  const { types, texts } = tokenize(prepare(src));
  return mergeTokens(
    types.map((t, i) => [TOKEN_NAMES[t], texts[i]] as Pair),
    MERGED
  );
}

const semantic = (calls: unknown[][]): unknown[][] => calls.filter((call) => call[0] !== 'destructLink');

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(prepare(src), yy);
  return { ok: result.ok, calls: semantic(calls), error: result.error };
}

export function pelersParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseFlowchart(src, yy as unknown as FlowDb);
    return { ok: true, calls: semantic(calls) };
  } catch (e) {
    return { ok: false, calls: semantic(calls), error: String((e as Error).message) };
  }
}
