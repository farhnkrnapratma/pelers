pub mod api;
#[cfg(feature = "napi")]
mod bindings;
pub mod detect;
pub mod errors;
pub mod flowchart;
pub mod preprocess;
pub mod render;
pub mod util;

pub use api::{ParsedDiagram, RenderResult, parse, render};
pub use detect::{Detected, DiagramType, MermaidType, detect_diagram};
pub use errors::{PeleError, PeleErrorCode};
pub use flowchart::{
    Direction, Edge, EdgeEndpoint, FlowchartModel, Node, Shape, Subgraph, parse as parse_flowchart,
};
pub use preprocess::{Config, Preprocessed, preprocess};
pub use render::{Rendered, render_flowchart};
