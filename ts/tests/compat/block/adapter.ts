import { BlockDb } from '../../../src/diagrams/block/db.js';
import { cell, layout as gridLayout, type Cell } from '../../../src/diagrams/block/layout.js';
import { parseBlock, type BlockBuilder } from '../../../src/diagrams/block/parser.js';
import type { Block as ModelBlock, Statement } from '../../../src/diagrams/block/types.js';
import { getConfig, setConfig } from './config.js';

export { calculateBlockPosition } from '../../../src/diagrams/block/layout.js';

// Mermaid keeps a block's measured and assigned box on the block itself.
export interface Block extends ModelBlock {
  children: Block[];
  size?: { width: number; height: number; x: number; y: number };
}

export const log = {
  debug: (..._args: unknown[]): void => {},
  info: (..._args: unknown[]): void => {},
  warn: (..._args: unknown[]): void => {},
  error: (..._args: unknown[]): void => {},
};

export const configApi = {
  setSiteConfig: setConfig,
  reset(): void {},
  getConfig,
};

const create = (): BlockDb => new BlockDb({ warn: (message) => log.warn(message) });

let model = create();

// Mermaid's block database is a module-level singleton. This one exposes the current model
// through the same method names; `root` is not an id in Pele's model, so it is looked up apart.
export const db = {
  clear(): void {
    model = create();
  },
  typeStr2Type: (text: string) => model.typeStr2Type(text),
  edgeStrToEdgeData: (text: string) => model.edgeStrToEdgeData(text),
  edgeStrToEdgeStartData: (text: string) => model.edgeStrToEdgeStartData(text),
  edgeStrToThickness: (text: string) => model.edgeStrToThickness(text),
  edgeStrToPattern: (text: string) => model.edgeStrToPattern(text),
  generateId: () => model.generateId(),
  setHierarchy: (statements: Statement[]) => model.setHierarchy(statements),
  getLogger: (): unknown => log,
  getBlocks: () => model.blocks as Block[],
  getBlocksFlat: () => [model.root, ...model.byId.values()] as Block[],
  getEdges: () => model.edges,
  getClasses: () => model.classes,
  getBlock: (id: string) => (model.byId.get(id) ?? (id === 'root' ? model.root : undefined)) as Block | undefined,
  getColumns(id: string): number {
    const found = db.getBlock(id);
    if (!found) return -1;
    if (found.columns) return found.columns;
    return found.children ? found.children.length : -1;
  },
};

export type BlockDB = typeof db;

export const block = {
  parser: { yy: db as unknown },
  parse(src: string): void {
    parseBlock(src, block.parser.yy as BlockBuilder);
  },
};

function toCell(source: Block): Cell {
  const made = cell(source.size?.width ?? 0, source.size?.height ?? 0, source.widthInColumns ?? 1);
  made.columns = source.columns ?? -1;
  made.space = source.type === 'space';
  made.children = (source.children ?? []).map(toCell);
  return made;
}

// Mermaid's layout reads the tree from the database and the padding from the global config.
export function layout(source: Pick<BlockDB, 'getBlock'>): { x: number; y: number; width: number; height: number } | undefined {
  const root = source.getBlock('root');
  if (!root) return undefined;
  return { x: 0, y: 0, ...gridLayout(toCell(root), getConfig()?.block?.padding ?? 8) };
}

export function getStyles(_options: unknown): string {
  throw new Error('Pele has no stylesheet.');
}
