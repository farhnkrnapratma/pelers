import { isValidShape as isFlowchartShape } from '../flowchart/shapes.js';

// Author-facing names for the shapes that carry meaning in an agentflow.
export const SHAPE_ALIASES: ReadonlyMap<string, string> = new Map([
  ['task', 'roundedRect'],
  ['tool', 'subroutine'],
  ['input', 'lean-right'],
  ['decision', 'diamond'],
  ['refdoc', 'lin-doc'],
  ['action', 'hexagon'],
  ['round', 'rect'],
]);

export const REMOVED_SHAPES: ReadonlySet<string> = new Set([
  'doc',
  'stadium',
  'terminal',
  'circle',
  'trapezoid',
  'inv_trapezoid',
  'inv-trapezoid',
  'doublecircle',
  'double-circle',
  'typeDeclaration',
  'procs',
  'lean_left',
  'lean-left',
  'in-out',
  'cylinder',
  'ellipse',
  'odd',
  'tag-rect',
  'tagged-rectangle',
  'delay',
  'half-rounded-rectangle',
  'lin-rect',
  'lined-rectangle',
  'win-pane',
  'window-pane',
  'curv-trap',
  'curved-trapezoid',
]);

export const ALLOWED_SHAPES: ReadonlySet<string> = new Set([
  'roundedRect',
  'subroutine',
  'subprocess',
  'subproc',
  'framed-rectangle',
  'lean-right',
  'diamond',
  'lin-doc',
  'lined-document',
  'hexagon',
  'hex',
  'connector',
  'collapsedGroup',
]);

export const DEFAULT_SHAPE = 'roundedRect';

export function resolveShapeAlias(shape: string | undefined): string | undefined {
  if (!shape) return shape;
  return SHAPE_ALIASES.get(shape) ?? shape;
}

// Mermaid looks a shape name up with `in` on a plain object, so `constructor` passes there too.
export function isValidShape(name: string): boolean {
  return isFlowchartShape(name) || name === 'constructor';
}

export interface ShapeSink {
  emitError(id: 'SHAPE_REMOVED', message: string, ctx: { nodeId: string }): void;
  emitWarning(id: 'SHAPE_UNSUPPORTED', message: string, ctx: { nodeId: string }): void;
}

interface ShapedNode {
  id: string;
  shape?: string;
  isGroup?: boolean;
}

// Reduces every node's shape to one agentflow draws, reporting the removed and unsupported ones.
export function normaliseNodeShapes(nodes: ShapedNode[], sink: ShapeSink): void {
  for (const node of nodes) {
    if (node.isGroup) continue;
    if (!node.shape || node.shape === 'squareRect' || node.shape === 'rect') node.shape = DEFAULT_SHAPE;
    if (REMOVED_SHAPES.has(node.shape)) {
      sink.emitError('SHAPE_REMOVED', `shape "${node.shape}" was removed in v0.8.1, using "${DEFAULT_SHAPE}"`, {
        nodeId: node.id,
      });
      node.shape = DEFAULT_SHAPE;
    } else if (!ALLOWED_SHAPES.has(node.shape)) {
      sink.emitWarning('SHAPE_UNSUPPORTED', `shape "${node.shape}" is not supported, using "${DEFAULT_SHAPE}"`, {
        nodeId: node.id,
      });
      node.shape = DEFAULT_SHAPE;
    }
  }
}
