import { resolve } from 'node:path';
import type { VennDb } from '../../src/diagrams/venn/db.js';
import { T, TOKEN_NAMES, VennLexer, type IndentHost } from '../../src/diagrams/venn/lexer.js';
import { parseVenn } from '../../src/diagrams/venn/parser.js';
import { createOracle, recordingYy, type Outcome, type Pair } from './oracle.js';

export const oracle = createOracle(resolve('tests/compat/venn/upstream/venn.jison'));

// Match Pelers: keep titles on one line.
const titleRules = (oracle.parser.lexer as unknown as { rules: RegExp[] }).rules;
const titleMatches = titleRules.filter((rule) => rule.source === '^(?:title\\s[^#\\n;]+)');
if (titleMatches.length !== 1) throw new Error('Expected one Venn title rule');
const titleRule = titleMatches[0];
titleRules[titleRules.indexOf(titleRule)] = new RegExp(
  titleRule.source.replace('title\\s', 'title[^\\S\\r\\n]'),
  titleRule.flags
);

// The lexer asks the model whether an indented `text` belongs to the set before it. With `indent`
// given the answer is fixed, to compare the lexers alone; without it the recorder answers the way
// Mermaid's DB does, from the statements the parser has reported.
function recorder(indent?: boolean) {
  let mode = false;
  let current: unknown;
  return recordingYy(oracle.methods, {
    getIndentMode: () => indent ?? mode,
    setIndentMode: ((enabled: boolean) => {
      mode = enabled;
    }) as never,
    addSubsetData: ((ids: string[]) => {
      current = ids;
    }) as never,
    getCurrentSets: () => current,
  });
}

export function oracleTokens(src: string, indent: boolean): Pair[] {
  return oracle.tokens(src, recorder(indent).yy);
}

export function pelersTokens(src: string, indent: boolean): Pair[] {
  const lexer = new VennLexer(src, recorder(indent).yy as unknown as IndentHost);
  const out: Pair[] = [];
  for (let i = 0; i < 1e6; i++) {
    const type = lexer.next();
    out.push([TOKEN_NAMES[type], type === T.ERROR ? '' : lexer.text]);
    if (type === T.END || type === T.ERROR) break;
  }
  return out;
}

export function oracleParse(src: string): Outcome {
  const { yy, calls } = recorder();
  const result = oracle.parse(src, yy);
  return { ok: result.ok, calls, error: result.error };
}

export function pelersParse(src: string): Outcome {
  const { yy, calls } = recorder();
  try {
    parseVenn(src, yy as unknown as VennDb);
    return { ok: true, calls };
  } catch (e) {
    return { ok: false, calls, error: String((e as Error).message) };
  }
}

// Match Pelers: comments leave the preceding character intact.
{
  const rules = (oracle.parser.lexer as unknown as { rules: RegExp[] }).rules;
  const comments = rules.filter((rule) => rule.source === String.raw`^(?:[^\}]%%[^\n]*)`);
  if (comments.length !== 1) throw new Error('Expected one Venn preceding-character comment rule');
  rules[rules.indexOf(comments[0])] = /(?!)/;
}

// Match Pelers: skip bare titles like comments.
{
  const lexer = oracle.parser.lexer as unknown as { next(): unknown; _input: string; matched: string };
  const next = lexer.next;
  lexer.next = function (this: typeof lexer) {
    const bare = /^title[^\S\n]*(?:\n[\n\r]*|$)/i.exec(this._input);
    if (bare && /\n[ \t]*$/.test(this.matched)) {
      this._input = this._input.slice(bare[0].length);
      this.matched += bare[0];
    }
    return next.call(this);
  };
}

// Match Pelers: allow hyphenated names, but still strip indented `text`.
{
  const lexer = oracle.parser.lexer as unknown as {
    rules: RegExp[];
    next(): unknown;
    yy: { consumeIndentText?: boolean };
  };
  const rules = lexer.rules;
  const guard = (word: string): number => {
    const matches = rules.filter((rule) => rule.source === `^(?:${word}\\b)`);
    if (matches.length !== 1) throw new Error(`Expected one Venn ${word} rule`);
    const index = rules.indexOf(matches[0]);
    rules[index] = new RegExp(`^(?:${word}\\b(?!-))`, matches[0].flags);
    return index;
  };
  for (const word of ['venn-beta', 'set', 'union', 'style']) guard(word);
  const textIndex = rules.findIndex((rule) => rule.source === String.raw`^(?:text\b)`);
  const text = rules[textIndex];
  const hyphenText = rules[guard('text')];
  const next = lexer.next;
  lexer.next = function (this: typeof lexer) {
    rules[textIndex] = this.yy.consumeIndentText ? text : hyphenText;
    return next.call(this);
  };
}
