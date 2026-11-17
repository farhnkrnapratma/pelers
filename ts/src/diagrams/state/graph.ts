import { PelersError } from '../../errors.js';
import type { StateClassDef, StateStmt, Stmt } from './types.js';

export interface StateNode {
  id: string;
  // rect, rectWithTitle, stateStart, stateEnd, choice, fork, join, divider, roundedWithTitle, note or noteGroup.
  shape: string | undefined;
  label: string | undefined;
  description: string[];
  cssClasses: string;
  cssCompiledStyles: string[];
  cssStyles: string[];
  type: string | undefined;
  isGroup: boolean;
  parentId?: string;
  // The direction inside a composite: its last `direction` statement, or TB.
  dir: string | undefined;
  position?: string;
  noteFor?: string;
  colorIndex?: number;
}

// Labels are markdown throughout, as in Mermaid.
export interface StateEdge {
  id: string;
  start: string;
  end: string;
  label: string;
  classes: string;
  pattern?: 'dashed';
}

export interface StateGraph {
  nodes: StateNode[];
  edges: StateEdge[];
}

interface Draft {
  shape: string | undefined;
  description: string | string[];
  cssClasses: string;
  cssStyles: string[];
  type: string | undefined;
  dir: string | undefined;
  colorIndex: number | undefined;
}

interface Frame {
  item: StateStmt | undefined;
  doc: Stmt[];
  at: number;
  alt: boolean;
}

function direction(doc: Stmt[]): string {
  let dir = 'TB';
  for (const item of doc) if (typeof item !== 'string' && item.stmt === 'dir') dir = item.value;
  return dir;
}

