import { decodeEntities } from '../../text/entities.js';
import { sanitizeUrl } from '../../util/url.js';
import { ClassMember } from './members.js';
import type {
  ClassInterface,
  ClassModel,
  ClassNode,
  ClassNote,
  ClassRelation,
  NamespaceNode,
  StyleClass,
} from './types.js';

export const LINE_TYPE = { LINE: 0, DOTTED_LINE: 1 } as const;
export const RELATION_TYPE = { AGGREGATION: 0, EXTENSION: 1, COMPOSITION: 2, DEPENDENCY: 3, LOLLIPOP: 4 } as const;

// Mermaid looks up ids that start with a digit under this prefix, so they never match a class.
const DOM_ID_PREFIX = 'classId-';

function splitName(id: string): { className: string; type: string } {
  const at = id.indexOf('~');
  if (at <= 0) return { className: id, type: '' };
  const end = id.indexOf('~', at + 1);
  return { className: id.slice(0, at), type: id.slice(at + 1, end === -1 ? id.length : end) };
}

function lookupId(id: string): string {
  const c = id.charCodeAt(0);
  return splitName(c >= 48 && c <= 57 ? DOM_ID_PREFIX + id : id).className;
}

export class ClassDb implements ClassModel {
  readonly type = 'class' as const;
  readonly lineType = LINE_TYPE;
  readonly relationType = RELATION_TYPE;
  direction = 'TB';
  classes = new Map<string, ClassNode>();
  relations: ClassRelation[] = [];
  notes = new Map<string, ClassNote>();
  namespaces = new Map<string, NamespaceNode>();
  interfaces: ClassInterface[] = [];
  styleClasses = new Map<string, StyleClass>();
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;

  private namespaceStack: string[] = [];
  private withCssClass = new Map<string, Set<ClassNode>>();

  addClass(id: string): void {
    const { className, type } = splitName(id);
    if (this.classes.has(className)) return;
    const node: ClassNode = {
      id: className,
      type,
      label: className,
      cssClasses: 'default',
      methods: [],
      members: [],
      annotations: [],
      styles: [],
    };
    this.classes.set(className, node);
    this.tag(node, 'default');
  }

  private tag(node: ClassNode, cssClass: string): void {
    const tagged = this.withCssClass.get(cssClass);
    if (tagged) tagged.add(node);
    else this.withCssClass.set(cssClass, new Set([node]));
  }

  // Returns the class for a name that may carry a generic type, creating it if needed.
  // Mermaid throws a TypeError where this creates, as for an annotation on a class not yet declared.
  private ensure(id: string): ClassNode {
    this.addClass(id);
    return this.classes.get(splitName(id).className)!;
  }

  setClassLabel(id: string, label: string): void {
    this.ensure(id).label = label;
  }

  addRelation(relation: ClassRelation): void {
    const { type1, type2 } = relation.relation;
    if (type1 === RELATION_TYPE.LOLLIPOP && type2 === 'none') {
      this.addClass(relation.id2);
      relation.id1 = this.addInterface(relation.id1, relation.id2);
    } else if (type2 === RELATION_TYPE.LOLLIPOP && type1 === 'none') {
      this.addClass(relation.id1);
      relation.id2 = this.addInterface(relation.id2, relation.id1);
    } else {
      this.addClass(relation.id1);
      this.addClass(relation.id2);
    }
    relation.id1 = splitName(relation.id1).className;
    relation.id2 = splitName(relation.id2).className;
    relation.relationTitle1 = relation.relationTitle1.trim();
    relation.relationTitle2 = relation.relationTitle2.trim();
    this.relations.push(relation);
  }

  private addInterface(label: string, classId: string): string {
    const id = `interface${this.interfaces.length}`;
    this.interfaces.push({ id, label, classId });
    return id;
  }

  addAnnotation(className: string, annotation: string): void {
    this.ensure(className).annotations.push(annotation);
  }

  addMember(className: string, member: string): void {
    const node = this.ensure(className);
    if (typeof member !== 'string') return;
    const text = member.trim();
    if (text.startsWith('<<') && text.endsWith('>>')) {
      node.annotations.push(text.substring(2, text.length - 2));
    } else if (text.indexOf(')') > 0) {
      const parsed = new ClassMember(text, 'method');
      (parsed.memberType === 'method' ? node.methods : node.members).push(parsed);
    } else if (text) {
      node.members.push(new ClassMember(text, 'attribute'));
    }
  }

  addMembers(className: string, members: string[]): void {
    if (!Array.isArray(members)) return;
    members.reverse();
    for (const member of members) this.addMember(className, member);
  }

  addNote(text: string, className?: string): string {
    const index = this.notes.size;
    const id = `note${index}`;
    this.notes.set(id, { id, class: className, text, index });
    return id;
  }

