use crate::diagrams::agentflow::{self, AgentflowModel, render_agentflow};
use crate::diagrams::architecture::{self, ArchitectureModel, render_architecture};
use crate::diagrams::block::{self, BlockDb, render_block};
use crate::diagrams::c4::{self, C4Db, render_c4};
use crate::diagrams::class::{self, ClassDb, render_class};
use crate::diagrams::cynefin::{self, CynefinModel, render_cynefin};
use crate::diagrams::er::{self, ErDb, render_er};
use crate::diagrams::eventmodeling::{self, EventModelingModel, render_eventmodel};
use crate::diagrams::flowchart::db::{DbOptions, FlowDb};
use crate::diagrams::flowchart::parser::parse_flowchart;
use crate::diagrams::flowchart::render::render_flowchart;
use crate::diagrams::flowchart::swimlane::render_swimlane;
use crate::diagrams::gantt::{self, GanttDb, render_gantt};
use crate::diagrams::git::{self, GitModel, render_git};
use crate::diagrams::info::{self, InfoModel, render_info};
use crate::diagrams::ishikawa::{self, IshikawaDb, render_ishikawa};
use crate::diagrams::journey::{self, JourneyModel, render_journey};
use crate::diagrams::kanban::{self, KanbanDb, render_kanban};
use crate::diagrams::mindmap::{self, MindmapDb, render_mindmap};
use crate::diagrams::packet::{self, PacketModel, render_packet};
use crate::diagrams::pie::{self, PieModel, render_pie};
use crate::diagrams::quadrant::{self, QuadrantDb, render_quadrant};
use crate::diagrams::radar::{self, RadarModel, render_radar};
use crate::diagrams::railroad::{self, RailroadModel, render_railroad};
use crate::diagrams::requirement::{self, RequirementModel, render_requirement};
use crate::diagrams::sankey::{self, SankeyDb, render_sankey};
use crate::diagrams::sequence::{self, SeqDb, render_sequence};
use crate::diagrams::state::{self, StateDb, render_state};
use crate::diagrams::timeline::{self, TimelineModel, render_timeline};
use crate::diagrams::treemap::{self, TreemapModel, render_treemap};
use crate::diagrams::treeview::{self, TreeViewModel, render_treeview};
use crate::diagrams::usecase::{self, UsecaseModel, render_usecase};
use crate::diagrams::venn::{self, VennDb, render_venn};
use crate::diagrams::wardley::{self, WardleyModel, render_wardley};
use crate::diagrams::xychart::{self, XyChartDb, render_xychart};
use crate::errors::{Edit, input_offset, locating_in};
use crate::preprocess::{encode_entities, source_map};
use crate::svg::palette::{palette, with_palette};
use crate::svg::root::RenderOptions;
use crate::text::entities::written_codes;
use crate::text::icons::with_icons;
use crate::text::math::with_math;
use crate::text::measurer::with_measurer;
use crate::util::value::Value;
use crate::{
    Config, Detected, DiagramType, MermaidType, PelersError, PelersErrorCode, Preprocessed,
    detect_diagram, preprocess,
};

const DEFAULT_LIMIT: usize = 50_000;
const DEFAULT_OUTPUT_LIMIT: usize = 4_000_000;
const DEFAULT_MAX_EDGES: usize = 5_000;

#[derive(Debug, Clone, PartialEq)]
pub enum Model {
    Flowchart(Box<FlowDb>),
    Swimlane(Box<FlowDb>),
    Git(Box<GitModel>),
    Cynefin(Box<CynefinModel>),
    Er(Box<ErDb>),
    C4(Box<C4Db>),
    Block(Box<BlockDb>),
    Gantt(Box<GanttDb>),
    State(Box<StateDb>),
    Class(Box<ClassDb>),
    Sequence(Box<SeqDb>),
    Usecase(Box<UsecaseModel>),
    Agentflow(Box<AgentflowModel>),
    EventModel(Box<EventModelingModel>),
    Wardley(Box<WardleyModel>),
    Architecture(Box<ArchitectureModel>),
    Requirement(Box<RequirementModel>),
    Railroad(Box<RailroadModel>),
    TreeView(Box<TreeViewModel>),
    Info(Box<InfoModel>),
    Journey(Box<JourneyModel>),
    Kanban(Box<KanbanDb>),
    Mindmap(Box<MindmapDb>),
    Ishikawa(Box<IshikawaDb>),
    Timeline(Box<TimelineModel>),
    Radar(Box<RadarModel>),
    Treemap(Box<TreemapModel>),
    Quadrant(Box<QuadrantDb>),
    Venn(Box<VennDb>),
    Sankey(Box<SankeyDb>),
    XyChart(Box<XyChartDb>),
    Packet(Box<PacketModel>),
    Pie(Box<PieModel>),
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
    pub max_edges: Option<usize>,
}

