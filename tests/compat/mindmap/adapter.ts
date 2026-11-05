import { NODE_TYPE } from '../../../src/diagrams/common/outline.js';
import { MindmapDb, type MindmapNode } from '../../../src/diagrams/mindmap/db.js';
import { parseMindmap } from '../../../src/diagrams/mindmap/parser.js';
import { getConfig } from './config.js';

export function setLogLevel(_level: unknown): void {}

// Fields Mermaid's renderer writes onto nodes; the getData specs set some by hand.
type LayoutFields = { width?: number; height?: number; padding?: number; x?: number; y?: number };

export interface MindmapLayoutNode extends LayoutFields {
  id: string;
  label: string;
  level: number;
  nodeId: string;
  type: number;
  section: number | undefined;
  cssClasses: string;
  icon: string | undefined;
}

export interface MindmapLayoutEdge {
  id: string;
  start: string;
  end: string;
  depth: number;
  section: number | undefined;
  classes: string;
}

export type Edge = MindmapLayoutEdge;

// Exposes Pele's mindmap model through the names Mermaid's specs call on MindmapDB.
export class MindmapDB extends MindmapDb {
  readonly nodeType = NODE_TYPE;

  clear(): void {
    Object.assign(this, new MindmapDb());
  }

  getMindmap(): MindmapNode | null {
    return this.nodes[0] ?? null;
  }

  // Mermaid's flat list of nodes and edges, with its class names.
  getData() {
    const root = this.getMindmap();
    const config = getConfig();
    if (!root) return { nodes: [], edges: [], config };
    const nodes: MindmapLayoutNode[] = [];
    const edges: MindmapLayoutEdge[] = [];
    const visit = (node: MindmapNode & LayoutFields): void => {
      // Mermaid recomputes sections here, so the root has none whatever a caller wrote on it.
      const section = node.isRoot ? undefined : node.section;
      nodes.push({
        id: String(node.id),
        label: node.descr,
        level: node.level,
        nodeId: node.nodeId,
        type: node.type,
        section,
        cssClasses:
          'mindmap-node' +
          (node.isRoot ? ' section-root section--1' : section !== undefined ? ` section-${section}` : '') +
          (node.class ? ' ' + node.class : ''),
        icon: node.icon,
        width: node.width,
        height: node.height,
        padding: node.padding,
        x: node.x,
        y: node.y,
      });
      for (const child of node.children) {
        edges.push({
          id: `edge_${node.id}_${child.id}`,
          start: String(node.id),
          end: String(child.id),
          depth: node.level,
          section: child.section,
          classes: `edge section-edge-${child.section} edge-depth-${node.level + 1}`,
        });
        visit(child);
      }
    };
    visit(root);
    return { nodes, edges, config, rootNode: root };
  }
}

export const parser = {
  yy: undefined as unknown as MindmapDB,
  parse(src: string): void {
    parseMindmap(src, parser.yy);
  },
};
