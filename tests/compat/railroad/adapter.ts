import { detect } from '../../../src/detect.js';
import { RailroadDb, notationOf, populate } from '../../../src/diagrams/railroad/db.js';
import { railroad as diagram } from '../../../src/diagrams/railroad/index.js';
import { parseIr, type IrAst, type IrExpression } from '../../../src/diagrams/railroad/ir.js';
import type { Notation } from '../../../src/diagrams/railroad/types.js';
import { PeleError } from '../../../src/errors.js';
import { toResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';

export type Railroad = IrAst;
export type RailroadChoiceExpr = Extract<IrExpression, { $type: 'RailroadChoiceExpr' }>;
export type RailroadSequenceExpr = Extract<IrExpression, { $type: 'RailroadSequenceExpr' }>;
export type RailroadSpecialExpr = Extract<IrExpression, { $type: 'RailroadSpecialExpr' }>;

// Mermaid's railroad database is a module-level singleton that its four parsers fill.
// This exposes one Pele model through the names the specs call on it.
class Db extends RailroadDb {
  clear(): void {
    Object.assign(this, new RailroadDb());
  }

  getTitle(): string {
    return this.title ?? '';
  }

  getDiagramTitle(): string {
    return this.getTitle();
  }

  setDiagramTitle(text: string): void {
    this.setTitle(text);
  }

  getRules() {
    return this.rules;
  }

  getAccTitle(): string {
    return this.accTitle ?? '';
  }

  getAccDescription(): string {
    return this.accDescr ?? '';
  }
}

export const db = new Db();

function parserFor(notation: Notation) {
  return {
    parse(input: string): void {
      db.clear();
      populate(db, input, notation);
    },
    parser: { yy: db },
  };
}

export const railroadParser = parserFor('railroad');
export const ebnfParser = parserFor('ebnf');
export const abnfParser = parserFor('abnf');
export const pegParser = parserFor('peg');

// Pele detects the four keywords as one diagram type and then picks the notation from the keyword.
function definition(id: string, notation: Notation) {
  return {
    id,
    detector: (text: string): boolean => detect(text) === 'railroad' && notationOf(text) === notation,
    loader: async () => ({ id, diagram: { parser: parserFor(notation), db, renderer: { draw: diagram.render } } }),
  };
}

export const railroad = definition('railroad', 'railroad');
export const railroadEbnf = definition('railroadEbnf', 'ebnf');
export const railroadAbnf = definition('railroadAbnf', 'abnf');
export const railroadPeg = definition('railroadPeg', 'peg');

export function createRailroadServices() {
  return { Railroad: { parser: { LangiumParser: { parse: (input: string) => toResult<IrAst>(parseIr, input) } } } };
}

export class MermaidParseError extends Error {}

export async function parse(_type: 'railroad', text: string): Promise<IrAst> {
  try {
    return parseIr(text);
  } catch (error) {
    if (error instanceof PeleError) throw new MermaidParseError(error.message);
    throw error;
  }
}
