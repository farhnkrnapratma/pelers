use std::cell::RefCell;
use std::rc::Rc;

use crate::util::value::{Object, Value};

pub type EdgeStyle = Rc<RefCell<Vec<String>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelType {
    Text,
    String,
    Markdown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowText {
    pub text: String,
    pub kind: LabelType,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loc {
    pub first_line: usize,
    pub first_column: usize,
    pub last_line: usize,
    pub last_column: usize,
}

pub type Metadata = Object;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VertexKind {
    Tool,
    Action,
    Input,
    Refdoc,
    Decision,
    Connector,
    Task,
}

impl VertexKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::Action => "action",
            Self::Input => "input",
            Self::Refdoc => "refdoc",
            Self::Decision => "decision",
            Self::Connector => "connector",
            Self::Task => "task",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeSemantic {
    Sequence,
    Reference,
    Failure,
}

impl EdgeSemantic {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sequence => "sequence",
            Self::Reference => "reference",
            Self::Failure => "failure",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentNode {
    pub id: String,
    pub text: Option<String>,
    pub label_type: LabelType,
    pub shape: Option<String>,
    pub styles: Vec<String>,
    pub classes: Vec<String>,
    pub dir: Option<String>,
    pub link: Option<String>,
    pub link_target: Option<String>,
    pub metadata: Option<Metadata>,
    pub is_connector: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentEdge {
    pub start: String,
    pub end: String,
    pub kind: Option<String>,
    pub text: String,
    pub label_type: LabelType,
    pub stroke: Option<String>,
    pub edge_semantic: Option<EdgeSemantic>,
    pub length: Option<i64>,
    pub style: Option<EdgeStyle>,
    pub classes: Vec<String>,
    pub id: Option<String>,
    pub is_user_defined_id: bool,
    pub interpolate: Option<String>,
    pub animate: Option<Value>,
    pub animation: Option<Value>,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentSubgraph {
    pub id: String,
    pub nodes: Vec<String>,
    pub title: String,
    pub classes: Vec<String>,
    pub dir: Option<String>,
    pub label_type: LabelType,
    pub kind: String,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkInfo {
    pub kind: String,
    pub stroke: String,
    pub length: Option<i64>,
    pub edge_semantic: Option<EdgeSemantic>,
    pub text: Option<FlowText>,
    pub id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DocItem {
    Node(String),
    Nodes(Vec<String>),
    Dir(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ElementPosition {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub start_index: usize,
    pub end_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementType {
    Vertex,
    Edge,
    Subgraph,
    Connector,
    Attachment,
}

impl StatementType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vertex => "vertex",
            Self::Edge => "edge",
            Self::Subgraph => "subgraph",
            Self::Connector => "connector",
            Self::Attachment => "attachment",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementMapping {
    pub id: String,
    pub kind: StatementType,
    pub position: ElementPosition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticId {
    ShapeUnsupported,
    ShapeRemoved,
    EdgeOperatorUnsupported,
    ReferenceEdgeLabelRejected,
    ConnectorRefUnresolved,
    ConnectorRefNotAConnector,
    MetadataKeyMisapplied,
    DuplicateIdNode,
    ReservedSyntheticId,
    ContainmentViolation,
    EdgeSemanticContradiction,
    FlowNoInput,
}

impl DiagnosticId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShapeUnsupported => "SHAPE_UNSUPPORTED",
            Self::ShapeRemoved => "SHAPE_REMOVED",
            Self::EdgeOperatorUnsupported => "EDGE_OPERATOR_UNSUPPORTED",
            Self::ReferenceEdgeLabelRejected => "REFERENCE_EDGE_LABEL_REJECTED",
            Self::ConnectorRefUnresolved => "CONNECTOR_REF_UNRESOLVED",
            Self::ConnectorRefNotAConnector => "CONNECTOR_REF_NOT_A_CONNECTOR",
            Self::MetadataKeyMisapplied => "METADATA_KEY_MISAPPLIED",
            Self::DuplicateIdNode => "DUPLICATE_ID_NODE",
            Self::ReservedSyntheticId => "RESERVED_SYNTHETIC_ID",
            Self::ContainmentViolation => "CONTAINMENT_VIOLATION",
            Self::EdgeSemanticContradiction => "EDGE_SEMANTIC_CONTRADICTION",
            Self::FlowNoInput => "FLOW_NO_INPUT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub id: DiagnosticId,
    pub severity: Severity,
    pub message: String,
    pub node_id: Option<String>,
    pub edge_id: Option<String>,
    pub position: Option<ElementPosition>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticVertex {
    pub id: String,
    pub label: Option<String>,
    pub shape: Option<String>,
    pub vertex_kind: &'static str,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticEdge {
    pub start: String,
    pub end: String,
    pub id: Option<String>,
    pub label: Option<String>,
    pub kind: Option<String>,
    pub stroke: Option<String>,
    pub edge_semantic: Option<EdgeSemantic>,
    pub length: Option<i64>,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticSubGraph {
    pub id: String,
    pub kind: String,
    pub nodes: Vec<String>,
    pub title: String,
    pub direction: Option<String>,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticConnector {
    pub id: String,
    pub title: Option<String>,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SemanticModel {
    pub direction: Option<String>,
    pub vertices: Vec<SemanticVertex>,
    pub edges: Vec<SemanticEdge>,
    pub sub_graphs: Vec<SemanticSubGraph>,
    pub connectors: Vec<SemanticConnector>,
    pub diagnostics: Vec<Diagnostic>,
}