fn with_types(mut error: PelersError, found: Detected) -> PelersError {
    error
        .diagram_type
        .get_or_insert_with(|| found.diagram_type.as_str().to_owned());
    error
        .mermaid_type
        .get_or_insert_with(|| found.mermaid_type.as_str().to_owned());
    error.message = written_codes(&error.message);
    error.snippet = written_codes(&error.snippet);
    error
}

fn unsupported(detected: Option<Detected>) -> PelersError {
    match detected {
        Some(found) => with_types(
            PelersError::new(
                format!(
                    "Diagram type \"{}\" is not supported yet.",
                    found.diagram_type.as_str()
                ),
                PelersErrorCode::UnsupportedDiagram,
            ),
            found,
        ),
        None => PelersError::new(
            "No diagram type detected for the given text.",
            PelersErrorCode::UnsupportedDiagram,
        ),
    }
}

fn source_finder(source: &str, keep_comments: bool) -> impl Fn(usize) -> (String, usize) + 'static {
    let source = source.to_owned();
    let mapped: std::cell::OnceCell<(String, Vec<Vec<Edit>>)> = std::cell::OnceCell::new();
    move |at| {
        let (text, steps) = mapped.get_or_init(|| source_map(&source, keep_comments));
        (text.clone(), input_offset(steps, at))
    }
}

pub fn parse(source: &str) -> Result<ParsedDiagram, PelersError> {
    parse_with(source, &ParseOptions::default())
}

pub fn parse_with(source: &str, options: &ParseOptions) -> Result<ParsedDiagram, PelersError> {
    let limit = options.limit.unwrap_or(DEFAULT_LIMIT);
    if source.encode_utf16().count() > limit {
        return Err(PelersError::new(
            format!("Diagram source is longer than the limit of {limit} characters."),
            PelersErrorCode::Limit,
        ));
    }
    let prepared: Preprocessed = preprocess(source)?;
    let found = detect_diagram(&prepared.text).ok_or_else(|| unsupported(None))?;
    let keep_comments = matches!(
        found.diagram_type,
        DiagramType::Eventmodeling | DiagramType::Agentflow
    );
    let kept = if keep_comments {
        &prepared.with_comments
    } else {
        &prepared.text
    };
    let parsed = encode_entities(kept, None) + "\n";
    let model = locating_in(
        parsed.clone(),
        source_finder(source, keep_comments),
        || match found.diagram_type {
            DiagramType::Flowchart | DiagramType::Swimlane => {
                let swimlane = found.diagram_type == DiagramType::Swimlane;
                let mut db = FlowDb::new(DbOptions {
                    max_edges: Some(options.max_edges.unwrap_or(DEFAULT_MAX_EDGES)),
                    ..DbOptions::default()
                });
                db.title = prepared.title.clone();
                parse_flowchart(&parsed, &mut db).map(|()| {
                    if swimlane {
                        Model::Swimlane(Box::new(db))
                    } else {
                        Model::Flowchart(Box::new(db))
                    }
                })
            }
            DiagramType::GitGraph => git::parse(&parsed, &prepared.config, prepared.title.clone())
                .map(|model| Model::Git(Box::new(model))),
            DiagramType::TreeView => treeview::parse(&parsed, prepared.title.clone())
                .map(|model| Model::TreeView(Box::new(model))),
            DiagramType::Cynefin => cynefin::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Cynefin(Box::new(model))),
            DiagramType::Railroad => railroad::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Railroad(Box::new(model))),
            DiagramType::Er => {
                er::parse(&parsed, prepared.title.clone()).map(|model| Model::Er(Box::new(model)))
            }
            DiagramType::C4 => {
                c4::parse(&parsed, prepared.title.clone()).map(|model| Model::C4(Box::new(model)))
            }
            DiagramType::Block => block::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Block(Box::new(model))),
            DiagramType::Gantt => gantt::parse(&parsed, &prepared.config, prepared.title.clone())
                .map(|model| Model::Gantt(Box::new(model))),
            DiagramType::State => state::parse(&parsed, prepared.title.clone())
                .map(|model| Model::State(Box::new(model))),
            DiagramType::Class => class::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Class(Box::new(model))),
            DiagramType::Usecase => usecase::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Usecase(Box::new(model))),
            DiagramType::Agentflow => agentflow::parse(
                &parsed,
                &prepared.config,
                prepared.title.clone(),
                prepared.line_offset,
                options.max_edges.unwrap_or(DEFAULT_MAX_EDGES),
            )
            .map(|model| Model::Agentflow(Box::new(model))),
            DiagramType::Sequence => {
                sequence::parse(&parsed, &prepared.config, prepared.title.clone())
                    .map(|model| Model::Sequence(Box::new(model)))
            }
            DiagramType::Eventmodeling => eventmodeling::parse(&parsed, prepared.title.clone())
                .map(|model| Model::EventModel(Box::new(model))),
            DiagramType::Wardley => wardley::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Wardley(Box::new(model))),
            DiagramType::Architecture => architecture::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Architecture(Box::new(model))),
            DiagramType::Requirement => requirement::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Requirement(Box::new(model))),
            DiagramType::Info => info::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Info(Box::new(model))),
            DiagramType::Packet => packet::parse(&parsed, &prepared.config, prepared.title.clone())
                .map(|model| Model::Packet(Box::new(model))),
            DiagramType::Kanban => kanban::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Kanban(Box::new(model))),
            DiagramType::Mindmap => mindmap::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Mindmap(Box::new(model))),
            DiagramType::Ishikawa => ishikawa::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Ishikawa(Box::new(model))),
            DiagramType::Timeline => timeline::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Timeline(Box::new(model))),
            DiagramType::Radar => radar::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Radar(Box::new(model))),
            DiagramType::Treemap => treemap::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Treemap(Box::new(model))),
            DiagramType::QuadrantChart => quadrant::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Quadrant(Box::new(model))),
            DiagramType::Venn => venn::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Venn(Box::new(model))),
            DiagramType::Sankey => sankey::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Sankey(Box::new(model))),
            DiagramType::Xychart => xychart::parse(&parsed, prepared.title.clone())
                .map(|model| Model::XyChart(Box::new(model))),
            DiagramType::Journey => journey::parse(&parsed, prepared.title.clone())
                .map(|model| Model::Journey(Box::new(model))),
            DiagramType::Pie => {
                pie::parse(&parsed, prepared.title.clone()).map(|model| Model::Pie(Box::new(model)))
            }
        },
    )
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

