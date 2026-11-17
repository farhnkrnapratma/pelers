import { resolve } from 'node:path';
import {
  Relationships,
  RequirementType,
  RiskLevel,
  VerifyType,
  type RequirementDb,
} from '../../src/diagrams/requirement/db.js';
import { tokenize } from '../../src/diagrams/requirement/lexer.js';
import { parseRequirement } from '../../src/diagrams/requirement/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/requirement/tokens.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/requirement/upstream/requirementDiagram.jison'));

// The grammar reads these constants off `yy`.
function recorder() {
  const rec = recordingYy(oracle.methods);
  Object.assign(rec.yy, { RequirementType, RiskLevel, VerifyType, Relationships });
  return rec;
}

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, {});
}

export function pelersTokens(src: string): Pair[] {
  const { types, texts } = tokenize(src);
  return types.map((t, i) => [TOKEN_NAMES[t], texts[i]] as Pair);
}

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

export function pelersParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseRequirement(src, yy as unknown as RequirementDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}
