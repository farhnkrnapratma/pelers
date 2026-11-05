import { buildUsecaseAst } from './usecase-graph-ast.js';
import type { UsecaseModel } from '../../src/diagrams/usecase/types.js';

// Mermaid keeps nodes, groups, class definitions and JSON key orders as plain objects keyed by id,
// where an id of `__proto__` is silently lost. Pele keeps Maps; this gives the object Mermaid
// would have built from the same entries.
export function record<V>(map: ReadonlyMap<string, V>): Record<string, V> {
  const out: Record<string, V> = {};
  for (const [key, value] of map) if (key !== '__proto__') out[key] = value;
  return out;
}

// Pele's use case AST in the shape Mermaid publishes it.
export function mermaidAst(model: UsecaseModel): Record<string, unknown> {
  const ast = buildUsecaseAst(model);
  const nodes = record(ast.nodes);
  for (const node of Object.values(nodes)) {
    if (node.attrs?.kind === 'json') {
      node.attrs = { ...node.attrs, propertyOrder: record(node.attrs.propertyOrder as Map<string, string[]>) };
    }
  }
  return { ...ast, nodes, groups: record(ast.groups), classDefs: record(ast.classDefs) };
}
