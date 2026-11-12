import { locate, PelersError } from '../../errors.js';
import { safeUrl } from '../../util/url.js';
import { parseYaml, type YamlValue } from '../../util/yaml.js';
import { isValidShape, resolveShapeAlias } from './shapes.js';
import type {
  AgentClassDef,
  AgentEdge,
  AgentNode,
  AgentSubgraph,
  AgentflowModel,
  Diagnostic,
  DiagnosticId,
  DocItem,
  EdgeSemantic,
  ElementMapping,
  ElementPosition,
  FlowText,
  LabelType,
  LinkInfo,
  Loc,
  Metadata,
  SemanticConnector,
  SemanticEdge,
  SemanticModel,
  SemanticSubgraph,
  SemanticVertex,
  StatementType,
  VertexKind,
} from './types.js';

export interface AgentflowDbOptions {
  maxEdges?: number;
  inheritDir?: boolean;
  direction?: string;
  // Lines of front matter before the text the parser sees, added to every reported line.
  lineOffset?: number;
  // The text the parser sees, where errors are located.
  source?: string;
  log?: (severity: 'warning' | 'error', message: string) => void;
}

// Presentation-only metadata keys, left out of the semantic model.
const PRESENTATION_KEYS = new Set([
  'shape',
  'view',
  'icon',
  'img',
  'form',
  'pos',
  'w',
  'h',
  'class',
  'style',
  'labelType',
]);

const TOOL_SHAPES = new Set(['subroutine', 'subprocess', 'subproc', 'framed-rectangle', 'tool']);

export function isToolDefinition(vertex: AgentNode): boolean {
  return TOOL_SHAPES.has(vertex.type as string);
}

function labelType(type: unknown): LabelType {
  return type === 'markdown' || type === 'string' || type === 'text' ? type : 'markdown';
}

function unquote(text: string): string {
  return text.startsWith('"') && text.endsWith('"') ? text.substring(1, text.length - 1) : text;
}

