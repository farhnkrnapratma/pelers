import { ErDb } from '../../../src/diagrams/er/db.js';
import { buildErGraph } from '../../../src/diagrams/er/graph.js';
import { parseEr } from '../../../src/diagrams/er/parser.js';

let config: Record<string, unknown> = {};

export function setConfig(next: Record<string, unknown>): void {
  config = { ...config, ...next };
}

// Exposes Pele's ER model through the names Mermaid's specs call on ErDB.
export class ErDB extends ErDb {
  Cardinality = {
    ZERO_OR_ONE: 'ZERO_OR_ONE',
    ZERO_OR_MORE: 'ZERO_OR_MORE',
    ONE_OR_MORE: 'ONE_OR_MORE',
    ONLY_ONE: 'ONLY_ONE',
    MD_PARENT: 'MD_PARENT',
  };

  Identification = {
    NON_IDENTIFYING: 'NON_IDENTIFYING',
    IDENTIFYING: 'IDENTIFYING',
  };

  clear(): void {
    Object.assign(this, new ErDb());
  }

  getEntity(name: string) {
    return this.entities.get(name);
  }

  getEntities() {
    return this.entities;
  }

  getRelationships() {
    return this.relationships;
  }

  getClasses() {
    return this.classes;
  }

  getSubGraphs() {
    return this.subgraphs;
  }

  getDirection() {
    return this.direction;
  }

  getAccTitle() {
    return this.accTitle ?? '';
  }

  getAccDescription() {
    return this.accDescr ?? '';
  }

  getData() {
    return { ...buildErGraph(this), other: {}, config, direction: this.direction };
  }
}

const parser = {
  yy: undefined as unknown as ErDB,
  parse(src: string): void {
    parseEr(src, parser.yy);
  },
  parser: undefined as unknown,
};
parser.parser = parser;

export default parser;
