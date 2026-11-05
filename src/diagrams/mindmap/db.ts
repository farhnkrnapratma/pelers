import { PeleError } from '../../errors.js';
import { getType } from '../common/outline.js';

export interface MindmapNode {
  id: number;
  nodeId: string;
  level: number;
  descr: string;
  type: number;
  children: MindmapNode[];
  isRoot: boolean;
  // Index of the first-level branch the node belongs to, wrapping after eleven as in Mermaid.
  section?: number;
  class?: string;
  icon?: string;
}

export interface MindmapModel {
  type: 'mindmap';
  title: string | undefined;
  // In source order, which is depth-first. The first node is the root.
  nodes: MindmapNode[];
}

const SECTIONS = 11;

export class MindmapDb implements MindmapModel {
  readonly type = 'mindmap';
  title: string | undefined;
  nodes: MindmapNode[] = [];
  getType = getType;

  private baseLevel = 0;
  // The chain of nodes that can still take children, with strictly increasing levels.
  private open: MindmapNode[] = [];

  addNode(level: number, id: string, descr: string, type: number): void {
    const isRoot = this.nodes.length === 0;
    if (isRoot) this.baseLevel = level;
    level -= this.baseLevel;

    // Mermaid's parent is the last node with a smaller indentation, wherever it is in the tree.
    const open = this.open;
    let depth = open.length;
    while (depth > 0 && open[depth - 1].level >= level) depth--;
    const parent = open[depth - 1];
    if (!parent && !isRoot) {
      throw new PeleError(`There can be only one root. No parent could be found for ("${descr}")`, 'semantic', {
        type: 'mindmap',
      });
    }
    const node: MindmapNode = { id: this.nodes.length, nodeId: id, level, descr, type, children: [], isRoot };
    if (parent) {
      node.section = parent.isRoot ? parent.children.length % SECTIONS : parent.section;
      parent.children.push(node);
    }
    open.length = depth;
    open.push(node);
    this.nodes.push(node);
  }

  // Mermaid throws a TypeError when a decoration comes before any node. Here it is ignored.
  decorateNode(decoration?: { class?: string; icon?: string }): void {
    const node = this.nodes[this.nodes.length - 1];
    if (!decoration || !node) return;
    if (decoration.icon) node.icon = decoration.icon;
    if (decoration.class) node.class = decoration.class;
  }
}
