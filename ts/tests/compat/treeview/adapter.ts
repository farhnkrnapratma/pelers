import { BUILTIN_PACK, GLYPHS, iconConfig } from '../../../src/diagrams/treeview/icons.js';
import { TreeViewDb, populate, readTreeView, type NodeType } from '../../../src/diagrams/treeview/model.js';
import { convertValue, parseTreeView, type TreeViewAst } from '../../../src/diagrams/treeview/parser.js';
import { treeSettings } from '../../../src/diagrams/treeview/render.js';
import { toResult } from '../../support/langium.js';

export { expectNoErrorsOrAlternatives } from '../../support/langium.js';
export { isBoxDrawingFormat, preprocessBoxDrawing, remapErrorLines } from '../../../src/diagrams/treeview/boxDrawing.js';
export { detectIcon, getNodeIcon } from '../../../src/diagrams/treeview/icons.js';

// Mermaid's built-in icons as an Iconify pack; Pele keeps only the path data.
export const treeViewIcons = {
  prefix: BUILTIN_PACK,
  icons: Object.fromEntries([...GLYPHS].map(([name, d]) => [name, { body: `<path d="${d}"/>` }])),
};

export const createTreeViewServices = () => ({
  TreeView: { parser: { LangiumParser: { parse: <T>(src: string) => toResult(parseTreeView, src) as { value: T & TreeViewAst; lexerErrors: unknown[]; parserErrors: unknown[] } } } },
});

export class TreeViewValueConverter {
  protected runCustomConverter(rule: { name: string }, input: string, _cstNode: unknown): string | number | undefined {
    return convertValue(rule.name, input);
  }
}

let state = new TreeViewDb();
let accTitle = '';
let accDescription = '';
let diagramTitle = '';

// Mermaid's treeView database is a module-level singleton.
const db = {
  clear(): void {
    state = new TreeViewDb();
    accTitle = accDescription = diagramTitle = '';
  },
  addNode: (level: number, name: string, nodeType: NodeType, cssClass?: string, icon?: string, description?: string) =>
    state.addNode(level, name, nodeType, cssClass, icon, description),
  getRoot: () => state.root,
  getCount: () => state.count,
  getConfig: () => ({ ...treeSettings({}), ...iconConfig({}) }),
  getAccTitle: () => accTitle,
  setAccTitle: (text: string) => void (accTitle = text),
  getAccDescription: () => accDescription,
  setAccDescription: (text: string) => void (accDescription = text),
  getDiagramTitle: () => diagramTitle,
  setDiagramTitle: (text: string) => void (diagramTitle = text),
};

export default db;

export const parser = {
  async parse(src: string): Promise<void> {
    populate(readTreeView(src), state);
  },
};
