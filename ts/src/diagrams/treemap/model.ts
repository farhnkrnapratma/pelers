import type { TreemapAst } from './parser.js';

export interface TreemapNode {
  name: string;
  // Sections have a list, which may be empty. Leaves have none.
  children?: TreemapNode[];
  value?: number;
  classSelector?: string;
  cssCompiledStyles?: string[];
}

export interface TreemapItem {
  level: number;
  name: string;
  type: string;
  value?: number;
  classSelector?: string;
  cssCompiledStyles?: string[];
}

export interface TreemapModel {
  type: 'treemap';
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  roots: TreemapNode[];
  classes: Map<string, string[]>;
}

// A row belongs to the nearest section above it that is indented less.
export function buildHierarchy(items: TreemapItem[]): TreemapNode[] {
  const roots: TreemapNode[] = [];
  const stack: { node: TreemapNode; level: number }[] = [];
  for (const item of items) {
    const leaf = item.type === 'Leaf';
    const node: TreemapNode = { name: item.name, children: leaf ? undefined : [] };
    node.classSelector = item.classSelector;
    if (item.cssCompiledStyles) node.cssCompiledStyles = item.cssCompiledStyles;
    if (leaf && item.value !== undefined) node.value = item.value;

    while (stack.length > 0 && stack[stack.length - 1].level >= item.level) stack.pop();
    if (stack.length === 0) roots.push(node);
    else stack[stack.length - 1].node.children!.push(node);
    if (!leaf) stack.push({ node, level: item.level });
  }
  return roots;
}

export function buildTreemap(ast: TreemapAst, title: string | undefined): TreemapModel {
  const classes = new Map<string, string[]>();
  for (const row of ast.TreemapRows) {
    if (row.$type !== 'ClassDefStatement') continue;
    let styles = classes.get(row.className);
    if (!styles) classes.set(row.className, (styles = []));
    // An escaped comma stays in the value; the others separate declarations.
    const text = (row.styleText ?? '').replace(/\\,/g, '§§§').replace(/,/g, ';').replace(/§§§/g, ',');
    for (const style of text.split(';')) styles.push(style);
  }

  const items: TreemapItem[] = [];
  for (const row of ast.TreemapRows) {
    if (row.$type !== 'TreemapRow') continue;
    const item = row.item;
    const styles = item.classSelector ? classes.get(item.classSelector) : undefined;
    items.push({
      level: row.indent ?? 0,
      name: item.name,
      type: item.$type,
      value: item.$type === 'Leaf' ? item.value : undefined,
      classSelector: item.classSelector,
      cssCompiledStyles: styles && styles.length > 0 ? styles : undefined,
    });
  }

  return {
    type: 'treemap',
    title: ast.title || title,
    accTitle: ast.accTitle ? ast.accTitle.replace(/^\s+/g, '') : undefined,
    accDescr: ast.accDescr ? ast.accDescr.replace(/\n\s+/g, '\n') : undefined,
    roots: buildHierarchy(items),
    classes,
  };
}
