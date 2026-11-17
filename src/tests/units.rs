use crate::diagrams::agentflow::diagnostics::{EMITTED, is_emitted, is_reserved};
use crate::diagrams::agentflow::types::DiagnosticId;

const VOCABULARY: [(DiagnosticId, &str); 12] = [
    (DiagnosticId::ShapeUnsupported, "SHAPE_UNSUPPORTED"),
    (DiagnosticId::ShapeRemoved, "SHAPE_REMOVED"),
    (
        DiagnosticId::EdgeOperatorUnsupported,
        "EDGE_OPERATOR_UNSUPPORTED",
    ),
    (
        DiagnosticId::ReferenceEdgeLabelRejected,
        "REFERENCE_EDGE_LABEL_REJECTED",
    ),
    (
        DiagnosticId::ConnectorRefUnresolved,
        "CONNECTOR_REF_UNRESOLVED",
    ),
    (
        DiagnosticId::ConnectorRefNotAConnector,
        "CONNECTOR_REF_NOT_A_CONNECTOR",
    ),
    (
        DiagnosticId::MetadataKeyMisapplied,
        "METADATA_KEY_MISAPPLIED",
    ),
    (DiagnosticId::DuplicateIdNode, "DUPLICATE_ID_NODE"),
    (DiagnosticId::ReservedSyntheticId, "RESERVED_SYNTHETIC_ID"),
    (DiagnosticId::ContainmentViolation, "CONTAINMENT_VIOLATION"),
    (
        DiagnosticId::EdgeSemanticContradiction,
        "EDGE_SEMANTIC_CONTRADICTION",
    ),
    (DiagnosticId::FlowNoInput, "FLOW_NO_INPUT"),
];

#[test]
fn diagnostic_ids_match_the_typescript_vocabulary() {
    for (id, name) in VOCABULARY {
        assert_eq!(id.as_str(), name);
        let emitted = matches!(
            id,
            DiagnosticId::ShapeUnsupported
                | DiagnosticId::ShapeRemoved
                | DiagnosticId::ContainmentViolation
        );
        assert_eq!(is_emitted(id), emitted, "{name}");
        assert_eq!(is_reserved(id), !emitted, "{name}");
    }
    assert_eq!(EMITTED.len(), 3);
}

#[test]
fn hook_reports_and_compares_by_identity() {
    use crate::util::hook::Hook;
    use std::rc::Rc;
    let shared: Rc<dyn Fn(f64) -> f64> = Rc::new(|x| x);
    let first = Hook(Rc::clone(&shared));
    let second = Hook(Rc::clone(&shared));
    let other = Hook::<dyn Fn(f64) -> f64>(Rc::new(|x| x));
    assert_eq!(format!("{first:?}"), "Hook");
    assert!(first == second);
    assert!(first != other);
}

#[test]
fn ordered_map_remove_keeps_later_indexes() {
    use crate::util::ordered::OrderedMap;
    let mut map = OrderedMap::default();
    for (key, value) in [("a", 1), ("b", 2), ("c", 3)] {
        map.set(key, value);
    }
    assert_eq!(map.remove("b"), Some(2));
    assert_eq!(map.remove("missing"), None);
    assert_eq!(map.get("c"), Some(&3));
    assert_eq!(map.get("b"), None);
}
