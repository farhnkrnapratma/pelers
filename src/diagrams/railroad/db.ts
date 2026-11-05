import { PeleError } from '../../errors.js';
import { abnfRules, parseAbnf } from './abnf.js';
import { ebnfRules, parseEbnf } from './ebnf.js';
import { irRules, parseIr } from './ir.js';
import { parsePeg, pegRules } from './peg.js';
import type { Notation, RailroadModel, RailroadRule } from './types.js';

export class RailroadDb implements RailroadModel {
  readonly type = 'railroad' as const;
  notation: Notation = 'railroad';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  rules: RailroadRule[] = [];
  private byName = new Map<string, RailroadRule>();

  setTitle(text: string): void {
    this.title = text;
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }

  // A name defined again is drawn again; looking the name up gives the later definition.
  addRule(rule: RailroadRule): void {
    this.rules.push(rule);
    this.byName.set(rule.name, rule);
  }

  getRule(name: string): RailroadRule | undefined {
    return this.byName.get(name);
  }
}

const RE_KEYWORD = /^\s*railroad-(ebnf-|abnf-|peg-)?beta/i;

// Mermaid has a detector per keyword, each matching without regard to case.
export function notationOf(source: string): Notation | undefined {
  const m = RE_KEYWORD.exec(source);
  if (m === null) return undefined;
  const infix = m[1]?.toLowerCase();
  return infix === 'ebnf-' ? 'ebnf' : infix === 'abnf-' ? 'abnf' : infix === 'peg-' ? 'peg' : 'railroad';
}

interface Parsed {
  title?: string;
  accTitle?: string;
  accDescr?: string;
}

function grammar(source: string, notation: Notation): [Parsed, RailroadRule[]] {
  switch (notation) {
    case 'ebnf': {
      const ast = parseEbnf(source);
      return [ast, ebnfRules(ast)];
    }
    case 'abnf': {
      const ast = parseAbnf(source);
      return [ast, abnfRules(ast)];
    }
    case 'peg': {
      const ast = parsePeg(source);
      return [ast, pegRules(ast)];
    }
    default: {
      const ast = parseIr(source);
      return [ast, irRules(ast)];
    }
  }
}

// Parses the text in one notation and loads the result into the model, as Mermaid's four parsers do.
export function populate(db: RailroadDb, source: string, notation: Notation): void {
  const [ast, rules] = grammar(source, notation);
  db.notation = notation;
  if (ast.accDescr) db.setAccDescription(ast.accDescr);
  if (ast.accTitle) db.setAccTitle(ast.accTitle);
  if (ast.title) db.setTitle(ast.title);
  for (const rule of rules) db.addRule(rule);
}

export function parseRailroad(source: string, title: string | undefined): RailroadDb {
  const notation = notationOf(source);
  if (notation === undefined) throw new PeleError('No railroad diagram keyword found.', 'syntax', { type: 'railroad' });
  const db = new RailroadDb();
  if (title) db.title = title;
  populate(db, source, notation);
  return db;
}
