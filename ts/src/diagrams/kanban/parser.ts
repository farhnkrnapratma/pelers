import { parseOutline, tokenize as tokenizeOutline, type OutlineDb, type Tokens } from '../common/outline.js';

export function tokenize(src: string): Tokens {
  return tokenizeOutline(src, true);
}

export function parseKanban(src: string, db: OutlineDb): void {
  parseOutline(src, db, true);
}