fn config_section(diagram: DiagramType) -> &'static str {
    match diagram {
        DiagramType::Wardley => "wardley-beta",
        DiagramType::Xychart => "xyChart",
        other => other.as_str(),
    }
}

fn fixed_width(config: &Config, section: &str) -> bool {
    matches!(config.get(section), Some(Value::Object(map)) if map.get("useMaxWidth") == Some(&Value::Bool(false)))
}

pub fn render(model: &ParsedDiagram) -> Result<RenderResult, PelersError> {
    render_with(model, &RenderOptions::default())
}

pub fn render_with(
    model: &ParsedDiagram,
    options: &RenderOptions,
) -> Result<RenderResult, PelersError> {
    let found = Detected {
        diagram_type: model.diagram_type,
        mermaid_type: model.mermaid_type,
    };
    let mut options = options.clone();
    options.responsive.get_or_insert(!fixed_width(
        &model.config,
        config_section(model.diagram_type),
    ));
    let (svg, width, height) = with_measurer(options.measurer.clone(), || {
        with_math(options.math.clone(), || {
            with_icons(options.icons.clone(), || {
                with_palette(palette(model.diagram_type.as_str(), &model.config), || {
                    render_model(model, &options)
                })
            })
        })
    })
    .map_err(|error| with_types(error, found))?;
    let units = if svg.is_ascii() {
        svg.len()
    } else {
        svg.encode_utf16().count()
    };
    if units > DEFAULT_OUTPUT_LIMIT {
        return Err(with_types(
            PelersError::new(
                format!(
                    "Diagram output is longer than the limit of {DEFAULT_OUTPUT_LIMIT} characters."
                ),
                PelersErrorCode::Limit,
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

fn render_model(
    model: &ParsedDiagram,
    options: &RenderOptions,
) -> Result<(String, f64, f64), PelersError> {
    Ok(match &model.model {
        Model::XyChart(diagram) => {
            let drawn = render_xychart(diagram, &model.config, options)?;
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Flowchart(db) => {
            let drawn = render_flowchart(db, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Swimlane(db) => {
            let drawn = render_swimlane(db, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Git(diagram) => {
            let drawn = render_git(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::TreeView(diagram) => {
            let drawn = render_treeview(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Cynefin(diagram) => {
            let drawn = render_cynefin(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Railroad(diagram) => {
            let drawn = render_railroad(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Er(diagram) => {
            let drawn = render_er(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::C4(diagram) => {
            let drawn = render_c4(diagram, &model.config, options)?;
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Block(diagram) => {
            let drawn = render_block(diagram, &model.config, options)?;
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Gantt(diagram) => {
            let mut scratch = diagram.clone();
            let drawn = render_gantt(&mut scratch, &model.config, options)?;
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::State(diagram) => {
            let drawn = render_state(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Class(diagram) => {
            let drawn = render_class(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Usecase(diagram) => {
            let drawn = render_usecase(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Agentflow(diagram) => {
            let drawn = render_agentflow(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Sequence(diagram) => {
            let drawn = render_sequence(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::EventModel(diagram) => {
            let drawn = render_eventmodel(diagram, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Wardley(diagram) => {
            let drawn = render_wardley(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Architecture(diagram) => {
            let drawn = render_architecture(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Requirement(diagram) => {
            let drawn = render_requirement(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Info(info) => {
            let drawn = render_info(info, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Packet(packet) => {
            let drawn = render_packet(packet, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Venn(diagram) => {
            let drawn = render_venn(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Sankey(diagram) => {
            let drawn = render_sankey(diagram, &model.config, options)?;
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Quadrant(diagram) => {
            let drawn = render_quadrant(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Treemap(diagram) => {
            let drawn = render_treemap(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Radar(diagram) => {
            let drawn = render_radar(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Timeline(diagram) => {
            let drawn = render_timeline(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Ishikawa(diagram) => {
            let drawn = render_ishikawa(diagram, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Mindmap(map) => {
            let drawn = render_mindmap(map, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Kanban(board) => {
            let drawn = render_kanban(board, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Journey(journey) => {
            let drawn = render_journey(journey, options);
            (drawn.svg, drawn.width, drawn.height)
        }
        Model::Pie(pie) => {
            let drawn = render_pie(pie, &model.config, options);
            (drawn.svg, drawn.width, drawn.height)
        }
    })
}

pub fn render_batch(sources: &[&str], threads: usize) -> Vec<Result<RenderResult, PelersError>> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let render_one = |source: &str| {
        parse_with(source, &ParseOptions::default())
            .and_then(|model| render_with(&model, &RenderOptions::default()))
    };
    let workers = threads.clamp(1, sources.len().max(1));
    let next = AtomicUsize::new(0);
    let mut slots: Vec<Option<Result<RenderResult, PelersError>>> = std::iter::repeat_with(|| None)
        .take(sources.len())
        .collect();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let (next, render_one) = (&next, &render_one);
                scope.spawn(move || {
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(source) = sources.get(index) else {
                            break;
                        };
                        done.push((index, render_one(source)));
                    }
                    done
                })
            })
            .collect();
        for handle in handles {
            let done = handle
                .join()
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
            for (index, result) in done {
                slots[index] = Some(result);
            }
        }
    });
    slots
        .into_iter()
        .map(|slot| slot.expect("every source is rendered exactly once"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Model, ParseOptions, parse, parse_with, render};
    use crate::{DiagramType, PelersErrorCode};

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
    fn rejects_unrecognised_diagrams() {
        let error = parse("notADiagram\n  Alice->>Bob: Hello").unwrap_err();
        assert_eq!(error.code, PelersErrorCode::UnsupportedDiagram);
    }

    #[test]
    fn renders_info_with_the_crate_version() {
        let rendered = render(&parse("info showInfo").unwrap()).unwrap();
        assert!(
            rendered
                .svg
                .contains(&format!(">Pelers v{}</text>", env!("CARGO_PKG_VERSION")))
        );
        assert_eq!((rendered.width, rendered.height), (163.0, 40.0));
    }

    #[test]
    fn enforces_the_source_limit_in_utf16_units() {
        let error = parse_with(
            "info\n",
            &ParseOptions {
                limit: Some(4),
                ..ParseOptions::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.code, PelersErrorCode::Limit);
        assert!(
            parse_with(
                "info",
                &ParseOptions {
                    limit: Some(4),
                    ..ParseOptions::default()
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn tags_syntax_errors_with_the_detected_type() {
        let error = parse("info\nnope").unwrap_err();
        assert_eq!(error.diagram_type.as_deref(), Some("info"));
        assert_eq!(error.mermaid_type.as_deref(), Some("info"));
    }

    #[test]
    fn rejects_text_without_a_diagram_type() {
        let error = parse("hello").unwrap_err();
        assert_eq!(error.code, PelersErrorCode::UnsupportedDiagram);
        assert_eq!(error.diagram_type, None);
    }

    #[test]
    fn enforces_the_output_limit() {
        let rows = (0..9000)
            .map(|row| format!("{}-{}: \"{}\"", row * 32, row * 32 + 31, "x".repeat(200)))
            .collect::<Vec<_>>()
            .join("\n");
        let parsed = parse_with(
            &format!("packet\n{rows}"),
            &ParseOptions {
                limit: Some(10_000_000),
                ..ParseOptions::default()
            },
        )
        .unwrap();
        let code = render(&parsed).err().map(|error| error.code);
        assert_eq!(code, Some(PelersErrorCode::Limit));
    }
}
