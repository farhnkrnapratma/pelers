import * as reference from '@mermaid-js/parser';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import type { TokenType, Tokens } from '../../src/diagrams/common/tokens.js';
import { PeleError } from '../../src/errors.js';

// Additions to langium.ts and corpus.ts for the grammars that need them.

interface ReferenceToken {
  name: string;
  LONGER_ALT?: ReferenceToken | ReferenceToken[];
}

type Services = Record<string, { parser: { Lexer: { chevrotainLexer: { lexerDefinition: ReferenceToken[] } } } }>;

// Which token types the reference lexer retries with a longer alternative, and with what.
export function referenceLongerAlts(create: string, key: string): [string, string[]][] {
  const services = (reference as unknown as Record<string, () => Services>)[create]()[key];
  const out: [string, string[]][] = [];
  for (const t of services.parser.Lexer.chevrotainLexer.lexerDefinition) {
    const alts = t.LONGER_ALT === undefined ? [] : Array.isArray(t.LONGER_ALT) ? t.LONGER_ALT : [t.LONGER_ALT];
    if (alts.length > 0) out.push([t.name, alts.map((alt) => alt.name)]);
  }
  return out;
}

export function peleLongerAlts(types: readonly TokenType[]): [string, string[]][] {
  return types.filter((t) => t.longer !== undefined).map((t) => [t.name, t.longer!.map((alt) => alt.name)]);
}

interface Lexed {
  tokens: { image: string; tokenType: { name: string } }[];
  errors: unknown[];
}

// The visible tokens of the reference lexer as lines of `name image`, or null when it reports an error.
export function referenceLexer(create: string, key: string): (src: string) => string | null {
  const services = (reference as unknown as Record<string, () => Record<string, unknown>>)[create]()[key];
  const lexer = (services as { parser: { Lexer: { tokenize(src: string): Lexed } } }).parser.Lexer;
  return (src) => {
    const { tokens, errors } = lexer.tokenize(src);
    if (errors.length > 0) return null;
    return tokens.map((t) => `${t.tokenType.name} ${JSON.stringify(t.image)}`).join('\n');
  };
}

// The same for one of Pele's token tables, with null for a lexical error.
export function peleLexer(types: readonly TokenType[], lex: (src: string) => Tokens): (src: string) => string | null {
  return (src) => {
    let tokens: Tokens;
    try {
      tokens = lex(src);
    } catch (e) {
      if (!(e instanceof PeleError)) throw e;
      return null;
    }
    const out: string[] = [];
    for (let i = 0; i < tokens.kinds.length - 1; i++) {
      out.push(`${types[tokens.kinds[i]].name} ${JSON.stringify(src.slice(tokens.starts[i], tokens.ends[i]))}`);
    }
    return out.join('\n');
  };
}

// String literals in the ported Langium parser tests (`*.test.ts`), which loadCorpus does not read.
export function loadTestStrings(diagram: string, keyword: RegExp): string[] {
  const seeds = new Set<string>();
  const dir = join('tests/compat', diagram);
  for (const file of readdirSync(dir).filter((f) => /\.test\.[jt]s$/.test(f))) {
    const src = readFileSync(join(dir, file), 'utf8');
    for (const m of src.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'|"((?:[^"\\\n]|\\.)*)"/g)) {
      const raw = m[1] ?? m[2] ?? m[3];
      if (!keyword.test(raw)) continue;
      try {
        seeds.add(new Function('return `' + raw.replace(/`/g, '\\`').replace(/\$\{/g, '\\${') + '`')() as string);
      } catch {
        continue;
      }
    }
  }
  return [...seeds];
}
