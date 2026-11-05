import { resolve } from 'node:path';
import type { XyChartDb } from '../../src/diagrams/xychart/db.js';
import { tokenize } from '../../src/diagrams/xychart/lexer.js';
import { parseXyChart } from '../../src/diagrams/xychart/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/xychart/tokens.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/xychart/upstream/xychart.jison'));

// The text that the grammar's broken markdown string rule matches, read back from the generated lexer.
export function brokenRuleText(): string {
  const rules = (oracle.parser.lexer as unknown as { rules: RegExp[] }).rules;
  const rule = rules.find((r) => r.source.includes('pushState'))!;
  return rule.source
    .slice('^(?:(?:'.length, -'))'.length)
    .replace(/\\(.)/g, (_m, ch: string) => (ch === 'n' ? '\n' : ch));
}

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
    parseXyChart(src, yy as unknown as XyChartDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}

// Match Pele: comments leave the preceding character intact.
{
  const rules = (oracle.parser.lexer as unknown as { rules: RegExp[] }).rules;
  const comments = rules.filter((rule) => rule.source === String.raw`^(?:[^\}]%%[^\n]*)`);
  if (comments.length !== 1) throw new Error('Expected one XY chart preceding-character comment rule');
  rules[rules.indexOf(comments[0])] = /(?!)/;
}
