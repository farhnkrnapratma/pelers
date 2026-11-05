import {
  Arrow,
  type Direction,
  type GraphStatement,
  type Span,
  type UsecaseModel,
} from '../../src/diagrams/usecase/types.js';

export interface GraphNode {
  label?: string;
  shape?: string;
  classes?: string[];
  styles?: string[];
  attrs?: Record<string, unknown>;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  label?: string;
  classes?: string[];
  styles?: string[];
  attrs?: Record<string, unknown>;
}

export interface GraphGroup {
  title?: string;
  nodes: string[];
  classes?: string[];
  styles?: string[];
  attrs?: Record<string, unknown>;
}

// Mermaid's serializable view of a use case diagram: what each id resolved to, and the source
// statements with the positions of their parts.
export interface GraphAst {
  version: 1;
  diagramType: 'usecase';
  source: string;
  header: { keyword: 'usecase'; direction: Exclude<Direction, 'TD'>; span: Span };
  accTitle?: string;
  accDescr?: string;
  nodes: Map<string, GraphNode>;
  edges: GraphEdge[];
  groups: Map<string, GraphGroup>;
  classDefs: Map<string, { styles: string[] }>;
  statements: GraphStatement[];
}

const ACTOR_SHAPES = { normal: 'actor', hollow: 'actor-hollow', awesome: 'actor-awesome', icon: 'actor-icon' };

function styled<V extends { classes?: string[]; styles?: string[] }>(
  out: V,
  from: { classes: string[]; styles: string[] }
): V {
  if (from.classes.length > 0) out.classes = [...from.classes];
  if (from.styles.length > 0) out.styles = [...from.styles];
  return out;
}

export function buildUsecaseAst(model: UsecaseModel): GraphAst {
  const nodes = new Map<string, GraphNode>();
  for (const actor of model.actors.values()) {
    const attrs: Record<string, unknown> = {
      kind: 'actor',
      actorType: actor.type,
      business: actor.business,
      labelType: actor.labelType,
    };
    if (actor.icon) attrs.icon = actor.icon;
    if (actor.stereotype) attrs.stereotype = actor.stereotype;
    if (actor.parentId) attrs.parentId = actor.parentId;
    const node = styled<GraphNode>({ shape: ACTOR_SHAPES[actor.type], attrs }, actor);
    if (actor.label !== actor.id) node.label = actor.label;
    nodes.set(actor.id, node);
  }
  for (const useCase of model.useCases.values()) {
    const attrs: Record<string, unknown> = {
      kind: 'usecase',
      useCaseShape: useCase.shape,
      business: useCase.business,
      labelType: useCase.labelType,
    };
    if (useCase.stereotype) attrs.stereotype = useCase.stereotype;
    if (useCase.parentId) attrs.parentId = useCase.parentId;
    const node = styled<GraphNode>({ shape: useCase.shape, attrs }, useCase);
    if (useCase.label !== useCase.id) node.label = useCase.label;
    nodes.set(useCase.id, node);
  }
  for (const note of model.notes.values()) {
    nodes.set(note.id, {
      label: note.label,
      shape: 'note',
      attrs: { kind: 'note', target: note.target, labelType: note.labelType },
    });
  }
  for (const json of model.jsonNodes.values()) {
    const attrs = { kind: 'json', value: json.value, propertyOrder: json.propertyOrder, labelType: 'text' };
    nodes.set(json.id, styled<GraphNode>({ label: json.id, shape: 'json-table', attrs }, json));
  }

  const edges: GraphEdge[] = [];
  for (const relationship of model.relationships) {
    const attrs: Record<string, unknown> = {
      relationshipType: relationship.type,
      arrowType: relationship.arrowType,
      minlen: relationship.minlen,
      explicitId: relationship.explicitId,
      animate: relationship.animate,
    };
    if (relationship.animation) attrs.animation = relationship.animation;
    if (relationship.labelType) attrs.labelType = relationship.labelType;
    const edge = styled<GraphEdge>(
      { id: relationship.id, source: relationship.source, target: relationship.target, attrs },
      relationship
    );
    if (relationship.label) edge.label = relationship.label;
    edges.push(edge);
  }
  for (const note of model.notes.values()) {
    edges.push({
      id: `${note.id}-edge`,
      source: note.id,
      target: note.target,
      attrs: {
        relationshipType: 'note',
        arrowType: Arrow.LINE_SOLID,
        pattern: 'dotted',
        minlen: 1,
        explicitId: false,
        animate: false,
        internal: true,
      },
    });
  }

  const groups = new Map<string, GraphGroup>();
  for (const boundary of model.systemBoundaries.values()) {
    const attrs = { kind: 'systemBoundary', boundaryType: boundary.type, labelType: boundary.labelType };
    const group = styled<GraphGroup>({ nodes: [...boundary.members], attrs }, boundary);
    if (boundary.label !== boundary.id) group.title = boundary.label;
    groups.set(boundary.id, group);
  }

  const classDefs = new Map<string, { styles: string[] }>();
  for (const definition of model.classDefs.values()) classDefs.set(definition.id, { styles: [...definition.styles] });

  const ast: GraphAst = {
    version: 1,
    diagramType: 'usecase',
    source: model.source,
    header: {
      keyword: 'usecase',
      direction: model.direction === 'TD' ? 'TB' : model.direction,
      span: model.headerSpan,
    },
    nodes,
    edges,
    groups,
    classDefs,
    statements: model.statements,
  };
  if (model.accTitle) ast.accTitle = model.accTitle;
  if (model.accDescription) ast.accDescr = model.accDescription;
  return ast;
}
