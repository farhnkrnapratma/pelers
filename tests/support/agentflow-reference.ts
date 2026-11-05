import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { PeleError, parse } from '../../src/index.js';
import type { AgentflowDb } from '../../src/diagrams/agentflow/db.js';
import { buildAgentGraph } from '../../src/diagrams/agentflow/graph.js';
import { loadCorpus, mutator, random } from './corpus.js';

// tests/corpus/agentflow-reference.json holds what Mermaid's own browser build made of each input:
// its model, layout data, element positions, diagnostics and semantic model. To make it again:
//
//   node scripts/run.mjs tests/support/agentflow-reference.ts page <path to mermaid.min.js> > page.html
//   <Chrome> --headless --allow-file-access-from-files --virtual-time-budget=120000 --dump-dom file://$PWD/page.html > dump.html
//   node scripts/run.mjs tests/support/agentflow-reference.ts save dump.html

const FILE = 'tests/corpus/agentflow-reference.json';

export interface Reference {
  src: string;
  // What Mermaid produced, or the first line of the error it threw.
  model?: Record<string, unknown>;
  error?: string;
}

const FRAGMENTS = [
  ' ',
  '\n',
  ';',
  '-->',
  '--x',
  '-.-',
  '--',
  '|',
  '[',
  ']',
  '(',
  ')',
  '{',
  '}',
  '"',
  '@{ shape: tool }',
  '@{ view: collapsed }',
  '@{ shape: cylinder }',
  '@{ shape: tri }',
  ':::',
  '&',
  ' & ',
  'flow ',
  'flow f\n',
  'end\n',
  'global\n',
  'connector c\n',
  '%% c\n',
  'a --> b\n',
  'style a fill:red\n',
  'class a b\n',
  'direction LR\n',
  'e1@',
  'e1@{ curve: basis }\n',
  'linkStyle 0 stroke:red\n',
  '---\ntitle: T\n---\n',
  'accTitle: t\n',
  'click a href "https://x.y"\n',
  'x',
  'y',
  'f',
  '1',
];

function inputs(): string[] {
  const fixtures = readdirSync('tests/compat/agentflow/upstream')
    .filter((name) => name.endsWith('.mmd'))
    .map((name) => readFileSync(`tests/compat/agentflow/upstream/${name}`, 'utf8'));
  const own = [...new Set([...loadCorpus('agentflow', /agentflow-beta/), ...fixtures])];
  const next = mutator(own, FRAGMENTS, random(42));
  const mutated: string[] = [];
  for (let i = 0; i < 80; i++) mutated.push(next());
  return [...new Set([...own, ...mutated])].filter((src) => /agentflow-beta/.test(src));
}

// The same fields on both sides. `source` is Mermaid's AgentFlowDB in the page, or the view below.
const PROJECT = `(db) => {
  const data = db.getData();
  const pick = (o, keys) => Object.fromEntries(keys.filter((k) => o[k] !== undefined).map((k) => [k, o[k]]));
  return {
    direction: db.getDirection(),
    accTitle: db.getAccTitle(),
    accDescr: db.getAccDescription(),
    vertices: [...db.getVertices().values()].map((v) => pick(v, ['id', 'text', 'labelType', 'type', 'styles', 'classes', 'link', 'linkTarget', 'metadata', 'isConnector', 'props'])),
    edges: db.getEdges().map((e) => ({ ...e })),
    defaultStyle: db.getEdges().defaultStyle,
    defaultInterpolate: db.getEdges().defaultInterpolate,
    subGraphs: db.getSubGraphs().map((s) => ({ ...s })),
    classes: [...db.getClasses().values()],
    nodes: data.nodes.map((n) => pick(n, ['id', 'label', 'labelType', 'shape', 'isGroup', 'parentId', 'cssCompiledStyles', 'cssClasses', 'dir', 'link', 'linkTarget', 'tooltip', 'colorIndex', 'metadata'])),
    layoutEdges: data.edges.map((e) => pick(e, ['id', 'start', 'end', 'type', 'label', 'labelType', 'thickness', 'minlen', 'arrowTypeStart', 'arrowTypeEnd', 'cssCompiledStyles', 'style', 'curve', 'metadata', 'isUserDefinedId'])),
    mappings: db.getElementMappings().map((m) => [m.id, m.type, m.position.startLine, m.position.startColumn, m.position.endLine, m.position.endColumn, m.position.startIndex, m.position.endIndex]),
    diagnostics: db.getDiagnostics(),
    semantic: db.getSemanticModel(),
  };
}`;

