import { resolve } from 'node:path';
import type { C4Db } from '../../src/diagrams/c4/db.js';
import { tokenize } from '../../src/diagrams/c4/lexer.js';
import { parseC4 } from '../../src/diagrams/c4/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/c4/tokens.js';
import { createOracle, recordingYy, type Call, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/c4/upstream/c4Diagram.jison'));

// Match Pelers: titles stay on one line; bare titles are ignored.
{
  const lexer = oracle.parser.lexer as unknown as { rules: RegExp[]; performAction: (...args: unknown[]) => unknown };
  const titles = lexer.rules.filter((rule) => rule.source === String.raw`^(?:title\s[^#\n;]+)`);
  if (titles.length !== 1) throw new Error('Expected one C4 title rule');
  const index = lexer.rules.indexOf(titles[0]);
  lexer.rules[index] = new RegExp(
    String.raw`^(?:title[^\S\r\n]*(?=[\r\n]|$)\s*()|title[^\S\r\n][^#\n;]+)`,
    titles[0].flags
  );
  const action = lexer.performAction;
  lexer.performAction = function (this: unknown, yy, yy_, rule, ...rest) {
    if (rule === index && (yy_ as { matches: (string | undefined)[] }).matches[1] !== undefined) return undefined;
    return action.call(this, yy, yy_, rule, ...rest);
  };
}

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, recordingYy(oracle.methods).yy);
}

export function pelersTokens(src: string): Pair[] {
  const { types, starts, ends } = tokenize(src);
  return types.map((t, i) => [TOKEN_NAMES[t], src.slice(starts[i], ends[i])] as Pair);
}

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recordingYy(oracle.methods);
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

// The grammar spreads the argument list into each call and builds a one-key object for a named
// argument. Pelers passes the list as an array of strings and { key, value } pairs; this puts its
// calls in the grammar's form. The assignment mirrors the grammar's `kv[key] = value`.
function spread(call: Call): Call {
  const out: Call = [];
  for (const arg of call) {
    if (!Array.isArray(arg)) {
      out.push(arg);
      continue;
    }
    for (const attr of arg as (string | { key: string; value: string })[]) {
      if (typeof attr === 'string') {
        out.push(attr);
      } else {
        const kv: Record<string, string> = {};
        kv[attr.key] = attr.value;
        out.push(kv);
      }
    }
  }
  return out;
}

export function pelersParse(src: string): Outcome {
  const { yy, calls } = recordingYy(oracle.methods);
  try {
    parseC4(src, yy as unknown as C4Db);
    return { ok: true, calls: calls.map(spread) };
  } catch (e) {
    return { ok: false, calls: calls.map(spread), error: String((e as Error).message) };
  }
}
