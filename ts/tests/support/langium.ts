import * as reference from '@mermaid-js/parser';
import { expect } from 'vitest';
import type { TokenType } from '../../src/diagrams/common/tokens.js';
import { PeleError } from '../../src/errors.js';

// Helpers for the diagram types Mermaid parses with Langium. `@mermaid-js/parser` is the reference:
// its lexer definitions check Pele's token tables, and its syntax trees check Pele's parsers.

interface ReferenceToken {
  name: string;
  PATTERN: RegExp | string;
  GROUP?: string;
}

type Services = Record<string, { parser: { Lexer: { chevrotainLexer: { lexerDefinition: ReferenceToken[] } } } }>;

export interface TableRow {
  name: string;
  pattern: string;
  hidden: boolean;
}

// The reference lexer's token types, in matching order.
export function referenceTokens(create: string, key: string): TableRow[] {
  const services = (reference as unknown as Record<string, () => Services>)[create]()[key];
  return services.parser.Lexer.chevrotainLexer.lexerDefinition.map((t) => ({
    name: t.name,
    pattern: typeof t.PATTERN === 'string' ? t.PATTERN : t.PATTERN.source,
    hidden: t.GROUP !== undefined,
  }));
}

export function peleTokens(types: readonly TokenType[]): TableRow[] {
  return types.map((t) => ({
    name: t.name,
    pattern: typeof t.pattern === 'string' ? t.pattern : t.pattern.source,
    hidden: t.hidden === true,
  }));
}

// A syntax tree without Langium's bookkeeping, for comparison.
export function plain(node: unknown): unknown {
  if (Array.isArray(node)) return node.map(plain);
  if (node === null || typeof node !== 'object') return node;
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(node)) {
    if (key.startsWith('$') && key !== '$type') continue;
    if (value !== undefined) out[key] = plain(value);
  }
  return out;
}

export async function referenceAst(type: string, src: string): Promise<{ ok: boolean; ast?: unknown; error?: string }> {
  try {
    const parse = reference.parse as unknown as (type: string, text: string) => Promise<unknown>;
    return { ok: true, ast: plain(await parse(type, src)) };
  } catch (e) {
    return { ok: false, error: String((e as Error).message) };
  }
}

export function peleAst(parse: (src: string) => unknown, src: string): { ok: boolean; ast?: unknown; error?: string } {
  try {
    return { ok: true, ast: plain(parse(src)) };
  } catch (e) {
    if (!(e instanceof PeleError)) throw e;
    return { ok: false, error: e.message };
  }
}

export interface ParseResult<T> {
  value: T;
  lexerErrors: unknown[];
  parserErrors: unknown[];
}

// The shape Langium's parser returns, which the ported parser tests read.
export function toResult<T>(parse: (src: string) => T, src: string): ParseResult<T> {
  try {
    return { value: parse(src), lexerErrors: [], parserErrors: [] };
  } catch (e) {
    if (!(e instanceof PeleError)) throw e;
    const lexical = e.message.startsWith('Lexical error');
    return { value: {} as T, lexerErrors: lexical ? [e] : [], parserErrors: lexical ? [] : [e] };
  }
}

export function expectNoErrorsOrAlternatives(result: ParseResult<unknown>): void {
  expect(result.lexerErrors).toHaveLength(0);
  expect(result.parserErrors).toHaveLength(0);
}
