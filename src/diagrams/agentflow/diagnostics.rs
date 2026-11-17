use super::types::DiagnosticId;

pub const EMITTED: [DiagnosticId; 3] = [
    DiagnosticId::ShapeUnsupported,
    DiagnosticId::ShapeRemoved,
    DiagnosticId::ContainmentViolation,
];

pub fn is_emitted(id: DiagnosticId) -> bool {
    EMITTED.contains(&id)
}

pub fn is_reserved(id: DiagnosticId) -> bool {
    !is_emitted(id)
}
