import type { DiagnosticId } from './types.js';

// Mermaid's diagnostic vocabulary for agentflow. Only the ids in EMITTED are raised by a parser;
// the rest are reserved for tools that check a diagram's meaning.
export const AgentflowWarning = {
  SHAPE_UNSUPPORTED: 'SHAPE_UNSUPPORTED',
  SHAPE_REMOVED: 'SHAPE_REMOVED',
  EDGE_OPERATOR_UNSUPPORTED: 'EDGE_OPERATOR_UNSUPPORTED',
  REFERENCE_EDGE_LABEL_REJECTED: 'REFERENCE_EDGE_LABEL_REJECTED',
  CONNECTOR_REF_UNRESOLVED: 'CONNECTOR_REF_UNRESOLVED',
  CONNECTOR_REF_NOT_A_CONNECTOR: 'CONNECTOR_REF_NOT_A_CONNECTOR',
  METADATA_KEY_MISAPPLIED: 'METADATA_KEY_MISAPPLIED',
  DUPLICATE_ID_NODE: 'DUPLICATE_ID_NODE',
  RESERVED_SYNTHETIC_ID: 'RESERVED_SYNTHETIC_ID',
  CONTAINMENT_VIOLATION: 'CONTAINMENT_VIOLATION',
  EDGE_SEMANTIC_CONTRADICTION: 'EDGE_SEMANTIC_CONTRADICTION',
  FLOW_NO_INPUT: 'FLOW_NO_INPUT',
} as const satisfies Record<DiagnosticId, DiagnosticId>;

export const EMITTED: ReadonlySet<DiagnosticId> = new Set<DiagnosticId>([
  'SHAPE_UNSUPPORTED',
  'SHAPE_REMOVED',
  'CONTAINMENT_VIOLATION',
]);

export const RESERVED: ReadonlySet<DiagnosticId> = new Set(
  (Object.values(AgentflowWarning) as DiagnosticId[]).filter((id) => !EMITTED.has(id))
);
