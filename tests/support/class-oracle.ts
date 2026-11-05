import { resolve } from 'node:path';
import { LINE_TYPE, RELATION_TYPE, type ClassDb } from '../../src/diagrams/class/db.js';
import { tokenize } from '../../src/diagrams/class/lexer.js';
import { parseClassDiagram } from '../../src/diagrams/class/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/class/tokens.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/class/upstream/classDiagram.jison'));

// The grammar uses what addNamespace, addNote and cleanupLabel return, and reads two tables of constants.
function recorder() {
  let namespaces = 0;
  let notes = 0;
  const rec = recordingYy(oracle.methods, {
    addNamespace: () => 'NS' + ++namespaces,
    addNote: () => 'note' + notes++,
    cleanupLabel: (label: string) => `<${label}>`,
  });
  rec.yy.lineType = LINE_TYPE;
  rec.yy.relationType = RELATION_TYPE;
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
    parseClassDiagram(src, yy as unknown as ClassDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
