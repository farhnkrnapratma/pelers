import { ClassDb } from '../../../src/diagrams/class/db.js';
import { buildClassGraph } from '../../../src/diagrams/class/graph.js';
import { parseClassDiagram } from '../../../src/diagrams/class/parser.js';
import type { ClassNode, NamespaceNode as Namespace } from '../../../src/diagrams/class/types.js';

export { ClassMember } from '../../../src/diagrams/class/members.js';

export type ClassMap = Map<string, ClassNode>;
export type NamespaceNode = Namespace;

interface MermaidConfig {
  class?: { hierarchicalNamespaces?: boolean };
}

let config: MermaidConfig = {};

export function setConfig(next: MermaidConfig): void {
  config = { ...config, ...next };
}

export function reset(): void {
  config = {};
}

// Exposes Pele's class diagram model through the method names Mermaid's specs call on ClassDB.
export class ClassDB extends ClassDb {
  clear(): void {
    Object.assign(this, new ClassDb());
  }

  getClass(id: string) {
    return this.classes.get(id)!;
  }

  getClasses() {
    return this.classes;
  }

  getRelations() {
    return this.relations;
  }

  getNote(id: string | number) {
    return this.notes.get(typeof id === 'number' ? `note${id}` : id)!;
  }

  getNotes() {
    return this.notes;
  }

  getNamespace(name: string) {
    return this.namespaces.get(name)!;
  }

  getNamespaces() {
    return this.namespaces;
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
    return { ...buildClassGraph(this, config.class?.hierarchicalNamespaces ?? true), other: {}, config, direction: this.direction };
  }
}

export const parser = {
  yy: undefined as unknown as ClassDB,
  parse(src: string): void {
    parseClassDiagram(src, parser.yy);
  },
};
