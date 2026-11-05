import type {
  DocItem,
  ErAttribute,
  ErClassDef,
  ErEntity,
  ErModel,
  ErRelationship,
  ErSubgraph,
  ParsedAttribute,
  RelSpec,
} from './types.js';

function labelType(type: string | undefined): ErSubgraph['labelType'] {
  return type === 'markdown' || type === 'string' || type === 'text' ? type : 'markdown';
}

export class ErDb implements ErModel {
  readonly type = 'er' as const;
  direction = 'TB';
  entities = new Map<string, ErEntity>();
  relationships: ErRelationship[] = [];
  classes = new Map<string, ErClassDef>();
  subgraphs: ErSubgraph[] = [];
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  private subgraphLookup = new Map<string, ErSubgraph>();
  private claimed = new Set<string>();

  addEntity(name: string, alias = ''): ErEntity {
    let entity = this.entities.get(name);
    if (entity === undefined) {
      entity = {
        id: `entity-${name}-${this.entities.size}`,
        label: name,
        alias,
        attributes: [],
        cssClasses: 'default',
        cssStyles: [],
      };
      this.entities.set(name, entity);
    } else if (!entity.alias && alias) {
      entity.alias = alias;
    }
    return entity;
  }

  // The grammar builds the list back to front, so the last attribute written comes first.
  addAttributes(name: string, attributes: ParsedAttribute[]): void {
    const entity = this.addEntity(name);
    for (let i = attributes.length - 1; i >= 0; i--) {
      const attribute = attributes[i];
      attribute.keys ??= [];
      attribute.comment ??= '';
      entity.attributes.push(attribute as ErAttribute);
    }
  }

  addRelationship(entA: string, roleA: string, entB: string, relSpec: RelSpec): void {
    const entityA = this.subgraphLookup.has(entA) ? entA : this.addEntity(entA).id;
    const entityB = this.subgraphLookup.has(entB) ? entB : this.addEntity(entB).id;
    this.relationships.push({ entityA, roleA, entityB, relSpec });
  }

  setDirection(dir: string): void {
    this.direction = dir;
  }

  addCssStyles(ids: string[], styles: string[] | undefined): void {
    if (!styles) return;
    for (const id of ids) {
      const entity = this.entities.get(id);
      if (entity) for (const style of styles) entity.cssStyles.push(style);
      const subgraph = this.subgraphLookup.get(id);
      if (subgraph) for (const style of styles) subgraph.cssStyles.push(style);
    }
  }

  addClass(ids: string[], styles: string[] | undefined): void {
    for (const id of ids) {
      let def = this.classes.get(id);
      if (def === undefined) {
        def = { id, styles: [], textStyles: [] };
        this.classes.set(id, def);
      }
      if (!styles) continue;
      for (const style of styles) {
        if (style.includes('color')) def.textStyles.push(style.replace('fill', 'bgFill'));
        def.styles.push(style);
      }
    }
  }

  setClass(ids: string[], classNames: string[]): void {
    for (const id of ids) {
      const entity = this.entities.get(id);
      if (entity) for (const name of classNames) entity.cssClasses += ' ' + name;
      const subgraph = this.subgraphLookup.get(id);
      if (subgraph) for (const name of classNames) subgraph.classes.push(name);
    }
  }

  addSubGraph(idText: { text: string }, list: DocItem[], titleText: { text: string; type?: string }): string {
    const id = idText.text.trim();
    const seen = new Set<string>();
    const nodes: string[] = [];
    let dir: string | undefined;
    for (const item of list.flat() as DocItem[]) {
      if (typeof item !== 'string') {
        if (item?.stmt === 'dir') dir = item.value;
        continue;
      }
      const trimmed = item.trim();
      if (trimmed === '' || seen.has(trimmed)) continue;
      seen.add(trimmed);
      // A node belongs to the first subgraph that lists it.
      if (!this.claimed.has(item)) nodes.push(item);
    }
    for (const node of nodes) this.claimed.add(node);
    const subgraph: ErSubgraph = {
      id,
      nodes,
      title: (titleText.text || '').trim(),
      classes: [],
      cssStyles: [],
      dir,
      labelType: labelType(titleText.type),
    };
    this.subgraphs.push(subgraph);
    this.subgraphLookup.set(id, subgraph);
    return id;
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }
}
