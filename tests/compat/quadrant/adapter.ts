import { QuadrantDb, parseStyles, type QuadrantText } from '../../../src/diagrams/quadrant/db.js';
import { parseQuadrant } from '../../../src/diagrams/quadrant/parser.js';

export const parser = {
  yy: undefined as unknown as QuadrantDb,
  parse(src: string): void {
    parseQuadrant(src, parser.yy);
  },
};

export function getConfig(): object {
  return {};
}

let db = new QuadrantDb();

// Mermaid's quadrant database is a module-level singleton; this one holds a model that `clear` replaces.
export default {
  parseStyles,
  clear(): void {
    db = new QuadrantDb();
  },
  setXAxisLeftText(label: QuadrantText): void {
    db.setXAxisLeftText(label);
  },
};
