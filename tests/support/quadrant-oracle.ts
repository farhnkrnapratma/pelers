import { resolve } from 'node:path';
import type { QuadrantDb } from '../../src/diagrams/quadrant/db.js';
import { tokenize } from '../../src/diagrams/quadrant/lexer.js';
import { parseQuadrant } from '../../src/diagrams/quadrant/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/quadrant/tokens.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/quadrant/upstream/quadrant.jison'));

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, {});
}

export function peleTokens(src: string): Pair[] {
  const { types, texts } = tokenize(src);
  return types.map((t, i) => [TOKEN_NAMES[t], texts[i]] as Pair);
}

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recordingYy(oracle.methods);
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

export function peleParse(src: string): Outcome {
  const { yy, calls } = recordingYy(oracle.methods);
  try {
    parseQuadrant(src, yy as unknown as QuadrantDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}

// Match Pele: comments leave the preceding character intact.
{
  const rules = (oracle.parser.lexer as unknown as { rules: RegExp[] }).rules;
  const comments = rules.filter((rule) => rule.source === String.raw`^(?:[^\}]%%[^\n]*)`);
  if (comments.length !== 1) throw new Error('Expected one quadrant preceding-character comment rule');
  rules[rules.indexOf(comments[0])] = /(?!)/;
}