function isObject(value: unknown): value is Metadata {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

// Drops `__proto__`, `constructor` and `prototype` keys at every level, so that a consumer
// merging metadata into its own objects cannot be made to pollute a prototype.
function stripPrototypeKeys(root: YamlValue): YamlValue {
  const copy = (value: YamlValue): YamlValue => (Array.isArray(value) ? [] : isObject(value) ? {} : value);
  const out = copy(root);
  const pending: [YamlValue, YamlValue][] = [[root, out]];
  while (pending.length > 0) {
    const [from, to] = pending.pop()!;
    if (Array.isArray(from)) {
      for (const entry of from) {
        const next = copy(entry);
        (to as YamlValue[]).push(next);
        if (typeof entry === 'object' && entry !== null) pending.push([entry, next]);
      }
    } else if (isObject(from)) {
      for (const key of Object.keys(from)) {
        if (key === '__proto__' || key === 'constructor' || key === 'prototype') continue;
        const entry = from[key];
        const next = copy(entry);
        (to as Metadata)[key] = next;
        if (typeof entry === 'object' && entry !== null) pending.push([entry, next]);
      }
    }
  }
  return out;
}

// Adds what a metadata block holds to an element's metadata. Mermaid spreads the parsed YAML
// whatever it is, so a list adds its indexes as keys and a string its characters.
function merge(into: Metadata | undefined, doc: YamlValue): Metadata {
  return Object.assign(into ?? {}, doc);
}

const RE_BLOCK_SCALAR = /:\s*[>|][\d+-]*\s*$/;
const RE_TRAILING_COMMA = /,([\t ]*)$/;

// Removes the comma that ends a line of a multi-line metadata body, which block YAML does not allow.
// Commas inside quoted scalars, flow collections and block scalars are content and stay.
function stripLineTrailingCommas(body: string): string {
  let inSingle = false;
  let inDouble = false;
  let flowDepth = 0;
  let blockScalarIndent: number | undefined;
  const out: string[] = [];
  for (const line of body.split('\n')) {
    if (blockScalarIndent !== undefined) {
      const indent = line.length - line.trimStart().length;
      if (line.trim() === '' || indent > blockScalarIndent) {
        out.push(line);
        continue;
      }
      blockScalarIndent = undefined;
    }
    let commentStart = -1;
    for (let i = 0; i < line.length; i++) {
      const ch = line[i];
      if (inDouble) {
        if (ch === '\\') i++;
        else if (ch === '"') inDouble = false;
      } else if (inSingle) {
        if (ch === "'") inSingle = false;
      } else if (ch === '"') {
        inDouble = true;
      } else if (ch === "'") {
        inSingle = true;
      } else if (ch === '#' && (i === 0 || line[i - 1] === ' ' || line[i - 1] === '\t')) {
        commentStart = i;
        break;
      } else if (ch === '[' || ch === '{') {
        flowDepth++;
      } else if (ch === ']' || ch === '}') {
        flowDepth = Math.max(0, flowDepth - 1);
      }
    }
    if (inSingle || inDouble || flowDepth > 0) {
      out.push(line);
      continue;
    }
    const code = commentStart >= 0 ? line.slice(0, commentStart) : line;
    const comment = commentStart >= 0 ? line.slice(commentStart) : '';
    if (RE_BLOCK_SCALAR.test(code)) {
      blockScalarIndent = line.length - line.trimStart().length;
      out.push(line);
      continue;
    }
    out.push(code.replace(RE_TRAILING_COMMA, '$1') + comment);
  }
  return out.join('\n');
}

function edgeSemantic(type: string, stroke: string): EdgeSemantic | undefined {
  if (type === 'arrow_point' && stroke === 'normal') return 'sequence';
  if (type === 'arrow_cross' && stroke === 'normal') return 'failure';
  if (type === 'arrow_open' && stroke === 'dotted') return 'reference';
  return undefined;
}

function destructEndLink(raw: string): { type: string; stroke: string; length: number } {
  const str = raw.trim();
  let line = str.slice(0, -1);
  let type = 'arrow_open';
  switch (str.slice(-1)) {
    case 'x':
      type = 'arrow_cross';
      break;
    case '>':
      type = 'arrow_point';
      break;
    case '-':
    case '.':
      line = str;
      break;
  }
  let stroke = 'normal';
  let length = line.length - 1;
  let dots = 0;
  for (let i = 0; i < line.length; i++) if (line[i] === '.') dots++;
  if (dots > 0) {
    stroke = 'dotted';
    length = dots;
  }
  return { type, stroke, length };
}

function semanticMetadata(metadata: Metadata | undefined): Metadata | undefined {
  if (!metadata) return undefined;
  const out: Metadata = {};
  let any = false;
  for (const key of Object.keys(metadata)) {
    if (PRESENTATION_KEYS.has(key)) continue;
    out[key] = metadata[key];
    any = true;
  }
  return any ? out : undefined;
}

export class AgentflowDb implements AgentflowModel {
  readonly type = 'agentflow' as const;
  direction: string | undefined;
  nodes = new Map<string, AgentNode>();
  edges: AgentEdge[] = [];
  subgraphs: AgentSubgraph[] = [];
  connectors = new Map<string, AgentNode>();
  classes = new Map<string, AgentClassDef>();
  tooltips = new Map<string, string>();
  defaultEdgeStyle: string[] | undefined;
  defaultInterpolate: string | undefined;
  mappings: ElementMapping[] = [];
  diagnostics: Diagnostic[] = [];
  title: string | undefined;
  accTitle: string | undefined;
  accDescr: string | undefined;
  lineOffset: number;

  private subgraphLookup = new Map<string, AgentSubgraph>();
  private globalNodes = new Set<string>();
  private members = new Set<string>();
  private edgeById = new Map<string, AgentEdge>();
  private pairCount = new Map<string, number>();
  private subCount = 0;
  private bareMappings = new Set<ElementMapping>();
  private firstMapping = new Map<string, ElementMapping>();
  private reported = new Map<string, Diagnostic[]>();
  private options: AgentflowDbOptions;

  constructor(options: AgentflowDbOptions = {}) {
    this.options = options;
    this.lineOffset = options.lineOffset ?? 0;
  }

  private metadata(id: string, source: string, loc: Loc | undefined): YamlValue {
    const multiline = source.includes('\n');
    let parsed: YamlValue;
    try {
      parsed = parseYaml(multiline ? source + '\n' : '{\n' + source + '\n}');
    } catch (error) {
      // Block YAML rejects the comma-separated style of the single-line form, so try once more without the commas.
      const stripped = multiline ? stripLineTrailingCommas(source) : source;
      let retried: YamlValue | undefined;
      if (stripped !== source) {
        try {
          retried = parseYaml(stripped + '\n');
        } catch {
          retried = undefined;
        }
      }
      if (retried === undefined) {
        const { line, column } = this.errorAt(loc);
        const at = loc ? ` (${line}:${column})` : '';
        throw new PelersError(`Metadata of "${id}" is not valid YAML. ${(error as Error).message}${at}`, 'syntax', {
          type: 'agentflow',
          line,
          column,
        });
      }
      parsed = retried;
    }
    return stripPrototypeKeys(parsed ?? {});
  }

  private errorAt(loc: Loc | undefined): { line: number; column: number } {
    if (loc === undefined) return { line: 0, column: 0 };
    const source = this.options.source;
    if (source === undefined) return { line: loc.first_line + this.lineOffset, column: loc.first_column + 1 };
    let start = 0;
    for (let line = 1; line < loc.first_line; line++) start = source.indexOf('\n', start) + 1;
    const { line, column } = locate(source, start + loc.first_column);
    return { line, column };
  }

  private shapeError(message: string, loc: Loc | undefined): PelersError {
    return new PelersError(message, 'semantic', { type: 'agentflow', ...this.errorAt(loc) });
  }

  addVertex(
    id: string,
    textObj?: FlowText,
    type?: string,
    style?: string[] | null,
    classes?: string[] | null,
    dir?: string,
    props: Record<string, string> = {},
    metadata?: string,
    metadataLoc?: Loc
  ): void {
    if (!id || id.trim().length === 0) return;

    let doc: YamlValue | undefined;
    let fields: Metadata | undefined;
    let authoredShape: string | undefined;
    if (metadata !== undefined) {
      doc = this.metadata(id, metadata, metadataLoc);
      if (isObject(doc)) {
        fields = doc;
        if (typeof doc.shape === 'string') {
          authoredShape = doc.shape;
          doc.shape = resolveShapeAlias(doc.shape)!;
        }
      }
    }

    const subgraph = this.subgraphLookup.get(id);
    if (subgraph && doc) {
      subgraph.metadata = merge(subgraph.metadata, doc);
      return;
    }

    const connector = this.connectors.get(id);
    if (connector && doc) {
      connector.metadata = merge(connector.metadata, doc);
      return;
    }

    // Mermaid reserves this id and creates no node for it.
    if (id === 'connectors') return;

    const edge = this.edgeById.get(id);
    if (edge) {
      if (fields?.animate !== undefined) edge.animate = fields.animate;
      if (fields?.animation !== undefined) edge.animation = fields.animation;
      if (fields?.curve !== undefined) edge.interpolate = fields.curve as string;
      if (doc) edge.metadata = merge(edge.metadata, doc);
      return;
    }

    let vertex = this.nodes.get(id);
    if (vertex === undefined) {
      vertex = { id, labelType: 'text', styles: [], classes: [] };
      this.nodes.set(id, vertex);
    }

    if (textObj !== undefined) {
      vertex.labelType = textObj.type;
      vertex.text = unquote(textObj.text.trim());
    } else if (vertex.text === undefined) {
      vertex.text = id;
    }
    if (type !== undefined) vertex.type = type;
    if (style !== undefined && style !== null) for (const s of style) vertex.styles.push(s);
    if (classes !== undefined && classes !== null) for (const c of classes) vertex.classes.push(c);
    if (dir !== undefined) vertex.dir = dir;
    if (vertex.props === undefined) vertex.props = props;
    else if (props !== undefined) Object.assign(vertex.props, props);

    if (!doc) return;
    vertex.metadata = merge(vertex.metadata, doc);
    if (fields === undefined) return;

    if (fields.shape) {
      const authored = authoredShape ?? fields.shape;
      if (typeof authored !== 'string') {
        throw this.shapeError(`No such shape: ${JSON.stringify(authored)}.`, metadataLoc);
      }
      if (authored !== authored.toLowerCase() || authored.includes('_')) {
        throw this.shapeError(`No such shape: ${authored}. Shape names should be lowercase.`, metadataLoc);
      }
      const shape = fields.shape as string;
      if (!isValidShape(shape)) throw this.shapeError(`No such shape: ${shape}.`, metadataLoc);
      vertex.type = shape;
    }
    if (fields.label) {
      vertex.text = String(fields.label);
      vertex.labelType = labelType(fields.labelType);
    }
  }

  addSingleLink(start: string, end: string, type: LinkInfo | undefined, id?: string): void {
    const edge: AgentEdge = {
      start,
      end,
      type: undefined,
      text: '',
      labelType: 'text',
      classes: [],
      isUserDefinedId: false,
      interpolate: this.defaultInterpolate,
    };
    const linkText = type?.text;
    if (linkText !== undefined) {
      edge.text = unquote(linkText.text.trim());
      edge.labelType = labelType(linkText.type);
    }
    if (type !== undefined) {
      edge.type = type.type;
      edge.stroke = type.stroke;
      edge.length = type.length! > 10 ? 10 : type.length;
      if (type.edgeSemantic) edge.edgeSemantic = type.edgeSemantic;
    }
    const pair = start + '\0' + end;
    const existing = this.pairCount.get(pair) ?? 0;
    if (id && !this.edgeById.has(id)) {
      edge.id = id;
      edge.isUserDefinedId = true;
    } else {
      edge.id = `L_${start}_${end}_${existing === 0 ? 0 : existing + 1}`;
    }

    const max = this.options.maxEdges;
    if (max !== undefined && this.edges.length >= max) {
      throw new PelersError(`Edge limit exceeded. The limit is ${max}.`, 'limit', {
        type: 'agentflow',
      });
    }
    this.edges.push(edge);
    this.pairCount.set(pair, existing + 1);
    if (!this.edgeById.has(edge.id)) this.edgeById.set(edge.id, edge);
  }

  addLink(starts: string[], ends: string[], linkData: LinkInfo | undefined): void {
    const id = typeof linkData?.id === 'string' ? linkData.id.replace('@', '') : undefined;
    const lastStart = starts[starts.length - 1];
    const firstEnd = ends[0];
    for (const start of starts) {
      for (const end of ends) {
        this.addSingleLink(start, end, linkData, start === lastStart && end === firstEnd ? id : undefined);
      }
    }
  }

  updateLinkInterpolate(positions: (string | number)[], interpolate: string): void {
    for (const pos of positions) {
      if (pos === 'default') this.defaultInterpolate = interpolate;
      else this.edgeAt(pos).interpolate = interpolate;
    }
  }

  updateLink(positions: (string | number)[], style: string[]): void {
    for (const pos of positions) {
      if (pos === 'default') {
        this.defaultEdgeStyle = style;
        continue;
      }
      const edge = this.edgeAt(pos);
      edge.style = style;
      if (style.length > 0 && !style.some((s) => s?.startsWith('fill'))) style.push('fill:none');
    }
  }

  // Mermaid checks the bounds only for a numeric index and crashes on the strings its own parser passes.
  private edgeAt(pos: string | number): AgentEdge {
    const edge = this.edges[Number(pos)];
    if (edge === undefined) {
      throw new PelersError(
        `The index ${pos} for linkStyle is out of bounds. Valid indices for linkStyle are between 0 and ${
          this.edges.length - 1
        }. (Help: Ensure that the index is within the range of existing edges.)`,
        'semantic',
        { type: 'agentflow' }
      );
    }
    return edge;
  }

  addClass(ids: string, styles: string[]): void {
    const style = styles.join().replace(/\\,/g, '§§§').replace(/,/g, ';').replace(/§§§/g, ',').split(';');
    for (const id of ids.split(',')) {
      let classNode = this.classes.get(id);
      if (classNode === undefined) {
        classNode = { id, styles: [], textStyles: [] };
        this.classes.set(id, classNode);
      }
      for (const s of style) {
        if (s.includes('color')) classNode.textStyles.push(s.replace('fill', 'bgFill'));
        classNode.styles.push(s);
      }
    }
  }

  setDirection(dir: string): void {
    let d = dir.trim();
    if (d.includes('<')) d = 'RL';
    if (d.includes('^')) d = 'BT';
    if (d.includes('>')) d = 'LR';
    if (d.includes('v')) d = 'TB';
    if (d === 'TD') d = 'TB';
    this.direction = d;
  }

  setClass(ids: string, className: string): void {
    for (const id of ids.split(',')) {
      this.nodes.get(id)?.classes.push(className);
      this.edgeById.get(id)?.classes.push(className);
      this.subgraphLookup.get(id)?.classes.push(className);
    }
  }

  setTooltip(ids: string, tooltip: string | undefined): void {
    if (tooltip === undefined) return;
    for (const id of ids.split(',')) this.tooltips.set(id, tooltip);
  }

  setLink(ids: string, link: string, target?: string): void {
    const url = link.trim();
    for (const id of ids.split(',')) {
      const vertex = this.nodes.get(id);
      if (vertex !== undefined) {
        vertex.link = url ? safeUrl(url) : undefined;
        vertex.linkTarget = target;
      }
    }
    this.setClass(ids, 'clickable');
  }

  // Callbacks are never executed; the node is only marked clickable, as in Mermaid's strict mode.
  setClickEvent(ids: string, _functionName?: string, _functionArgs?: string): void {
    this.setClass(ids, 'clickable');
  }

  setAccTitle(text: string): void {
    this.accTitle = text.replace(/^\s+/g, '');
  }

  setAccDescription(text: string): void {
    this.accDescr = text.replace(/\n\s+/g, '\n');
  }

  addSubGraph(
    idText: { text: string } | undefined,
    list: DocItem[],
    titleText: { text: string; type?: string } | undefined,
    _type?: 'flow'
  ): string {
    let id: string | undefined = idText?.text?.trim();
    const title = titleText?.text || '';
    if (idText !== undefined && idText === titleText && /\s/.test(title)) id = undefined;

    const seen = new Set<string>();
    const nodeList: string[] = [];
    let dir: string | undefined;
    for (const entry of list) {
      if (typeof entry === 'string') {
        if (entry.trim() !== '' && !seen.has(entry)) {
          seen.add(entry);
          nodeList.push(entry);
        }
      } else if (Array.isArray(entry)) {
        for (const item of entry) {
          if (item.trim() !== '' && !seen.has(item)) {
            seen.add(item);
            nodeList.push(item);
          }
        }
      } else {
        dir = entry.value;
      }
    }
    dir ??= this.options.inheritDir ? (this.direction ?? this.options.direction) : undefined;

    id ??= 'subGraph' + this.subCount;
    this.subCount++;

    // A flow never contains itself, a globally scoped node, or a node another flow already holds.
    const members = this.members;
    const nodes = nodeList.filter((nodeId) => nodeId !== id && !this.globalNodes.has(nodeId) && !members.has(nodeId));
    for (const nodeId of nodes) members.add(nodeId);

    const existing = this.subgraphLookup.get(id);
    if (existing) {
      // A repeated id adds its members to the first flow; its title and direction win.
      for (const nodeId of nodes) existing.nodes.push(nodeId);
      if (title.trim()) existing.title = title.trim();
      if (dir) existing.dir = dir;
      return id;
    }
    const subgraph: AgentSubgraph = {
      id,
      nodes,
      title: title.trim(),
      classes: [],
      dir,
      labelType: labelType(titleText?.type),
      type: 'flow',
    };
    this.subgraphs.push(subgraph);
    this.subgraphLookup.set(id, subgraph);
    return id;
  }

  // Every node named inside a `global` block stays out of the flows that mention it.
  addGlobal(list: DocItem[]): void {
    let release = false;
    const add = (item: string): void => {
      const id = item.trim();
      if (id === '') return;
      this.globalNodes.add(id);
      if (this.members.delete(id)) release = true;
    };
    for (const entry of list) {
      if (typeof entry === 'string') add(entry);
      else if (Array.isArray(entry)) for (const item of entry) add(item);
    }
    if (!release) return;
    for (const subgraph of this.subgraphs) {
      subgraph.nodes = subgraph.nodes.filter((nodeId) => !this.globalNodes.has(nodeId));
    }
  }

  addConnector(textObj: FlowText | undefined, titleObj?: FlowText): string {
    const id = textObj?.text?.trim();
    if (!id) return '';
    const title = titleObj?.text?.trim() ?? '';
    const connector = this.connectors.get(id);
    if (!connector) {
      const created: AgentNode = {
        id,
        labelType: 'text',
        styles: [],
        classes: [],
        isConnector: true,
        text: title.length > 0 ? title : id,
      };
      this.connectors.set(id, created);
      this.nodes.set(id, created);
    } else if (title.length > 0) {
      connector.text = title;
    }
    return id;
  }

  destructLink(end: string, start?: string): LinkInfo {
    const info = destructEndLink(end);
    if (start) {
      // The first half of a split arrow only says whether the line is dotted.
      const stroke = start.includes('.') ? 'dotted' : 'normal';
      if (stroke !== info.stroke) return { type: 'INVALID', stroke: 'INVALID' };
    }
    return { ...info, edgeSemantic: edgeSemantic(info.type, info.stroke) };
  }

  subgraph(id: string): AgentSubgraph | undefined {
    return this.subgraphLookup.get(id);
  }

  vertexKind(vertex: AgentNode): VertexKind {
    if (isToolDefinition(vertex)) return 'tool';
    const shape = resolveShapeAlias(vertex.type);
    if (shape === 'hexagon' || shape === 'hex') return 'action';
    if (shape === 'lean-right' || shape === 'lean_right') return 'input';
    if (shape === 'lin-doc' || shape === 'lined-document') return 'refdoc';
    if (shape === 'diamond') return 'decision';
    return 'task';
  }

  semanticModel(): SemanticModel {
    const vertices: SemanticVertex[] = [];
    const connectors: SemanticConnector[] = [];
    for (const [id, v] of this.nodes) {
      if (this.subgraphLookup.has(id)) continue;
      const metadata = semanticMetadata(v.metadata);
      if (v.isConnector) {
        const connector: SemanticConnector = { id };
        if (v.text !== undefined && v.text !== id) connector.title = v.text;
        if (metadata) connector.metadata = metadata;
        connectors.push(connector);
        continue;
      }
      const vertex: SemanticVertex = { id };
      if (v.text !== undefined) vertex.label = v.text;
      const shape = resolveShapeAlias(v.type);
      if (shape !== undefined) vertex.shape = shape;
      vertex.vertexKind = this.vertexKind(v);
      if (metadata) vertex.metadata = metadata;
      vertices.push(vertex);
    }

    const edges = this.edges.map((e) => {
      const edge: SemanticEdge = { start: e.start, end: e.end };
      if (e.id !== undefined) edge.id = e.id;
      if (e.text.length > 0) edge.label = e.text;
      if (e.type !== undefined) edge.type = e.type;
      if (e.stroke !== undefined) edge.stroke = e.stroke;
      if (e.edgeSemantic !== undefined) edge.edgeSemantic = e.edgeSemantic;
      if (e.length !== undefined) edge.length = e.length;
      if (e.metadata && Object.keys(e.metadata).length > 0) edge.metadata = { ...e.metadata };
      return edge;
    });

    const subGraphs = this.subgraphs.map((sg) => {
      const out: SemanticSubgraph = { id: sg.id, nodes: [...sg.nodes], type: sg.type, title: sg.title };
      if (sg.dir !== undefined) out.direction = sg.dir;
      const metadata = semanticMetadata(sg.metadata);
      if (metadata) out.metadata = metadata;
      return out;
    });

    const model: SemanticModel = { vertices, edges, subGraphs, connectors, diagnostics: this.diagnostics };
    if (this.direction !== undefined) model.direction = this.direction;
    return model;
  }

  private position(loc: Loc | undefined): ElementPosition {
    const startLine = loc?.first_line ?? 0;
    const startColumn = loc?.first_column ?? 0;
    return {
      startLine: startLine + this.lineOffset,
      startColumn,
      endLine: (loc?.last_line ?? startLine) + this.lineOffset,
      endColumn: loc?.last_column ?? startColumn,
      // Mermaid builds its parser without ranges, so these are always zero.
      startIndex: 0,
      endIndex: 0,
    };
  }

  private pushMapping(id: string, type: StatementType, position: ElementPosition): ElementMapping {
    const mapping: ElementMapping = { id, type, position };
    this.mappings.push(mapping);
    if (!this.firstMapping.has(id)) this.firstMapping.set(id, mapping);
    return mapping;
  }

  addVertexMapping(id: string, _text: unknown, shape: unknown, loc: Loc | undefined): void {
    if (!id) return;
    const mapping = this.pushMapping(id, 'vertex', this.position(loc));
    // A bare reference becomes an attachment if a metadata block follows it.
    if (shape == null) this.bareMappings.add(mapping);
  }

  // Widens a node's span over the metadata block that follows it.
  extendVertexMapping(id: string, loc: Loc | undefined): void {
    if (!id || !loc) return;
    const end = this.position(loc);
    for (let i = this.mappings.length - 1; i >= 0; i--) {
      const m = this.mappings[i];
      if (m.type !== 'vertex' || m.id !== id) continue;
      if (this.bareMappings.delete(m)) m.type = 'attachment';
      const p = m.position;
      if (end.endLine > p.endLine || (end.endLine === p.endLine && end.endColumn > p.endColumn)) {
        p.endLine = end.endLine;
        p.endColumn = end.endColumn;
      }
      return;
    }
    this.pushMapping(id, 'attachment', end);
  }

  addEdgeMapping(_from: unknown, toNodes: unknown, _link: unknown, loc: Loc | undefined): void {
    const ids = Array.isArray(toNodes) ? toNodes.filter((n) => typeof n === 'string' && n !== '') : [];
    this.pushMapping(ids.length > 0 ? ids.join('>') : 'edge', 'edge', this.position(loc));
  }

  private blockMapping(id: string, type: StatementType, startLoc: Loc | undefined, endLoc: Loc | undefined): void {
    if (!id) return;
    const start = this.position(startLoc);
    const end = endLoc ? this.position(endLoc) : start;
    this.pushMapping(id, type, {
      startLine: start.startLine,
      startColumn: start.startColumn,
      endLine: end.endLine,
      endColumn: end.endColumn,
      startIndex: 0,
      endIndex: 0,
    });
  }

  addSubgraphMapping(id: unknown, _title: unknown, startLoc: Loc | undefined, endLoc: Loc | undefined): void {
    const text = typeof id === 'string' ? id : (id as { text?: string } | undefined)?.text;
    this.blockMapping(text ?? '', 'subgraph', startLoc, endLoc);
  }

  addConnectorMapping(
    textObj: FlowText | undefined,
    _title: unknown,
    startLoc: Loc | undefined,
    endLoc: Loc | undefined
  ): void {
    this.blockMapping(textObj?.text?.trim() ?? '', 'connector', startLoc, endLoc);
  }

  private emit(
    id: DiagnosticId,
    severity: 'warning' | 'error',
    message: string,
    ctx: { nodeId?: string; edgeId?: string } | undefined
  ): void {
    const anchor = ctx?.nodeId ?? ctx?.edgeId;
    const diagnostic: Diagnostic = { id, severity, message };
    if (ctx?.nodeId) diagnostic.nodeId = ctx.nodeId;
    else if (ctx?.edgeId) diagnostic.edgeId = ctx.edgeId;
    const mapping = anchor ? this.firstMapping.get(anchor) : undefined;
    if (mapping) diagnostic.position = mapping.position;

    // The graph may be built more than once; each finding is recorded once.
    const key = anchor ?? '';
    const same = this.reported.get(key);
    if (same === undefined) {
      this.reported.set(key, [diagnostic]);
    } else {
      for (const d of same) {
        if (
          d.id === id &&
          d.severity === severity &&
          d.message === message &&
          d.nodeId === diagnostic.nodeId &&
          d.edgeId === diagnostic.edgeId
        ) {
          return;
        }
      }
      same.push(diagnostic);
    }
    this.diagnostics.push(diagnostic);
    this.options.log?.(severity, `agentflow[${id}]: ${message}`);
  }

  emitWarning(id: DiagnosticId, message: string, ctx?: { nodeId?: string; edgeId?: string }): void {
    this.emit(id, 'warning', message, ctx);
  }

  emitError(id: DiagnosticId, message: string, ctx?: { nodeId?: string; edgeId?: string }): void {
    this.emit(id, 'error', message, ctx);
  }
}
