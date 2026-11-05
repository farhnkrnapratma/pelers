import { IshikawaDb } from '../../../src/diagrams/ishikawa/db.js';
import { parseIshikawa } from '../../../src/diagrams/ishikawa/parser.js';

export function setLogLevel(_level: string): void {}

// Exposes Pele's Ishikawa model through the names Mermaid's specs call on IshikawaDB.
export class IshikawaDB extends IshikawaDb {
  clear(): void {
    Object.assign(this, new IshikawaDb());
  }

  getRoot() {
    return this.root;
  }
}

export const parser = {
  yy: undefined as unknown as IshikawaDB,
  parse(src: string): void {
    parseIshikawa(src, parser.yy);
  },
};
