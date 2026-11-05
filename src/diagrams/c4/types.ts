// A `$key="value"` argument.
export interface C4Named {
  key: string;
  value: string;
}

export type C4Attr = string | C4Named;

export interface C4Element {
  alias: string;
  label: string;
  // A named argument written where the alias or the label belongs. It is applied to the field it names.
  aliasAttr?: C4Named;
  labelAttr?: C4Named;
  parent: C4Boundary | undefined;
  type?: string;
  descr?: string;
  techn?: string;
  sprite?: string;
  tags?: string;
  link?: string;
  bgColor?: string;
  fontColor?: string;
  borderColor?: string;
  shadowing?: string;
  shape?: string;
  legendText?: string;
  legendSprite?: string;
  // Named arguments that set no known field.
  extra?: Map<string, string>;
}

export interface C4Shape extends C4Element {
  // Mermaid's typeC4Shape: person, external_system_db, container_queue and so on.
  kind: string;
}

export interface C4Boundary extends C4Element {
  // Set on deployment nodes: node, nodeL or nodeR.
  nodeType?: string;
}

export interface C4Rel {
  type: string;
  from: string;
  to: string;
  label: string;
  fromAttr?: C4Named;
  toAttr?: C4Named;
  labelAttr?: C4Named;
  techn?: string;
  descr?: string;
  sprite?: string;
  tags?: string;
  link?: string;
  textColor?: string;
  lineColor?: string;
  offsetX?: number;
  offsetY?: number;
  extra?: Map<string, string>;
}

export interface C4Model {
  type: 'c4';
  // The header keyword: C4Context, C4Container, C4Component, C4Dynamic or C4Deployment.
  c4Type: string | undefined;
  title?: string;
  accTitle?: string;
  accDescr?: string;
  shapes: C4Shape[];
  // The first boundary is the unnamed one that holds everything at the top level.
  boundaries: C4Boundary[];
  rels: C4Rel[];
  shapeInRow: number;
  boundaryInRow: number;
}
