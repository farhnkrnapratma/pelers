import { buildStateGraph, type StateGraph } from './graph.js';
import type { StateBuilder } from './parser.js';
import type { StateClassDef, StateLink, StateModel, StateNote, StateRelation, StateStmt, Stmt } from './types.js';

interface Root {
  stmt: 'root';
  id: string;
  doc: Stmt[];
}

// Mermaid's StateDB in its version 2 form, which is what Mermaid uses for both `stateDiagram`
// and `stateDiagram-v2`. The legacy version, which numbers `[*]` states, is not ported.
export class StateDb implements StateModel, StateBuilder {
  readonly type = 'state' as const;
  rootDoc: Stmt[] = [];
  states = new Map<string, StateStmt>();
  relations: StateRelation[] = [];
  classes = new Map<string, StateClassDef>();
  links = new Map<string, StateLink>();
  graph: StateGraph = { nodes: [], edges: [] };
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;

  private dividerCount = 0;

  // The direction the root document is laid out in: its last `direction` statement.
  get direction(): string {
    let dir = 'TB';
    for (const item of this.rootDoc) if (typeof item !== 'string' && item.stmt === 'dir') dir = item.value;
    return dir;
  }

  setRootDoc(doc: Stmt[]): void {
    this.rootDoc = doc;
    this.translate();
    this.extract(doc);
  }

  // Only the root document's statements become states, relations and classes here; the
  // statements inside composites are read when the graph is built.
  extract(statements: Stmt[] | { doc: Stmt[] }): void {
    this.states = new Map();
    this.relations = [];
    this.classes = new Map();
    const doc = Array.isArray(statements) ? statements : statements.doc;
    for (const item of doc) {
      if (typeof item === 'string') continue;
      switch (item.stmt) {
        case 'state':
          this.addState(item.id.trim(), item.type, item.doc, item.description, item.note);
          break;
        case 'relation':
          this.addRelation(item.state1, item.state2, item.description);
          break;
        case 'classDef':
          this.addStyleClass(item.id.trim(), item.classes);
          break;
        case 'style': {
          const styles = item.styleClass.split(',').map((s) => s.replace(/;/g, '').trim());
          for (const state of this.named(item.id.trim())) state.styles = styles.slice();
          break;
        }
        case 'applyClass':
          this.setCssClass(item.id.trim(), item.styleClass);
          break;
        case 'click':
          this.addLink(typeof item.id === 'string' ? item.id : item.id.id, item.url, item.tooltip);
          break;
      }
    }
    this.graph = buildStateGraph(doc, this.states, this.classes);
  }

  // Mermaid's docTranslator: gives each `[*]` an id scoped to its parent, and turns a document
  // split by `--` into one region per part.
  translate(): void {
    const pending: [string, Stmt | Root, boolean][] = [['root', { stmt: 'root', id: 'root', doc: this.rootDoc }, true]];
    while (pending.length > 0) {
      const [parentId, node, first] = pending.pop()!;
      if (typeof node === 'string') continue;
      if (node.stmt === 'relation') {
        pending.push([parentId, node.state2, false], [parentId, node.state1, true]);
        continue;
      }
      if (node.stmt === 'state') {
        if (node.id === '[*]') {
          node.id = parentId + (first ? '_start' : '_end');
          node.start = first;
        } else {
          node.id = node.id.trim();
        }
      } else if (node.stmt !== 'root') {
        continue;
      }
      if (!node.doc) continue;

      const regions: StateStmt[] = [];
      let current: Stmt[] = [];
      for (const item of node.doc) {
        if (typeof item !== 'string' && item.stmt === 'state' && item.type === 'divider' && !item.doc) {
          regions.push({ ...item, doc: current });
          current = [];
        } else {
          current.push(item);
        }
      }
      // Mermaid names the region after the last divider with a random id; here it continues the
      // divider numbering. Mermaid also leaves a document that ends with a divider ungrouped and
      // then fails to draw it; here the regions before that divider are kept.
      if (regions.length > 0) {
        if (current.length > 0) regions.push({ stmt: 'state', id: this.getDividerId(), type: 'divider', doc: current });
        node.doc = regions;
      }
      for (let i = node.doc.length - 1; i >= 0; i--) pending.push([node.id, node.doc[i], true]);
    }
  }

  addState(
    id: string,
    type = 'default',
    doc?: Stmt[],
    descr?: string | string[],
    note?: StateNote,
    classes?: string[]
  ): void {
    const trimmedId = id.trim();
    let state = this.states.get(trimmedId);
    if (!state) {
      state = {
        stmt: 'state',
        id: trimmedId,
        descriptions: [],
        type,
        doc,
        note,
        classes: [],
        styles: [],
        textStyles: [],
      };
      this.states.set(trimmedId, state);
    } else {
      if (!state.doc) state.doc = doc;
      if (!state.type) state.type = type;
    }
    if (descr) for (const text of Array.isArray(descr) ? descr : [descr]) this.addDescription(trimmedId, text.trim());
    if (note) state.note = note;
    if (classes) for (const name of classes) this.setCssClass(trimmedId, name.trim());
  }

  getState(id: string): StateStmt | undefined {
    return this.states.get(id);
  }

  addLink(stateId: string, url: string, tooltip: string): void {
    this.links.set(stateId, { url, tooltip });
  }

  addRelation(item1: StateStmt, item2: StateStmt, title = ''): void {
    for (const item of [item1, item2])
      this.addState(item.id, item.type, item.doc, item.description, item.note, item.classes);
    this.relations.push({ id1: item1.id.trim(), id2: item2.id.trim(), relationTitle: title });
  }

  addDescription(id: string, descr: string): void {
    this.states.get(id)?.descriptions?.push(descr.startsWith(':') ? descr.replace(':', '').trim() : descr);
  }

  getDividerId(): string {
    return `divider-id-${++this.dividerCount}`;
  }

  addStyleClass(id: string, styleAttributes = ''): void {
    let found = this.classes.get(id);
    if (!found) {
      found = { id, styles: [], textStyles: [] };
      this.classes.set(id, found);
    }
    if (!styleAttributes) return;
    for (const attrib of styleAttributes.split(',')) {
      const fixed = attrib.replace(';', '').trim();
      if (attrib.includes('color')) found.textStyles.push(fixed.replace('fill', 'bgFill').replace('color', 'fill'));
      found.styles.push(fixed);
    }
  }

  setCssClass(itemIds: string, cssClassName: string): void {
    for (const state of this.named(itemIds)) state.classes?.push(cssClassName);
  }

  getDirection(): string {
    for (const item of this.rootDoc) if (typeof item !== 'string' && item.stmt === 'dir') return item.value;
    return 'TB';
  }

  // The grammar calls this while it is still building the document, so the statement lands in the
  // previous root document and the direction is read from the parsed `dir` statements instead.
  setDirection(dir: string): void {
    for (const item of this.rootDoc) {
      if (typeof item !== 'string' && item.stmt === 'dir') {
        item.value = dir;
        return;
      }
    }
    this.rootDoc.unshift({ stmt: 'dir', value: dir });
  }

  trimColon(text: string): string {
    return text.startsWith(':') ? text.slice(1).trim() : text.trim();
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }

  // The states a comma-separated list of ids names, created where they do not exist yet. As in
  // Mermaid, an id is looked up as written and trimmed only when it has to be created.
  private named(ids: string): StateStmt[] {
    const out: StateStmt[] = [];
    for (const id of ids.split(',')) {
      let state = this.states.get(id);
      if (!state) {
        this.addState(id);
        state = this.states.get(id.trim());
      }
      if (state) out.push(state);
    }
    return out;
  }
}
