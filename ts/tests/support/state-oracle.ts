import { resolve } from 'node:path';
import { tokenize } from '../../src/diagrams/state/lexer.js';
import { parseState, type StateBuilder } from '../../src/diagrams/state/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/state/tokens.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/state/upstream/stateDiagram.jison'));

function recorder() {
  let dividers = 0;
  return recordingYy(oracle.methods, {
    trimColon: (text: string) => (text.startsWith(':') ? text.slice(1).trim() : text.trim()),
    getDividerId: () => `divider-id-${++dividers}`,
  });
}

// The grammar's processId() pushes text back through `yy.lexer`, which Jison only sets while
// parsing, so the lexer is driven here with that reference in place.
export function oracleTokens(src: string): Pair[] {
  const lexer = Object.create(oracle.parser.lexer) as typeof oracle.parser.lexer;
  lexer.setInput(src, { lexer });
  const out: Pair[] = [];
  for (let i = 0; i < 1e6; i++) {
    let t: number | string;
    try {
      t = lexer.lex();
    } catch {
      out.push(['INVALID', '']);
      break;
    }
    if (t === 1) {
      out.push(['$end', '']);
      break;
    }
    out.push([oracle.parser.terminals_[t as number] ?? String(t), lexer.yytext]);
  }
  return out;
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
    parseState(src, yy as unknown as StateBuilder);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
