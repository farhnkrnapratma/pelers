// One shape for blocks, edges, and the statements that set columns, classes, and styles, as in Mermaid.
export interface Block {
  id: string;
  type?: string;
  label?: string;
  children?: Block[];
  columns?: number;
  widthInColumns?: number;
  // Number of cells a `space` statement asked for.
  width?: number;
  directions?: string[];
  classes?: string[];
  styles?: string[];
  colorIndex?: number;
  start?: string;
  end?: string;
  arrowTypeStart?: string;
  arrowTypeEnd?: string;
  thickness?: string;
  pattern?: string;
  css?: string;
  styleClass?: string;
  stylesStr?: string;
}

// A statement with links is a list: block, edge, block, and so on.
export type Statement = Block | Block[];

export interface BlockClassDef {
  id: string;
  styles: string[];
  textStyles: string[];
}

export interface BlockModel {
  type: 'block';
  root: Block;
  blocks: Block[];
  edges: Block[];
  classes: Map<string, BlockClassDef>;
  byId: Map<string, Block>;
  title?: string;
  // Never set: the grammar reads accTitle and accDescr but has no statement that accepts them.
  accTitle?: string;
  accDescr?: string;
}
