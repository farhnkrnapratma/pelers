pub mod api;
#[cfg(feature = "napi")]
mod bindings;
pub mod detect;
pub mod diagrams;
pub mod errors;
pub mod flowchart;
pub mod preprocess;
pub mod render;
pub mod svg;
pub mod text;
pub mod util;

pub use api::{
    Model, ParseOptions, ParsedDiagram, RenderResult, parse, parse_with, render, render_with,
};
pub use detect::{Detected, DiagramType, MermaidType, detect_diagram};
pub use errors::{PeleError, PeleErrorCode};
pub use flowchart::{
    Direction, Edge, EdgeEndpoint, FlowchartModel, Node, Shape, Subgraph, parse as parse_flowchart,
};
pub use preprocess::{Config, Preprocessed, preprocess};
pub use render::{Rendered, render_flowchart};
pub use svg::root::RenderOptions;