// Mermaid's dataFetcher: walks the documents and resolves them into the nodes and edges that are drawn.
export function buildStateGraph(
  rootDoc: Stmt[],
  states: Map<string, StateStmt>,
  classes: Map<string, StateClassDef>
): StateGraph {
  const nodes: StateNode[] = [];
  const edges: StateEdge[] = [];
  const byId = new Map<string, StateNode>();
  const drafts = new Map<string, Draft>();
  const slots = new Map<string, number | undefined>();
  let counter = 0;
  let nextSlot = 0;

  const insert = (node: StateNode): void => {
    // The two literal ids are what DOMPurify leaves of `<<fork>>` and `<<choice>>` in Mermaid; kept for parity.
    if (!node.id || node.id === '</join></fork>' || node.id === '</choice>') return;
    if (node.cssClasses) {
      // One copy, however many classes: a state named in a class statement many times has a long list.
      let compiled: string[] | undefined;
      for (const name of node.cssClasses.split(' ')) {
        const def = classes.get(name);
        if (!def) continue;
        compiled ??= node.cssCompiledStyles.slice();
        for (const style of def.styles) compiled.push(style);
      }
      if (compiled) node.cssCompiledStyles = compiled;
    }
    const existing = byId.get(node.id);
    if (existing) {
      const parentId = node.parentId ?? existing.parentId;
      Object.assign(existing, node);
      if (parentId !== undefined) existing.parentId = parentId;
    } else {
      byId.set(node.id, node);
      nodes.push(node);
    }
  };

  const visit = (parent: StateStmt | undefined, item: StateStmt, alt: boolean): void => {
    const id = item.id;
    if (id === 'root') return;
    const known = states.get(id);
    const classStr = known?.classes?.join(' ') ?? '';
    const styles = known?.styles ?? [];

    let draft = drafts.get(id);
    if (!draft) {
      let shape: string | undefined = 'rect';
      if (item.start === true) shape = 'stateStart';
      else if (item.start === false) shape = 'stateEnd';
      if (item.type !== 'default') shape = item.type;
      draft = {
        shape,
        description: id,
        cssClasses: `${classStr} statediagram-state`,
        cssStyles: styles,
        type: undefined,
        dir: undefined,
        colorIndex: undefined,
      };
      drafts.set(id, draft);
    }

    if (item.description) {
      if (Array.isArray(draft.description)) {
        draft.shape = 'rectWithTitle';
        draft.description = draft.description.concat(item.description);
      } else if (draft.description.length > 0) {
        draft.shape = 'rectWithTitle';
        draft.description =
          draft.description === id ? [item.description].flat() : [draft.description, item.description].flat();
      } else {
        draft.shape = 'rect';
        draft.description = Array.isArray(item.description) ? item.description.slice() : item.description;
      }
    }
    if (draft.description.length === 1 && draft.shape === 'rectWithTitle') {
      draft.shape = draft.type === 'group' ? 'roundedWithTitle' : 'rect';
    }

    if (!draft.type && item.doc) {
      draft.type = 'group';
      draft.dir = direction(item.doc);
      draft.shape = item.type === 'divider' ? 'divider' : 'roundedWithTitle';
      // A container takes the next palette slot unless it is a region, which shares its composite's.
      if (draft.shape === 'divider' && parent?.id !== undefined && slots.has(parent.id)) {
        draft.colorIndex = slots.get(parent.id);
      } else {
        const slot = nextSlot++;
        draft.colorIndex = classStr.trim() !== '' || styles.length > 0 ? undefined : slot;
      }
      slots.set(id, draft.colorIndex);
      draft.cssClasses = `${draft.cssClasses} statediagram-cluster ${alt ? 'statediagram-cluster-alt' : ''}`;
    }

    const full = draft.description;
    const node: StateNode = {
      id,
      shape: draft.shape,
      label: Array.isArray(full) ? full[0] : full,
      description: Array.isArray(full) ? full.slice(1) : [],
      cssClasses: draft.cssClasses,
      cssCompiledStyles: [],
      cssStyles: draft.cssStyles,
      type: draft.type,
      isGroup: draft.type === 'group',
      dir: draft.dir,
      colorIndex: draft.colorIndex,
    };
    if (node.shape === 'divider') node.label = '';
    if (parent && parent.id !== 'root') node.parentId = parent.id;

    const note = item.note;
    if (!note) {
      insert(node);
      return;
    }
    // Mermaid puts each note in a group of its own, named after the state. Pelers keeps that
    // group in the model and lays the note out beside its state instead.
    const aside = (asideId: string, shape: string, cssClasses: string, isGroup: boolean): StateNode => ({
      id: asideId,
      shape,
      label: note.text,
      description: [],
      cssClasses,
      cssCompiledStyles: [],
      cssStyles: [],
      type: isGroup ? 'group' : 'node',
      isGroup,
      dir: undefined,
      position: note.position,
    });
    const noteNode = aside(`${id}----note-${counter++}`, 'note', 'statediagram-note', false);
    noteNode.parentId = `${id}----parent`;
    noteNode.noteFor = id;
    insert(aside(noteNode.parentId, 'noteGroup', draft.cssClasses, true));
    insert(noteNode);
    insert(node);
    const left = note.position === 'left of';
    const from = left ? noteNode.id : id;
    const to = left ? id : noteNode.id;
    edges.push({
      id: `${from}-${to}`,
      start: from,
      end: to,
      label: '',
      classes: 'transition note-edge',
      pattern: 'dashed',
    });
  };

  const stack: Frame[] = [{ item: undefined, doc: rootDoc, at: 0, alt: true }];
  while (stack.length > 0) {
    const frame = stack[stack.length - 1];
    if (frame.at >= frame.doc.length) {
      stack.pop();
      continue;
    }
    const item = frame.doc[frame.at++];
    if (typeof item === 'string') continue;
    if (item.stmt === 'state') {
      visit(frame.item, item, frame.alt);
      if (item.doc) stack.push({ item, doc: item.doc, at: 0, alt: !frame.alt });
    } else if (item.stmt === 'relation') {
      visit(frame.item, item.state1, frame.alt);
      visit(frame.item, item.state2, frame.alt);
      edges.push({
        id: `edge${counter++}`,
        start: item.state1.id,
        end: item.state2.id,
        label: item.description ?? '',
        classes: 'transition',
      });
    }
  }

  for (const node of nodes) {
    if (node.isGroup && node.description.length > 0) {
      throw new PelersError(
        `Group nodes can only have label. Remove the additional description for node [${node.id}]`,
        'semantic',
        { type: 'state' }
      );
    }
  }
  return { nodes, edges };
}
