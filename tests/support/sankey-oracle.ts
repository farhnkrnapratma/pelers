import { resolve } from 'node:path';
import type { SankeyDb } from '../../src/diagrams/sankey/db.js';
import { TOKEN_NAMES, tokenize } from '../../src/diagrams/sankey/lexer.js';
import { parseSankey } from '../../src/diagrams/sankey/parser.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/sankey/upstream/sankey.jison'));

// Nodes are stood in for by strings, so the links recorded on both sides can be compared.
function recorder() {
  return recordingYy(oracle.methods, { findOrCreateNode: (id: string) => `node:${id}` });
}

// The reference lexer matches a character outside the CSV alphabet as empty text without
// consuming it, and would return that token forever. Listing stops there, as Pele's lexer does.
export function oracleTokens(src: string): Pair[] {
  const lexer = Object.create(oracle.parser.lexer) as typeof oracle.parser.lexer & { _input: string };
  lexer.setInput(src, {});
  const out: Pair[] = [];
  for (let i = 0; i < 1e6; i++) {
    const t = lexer.lex();
    if (t === 1) {
      out.push(['$end', '']);
      break;
    }
    out.push([oracle.parser.terminals_[t] ?? String(t), lexer.yytext]);
    if (lexer.yytext === '' && lexer._input !== '') {
      out.push(['STUCK', '']);
      break;
    }
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
    parseSankey(src, yy as unknown as SankeyDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
