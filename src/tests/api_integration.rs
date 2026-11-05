use crate::{DiagramType, Model, PeleErrorCode, parse, render};

#[test]
fn generic_api_parses_and_renders_flowcharts() {
    let parsed = parse("flowchart LR\n  start[Start] --> finish[Finish]").unwrap();
    let rendered = render(&parsed).unwrap();

    assert_eq!(parsed.diagram_type, DiagramType::Flowchart);
    assert!(matches!(&parsed.model, Model::Flowchart(m) if m.nodes.len() == 2));
    assert!(rendered.svg.contains(">Start</text>"));
    assert!(rendered.svg.contains(">Finish</text>"));
    assert!(rendered.width > rendered.height);
}

#[test]
fn generic_api_keeps_front_matter_metadata_and_dimensions() {
    let parsed = parse(
        "---\n\
title: Overview\n\
---\n\
flowchart TB\n\
  top[Top] --> bottom[Bottom]",
    )
    .unwrap();
    let rendered = render(&parsed).unwrap();

    assert_eq!(parsed.title.as_deref(), Some("Overview"));
    assert_eq!(rendered.title.as_deref(), Some("Overview"));
    assert!(rendered.height > rendered.width);
}

#[test]
fn generic_api_reports_unsupported_detected_diagrams() {
    let error = parse("sequenceDiagram\n  Alice->>Bob: Hello").unwrap_err();

    assert_eq!(error.code, PeleErrorCode::UnsupportedDiagram);
    assert_eq!(error.diagram_type.as_deref(), Some("sequence"));
    assert_eq!(error.mermaid_type.as_deref(), Some("sequence"));
}
