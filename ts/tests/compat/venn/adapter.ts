import { VennDb } from '../../../src/diagrams/venn/db.js';
import { parseVenn } from '../../../src/diagrams/venn/parser.js';

export { ensurePairwiseSubsets } from '../../../src/diagrams/venn/layout.js';

export type Diagram = unknown;

// Mermaid's renderer, which the specs that are skipped draw with.
export function draw(): void {}

// Exposes Pele's venn model through the names Mermaid's specs call on its DB.
class VennDB extends VennDb {
  getLogger: unknown;

  clear(): void {
    Object.assign(this, new VennDb());
  }

  getDiagramTitle() {
    return this.title ?? '';
  }

  getSubsetData() {
    return this.subsets;
  }

  getTextData() {
    return this.textNodes;
  }

  // Mermaid keeps the declarations of a style as a plain object.
  getStyleData() {
    return this.styles.map(({ targets, styles }) => ({ targets, styles: Object.fromEntries(styles) }));
  }
}

export const db = new VennDB();

const parser = {
  yy: db,
  parse(src: string): void {
    parseVenn(src, parser.yy);
  },
  parser: undefined as unknown as { yy: VennDB },
};
parser.parser = parser;

export default parser;
