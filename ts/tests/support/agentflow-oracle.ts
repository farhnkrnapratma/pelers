import { resolve } from 'node:path';
import { tokenize } from '../../src/diagrams/agentflow/lexer.js';
import { parseAgentflow } from '../../src/diagrams/agentflow/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/agentflow/tokens.js';
import type { AgentflowDb } from '../../src/diagrams/agentflow/db.js';
import { createOracle, recordingYy, type Outcome } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/agentflow/upstream/agentflow.jison'));

// A token with its source span: name, text, first line, first column, last line, last column.
export type Token = [string, string, number, number, number, number];

// Jison matches edge text, and text inside `(- -)`, one character at a time. Adjacent tokens of
// these kinds are joined on both sides, text and span.
const MERGED = ['TEXT', 'EDGE_TEXT'];

function merge(tokens: Token[]): Token[] {
  const out: Token[] = [];
  for (const t of tokens) {
    const last = out[out.length - 1];
    if (last && last[0] === t[0] && MERGED.includes(t[0])) {
      last[1] += t[1];
      last[4] = t[4];
      last[5] = t[5];
    } else {
      out.push([...t]);
    }
  }
  return out;
}

function recorder() {
  let first = true;
  let blocks = 0;
  const rec = recordingYy(oracle.methods, {
    destructLink: (end: string, start?: string) => ({
      type: `${start ?? ''}|${end}`,
      stroke: 's',
      length: 1,
      edgeSemantic: 'e',
    }),
    addSubGraph: () => 'SG' + ++blocks,
    addConnector: () => 'C' + ++blocks,
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

interface Lexer {
  setInput(src: string, yy: unknown): void;
  lex(): number;
  yytext: string;
  yylloc: { first_line: number; first_column: number; last_line: number; last_column: number };
}

export function oracleTokens(src: string): Token[] {
  const lexer = Object.create(oracle.parser.lexer) as Lexer;
  lexer.setInput(src, recorder().yy);
  const out: Token[] = [];
  for (let i = 0; i < 1e6; i++) {
    let t: number;
    try {
      t = lexer.lex();
    } catch {
      out.push(['INVALID', '', 0, 0, 0, 0]);
      break;
    }
    if (t === 1) {
      out.push(['$end', '', 0, 0, 0, 0]);
      break;
    }
    const l = lexer.yylloc;
    out.push([
      oracle.parser.terminals_[t] ?? String(t),
      lexer.yytext,
      l.first_line,
      l.first_column,
      l.last_line,
      l.last_column,
    ]);
  }
  return merge(out);
}

export function peleTokens(src: string): Token[] {
  const { types, texts, locs } = tokenize(src);
  const last = types.length - 1;
  return merge(
    types.map((t, i): Token =>
      i === last
        ? [TOKEN_NAMES[t], '', 0, 0, 0, 0]
        : [TOKEN_NAMES[t], texts[i], locs[4 * i], locs[4 * i + 1], locs[4 * i + 2], locs[4 * i + 3]]
    )
  );
}

const semantic = (calls: unknown[][]): unknown[][] => calls.filter((call) => call[0] !== 'destructLink');

// Both sides parse the text as it is, without the trimming Mermaid's wrapper does first.
export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls: semantic(calls), error: result.error };
}

export function peleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseAgentflow(src, yy as unknown as AgentflowDb, true);
    return { ok: true, calls: semantic(calls) };
  } catch (e) {
    return { ok: false, calls: semantic(calls), error: String((e as Error).message) };
  }
}
