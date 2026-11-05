import { NODE_TYPE, getType } from '../../../src/diagrams/common/outline.js';
import { KanbanDb, type KanbanNode as Node } from '../../../src/diagrams/kanban/db.js';
import { parseKanban } from '../../../src/diagrams/kanban/parser.js';

export function setLogLevel(_level: unknown): void {}

export type KanbanNode = Node & { parentId?: string; isGroup: boolean };

let db = new KanbanDb();

// Mermaid's kanban database is a module-level singleton; this one forwards to the current model.
const kanbanDB = {
  clear(): void {
    db = new KanbanDb();
  },
  nodeType: NODE_TYPE,
  getType,
  addNode: (...args: Parameters<KanbanDb['addNode']>): void => db.addNode(...args),
  decorateNode: (...args: Parameters<KanbanDb['decorateNode']>): void => db.decorateNode(...args),
  getSections: () => db.sections,
  // Mermaid's flat list: each column followed by its cards, which point back at it.
  getData() {
    const nodes: KanbanNode[] = [];
    for (const section of db.sections) {
      nodes.push({ ...section, isGroup: true });
      for (const item of section.items) nodes.push({ ...item, parentId: section.id, isGroup: false });
    }
    return { nodes, edges: [], other: {}, config: {} };
  },
};

export default kanbanDB;

export const parser = {
  yy: kanbanDB,
  parse(src: string): void {
    parseKanban(src, parser.yy);
  },
};
