use crate::diagrams::info::{self, InfoModel, render_info};
use crate::diagrams::packet::{self, PacketModel, render_packet};
use crate::diagrams::pie::{self, PieModel, render_pie};
use crate::errors::{Edit, input_offset, locating_in};
use crate::preprocess::{encode_entities, source_map};
use crate::svg::palette::{palette, with_palette};
use crate::svg::root::RenderOptions;
use crate::util::value::Value;
use crate::{
    Config, Detected, DiagramType, FlowchartModel, MermaidType, PeleError, PeleErrorCode,
    Preprocessed, detect_diagram, parse_flowchart, preprocess, render_flowchart,
};

const DEFAULT_LIMIT: usize = 50_000;
const DEFAULT_OUTPUT_LIMIT: usize = 4_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum Model {
    Flowchart(FlowchartModel),
    Info(InfoModel),
    Packet(PacketModel),
    Pie(PieModel),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDiagram {
    pub diagram_type: DiagramType,
    pub mermaid_type: MermaidType,
    pub model: Model,
    pub title: Option<String>,
    pub config: Config,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderResult {
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParseOptions {
    pub limit: Option<usize>,
}

fn with_types(mut error: PeleError, found: Detected) -> PeleError {
    error
        .diagram_type
        .get_or_insert_with(|| found.diagram_type.as_str().to_owned());
    error
        .mermaid_type
        .get_or_insert_with(|| found.mermaid_type.as_str().to_owned());
    error
}

fn unsupported(detected: Option<Detected>) -> PeleError {
    match detected {
        Some(found) => with_types(
            PeleError::new(
                format!(
                    "Diagram type \"{}\" is not supported yet.",
                    found.diagram_type.as_str()
                ),
                PeleErrorCode::UnsupportedDiagram,
            ),
            found,
        ),
        None => PeleError::new(
            "No diagram type detected for the given text.",
            PeleErrorCode::UnsupportedDiagram,
        ),
    }
}

fn source_finder(source: &str) -> impl Fn(usize) -> (String, usize) + 'static {
    let source = source.to_owned();
    let mapped: std::cell::OnceCell<(String, Vec<Vec<Edit>>)> = std::cell::OnceCell::new();
    move |at| {
        let (text, steps) = mapped.get_or_init(|| source_map(&source, false));
        (text.clone(), input_offset(steps, at))
    }
}

pub fn parse(source: &str) -> Result<ParsedDiagram, PeleError> {
    parse_with(source, &ParseOptions::default())
}

pub fn parse_with(source: &str, options: &ParseOptions) -> Result<ParsedDiagram, PeleError> {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT);
    if source.encode_utf16().count() > limit {
        return Err(PeleError::new(
            format!("Diagram source is longer than the limit of {limit} characters."),
            PeleErrorCode::Limit,
        ));
    }
    let prepared: Preprocessed = preprocess(source)?;
    let found = detect_diagram(&prepared.text).ok_or_else(|| unsupported(None))?;
    let parsed = encode_entities(&prepared.text, None) + "\n";
    let model = locating_in(parsed.clone(), source_finder(source), || {
        match found.diagram_type {
            DiagramType::Flowchart => parse_flowchart(&prepared.text).map(Model::Flowchart),
            DiagramType::Info => info::parse(&parsed, prepared.title.clone()).map(Model::Info),
            DiagramType::Packet => {
                packet::parse(&parsed, &prepared.config, prepared.title.clone()).map(Model::Packet)
            }
            DiagramType::Pie => pie::parse(&parsed, prepared.title.clone()).map(Model::Pie),
            _ => Err(unsupported(Some(found))),
        }
    })
    .map_err(|error| with_types(error, found))?;
    Ok(ParsedDiagram {
        diagram_type: found.diagram_type,
        mermaid_type: found.mermaid_type,
        model,
        title: prepared.title,
        config: prepared.config,
        source: prepared.text,
    })
}

fn fixed_width(config: &Config, section: &str) -> bool {
    matches!(config.get(section), Some(Value::Object(map)) if map.get("useMaxWidth") == Some(&Value::Bool(false)))
}

pub fn render(model: &ParsedDiagram) -> Result<RenderResult, PeleError> {
    render_with(model, &RenderOptions::default())
}

pub fn render_with(
    model: &ParsedDiagram,
    options: &RenderOptions,
) -> Result<RenderResult, PeleError> {
    let found = Detected {
        diagram_type: model.diagram_type,
        mermaid_type: model.mermaid_type,
    };
    let mut options = options.clone();
    options
        .responsive
        .get_or_insert(!fixed_width(&model.config, model.diagram_type.as_str()));
    let (svg, width, height) = with_palette(
        palette(model.diagram_type.as_str(), &model.config),
        || match &model.model {
            Model::Flowchart(flowchart) => {
                let rendered = render_flowchart(flowchart);
                (rendered.svg, rendered.width as f64, rendered.height as f64)
            }
            Model::Info(info) => {
                let drawn = render_info(info, &options);
                (drawn.svg, drawn.width, drawn.height)
            }
            Model::Packet(packet) => {
                let drawn = render_packet(packet, &model.config, &options);
                (drawn.svg, drawn.width, drawn.height)
            }
            Model::Pie(pie) => {
                let drawn = render_pie(pie, &model.config, &options);
                (drawn.svg, drawn.width, drawn.height)
            }
        },
    );
    if svg.encode_utf16().count() > DEFAULT_OUTPUT_LIMIT {
        return Err(with_types(
            PeleError::new(
                format!(
                    "Diagram output is longer than the limit of {DEFAULT_OUTPUT_LIMIT} characters."
                ),
                PeleErrorCode::Limit,
            ),
            found,
        ));
    }
    Ok(RenderResult {
        svg,
        width,
        height,
        title: model.title.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::{Model, ParseOptions, parse, parse_with, render};
    use crate::{DiagramType, PeleErrorCode};

    #[test]
    fn parses_and_renders_supported_flowcharts() {
        let parsed = parse("flowchart LR\n  A[Start] --> B[End]").unwrap();
        let rendered = render(&parsed).unwrap();
        assert_eq!(parsed.diagram_type, DiagramType::Flowchart);
        assert!(matches!(&parsed.model, Model::Flowchart(m) if m.nodes.len() == 2));
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
        assert_eq!(error.diagram_type.as_deref(), Some("sequence"));
        assert_eq!(error.mermaid_type.as_deref(), Some("sequence"));
    }

    #[test]
    fn renders_info_with_the_crate_version() {
        let rendered = render(&parse("info showInfo").unwrap()).unwrap();
        assert!(
            rendered
                .svg
                .contains(&format!(">Pele v{}</text>", env!("CARGO_PKG_VERSION")))
        );
        assert_eq!((rendered.width, rendered.height), (150.0, 40.0));
    }

    #[test]
    fn enforces_the_source_limit_in_utf16_units() {
        let error = parse_with("info\n", &ParseOptions { limit: Some(4) }).unwrap_err();
        assert_eq!(error.code, PeleErrorCode::Limit);
        assert!(parse_with("info", &ParseOptions { limit: Some(4) }).is_ok());
    }

    #[test]
    fn tags_syntax_errors_with_the_detected_type() {
        let error = parse("info\nnope").unwrap_err();
        assert_eq!(error.diagram_type.as_deref(), Some("info"));
        assert_eq!(error.mermaid_type.as_deref(), Some("info"));
    }
}
