import { resolve } from 'node:path';
import { tokenize } from '../../src/diagrams/er/lexer.js';
import { parseEr } from '../../src/diagrams/er/parser.js';
import { T, TOKEN_NAMES } from '../../src/diagrams/er/tokens.js';
import type { ErDb } from '../../src/diagrams/er/db.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/er/upstream/erDiagram.jison'));

function recorder() {
  let subgraphs = 0;
  const rec = recordingYy(oracle.methods, { addSubGraph: () => 'SG' + ++subgraphs });
  rec.yy.Cardinality = {
    ZERO_OR_ONE: 'ZERO_OR_ONE',
    ZERO_OR_MORE: 'ZERO_OR_MORE',
    ONE_OR_MORE: 'ONE_OR_MORE',
    ONLY_ONE: 'ONLY_ONE',
    MD_PARENT: 'MD_PARENT',
  };
  rec.yy.Identification = { NON_IDENTIFYING: 'NON_IDENTIFYING', IDENTIFYING: 'IDENTIFYING' };
  return rec;
}

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, recorder().yy);
}

// Inside a block Jison returns an unmatched character as the token itself. The parser then reads a
// digit as a symbol number, so `2`, `4`, `6` and `9` take the names of the symbols with those numbers.
const DIGIT_SYMBOLS = new Map([
  ['2', 'error'],
  ['4', 'ER_DIAGRAM'],
  ['6', 'EOF'],
  ['9', 'NEWLINE'],
]);

export function peleTokens(src: string): Pair[] {
  const { types, texts } = tokenize(src);
  return types.map((t, i) => {
    if (t === T.END || t === T.ERROR) return [TOKEN_NAMES[t], ''] as Pair;
    if (t === T.CHAR) return [DIGIT_SYMBOLS.get(texts[i]) ?? texts[i], texts[i]] as Pair;
    return [TOKEN_NAMES[t], texts[i]] as Pair;
  });
}

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

export function peleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseEr(src, yy as unknown as ErDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
