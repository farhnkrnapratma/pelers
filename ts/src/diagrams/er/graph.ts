import type { ErAttribute, ErModel } from './types.js';

export interface ErGraphNode {
  id: string;
  label: string;
  alias: string;
  attributes: ErAttribute[];
  isGroup: boolean;
  parentId: string | undefined;
  cssStyles: string[];
  cssCompiledStyles: string[];
  cssClasses: string;
  dir: string | undefined;
  labelType: string;
}

export interface ErGraphEdge {
  id: string;
  start: string;
  end: string;
  label: string;
  arrowTypeStart: string;
  arrowTypeEnd: string;
  pattern: 'solid' | 'dashed';
}

export interface ErGraph {
  nodes: ErGraphNode[];
  edges: ErGraphEdge[];
}

const NO_ATTRIBUTES: ErAttribute[] = [];

// Resolves a parsed ER diagram into what is drawn: subgraphs as groups, entities as nodes with
// their class styles, and relationships as edges with a marker type at each end.
export function buildErGraph(model: ErModel): ErGraph {
  const nodes: ErGraphNode[] = [];
  const edges: ErGraphEdge[] = [];
  const subgraphs = model.subgraphs;

  const compiled = (classNames: string[]): string[] => {
    const out: string[] = [];
    for (const name of classNames) {
      const def = model.classes.get(name);
      if (def === undefined) continue;
      for (const style of def.styles) out.push(style.trim());
      for (const style of def.textStyles) out.push(style.trim());
    }
    return out;
  };

  const parentOf = new Map<string, string>();
  const subgraphIds = new Set<string>();
  for (let i = subgraphs.length - 1; i >= 0; i--) {
    const subgraph = subgraphs[i];
    subgraphIds.add(subgraph.id);
    for (const id of subgraph.nodes) parentOf.set(id, subgraph.id);
  }

  for (let i = subgraphs.length - 1; i >= 0; i--) {
    const subgraph = subgraphs[i];
    nodes.push({
      id: subgraph.id,
      label: subgraph.title,
      alias: '',
      attributes: NO_ATTRIBUTES,
      isGroup: true,
      parentId: parentOf.get(subgraph.id),
      cssStyles: subgraph.cssStyles,
      cssCompiledStyles: compiled(subgraph.classes),
      cssClasses: subgraph.classes.join(' '),
      dir: subgraph.dir,
      labelType: subgraph.labelType,
    });
  }

  // A name that is also a subgraph id stands for the subgraph. Mermaid drops such an entity and
  // leaves relationships written before the subgraph pointing at nothing; here they follow it.
  const moved = new Map<string, string>();
  for (const [name, entity] of model.entities) {
    if (subgraphIds.has(name)) {
      moved.set(entity.id, name);
      continue;
    }
    nodes.push({
      id: entity.id,
      label: entity.label,
      alias: entity.alias,
      attributes: entity.attributes,
      isGroup: false,
      parentId: parentOf.get(name),
      cssStyles: entity.cssStyles,
      cssCompiledStyles: compiled(entity.cssClasses.split(' ')),
      cssClasses: entity.cssClasses,
      dir: undefined,
      labelType: 'markdown',
    });
  }

  model.relationships.forEach((relationship, index) => {
    edges.push({
      id: `id_${relationship.entityA}_${relationship.entityB}_${index}`,
      start: moved.get(relationship.entityA) ?? relationship.entityA,
      end: moved.get(relationship.entityB) ?? relationship.entityB,
      label: relationship.roleA,
      arrowTypeStart: relationship.relSpec.cardB.toLowerCase(),
      arrowTypeEnd: relationship.relSpec.cardA.toLowerCase(),
      pattern: relationship.relSpec.relType === 'IDENTIFYING' ? 'solid' : 'dashed',
    });
  });

  return { nodes, edges };
}
