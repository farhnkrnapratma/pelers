import { Relationships } from './db.js';
import type { Requirement, RequirementElement, RequirementModel } from './types.js';

export interface GraphNode {
  id: string;
  requirement: Requirement | undefined;
  element: RequirementElement | undefined;
  cssStyles: string[];
  cssClasses: string;
}

export interface GraphEdge {
  id: string;
  start: string | undefined;
  end: string | undefined;
  type: string;
  contains: boolean;
}

export interface RequirementGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  direction: string;
}

// The nodes and edges to lay out: requirements first, then elements, as Mermaid orders them.
export function buildRequirementGraph(model: RequirementModel): RequirementGraph {
  const nodes: GraphNode[] = [];
  for (const requirement of model.requirements.values()) {
    nodes.push({
      id: requirement.name,
      requirement,
      element: undefined,
      cssStyles: requirement.cssStyles,
      cssClasses: requirement.classes.join(' '),
    });
  }
  for (const element of model.elements.values()) {
    nodes.push({
      id: element.name,
      requirement: undefined,
      element,
      cssStyles: element.cssStyles,
      cssClasses: element.classes.join(' '),
    });
  }
  const has = (id: string): string | undefined =>
    model.requirements.has(id) || model.elements.has(id) ? id : undefined;
  const edges = model.relations.map((relation, index) => ({
    id: `${relation.src}-${relation.dst}-${index}`,
    start: has(relation.src),
    end: has(relation.dst),
    type: relation.type,
    contains: relation.type === Relationships.CONTAINS,
  }));
  return { nodes, edges, direction: model.direction };
}
