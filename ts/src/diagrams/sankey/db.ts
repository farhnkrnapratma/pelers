export interface SankeyNode {
  id: string;
  index: number;
}

export interface SankeyLink {
  source: SankeyNode;
  target: SankeyNode;
  value: number;
}

export interface SankeyModel {
  type: 'sankey';
  nodes: SankeyNode[];
  links: SankeyLink[];
  title?: string;
  accTitle?: string;
  accDescr?: string;
}

export class SankeyDb implements SankeyModel {
  readonly type = 'sankey' as const;
  nodes: SankeyNode[] = [];
  links: SankeyLink[] = [];
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;

  private lookup = new Map<string, SankeyNode>();

  findOrCreateNode(id: string): SankeyNode {
    let node = this.lookup.get(id);
    if (node === undefined) {
      node = { id, index: this.nodes.length };
      this.lookup.set(id, node);
      this.nodes.push(node);
    }
    return node;
  }

  addLink(source: SankeyNode, target: SankeyNode, value: number): void {
    this.links.push({ source, target, value });
  }
}
