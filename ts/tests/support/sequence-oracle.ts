import { resolve } from 'node:path';
import { tokenize } from '../../src/diagrams/sequence/lexer.js';
import { parseSequence, type SeqBuilder } from '../../src/diagrams/sequence/parser.js';
import { TOKEN_NAMES } from '../../src/diagrams/sequence/tokens.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';
import { LINETYPE, PLACEMENT } from './sequence-constants.js';

const oracle = createOracle(resolve('tests/compat/sequence/upstream/sequenceDiagram.jison'));

// The grammar puts the return values of these two into the statement list it hands to `apply`,
// so the recorder returns something that shows which text each was called with.
function recorder() {
  const rec = recordingYy(oracle.methods, {
    parseMessage: (str: string) => ({ message: str }),
    parseBoxData: (str: string) => ({ box: str }),
  });
  rec.yy.LINETYPE = LINETYPE;
  rec.yy.PLACEMENT = PLACEMENT;
  return rec;
}

export function oracleTokens(src: string): Pair[] {
  return oracle.tokens(src, recorder().yy);
}

export function peleTokens(src: string): Pair[] {
  const { types, texts } = tokenize(src);
  return types.map((t, i) => [TOKEN_NAMES[t], texts[i]] as Pair);
}

// The grammar keeps changing the statement lists after it has built them, so the calls are
// compared as they stand once parsing is over.
const snapshot = (calls: unknown[][]): unknown[][] => JSON.parse(JSON.stringify(calls)) as unknown[][];

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls: snapshot(calls), error: result.error };
}

export function peleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseSequence(src, yy as unknown as SeqBuilder);
    return { ok: true, calls: snapshot(calls) };
  } catch (e) {
    return { ok: false, calls: snapshot(calls), error: String((e as Error).message) };
  }
}

// Match Pele: comments leave the preceding character intact.
{
  const rules = (oracle.parser.lexer as unknown as { rules: RegExp[] }).rules;
  const comments = rules.filter((rule) => rule.source === String.raw`^(?:[^\}]%%[^\n]*)`);
  if (comments.length !== 1) throw new Error('Expected one sequence preceding-character comment rule');
  rules[rules.indexOf(comments[0])] = /(?!)/;
}

// Match Pele: titles stay on one line; bare titles are ignored.
{
  const lexer = oracle.parser.lexer as unknown as {
    rules: RegExp[];
    performAction: (this: unknown, yy: unknown, yy_: { yytext: string }, rule: number, start: string) => unknown;
  };
  const titles: number[] = [];
  for (const name of ['title', 'title:']) {
    const source = `^(?:${name}\\s[^#\\n;]+)`;
    const matches = lexer.rules.filter((rule) => rule.source === source);
    if (matches.length !== 1) throw new Error(`Expected one sequence ${name} rule`);
    const index = lexer.rules.indexOf(matches[0]);
    lexer.rules[index] = new RegExp(`^(?:${name}[^\\S\\r\\n][^#\\n;]+|${name}[^\\S\\n]*(?=\\n|$))`, matches[0].flags);
    titles.push(index);
  }
  const perform = lexer.performAction;
  lexer.performAction = function (yy, yy_, rule, start) {
    const rest = (this as { _input: string })._input;
    if (titles.includes(rule) && /^title:?\s*$/i.test(yy_.yytext) && /^(?:\n|$)/.test(rest)) return undefined;
    return perform.call(this, yy, yy_, rule, start);
  };
}
