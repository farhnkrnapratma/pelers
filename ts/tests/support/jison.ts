import { readFileSync } from 'node:fs';
// @ts-expect-error jison ships no types
import jison from 'jison';

export interface JisonParser {
  yy: Record<string, unknown>;
  parse(src: string): unknown;
  lexer: { setInput(src: string, yy: unknown): void; lex(): number; yytext: string };
  terminals_: Record<number, string>;
}

// Builds the same parser Mermaid generates from a grammar file, for use as a reference in tests.
export function buildParser(grammarPath: string): JisonParser {
  const grammar = readFileSync(grammarPath, 'utf8');
  const source = new jison.Generator(grammar, { moduleType: 'js', 'token-stack': true }).generate({
    moduleMain: '() => {}',
  });
  return new Function(`${source}; return parser;`)() as JisonParser;
}
