import { PeleError } from '../../errors.js';
import type { ArchitectureAst, Side } from './parser.js';

export interface ArchService {
  id: string;
  type: 'service';
  icon?: string;
  iconText?: string;
  title?: string;
  in?: string;
}

export interface ArchJunction {
  id: string;
  type: 'junction';
  in?: string;
}

export type ArchNode = ArchService | ArchJunction;

export interface ArchGroup {
  id: string;
  icon?: string;
  title?: string;
  in?: string;
}

export interface ArchEdge {
  lhsId: string;
  lhsDir: Side;
  lhsInto?: boolean;
  lhsGroup?: boolean;
  rhsId: string;
  rhsDir: Side;
  rhsInto?: boolean;
  rhsGroup?: boolean;
  title?: string;
}

export interface ArchLayoutHint {
  direction: 'row' | 'column';
  members: string[];
}

export interface ArchitectureModel {
  type: 'architecture';
  title?: string;
  accTitle?: string;
  accDescr?: string;
  nodes: Map<string, ArchNode>;
  groups: Map<string, ArchGroup>;
  edges: ArchEdge[];
  layoutHints: ArchLayoutHint[];
}

function fail(message: string): never {
  throw new PeleError(message, 'semantic', { type: 'architecture' });
}

export class ArchitectureDb implements ArchitectureModel {
  readonly type = 'architecture';
  title?: string;
  accTitle?: string;
  accDescr?: string;
  nodes = new Map<string, ArchNode>();
  groups = new Map<string, ArchGroup>();
  edges: ArchEdge[] = [];
  layoutHints: ArchLayoutHint[] = [];
  private registered = new Map<string, 'node' | 'group'>();

  private register(kind: 'service' | 'junction' | 'group', id: string, parent: string | undefined): void {
    if (this.registered.has(id)) {
      fail(`The ${kind} id [${id}] is already in use by another ${this.registered.get(id)}`);
    }
    if (parent !== undefined) {
      if (id === parent) fail(`The ${kind} [${id}] cannot be placed within itself`);
      if (!this.registered.has(parent)) {
        fail(`The ${kind} [${id}]'s parent does not exist. Please make sure the parent is created before this ${kind}`);
      }
      if (this.registered.get(parent) === 'node') fail(`The ${kind} [${id}]'s parent is not a group`);
    }
    this.registered.set(id, kind === 'group' ? 'group' : 'node');
  }

  addService({ id, icon, iconText, title, in: parent }: Omit<ArchService, 'type'> & { type?: 'service' }): void {
    this.register('service', id, parent);
    this.nodes.set(id, { id, type: 'service', icon, iconText, title, in: parent });
  }

  addJunction({ id, in: parent }: Omit<ArchJunction, 'type'> & { type?: 'junction' }): void {
    this.register('junction', id, parent);
    this.nodes.set(id, { id, type: 'junction', in: parent });
  }

  addGroup({ id, icon, title, in: parent }: ArchGroup): void {
    this.register('group', id, parent);
    this.groups.set(id, { id, icon, title, in: parent });
  }

  addEdge({ lhsId, rhsId, lhsDir, rhsDir, lhsInto, rhsInto, lhsGroup, rhsGroup, title }: ArchEdge): void {
    const ends = [
      ['left', lhsId, lhsGroup],
      ['right', rhsId, rhsGroup],
    ] as const;
    for (const [side, id] of ends) {
      if (!this.registered.has(id)) {
        fail(
          `The ${side}-hand id [${id}] does not yet exist. Please create the service/group before declaring an edge to it.`
        );
      }
    }
    // Mermaid lets a group id through the check above and then fails with a TypeError.
    for (const [side, id] of ends) {
      if (!this.nodes.has(id))
        fail(`The ${side}-hand id [${id}] is a group. Add {group} to a service inside it instead.`);
    }
    const parent = this.nodes.get(lhsId)!.in;
    if (parent !== undefined && parent === this.nodes.get(rhsId)!.in) {
      for (const [side, id, crosses] of ends) {
        if (crosses)
          fail(
            `The ${side}-hand id [${id}] is modified to traverse the group boundary, but the edge does not pass through two groups.`
          );
      }
    }
    this.edges.push({ lhsId, lhsDir, lhsInto, lhsGroup, rhsId, rhsDir, rhsInto, rhsGroup, title });
  }

  addLayoutHint(hint: ArchLayoutHint): void {
    if (hint.members.length < 2) fail(`An align directive requires at least two members; got ${hint.members.length}`);
    const seen = new Set<string>();
    for (const id of hint.members) {
      if (this.registered.get(id) !== 'node') {
        fail(`align ${hint.direction} references [${id}], which is not a service or junction`);
      }
      if (seen.has(id)) fail(`align ${hint.direction} lists [${id}] more than once`);
      seen.add(id);
    }
    this.layoutHints.push(hint);
  }
}

// Mermaid's populateDb: every group, then services, junctions, edges, and alignments.
export function populate(ast: ArchitectureAst, db: ArchitectureDb): void {
  if (ast.accDescr) db.accDescr = ast.accDescr.replace(/\n\s+/g, '\n');
  if (ast.accTitle) db.accTitle = ast.accTitle.replace(/^\s+/g, '');
  if (ast.title) db.title = ast.title;
  for (const group of ast.groups) db.addGroup(group);
  for (const service of ast.services) db.addService(service);
  for (const junction of ast.junctions) db.addJunction(junction);
  for (const edge of ast.edges) db.addEdge(edge);
  for (const alignment of ast.alignments)
    db.addLayoutHint({ direction: alignment.direction, members: [...alignment.members] });
}
