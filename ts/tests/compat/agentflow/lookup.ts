import { isToolDefinition } from '../../../src/diagrams/agentflow/db.js';
import type { AgentNode, AgentflowModel, ElementMapping } from '../../../src/diagrams/agentflow/types.js';

// Mermaid's lookups over a parsed agentflow, which its specs call. Pele itself does not use them.

export { isToolDefinition };

export function tools(model: AgentflowModel): AgentNode[] {
  const out: AgentNode[] = [];
  for (const vertex of model.nodes.values()) if (isToolDefinition(vertex)) out.push(vertex);
  return out;
}

export function elementById(model: AgentflowModel, id: string): ElementMapping | undefined {
  return model.mappings.find((m) => m.id === id);
}

export function elementsOnLine(model: AgentflowModel, line: number): ElementMapping[] {
  return model.mappings.filter((m) => line >= m.position.startLine && line <= m.position.endLine);
}

// The innermost element covering a point: the fewest lines, then the fewest columns.
export function elementAt(model: AgentflowModel, line: number, column: number): ElementMapping | undefined {
  let best: ElementMapping | undefined;
  for (const m of model.mappings) {
    const p = m.position;
    if (line < p.startLine || line > p.endLine) continue;
    if (line === p.startLine && column < p.startColumn) continue;
    if (line === p.endLine && column > p.endColumn) continue;
    if (best === undefined) {
      best = m;
      continue;
    }
    const b = best.position;
    const lines = p.endLine - p.startLine;
    const bestLines = b.endLine - b.startLine;
    if (lines < bestLines || (lines === bestLines && p.endColumn - p.startColumn < b.endColumn - b.startColumn)) {
      best = m;
    }
  }
  return best;
}

export function mappingStats(
  model: AgentflowModel
): Record<'vertices' | 'edges' | 'subgraphs' | 'connectors' | 'attachments' | 'totalElements', number> {
  const stats = { vertices: 0, edges: 0, subgraphs: 0, connectors: 0, attachments: 0, totalElements: model.mappings.length };
  for (const m of model.mappings) {
    if (m.type === 'vertex') stats.vertices++;
    else if (m.type === 'edge') stats.edges++;
    else if (m.type === 'subgraph') stats.subgraphs++;
    else if (m.type === 'connector') stats.connectors++;
    else stats.attachments++;
  }
  return stats;
}
