import { resolve } from 'node:path';
import { T, getType, parseOutline, tokenName, tokenize, type OutlineDb } from '../../src/diagrams/common/outline.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export interface OutlineOracle {
  oracleTokens(src: string): Pair[];
  peleTokens(src: string): Pair[];
  oracleParse(src: string): Outcome;
  peleParse(src: string): Outcome;
}

// What a parse failure was about: a lexical error, or the tokens the parser expected and the one it got.
export function failure(message: string | undefined): string {
  if (message === undefined) return '';
  if (message.startsWith('Lexical error')) return 'lexical';
  const at = message.lastIndexOf('Expecting ');
  return at === -1 ? message : message.slice(at);
}

const logger = { trace: (): void => {}, debug: (): void => {}, info: (): void => {} };

// Compares the shared outline lexer and parser with the parser Mermaid generates from a mindmap or kanban grammar.
export function outlineOracle(diagram: 'mindmap' | 'kanban'): OutlineOracle {
  const kanban = diagram === 'kanban';
  const oracle = createOracle(resolve(`tests/compat/${diagram}/upstream/${diagram}.jison`));

  const recorder = () => {
    const rec = recordingYy(oracle.methods, { getLogger: () => logger, getType });
    rec.yy.nodeType = { DEFAULT: 0 };
    return rec;
  };
  const semantic = (calls: unknown[][]): unknown[][] => calls.filter((call) => call[0] !== 'getLogger');

  return {
    oracleTokens: (src) => oracle.tokens(src, recorder().yy),
    peleTokens(src) {
      const { types, texts } = tokenize(src, kanban);
      return types.map((t, i) => [t === T.END ? '$end' : tokenName(t, kanban), texts[i]] as Pair);
    },
    oracleParse(src) {
      const { yy, calls } = recorder();
      const result = oracle.parse(src, yy);
      return { ok: result.ok, calls: semantic(calls), error: result.error };
    },
    peleParse(src) {
      const { yy, calls } = recorder();
      try {
        parseOutline(src, yy as unknown as OutlineDb, kanban);
        return { ok: true, calls: semantic(calls) };
      } catch (e) {
        return { ok: false, calls: semantic(calls), error: String((e as Error).message) };
      }
    },
  };
}
