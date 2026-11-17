use crate::{DiagramType, Model, parse, render};

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
fn generic_api_parses_and_renders_agentflow() {
    let parsed = parse("agentflow-beta LR\n  Start[Start] --> Finish[Finish]").unwrap();
    let rendered = render(&parsed).unwrap();

    assert_eq!(parsed.diagram_type, DiagramType::Agentflow);
    assert!(matches!(&parsed.model, Model::Agentflow(m) if m.graph.nodes.len() == 2));
    assert!(rendered.svg.contains(">Start</text>"));
}

#[test]
fn batch_render_matches_sequential_render_in_order() {
    let sources = [
        "flowchart TD\n  A --> B",
        "pie title Pets\n  \"Dogs\" : 3\n  \"Cats\" : 2",
        "sequenceDiagram\n  Alice->>Bob: Hi",
        "notADiagram",
    ];
    let batch = crate::api::render_batch(&sources, 3);
    assert_eq!(batch.len(), sources.len());
    for (source, result) in sources.iter().zip(&batch) {
        let single = crate::api::render_batch(&[source], 1).remove(0);
        match (result, &single) {
            (Ok(a), Ok(b)) => assert_eq!(a.svg, b.svg),
            (Err(a), Err(b)) => assert_eq!(a, b),
            _ => panic!("batch and single disagree for {source:?}"),
        }
    }
    assert!(batch[3].is_err());
}
