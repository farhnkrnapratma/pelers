use crate::util::ordered::OrderedMap;
use crate::util::value::Value;

pub type Span = (usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelType {
    Text,
    Markdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorType {
    Normal,
    Hollow,
    Awesome,
    Icon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseCaseShape {
    Ellipse,
    Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryType {
    Rect,
    Package,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipType {
    Association,
    Include,
    Extend,
    Generalization,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Animation {
    Fast,
    Slow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Actor,
    UseCase,
    Boundary,
    Json,
    Edge,
}

impl SymbolKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::UseCase => "usecase",
            Self::Boundary => "boundary",
            Self::Json => "json",
            Self::Edge => "edge",
        }
    }
}

pub mod arrow {
    pub const SOLID_ARROW: u8 = 0;
    pub const BACK_ARROW: u8 = 1;
    pub const LINE_SOLID: u8 = 2;
    pub const CIRCLE_ARROW: u8 = 3;
    pub const CROSS_ARROW: u8 = 4;
    pub const CIRCLE_ARROW_REVERSED: u8 = 5;
    pub const CROSS_ARROW_REVERSED: u8 = 6;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub id: String,
    pub label: String,
    pub label_type: LabelType,
    pub type_: ActorType,
    pub icon: Option<String>,
    pub business: bool,
    pub stereotype: Option<String>,
    pub parent_id: Option<String>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UseCase {
    pub id: String,
    pub label: String,
    pub label_type: LabelType,
    pub shape: UseCaseShape,
    pub business: bool,
    pub stereotype: Option<String>,
    pub parent_id: Option<String>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SystemBoundary {
    pub id: String,
    pub label: String,
    pub label_type: LabelType,
    pub type_: BoundaryType,
    pub members: Vec<String>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Relationship {
    pub id: String,
    pub explicit_id: bool,
    pub source: String,
    pub target: String,
    pub type_: RelationshipType,
    pub arrow_type: u8,
    pub label: Option<String>,
    pub label_type: Option<LabelType>,
    pub minlen: f64,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
    pub animate: bool,
    pub animation: Option<Animation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsecaseNote {
    pub id: String,
    pub target: String,
    pub label: String,
    pub label_type: LabelType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsecaseJsonNode {
    pub id: String,
    pub value: Value,
    pub property_order: OrderedMap<Vec<String>>,
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassDef {
    pub id: String,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataOccurrence {
    pub key: String,
    pub span: Span,
    pub key_span: Span,
    pub value_span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeOccurrence {
    pub id: String,
    pub span: Span,
    pub id_span: Span,
    pub label_span: Option<Span>,
    pub defines: Option<bool>,
    pub stereotype_span: Option<Span>,
    pub metadata: Option<Vec<MetadataOccurrence>>,
    pub class_spans: Option<Vec<Span>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeOccurrence {
    pub id: String,
    pub span: Span,
    pub label_span: Option<Span>,
    pub id_span: Option<Span>,
    pub metadata: Option<Vec<MetadataOccurrence>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Node,
    Edge,
    Group,
    Note,
    Json,
    Metadata,
    EdgeMetadata,
    ClassDef,
    ClassAssign,
    Style,
    Direction,
    AccTitle,
    AccDescr,
    Comment,
    Blank,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphStatement {
    pub kind: StatementKind,
    pub span: Span,
    pub nodes: Option<Vec<NodeOccurrence>>,
    pub edges: Option<Vec<EdgeOccurrence>>,
    pub group: Option<String>,
    pub id_span: Option<Span>,
    pub title_span: Option<Span>,
    pub end_span: Option<Span>,
    pub ref_: Option<String>,
    pub ref_span: Option<Span>,
    pub children: Option<Vec<GraphStatement>>,
    pub metadata: Option<Vec<MetadataOccurrence>>,
    pub class_spans: Option<Vec<Span>>,
}

impl GraphStatement {
    pub fn new(kind: StatementKind, span: Span) -> Self {
        Self {
            kind,
            span,
            nodes: None,
            edges: None,
            group: None,
            id_span: None,
            title_span: None,
            end_span: None,
            ref_: None,
            ref_span: None,
            children: None,
            metadata: None,
            class_spans: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsecaseModel {
    pub actors: OrderedMap<Actor>,
    pub use_cases: OrderedMap<UseCase>,
    pub system_boundaries: OrderedMap<SystemBoundary>,
    pub relationships: Vec<Relationship>,
    pub notes: OrderedMap<UsecaseNote>,
    pub json_nodes: OrderedMap<UsecaseJsonNode>,
    pub class_defs: OrderedMap<ClassDef>,
    pub direction: String,
    pub title: Option<String>,
    pub acc_title: String,
    pub acc_descr: String,
    pub acc_description: String,
    pub source: String,
    pub header_span: Span,
    pub statements: Vec<GraphStatement>,
}

pub fn empty_model() -> UsecaseModel {
    UsecaseModel {
        actors: OrderedMap::default(),
        use_cases: OrderedMap::default(),
        system_boundaries: OrderedMap::default(),
        relationships: Vec::new(),
        notes: OrderedMap::default(),
        json_nodes: OrderedMap::default(),
        class_defs: OrderedMap::default(),
        direction: "LR".to_owned(),
        title: None,
        acc_title: String::new(),
        acc_descr: String::new(),
        acc_description: String::new(),
        source: String::new(),
        header_span: (0, 0),
        statements: Vec::new(),
    }
}
