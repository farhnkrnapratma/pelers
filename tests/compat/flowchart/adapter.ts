import { FlowDb, type FlowDbOptions } from '../../../src/diagrams/flowchart/db.js';
import { buildFlowGraph } from '../../../src/diagrams/flowchart/graph.js';
import { parseFlowchart } from '../../../src/diagrams/flowchart/parser.js';
import type { FlowEdge, FlowSubgraph, FlowText as Text } from '../../../src/diagrams/flowchart/types.js';

export { cleanupComments } from '../../../src/preprocess.js';

export type FlowSubGraph = FlowSubgraph;
export type FlowText = Text;

interface MermaidConfig {
  securityLevel?: string;
  maxEdges?: number;
  flowchart?: { inheritDir?: boolean; curve?: string };
}

let config: MermaidConfig = {};

export function setConfig(next: MermaidConfig): void {
  config = { ...config, ...next };
}

export const log = {
  debug: (..._args: unknown[]): void => {},
  info: (..._args: unknown[]): void => {},
  warn: (..._args: unknown[]): void => {},
  error: (..._args: unknown[]): void => {},
};

function options(): FlowDbOptions {
  return {
    maxEdges: config.maxEdges ?? 500,
    inheritDir: config.flowchart?.inheritDir,
    warn: (message) => log.warn(message),
  };
}

// Exposes Pele's flowchart model through the method names Mermaid's specs call on FlowDB.
export class FlowDB extends FlowDb {
  constructor() {
    super(options());
  }

  clear(): void {
    Object.assign(this, new FlowDb(options()));
  }

  setGen(): void {}

  getVertices() {
    return this.nodes;
  }

  getEdges() {
    const edges = this.edges as FlowEdge[] & { defaultStyle?: string[]; defaultInterpolate?: string };
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

  getDirection() {
    return this.direction?.trim();
  }

  getTooltip(id: string) {
    return this.tooltips.get(id);
  }

  getAccTitle() {
    return this.accTitle ?? '';
  }

  getAccDescription() {
    return this.accDescr ?? '';
  }

  getData() {
    return { ...buildFlowGraph(this, config.flowchart?.curve), other: {}, config };
  }
}

const parser = {
  yy: undefined as unknown as FlowDB,
  parse(src: string): void {
    parseFlowchart(src, parser.yy);
  },
  parser: undefined as unknown,
};
parser.parser = parser;

export default parser;
