import { resolve } from 'node:path';
import type { TimelineDb } from '../../src/diagrams/timeline/db.js';
import { TOKEN_NAMES, tokenize } from '../../src/diagrams/timeline/lexer.js';
import { parseTimeline } from '../../src/diagrams/timeline/parser.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/timeline/upstream/timeline.jison'));

// Match Pele: comments leave the preceding character intact.
const rules = (oracle.parser.lexer as unknown as { rules: RegExp[] }).rules;
const comments = rules.filter((rule) => rule.source === String.raw`^(?:[^\}]%%[^\n]*)`);
if (comments.length !== 1) throw new Error('Expected one timeline preceding-character comment rule');
rules[rules.indexOf(comments[0])] = /(?!)/;

// Match Pele: keep titles on one line.
const titles = rules.filter((rule) => rule.source === String.raw`^(?:title\s[^\n]+)`);
if (titles.length !== 1) throw new Error('Expected one timeline title rule');
rules[rules.indexOf(titles[0])] = new RegExp(
  titles[0].source.replace(String.raw`title\s`, String.raw`title[^\S\r\n]`),
  titles[0].flags
);

// Use the whitespace rule to skip bare titles.
const spaces = rules.filter((rule) => rule.source === String.raw`^(?:\s+)`);
if (spaces.length !== 1) throw new Error('Expected one timeline whitespace rule');
rules[rules.indexOf(spaces[0])] = new RegExp(String.raw`^(?:\s+|title[^\S\n]*(?=\n|$))`, spaces[0].flags);

// The grammar reaches the title and accessibility setters through getCommonDb(); record those calls too.
function recorder() {
  const rec = recordingYy([...oracle.methods, 'setDiagramTitle', 'setAccTitle', 'setAccDescription'], {
    getCommonDb: () => rec.yy,
  });
  return rec;
}

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, recorder().yy);
}

export function peleTokens(src: string): Pair[] {
  const { types, texts } = tokenize(src);
  return types.map((t, i) => [TOKEN_NAMES[t], texts[i]] as Pair);
}

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

export function peleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseTimeline(src, yy as unknown as TimelineDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
