export type DiagramType =
  | "flowchart"
  | "sequence"
  | "class"
  | "state"
  | "er"
  | "pie"
  | "gantt"
  | "gitGraph"
  | "mindmap"
  | "timeline"
  | "journey"
  | "xychart"
  | "quadrantChart"
  | "requirement"
  | "c4"
  | "sankey"
  | "block"
  | "packet"
  | "kanban"
  | "architecture"
  | "radar"
  | "treemap"
  | "info"
  | "agentflow"
  | "usecase"
  | "railroad"
  | "treeView"
  | "venn"
  | "cynefin"
  | "wardley"
  | "swimlane"
  | "ishikawa"
  | "eventmodeling";

export type MermaidType =
  | "agentflow"
  | "architecture"
  | "block"
  | "classDiagram"
  | "cynefin"
  | "er"
  | "eventmodeling"
  | "flowchart-elk"
  | "flowchart-v2"
  | "gantt"
  | "gitGraph"
  | "info"
  | "ishikawa"
  | "journey"
  | "kanban"
  | "mindmap"
  | "packet"
  | "pie"
  | "quadrantChart"
  | "radar"
  | "railroad"
  | "railroadAbnf"
  | "railroadEbnf"
  | "railroadPeg"
  | "requirement"
  | "sankey"
  | "sequence"
  | "stateDiagram"
  | "swimlane"
  | "timeline"
  | "treemap"
  | "treeView"
  | "usecase"
  | "venn"
  | "wardley"
  | "xychart";

export type PelersErrorCode =
  | "unsupported-diagram"
  | "syntax"
  | "semantic"
  | "limit";

export declare class PelersError extends Error {
  code: PelersErrorCode;
  type: string | null;
  mermaidType: string | null;
  line: number;
  column: number;
  snippet: string;
  constructor(
    message: string,
    code?: PelersErrorCode,
    info?: {
      type?: string | null;
      mermaidType?: string | null;
      line?: number;
      column?: number;
      snippet?: string;
    },
  );
}

export type YamlValue =
  | string
  | number
  | boolean
  | null
  | YamlValue[]
  | { [key: string]: YamlValue };

export type Config = { [key: string]: YamlValue };

export interface TextMeasurer {
  width(text: string, size: number, style: number): number;
}

export type IconResolver = (name: string) => string | null | undefined;

export type MathMarkup =
  | { svg: string; width: number; height: number }
  | { html: string; width: number; height: number };

export type MathResolver = (
  tex: string,
  size: number,
) => MathMarkup | null | undefined;

export type MonospacePart = "members" | "attributes" | "files" | "messages";

export interface RenderOptions {
  measurer?: TextMeasurer;
  fontFamily?: string;
  fontFamilyMono?: string;
  monospace?: readonly MonospacePart[];
  fontSize?: number;
  idPrefix?: string;
  responsive?: boolean;
  maxWidth?: number;
  autoDirection?: boolean;
  directionBreakpoint?: number;
  padding?: number;
  limit?: number;
  outputLimit?: number;
  maxEdges?: number;
  linkSchemes?: string[];
  imageSchemes?: string[];
  links?: boolean;
  images?: boolean;
  linkRel?: string;
  icons?: IconResolver;
  math?: MathResolver;
  config?: Config;
  now?: number | Date;
}

export type ParseOptions = Pick<RenderOptions, "limit" | "maxEdges" | "config">;

export interface LinkInfo {
  id: string;
  href: string;
  internal: boolean;
}

export interface RenderResult {
  svg: string;
  width: number;
  height: number;
  links: LinkInfo[];
  type: DiagramType;
  mermaidType: MermaidType;
  title?: string;
}

export interface ParsedDiagramModel {
  readonly type: DiagramType;
  readonly mermaidType: MermaidType;
  readonly title: string | null;
}

export declare function parse(
  text: string,
  options?: ParseOptions,
): ParsedDiagramModel;
export declare function render(
  source: string | ParsedDiagramModel,
  options?: RenderOptions,
): RenderResult;
export declare function detectType(text: string): DiagramType | null;
export declare function supports(text: string): boolean;
export declare function register(...list: unknown[]): void;
export declare function registered(type: DiagramType): boolean;
export declare function forgetTextWidths(): void;
