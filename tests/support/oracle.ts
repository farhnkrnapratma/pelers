import { readFileSync } from 'node:fs';
import { buildParser, type JisonParser } from './jison.js';

export type Pair = [string, string];
export type Call = unknown[];

export interface Outcome {
  ok: boolean;
  calls: Call[];
  error?: string;
}

export interface Oracle {
  parser: JisonParser;
  // Names of the `yy` methods the grammar calls.
  methods: string[];
  tokens(src: string, yy: Record<string, unknown>): Pair[];
  parse(src: string, yy: Record<string, unknown>): { ok: boolean; error?: string; result?: unknown };
}

// Builds the parser Mermaid generates from a vendored grammar, to compare a hand-written parser with.
export function createOracle(grammarPath: string): Oracle {
  const parser = buildParser(grammarPath);
  const grammar = readFileSync(grammarPath, 'utf8');
  const methods = [...new Set([...grammar.matchAll(/\byy\.(\w+)\s*\(/g)].map((m) => m[1]))];
  return {
    parser,
    methods,
    tokens(src, yy) {
      const lexer = Object.create(parser.lexer) as JisonParser['lexer'];
      lexer.setInput(src, yy);
      const out: Pair[] = [];
      for (let i = 0; i < 1e6; i++) {
        let t: number;
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
        out.push([parser.terminals_[t] ?? String(t), lexer.yytext]);
      }
      return out;
    },
    parse(src, yy) {
      parser.yy = yy;
      try {
        return { ok: true, result: parser.parse(src) };
      } catch (e) {
        return { ok: false, error: String((e as Error).message) };
      }
    },
  };
}

// A stand-in for a diagram's model builder that records every call made on it.
// `returns` supplies methods whose return values the grammar uses.
export function recordingYy(
  methods: string[],
  returns: Record<string, (...args: never[]) => unknown> = {}
): { yy: Record<string, unknown>; calls: Call[] } {
  const calls: Call[] = [];
  const yy: Record<string, unknown> = {};
  for (const name of new Set([...methods, ...Object.keys(returns)])) {
    yy[name] = (...args: unknown[]) => {
      while (args.length > 0 && args[args.length - 1] === undefined) args.pop();
      calls.push([name, ...args]);
      return (returns[name] as ((...a: unknown[]) => unknown) | undefined)?.(...args);
    };
  }
  return { yy, calls };
}

// Adjacent tokens of these kinds are joined, so a lexer may emit a run of text as one token or several.
export function mergeTokens(tokens: Pair[], kinds: string[]): Pair[] {
  const out: Pair[] = [];
  for (const t of tokens) {
    const last = out[out.length - 1];
    if (last && last[0] === t[0] && kinds.includes(t[0])) last[1] += t[1];
    else out.push([t[0], t[1]]);
  }
  return out;
}
