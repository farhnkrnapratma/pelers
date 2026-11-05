import { SankeyDb } from '../../../src/diagrams/sankey/db.js';
import { parseSankey } from '../../../src/diagrams/sankey/parser.js';

export { prepareTextForParsing } from '../../../src/diagrams/sankey/parser.js';
export { cleanupComments } from '../../../src/preprocess.js';

// Mermaid's sankey database is a module-level singleton with these names.
class SankeyDB extends SankeyDb {
  clear(): void {
    Object.assign(this, new SankeyDb());
  }

  getNodes() {
    return this.nodes;
  }

  getLinks() {
    return this.links;
  }

  getGraph() {
    return {
      nodes: this.nodes.map((node) => ({ id: node.id })),
      links: this.links.map((link) => ({ source: link.source.id, target: link.target.id, value: link.value })),
    };
  }
}

export const db = new SankeyDB();

const parser = {
  yy: undefined as unknown as SankeyDB,
  parse(src: string): void {
    parseSankey(src, parser.yy);
  },
  parser: undefined as unknown,
};
parser.parser = parser;

export default parser;
