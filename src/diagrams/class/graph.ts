import { RELATION_TYPE } from './db.js';
import type { ClassModel, ClassNode, RelationEnd } from './types.js';

export type NodeShape = 'rect' | 'classBox' | 'note' | 'interface';
export type EndMarker = 'aggregation' | 'extension' | 'composition' | 'dependency' | 'lollipop' | 'none';

export interface GraphNode {
  id: string;
  label: string;
  isGroup: boolean;
  shape: NodeShape;
  parentId: string | undefined;
  // Index of the namespace node this one is drawn in, or -1.
  parent: number;
  classNode?: ClassNode;
  // Position among the classes, in declaration order. Namespaces and notes have none.
  colorIndex?: number;
}

export interface GraphEdge {
  id: string;
  start: string;
  end: string;
  // Indexes into the node list. Ids alone are ambiguous: a class may be named `note0` or `interface0`.
  source: number;
  target: number;
  label: string | undefined;
  arrowTypeStart: EndMarker;
  arrowTypeEnd: EndMarker;
  startLabelRight: string;
  endLabelLeft: string;
  pattern: 'solid' | 'dashed' | 'dotted';
  classes: string;
}

export interface ClassGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

const MARKERS: EndMarker[] = ['aggregation', 'extension', 'composition', 'dependency', 'lollipop'];

function marker(type: RelationEnd): EndMarker {
  return (typeof type === 'number' && MARKERS[type]) || 'none';
}

// Resolves a parsed class diagram into the nodes and edges that are drawn. With `hierarchical`
// off, only namespaces the source declares are kept and each is labeled with its full name.
export function buildClassGraph(model: ClassModel, hierarchical = true): ClassGraph {
  const nodes: GraphNode[] = [];
  const edges: GraphEdge[] = [];
  const namespaceAt = new Map<string, number>();
  const classAt = new Map<string, number>();
  const interfaceAt = new Map<string, number>();

  const declared = new Map<string, string | undefined>();
  const declaredAncestor = (id: string | undefined): string | undefined => {
    if (hierarchical || id === undefined) return id;
    const chain: string[] = [];
    let current: string | undefined = id;
    let found: string | undefined;
    while (current) {
      if (declared.has(current)) {
        found = declared.get(current);
        break;
      }
      const ns = model.namespaces.get(current);
      if (ns === undefined) break;
      if (ns.explicit) {
        found = current;
        break;
      }
      chain.push(current);
      current = ns.parent;
    }
    for (const visited of chain) declared.set(visited, found);
    return found;
  };
  const group = (id: string | undefined): number => (id === undefined ? -1 : (namespaceAt.get(id) ?? -1));

  for (const ns of model.namespaces.values()) {
    if (!hierarchical && !ns.explicit) continue;
    namespaceAt.set(ns.id, nodes.length);
    nodes.push({
      id: ns.id,
      label: hierarchical ? ns.label : ns.id,
      isGroup: true,
      shape: 'rect',
      parentId: hierarchical ? ns.parent : undefined,
      parent: -1,
    });
  }
  for (const node of nodes) node.parent = group(node.parentId);

  let colorIndex = 0;
  for (const classNode of model.classes.values()) {
    const parentId = declaredAncestor(classNode.parent);
    classAt.set(classNode.id, nodes.length);
    nodes.push({
      id: classNode.id,
      label: classNode.label,
      isGroup: false,
      shape: 'classBox',
      parentId,
      parent: group(parentId),
      classNode,
      colorIndex: colorIndex++,
    });
  }

  for (const note of model.notes.values()) {
    const parentId = declaredAncestor(note.parent);
    const target = note.class === undefined ? undefined : classAt.get(note.class);
    if (target !== undefined) {
      edges.push({
        id: `edgeNote${note.index}`,
        start: note.id,
        end: nodes[target].id,
        source: nodes.length,
        target,
        label: undefined,
        arrowTypeStart: 'none',
        arrowTypeEnd: 'none',
        startLabelRight: '',
        endLabelLeft: '',
        pattern: 'dotted',
        classes: 'relation',
      });
    }
    nodes.push({ id: note.id, label: note.text, isGroup: false, shape: 'note', parentId, parent: group(parentId) });
  }

  // Mermaid leaves every lollipop interface outside the namespaces. Here it stays beside its class.
  for (const item of model.interfaces) {
    const owner = classAt.get(item.classId);
    interfaceAt.set(item.id, nodes.length);
    nodes.push({
      id: item.id,
      label: item.label,
      isGroup: false,
      shape: 'interface',
      parentId: undefined,
      parent: owner === undefined ? -1 : nodes[owner].parent,
    });
  }

  let count = 0;
  for (const relation of model.relations) {
    count++;
    const { type1, type2 } = relation.relation;
    // addRelation swaps in an interface for the lollipop end of a relation with no other marker.
    const startAt = type1 === RELATION_TYPE.LOLLIPOP && type2 === 'none' ? interfaceAt : classAt;
    const endAt = startAt === classAt && type2 === RELATION_TYPE.LOLLIPOP && type1 === 'none' ? interfaceAt : classAt;
    edges.push({
      id: `id_${relation.id1}_${relation.id2}_${count}`,
      start: relation.id1,
      end: relation.id2,
      source: startAt.get(relation.id1) ?? -1,
      target: endAt.get(relation.id2) ?? -1,
      label: relation.title,
      arrowTypeStart: marker(type1),
      arrowTypeEnd: marker(type2),
      startLabelRight: relation.relationTitle1 === 'none' ? '' : relation.relationTitle1,
      endLabelLeft: relation.relationTitle2 === 'none' ? '' : relation.relationTitle2,
      pattern: relation.relation.lineType === 1 ? 'dashed' : 'solid',
      classes: 'relation',
    });
  }

  return { nodes, edges };
}
