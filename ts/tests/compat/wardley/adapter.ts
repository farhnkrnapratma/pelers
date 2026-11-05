import { WardleyBuilder, type WardleyBuildResult } from '../../../src/diagrams/wardley/builder.js';
import { populate } from '../../../src/diagrams/wardley/model.js';
import { parseWardley } from '../../../src/diagrams/wardley/parser.js';

export { WardleyBuilder } from '../../../src/diagrams/wardley/builder.js';

export interface WardleyDb {
  clear(): void;
  getWardleyData(): WardleyBuildResult;
  getDiagramTitle(): string;
}

let builder = new WardleyBuilder();
let title = '';

// Mermaid's Wardley database is a module-level singleton around one builder.
const db: WardleyDb = {
  clear(): void {
    builder = new WardleyBuilder();
    title = '';
  },
  getWardleyData: () => builder.build(),
  getDiagramTitle: () => title,
};

export const diagram = {
  db,
  parser: {
    parser: { yy: undefined as unknown },
    async parse(src: string): Promise<void> {
      const ast = parseWardley(src);
      title = ast.title ?? '';
      populate(ast, builder);
    },
  },
};
