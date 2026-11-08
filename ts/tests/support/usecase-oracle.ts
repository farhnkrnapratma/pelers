import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { mermaidAst, record } from './usecase-ast.js';
import { tokenize } from '../../src/diagrams/usecase/lexer.js';
import { parseUsecase } from '../../src/diagrams/usecase/parser.js';
import { T, TOKEN_NAMES } from '../../src/diagrams/usecase/tokens.js';
import type { UsecaseModel } from '../../src/diagrams/usecase/types.js';
// @ts-expect-error generated bundle of Mermaid's parser, without types
import * as reference from './usecase-reference.mjs';

// Mermaid's use case parser is hand-written on Chevrotain, so the reference is that parser itself,
// bundled by tests/support/build-usecase-reference.mjs, rather than one generated from a grammar.

export type Token = [name: string, image: string, offset: number];

export interface Lexed {
  tokens: Token[];
  error?: string;
}

export interface Parsed {
  ok: boolean;
  error?: string;
  model?: unknown;
}

interface ReferenceToken {
  image: string;
  startOffset: number;
  tokenType: { name: string };
}

interface ReferenceLexError {
  message: string;
  offset: number;
  length: number;
  line?: number;
  column?: number;
}

export function referenceTokens(src: string): Lexed {
  const result = reference.usecaseLexer.tokenize(src) as { tokens: ReferenceToken[]; errors: ReferenceLexError[] };
  if (result.errors.length > 0) {
    // The message Mermaid builds from the first lexer error.
    const e = result.errors[0];
    return {
      tokens: [],
      error: `Error lexing usecase diagram: ${e.message} at line ${e.line ?? 1}, column ${e.column ?? 1} [${e.offset},${e.offset + e.length})`,
    };
  }
  return { tokens: result.tokens.map((t) => [t.tokenType.name, t.image, t.startOffset]) };
}

export function peleTokens(src: string): Lexed {
  try {
    const { types, starts, ends } = tokenize(src);
    const tokens: Token[] = [];
    for (let i = 0; types[i] !== T.EOF; i++)
      tokens.push([TOKEN_NAMES[types[i]], src.slice(starts[i], ends[i]), starts[i]]);
    return { tokens };
  } catch (e) {
    return { tokens: [], error: (e as Error).message };
  }
}

function entries<V>(map: ReadonlyMap<string, V>): [string, V][] {
  return [...map];
}

// Everything Mermaid publishes after a parse, in the order it publishes it.
function snapshot(db: {
  getActors(): ReadonlyMap<string, unknown>;
  getUseCases(): ReadonlyMap<string, unknown>;
  getSystemBoundaries(): ReadonlyMap<string, unknown>;
  getRelationships(): readonly unknown[];
  getNotes(): ReadonlyMap<string, unknown>;
  getJsonNodes(): ReadonlyMap<string, unknown>;
  getClassDefs(): ReadonlyMap<string, unknown>;
  getDirection(): string;
  getAST(): unknown;
}): unknown {
  return {
    actors: entries(db.getActors()),
    useCases: entries(db.getUseCases()),
    boundaries: entries(db.getSystemBoundaries()),
    relationships: db.getRelationships(),
    notes: entries(db.getNotes()),
    jsonNodes: entries(db.getJsonNodes()),
    classDefs: entries(db.getClassDefs()),
    direction: db.getDirection(),
    ast: db.getAST(),
  };
}

export async function referenceParse(src: string): Promise<Parsed> {
  try {
    await reference.parser.parse(src);
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
  return {
    ok: true,
    model: {
      ...(snapshot(reference.db) as object),
      accTitle: reference.getAccTitle(),
      accDescr: reference.getAccDescription(),
    },
  };
}

export function modelSnapshot(model: UsecaseModel): unknown {
  return {
    actors: entries(model.actors),
    useCases: entries(model.useCases),
    boundaries: entries(model.systemBoundaries),
    relationships: model.relationships,
    notes: entries(model.notes),
    jsonNodes: entries(model.jsonNodes).map(([id, node]) => [
      id,
      { ...node, propertyOrder: record(node.propertyOrder) },
    ]),
    classDefs: entries(model.classDefs),
    direction: model.direction,
    ast: mermaidAst(model),
    accTitle: model.accTitle,
    accDescr: model.accDescr,
  };
}

export function peleParse(src: string): Parsed {
  try {
    return { ok: true, model: modelSnapshot(parseUsecase(src)) };
  } catch (e) {
    return { ok: false, error: (e as Error).message };
  }
}

// The specs write most statements without the header and add it in a helper, so the corpus for
// the parser is every string literal in them that has some structure, with the header put back.
// `fragments` are the same literals as written, for the lexer alone.
export function specCorpus(): { diagrams: string[]; fragments: string[] } {
  const diagrams = new Set<string>();
  const fragments = new Set<string>();
  for (const dir of ['tests/compat/usecase', 'tests/compat/usecase-parser']) {
    for (const file of readdirSync(dir).filter((f) => f.endsWith('.spec.ts'))) {
      const src = readFileSync(join(dir, file), 'utf8');
      for (const m of src.matchAll(/`((?:[^`\\]|\\.)*)`|'((?:[^'\\\n]|\\.)*)'/g)) {
        const raw = m[1] ?? m[2];
        if (raw.includes('${') || raw.includes('./') || !/\W/.test(raw)) continue;
        let text: string;
        try {
          text = new Function('return `' + raw.replace(/`/g, '\\`') + '`')() as string;
        } catch {
          continue;
        }
        if (text.length > 2000) continue;
        if (/^\s*usecase-beta/.test(text)) diagrams.add(text);
        else {
          fragments.add(text);
          diagrams.add('usecase-beta\n' + text);
        }
      }
    }
  }
  return { diagrams: [...diagrams], fragments: [...fragments] };
}
