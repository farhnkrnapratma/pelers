use crate::util::value::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelType {
    Text,
    String,
    Markdown,
}

impl LabelType {
    pub fn as_str(self) -> &'static str {
        match self {
            LabelType::Text => "text",
            LabelType::String => "string",
            LabelType::Markdown => "markdown",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowText {
    pub text: String,
    pub kind: LabelType,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct LinkInfo {
    pub kind: String,
    pub stroke: String,
    pub length: Option<f64>,
    pub text: Option<FlowText>,
    pub id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DocItem {
    Id(String),
    Ids(Vec<String>),
    Dir(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowNode {
    pub id: String,
    pub label_type: LabelType,
    pub text: Option<String>,
    pub kind: Option<String>,
    pub styles: Vec<String>,
    pub classes: Vec<String>,
    pub dir: Option<String>,
    pub props: Vec<(String, String)>,
    pub link: Option<String>,
    pub link_target: Option<String>,
    pub icon: Option<String>,
    pub form: Option<String>,
    pub pos: Option<String>,
    pub img: Option<String>,
    pub constraint: Option<String>,
    pub asset_width: Option<f64>,
    pub asset_height: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowEdge {
    pub start: String,
    pub end: String,
    pub kind: Option<String>,
    pub text: String,
    pub label_type: LabelType,
    pub classes: Vec<String>,
    pub style: Option<Vec<String>>,
    pub stroke: Option<String>,
    pub length: Option<f64>,
    pub id: Option<String>,
    pub is_user_defined_id: bool,
    pub interpolate: Option<Value>,
    pub animate: Option<Value>,
    pub animation: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowSubgraph {
    pub id: String,
    pub nodes: Vec<String>,
    pub title: String,
    pub classes: Vec<String>,
    pub dir: Option<String>,
    pub label_type: LabelType,
    pub metadata: Vec<(String, Value)>,
}

impl FlowSubgraph {
    pub fn metadata_ref(&self) -> Option<&Vec<(String, crate::util::value::Value)>> {
        if self.metadata.is_empty() {
            None
        } else {
            Some(&self.metadata)
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FlowClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}
