use crate::{
    Config, Detected, DiagramType, FlowchartModel, MermaidType, PeleError, PeleErrorCode,
    Preprocessed, preprocess,
};
use crate::{detect_diagram, parse_flowchart, render_flowchart};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDiagram {
    pub diagram_type: DiagramType,
    pub mermaid_type: MermaidType,
    pub model: FlowchartModel,
    pub title: Option<String>,
    pub config: Config,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderResult {
    pub svg: String,
    pub width: usize,
    pub height: usize,
    pub title: Option<String>,
}

fn unsupported(detected: Option<Detected>) -> PeleError {
    let mut error = PeleError::new(
        match detected {
            Some(found) => format!(
                "Rendering diagram type {:?} ({:?}) is not supported.",
                found.diagram_type, found.mermaid_type
            ),
            None => "No supported diagram type detected.".to_owned(),
        },
        PeleErrorCode::UnsupportedDiagram,
    );
    if let Some(found) = detected {
        error.diagram_type = Some(format!("{:?}", found.diagram_type));
        error.mermaid_type = Some(format!("{:?}", found.mermaid_type));
    }
    error
}

pub fn parse(source: &str) -> Result<ParsedDiagram, PeleError> {
    let prepared: Preprocessed = preprocess(source)?;
    let detected = detect_diagram(&prepared.text);
    let Some(found) = detected else {
        return Err(unsupported(None));
    };
    if !matches!(found.diagram_type, DiagramType::Flowchart) {
        return Err(unsupported(Some(found)));
    }
    let model = parse_flowchart(&prepared.text)?;
    Ok(ParsedDiagram {
        diagram_type: found.diagram_type,
        mermaid_type: found.mermaid_type,
        model,
        title: prepared.title,
        config: prepared.config,
        source: prepared.text,
    })
}

pub fn render(model: &ParsedDiagram) -> Result<RenderResult, PeleError> {
    if !matches!(model.diagram_type, DiagramType::Flowchart) {
        return Err(unsupported(Some(Detected {
            diagram_type: model.diagram_type,
            mermaid_type: model.mermaid_type,
        })));
    }
    let rendered = render_flowchart(&model.model);
    Ok(RenderResult {
        svg: rendered.svg,
        width: rendered.width,
        height: rendered.height,
        title: model.title.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse, render};
    use crate::{DiagramType, PeleErrorCode};

    #[test]
    fn parses_and_renders_supported_flowcharts() {
        let parsed = parse("flowchart LR\n  A[Start] --> B[End]").unwrap();
        let rendered = render(&parsed).unwrap();
        assert_eq!(parsed.diagram_type, DiagramType::Flowchart);
        assert_eq!(parsed.model.nodes.len(), 2);
        assert!(rendered.svg.contains(">Start</text>"));
        assert!(rendered.width > rendered.height);
    }

    #[test]
    fn preserves_front_matter_metadata_and_dimensions() {
        let parsed = parse(
            r#"---
title: Sales
---
flowchart TB
  A --> B"#,
        )
        .unwrap();
        let rendered = render(&parsed).unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Sales"));
        assert!(rendered.height > rendered.width);
        assert_eq!(rendered.title.as_deref(), Some("Sales"));
    }

    #[test]
    fn rejects_detected_unsupported_diagrams() {
        let error = parse("sequenceDiagram\n  Alice->>Bob: Hello").unwrap_err();
        assert_eq!(error.code, PeleErrorCode::UnsupportedDiagram);
        assert_eq!(error.diagram_type.as_deref(), Some("Sequence"));
    }
}
