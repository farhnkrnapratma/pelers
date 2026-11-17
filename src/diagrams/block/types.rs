#[derive(Clone, Debug, PartialEq, Default)]
pub struct Block {
    pub id: Option<String>,
    pub kind: Option<String>,
    pub label: Option<String>,
    pub inner: Option<Vec<Statement>>,
    pub children: Vec<usize>,
    pub columns: Option<f64>,
    pub width_in_columns: Option<f64>,
    pub width: Option<f64>,
    pub directions: Option<Vec<String>>,
    pub classes: Option<Vec<String>>,
    pub styles: Option<Vec<String>>,
    pub color_index: Option<usize>,
    pub start: Option<String>,
    pub end: Option<String>,
    pub arrow_type_start: Option<String>,
    pub arrow_type_end: Option<String>,
    pub thickness: Option<String>,
    pub pattern: Option<String>,
    pub css: Option<String>,
    pub style_class: Option<String>,
    pub styles_str: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    One(usize),
    Many(Vec<usize>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}
