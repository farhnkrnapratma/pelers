import { StateDb } from '../../../src/diagrams/state/db.js';
import { parseState } from '../../../src/diagrams/state/parser.js';

export const DEFAULT_DIAGRAM_DIRECTION = 'TB';

let config: Record<string, unknown> = {};

export function setConfig(next: Record<string, unknown>): void {
  config = { ...config, ...next };
}

// Exposes Pele's state diagram model through the method names Mermaid's specs call on StateDB.
export class StateDB extends StateDb {
  constructor(_version?: number) {
    super();
  }

  clear(): void {
    Object.assign(this, new StateDb());
  }

  getStates() {
    return this.states;
  }

  getRelations() {
    return this.relations;
  }

  getClasses() {
    return this.classes;
  }

  getLinks() {
    return this.links;
  }

  getAccTitle() {
    return this.accTitle ?? '';
  }

  getAccDescription() {
    return this.accDescr ?? '';
  }

  getData() {
    return { nodes: this.graph.nodes, edges: this.graph.edges, other: {}, config, direction: this.direction };
  }
}

export const parser = {
  yy: undefined as unknown as StateDB,
  parse(src: string): void {
    parseState(src, parser.yy);
  },
  parser: undefined as unknown,
};
parser.parser = parser;

export default parser;