const project = new Function(`return ${PROJECT}`)() as (db: unknown) => Record<string, unknown>;

function page(mermaidPath: string): string {
  return `<!doctype html><meta charset="utf-8"><body><pre id="out">PENDING</pre>
<script src="file://${resolve(mermaidPath)}"></script>
<script>
const inputs = ${JSON.stringify(inputs()).replace(/</g, '\\u003c')};
const project = ${PROJECT};
(async () => {
  // Twelve palette colors, as in the theme Mermaid gives agentflow by default.
  mermaid.initialize({ startOnLoad: false, theme: 'redux-color' });
  const out = [];
  for (const src of inputs) {
    try {
      const diagram = await mermaid.mermaidAPI.getDiagramFromText(src);
      out.push({ src, model: project(diagram.db) });
    } catch (e) {
      out.push({ src, error: String((e && e.message) || e).split('\\n')[0] });
    }
  }
  document.getElementById('out').textContent = 'B64:' + btoa(unescape(encodeURIComponent(JSON.stringify(out)))) + ':END';
})();
</script>`;
}

// Mermaid's palette length in the page above, and the curve its default config names.
const PALETTE = 12;
const CURVE = 'basis';

// Pele's model through the accessors the projection reads.
export function peleReference(src: string): Reference {
  let db: AgentflowDb;
  try {
    db = parse(src, { limit: Infinity }) as unknown as AgentflowDb;
  } catch (error) {
    if (error instanceof PeleError) return { src, error: error.message.split('\n')[0] };
    throw error;
  }
  const graph = buildAgentGraph(db, PALETTE, CURVE);
  const edges = db.edges as typeof db.edges & { defaultStyle?: string[]; defaultInterpolate?: string };
  edges.defaultStyle = db.defaultEdgeStyle;
  edges.defaultInterpolate = db.defaultInterpolate;
  const view = {
    getData: () => ({
      // Mermaid marks a node's kind with a class; Pele keeps the kind on the node.
      nodes: graph.nodes.map((n) => ({
        ...n,
        cssClasses: (n.cssClasses + (n.kind ? ` af-kind-${n.kind}` : '')).replace(/\s+/g, ' ').trim(),
      })),
      edges: graph.edges,
    }),
    getDirection: () => db.direction?.trim(),
    getAccTitle: () => db.accTitle ?? '',
    getAccDescription: () => db.accDescr ?? '',
    getVertices: () => db.nodes,
    getEdges: () => edges,
    getSubGraphs: () => db.subgraphs,
    getClasses: () => db.classes,
    getElementMappings: () => db.mappings,
    getDiagnostics: () => db.diagnostics,
    getSemanticModel: () => db.semanticModel(),
  };
  return { src, model: project(view) };
}

// Mermaid passes text through DOMPurify, which rewrites markup, so text holding any is not compared.
export function comparable(value: unknown): unknown {
  if (typeof value === 'string') return /[<>&]/.test(value) ? '(markup)' : value;
  if (Array.isArray(value)) return value.map(comparable);
  if (value !== null && typeof value === 'object') {
    const out: Record<string, unknown> = {};
    for (const key of Object.keys(value).sort()) {
      const entry = (value as Record<string, unknown>)[key];
      // Mermaid's DOM ids and click bookkeeping have no counterpart in Pele.
      if (entry === undefined || key === 'domId' || key === 'haveCallback') continue;
      out[key] = comparable(entry);
    }
    return out;
  }
  return value;
}

export function loadReference(): Reference[] {
  return JSON.parse(readFileSync(FILE, 'utf8')) as Reference[];
}

const [mode, path] = process.argv.slice(2);
if (mode === 'page') {
  process.stdout.write(page(path));
} else if (mode === 'save') {
  const match = /B64:([A-Za-z0-9+/=]+):END/.exec(readFileSync(path, 'utf8'));
  if (!match) throw new Error('The dump holds no result.');
  const all = JSON.parse(Buffer.from(match[1], 'base64').toString('utf8')) as Reference[];
  writeFileSync(FILE, JSON.stringify(all, null, 0).replace(/\},\{"src":/g, '},\n{"src":') + '\n');
  console.log(`${all.length} inputs, ${all.filter((r) => r.model).length} accepted by Mermaid`);
}
