import { parseOutline, tokenize as tokenizeOutline, type OutlineDb, type Tokens } from '../common/outline.js';

export function tokenize(src: string): Tokens {
  return tokenizeOutline(src, false);
}

export function parseMindmap(src: string, db: OutlineDb): void {
  parseOutline(src, db, false);
}
