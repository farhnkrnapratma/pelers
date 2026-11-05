import { PeleError } from '../../../src/errors.js';
import * as slots from '../../../src/diagrams/agentflow/colorSlots.js';
import { AgentflowDb, type AgentflowDbOptions } from '../../../src/diagrams/agentflow/db.js';
import { AgentflowWarning as Vocabulary, EMITTED, RESERVED } from '../../../src/diagrams/agentflow/diagnostics.js';
import { buildAgentGraph, type GraphEdge, type GraphNode } from '../../../src/diagrams/agentflow/graph.js';
import * as lookup from './lookup.js';
import { parseAgentflow } from '../../../src/diagrams/agentflow/parser.js';
import { normaliseNodeShapes, type ShapeSink } from '../../../src/diagrams/agentflow/shapes.js';
import type { AgentEdge, AgentNode, Diagnostic, SemanticModel } from '../../../src/diagrams/agentflow/types.js';
import { encodeEntities, preprocess } from '../../../src/preprocess.js';

export type AgentflowDiagnostic = Diagnostic;
export type AgentflowSemanticModel = SemanticModel;
export interface LayoutData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export const AgentflowWarning = Vocabulary;
export const PARSER_EMITTED_DIAGNOSTICS = EMITTED;
export const RESERVED_DIAGNOSTICS = RESERVED;
export const { KIND_COUNT, KIND_SLOT, containerSlot, containerSlotCount } = slots;

interface MermaidConfig {
  securityLevel?: string;
  maxEdges?: number;
  theme?: string;
  themeVariables?: { borderColorArray?: unknown };
  flowchart?: { inheritDir?: boolean; curve?: string };
}

let config: MermaidConfig = {};

export function setConfig(next: MermaidConfig): void {
  config = { ...config, ...next };
}

export function setSiteConfig(next: MermaidConfig): void {
  config = { ...next };
}

export function reset(): void {}

export function getConfig(): MermaidConfig {
  return { ...config, themeVariables: config.themeVariables ?? {} };
}

export function addDiagrams(): void {}

export function setLogLevel(_level: string): void {}

export const log = {
  debug: (..._args: unknown[]): void => {},
  info: (..._args: unknown[]): void => {},
  warn: (..._args: unknown[]): void => {},
  error: (..._args: unknown[]): void => {},
};

// Mermaid reads the palette from its theme. Its redux-color theme, the agentflow default, has twelve colors.
function paletteLength(): number {
  const palette = config.themeVariables?.borderColorArray;
  if (Array.isArray(palette)) return palette.length;
  return config.theme === 'redux-color' ? 12 : 0;
}

function options(): AgentflowDbOptions {
  return {
    maxEdges: config.maxEdges ?? 500,
    inheritDir: config.flowchart?.inheritDir,
    log: (severity, message) => (severity === 'error' ? log.error(message) : log.warn(message)),
  };
}

// Exposes Pele's agentflow model through the method names Mermaid's specs call on AgentFlowDB.
export class AgentFlowDB extends AgentflowDb {
  readonly preserveCommentsWhenParsing = true;

  constructor() {
    super(options());
  }

  clear(): void {
    Object.assign(this, new AgentflowDb(options()));
  }

  setGen(): void {}

  setFrontmatterLineOffset(offset: number): void {
    this.lineOffset = offset ?? 0;
  }

  getVertices() {
    return this.nodes;
  }

  getEdges() {
    const edges = this.edges as AgentEdge[] & { defaultStyle?: string[]; defaultInterpolate?: string };
    edges.defaultStyle = this.defaultEdgeStyle;
    edges.defaultInterpolate = this.defaultInterpolate;
    return edges;
  }

  getClasses() {
    return this.classes;
  }

  getSubGraphs() {
    return this.subgraphs;
  }

  getConnectors() {
    return [...this.connectors.values()];
  }

  isToolDefinition(vertex: AgentNode) {
    return lookup.isToolDefinition(vertex);
  }

  getTools() {
    return lookup.tools(this);
  }

  getDirection() {
    return this.direction?.trim();
  }

  getTooltip(id: string) {
    return this.tooltips.get(id);
  }

  getData() {
    return {
      ...buildAgentGraph(this, paletteLength(), config.flowchart?.curve),
      other: {},
      config,
      connectors: this.getConnectors(),
    };
  }

  getSemanticModel() {
    return this.semanticModel();
  }

  getDiagnostics() {
    return [...this.diagnostics];
  }

  getElementMappings() {
    return [...this.mappings];
  }

  getElementById(id: string) {
    return lookup.elementById(this, id);
  }

  getElementsOnLine(line: number) {
    return lookup.elementsOnLine(this, line);
  }

  getElementAtPosition(line: number, column: number) {
    return lookup.elementAt(this, line, column);
  }

  getMappingStats() {
    return lookup.mappingStats(this);
  }
}

const logSink: ShapeSink = {
  emitError: (_id, message) => log.warn(`agentflow: ${message}`),
  emitWarning: (_id, message) => log.warn(`agentflow: ${message}`),
};

export function transformData(data: LayoutData, db?: AgentFlowDB): void {
  normaliseNodeShapes(data.nodes, db ?? logSink);
}

// Mermaid marks a node's kind with a class its stylesheet matches; Pele keeps the kind on the node.
export function assignColorSlots(
  nodes: { id: string; shape?: string; isGroup?: boolean; colorIndex?: number; cssClasses?: string }[],
  kindOf: (id: string) => string | undefined,
  order: ReadonlyMap<string, number>
): void {
  const copies = nodes.map((n) => ({ id: String(n.id), shape: n.shape, isGroup: n.isGroup }));
  slots.assignColorSlots(copies, kindOf, order, paletteLength());
  copies.forEach((copy: { colorIndex?: number; kind?: string }, i) => {
    if (copy.colorIndex !== undefined) nodes[i].colorIndex = copy.colorIndex;
    if (copy.kind !== undefined) {
      nodes[i].cssClasses = `${nodes[i].cssClasses ?? ''} af-kind-${copy.kind}`.replace(/\s+/g, ' ').trim();
    }
  });
}

export function getStyles(): string {
  throw new Error('Pele writes no stylesheet.');
}

// Presents a PeleError's position the way Jison attaches one to its errors.
function located<R>(run: () => R): R {
  try {
    return run();
  } catch (error) {
    if (error instanceof PeleError && error.line > 0) {
      Object.assign(error, {
        hash: {
          line: error.line - 1,
          loc: {
            first_line: error.line,
            last_line: error.line,
            first_column: error.column - 1,
            last_column: error.column,
          },
        },
      });
    }
    throw error;
  }
}

const parser = {
  yy: undefined as unknown as AgentFlowDB,
  // The generated parser, without the wrapper that trims whitespace after a closing brace.
  parse(src: string): void {
    located(() => parseAgentflow(src, parser.yy, true));
  },
};

export default {
  parser,
  parse(src: string): void {
    located(() => parseAgentflow(src, parser.yy));
  },
};

export const Diagram = {
  // What Pele's own entry point does with the text before handing it to the agentflow parser.
  async fromText(text: string): Promise<{ db: AgentFlowDB }> {
    const pre = preprocess(text);
    const db = new AgentFlowDB();
    db.lineOffset = pre.lineOffset;
    parseAgentflow(encodeEntities(pre.withComments) + '\n', db);
    return { db };
  },
};
