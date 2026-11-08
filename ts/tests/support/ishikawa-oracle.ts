import { resolve } from 'node:path';
import type { IshikawaDb } from '../../src/diagrams/ishikawa/db.js';
import { TOKEN_NAMES, tokenize } from '../../src/diagrams/ishikawa/lexer.js';
import { parseIshikawa } from '../../src/diagrams/ishikawa/parser.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/ishikawa/upstream/ishikawa.jison'));

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, recordingYy(oracle.methods).yy);
}

export function peleTokens(src: string): Pair[] {
  const { types, starts, ends } = tokenize(src);
  return types.map((type, i) => [TOKEN_NAMES[type], src.slice(starts[i], ends[i])] as Pair);
}

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recordingYy(oracle.methods);
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

export function peleParse(src: string): Outcome {
  const { yy, calls } = recordingYy(oracle.methods);
  try {
    parseIshikawa(src, yy as unknown as IshikawaDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
