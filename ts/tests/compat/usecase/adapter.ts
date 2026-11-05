import { buildUsecaseGraph } from '../../../src/diagrams/usecase/graph.js';
import { parseUsecase } from '../../../src/diagrams/usecase/parser.js';
import { emptyModel, type UsecaseModel } from '../../../src/diagrams/usecase/types.js';
import { encodeEntities, preprocess } from '../../../src/preprocess.js';
import { mermaidAst } from '../../support/usecase-ast.js';

export const ARROW_TYPE = {
  SOLID_ARROW: 0,
  BACK_ARROW: 1,
  LINE_SOLID: 2,
  CIRCLE_ARROW: 3,
  CROSS_ARROW: 4,
  CIRCLE_ARROW_REVERSED: 5,
  CROSS_ARROW_REVERSED: 6,
} as const;

export type GraphAST = Record<string, unknown>;
export type UsecaseFields = UsecaseModel;

let config: Record<string, unknown> = {};

export function getConfig(): Record<string, unknown> {
  return config;
}

export function setConfig(next: Record<string, unknown>): void {
  config = { ...config, ...next };
}

export function addDiagrams(): void {}

// Pele stores text as written and escapes it when it writes SVG.
export function sanitizeText(text: string, _config?: unknown): string {
  return text;
}

export default function getStyles(_options: unknown): string {
  return '';
}

// Mermaid's use case DB is a module-level singleton that a parse commits into, or leaves empty
// when it fails. This one holds the model of the last parse, or one a spec built by hand.
let model = emptyModel();
let parsed = false;

function labelOf(id: string): string {
  return model.actors.get(id)?.label ?? model.useCases.get(id)?.label ?? model.jsonNodes.get(id)?.id ?? model.notes.get(id)?.label ?? id;
}

export const db = {
  clear(): void {
    model = emptyModel();
    parsed = false;
  },
  createModel: () => emptyModel() as UsecaseModel & { ast?: unknown },
  // Specs write a JSON node's key order as the plain object Mermaid uses.
  commit(next: UsecaseModel): void {
    for (const json of next.jsonNodes.values()) {
      if (!(json.propertyOrder instanceof Map)) json.propertyOrder = new Map(Object.entries(json.propertyOrder));
    }
    model = next;
    parsed = false;
  },
  getAST: () => (parsed ? mermaidAst(model) : undefined) as
    | {
        source: string;
        nodes: Record<string, unknown>;
        edges: { id: string; label?: string; attrs?: Record<string, unknown> }[];
        statements: Record<string, unknown>[];
      }
    | undefined,
  getActors: () => model.actors,
  getActor: (id: string) => model.actors.get(id),
  getUseCases: () => model.useCases,
  getUseCase: (id: string) => model.useCases.get(id),
  getSystemBoundaries: () => model.systemBoundaries,
  getSystemBoundary: (id: string) => model.systemBoundaries.get(id),
  getRelationships: () => model.relationships,
  getNotes: () => model.notes,
  getNote: (id: string) => model.notes.get(id),
  getJsonNodes: () => model.jsonNodes,
  getJsonNode: (id: string) => model.jsonNodes.get(id),
  getClassDefs: () => model.classDefs,
  getClassDef: (id: string) => model.classDefs.get(id),
  getDirection: () => model.direction,
  getAccTitle: () => model.accTitle,
  getAccDescription: () => model.accDescr,
  // Pele's graph under the names Mermaid's layout data uses, with the labels of each edge's ends.
  getData() {
    const graph = buildUsecaseGraph(model);
    return {
      nodes: graph.nodes.map((node) =>
        node.noteTarget === undefined ? node : { ...node, noteTargetLabel: labelOf(node.noteTarget) }
      ),
      edges: graph.edges.map((edge) => ({
        ...edge,
        source: edge.start,
        target: edge.end,
        sourceLabel: labelOf(edge.start),
        targetLabel: labelOf(edge.end),
        type: 'edge',
        isUserDefinedId: edge.explicitId,
      })),
      config,
      type: 'usecase',
      direction: model.direction,
      markers: ['point', 'circle', 'cross', 'extension'],
    };
  },
};

export const getAccTitle = db.getAccTitle;
export const getAccDescription = db.getAccDescription;

export const parser = {
  async parse(source: string): Promise<void> {
    db.clear();
    model = parseUsecase(source);
    parsed = true;
  },
};

export const Diagram = {
  async fromText(text: string): Promise<{ type: string }> {
    const pre = preprocess(text);
    db.clear();
    model = parseUsecase(encodeEntities(pre.text) + '\n', pre.title);
    parsed = true;
    return { type: 'usecase' };
  },
};