  cleanupLabel(label: string): string {
    return (label.startsWith(':') ? label.substring(1) : label).trim();
  }

  setCssClass(ids: string, className: string): void {
    for (const id of ids.split(',')) {
      const node = this.classes.get(lookupId(id));
      if (node === undefined) continue;
      node.cssClasses += ' ' + className;
      for (const word of className.split(' ')) this.tag(node, word);
    }
  }

  defineClass(ids: string[], style: string[]): void {
    for (const id of ids) {
      let styleClass = this.styleClasses.get(id);
      if (styleClass === undefined) {
        styleClass = { id, styles: [], textStyles: [] };
        this.styleClasses.set(id, styleClass);
      }
      if (!style) continue;
      for (const s of style) {
        if (s.includes('color')) styleClass.textStyles.push(s.replace('fill', 'bgFill'));
        styleClass.styles.push(s);
      }
      // Mermaid tests `cssClasses.includes(id)`, which also matches part of a longer name
      // (a classDef named `def` styles every class, through `default`). Whole names are compared here.
      const tagged = this.withCssClass.get(id);
      if (tagged === undefined) continue;
      const parts = style.flatMap((s) => s.split(','));
      for (const node of tagged) for (const part of parts) node.styles.push(part);
    }
  }

  setTooltip(ids: string, tooltip?: string): void {
    if (tooltip === undefined) return;
    for (const id of ids.split(',')) {
      const node = this.classes.get(splitName(id).className);
      if (node) node.tooltip = tooltip;
    }
  }

  setLink(ids: string, link: string, target?: string): void {
    // Entity codes are resolved first, so that what is checked is what a browser would follow.
    const url = decodeEntities(link).trim();
    for (const id of ids.split(',')) {
      const node = this.classes.get(lookupId(id));
      if (node) {
        node.link = url ? sanitizeUrl(url) : undefined;
        node.linkTarget = typeof target === 'string' ? target : '_blank';
      }
    }
    this.setCssClass(ids, 'clickable');
  }

  // Callbacks are never executed; the class is only marked clickable, as in Mermaid's strict mode.
  setClickEvent(ids: string, _functionName?: string, _functionArgs?: string): void {
    for (const id of ids.split(',')) {
      const node = this.classes.get(splitName(id).className);
      if (node) node.haveCallback = true;
    }
    this.setCssClass(ids, 'clickable');
  }

  setDirection(dir: string): void {
    this.direction = dir;
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }

  addNamespace(id: string, label?: string): string {
    const prefix = this.namespaceStack[this.namespaceStack.length - 1];
    const qualified = prefix ? `${prefix}.${id}` : id;
    this.namespaceStack.push(qualified);

    const existing = this.namespaces.get(qualified);
    if (existing) {
      existing.explicit = true;
      if (label) existing.label = label;
      return qualified;
    }

    // The enclosing namespace and its ancestors exist already, so only the parts of `id` are walked.
    let parentId: string | undefined = prefix || undefined;
    let from = prefix ? prefix.length + 1 : 0;
    while (true) {
      const dot = qualified.indexOf('.', from);
      const leaf = dot === -1;
      const current = leaf ? qualified : qualified.slice(0, dot);
      let node = this.namespaces.get(current);
      if (node === undefined) {
        const part = qualified.slice(from, leaf ? qualified.length : dot);
        node = {
          id: current,
          label: leaf && label ? label : part,
          classes: new Map(),
          notes: new Map(),
          children: new Map(),
          parent: parentId,
          explicit: leaf,
        };
        this.namespaces.set(current, node);
      } else if (leaf) {
        node.explicit = true;
      }
      const parent = parentId ? this.namespaces.get(parentId) : undefined;
      if (parent) {
        if (!parent.children.has(current)) parent.children.set(current, node);
        node.parent ??= parentId;
      }
      if (leaf) return qualified;
      parentId = current;
      from = dot + 1;
    }
  }

  popNamespace(): void {
    this.namespaceStack.pop();
  }

  addClassesToNamespace(id: string, classNames: string[], noteNames: string[]): void {
    const namespace = this.namespaces.get(id);
    if (namespace === undefined) return;
    for (const name of classNames) {
      const { className } = splitName(name);
      const node = this.classes.get(className);
      if (node === undefined) continue;
      node.parent = id;
      namespace.classes.set(className, node);
    }
    for (const name of noteNames) {
      const note = this.notes.get(name);
      if (note === undefined) continue;
      note.parent = id;
      namespace.notes.set(name, note);
    }
  }

  setCssStyle(id: string, styles: string[]): void {
    const node = this.classes.get(id);
    if (!styles || !node) return;
    for (const s of styles) for (const part of s.split(',')) node.styles.push(part);
  }
}
