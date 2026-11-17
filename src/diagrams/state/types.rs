#[derive(Clone, Debug, PartialEq)]
pub enum Desc {
    Text(String),
    List(Vec<String>),
}

pub fn truthy(description: &Desc) -> bool {
    match description {
        Desc::Text(text) => !text.is_empty(),
        Desc::List(_) => true,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateNote {
    pub position: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateStmt {
    pub id: String,
    pub type_: Option<String>,
    pub description: Option<Desc>,
    pub doc: Option<Vec<Stmt>>,
    pub note: Option<StateNote>,
    pub start: Option<bool>,
    pub classes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RelationStmt {
    pub state1: StateStmt,
    pub state2: StateStmt,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Text(String),
    State(StateStmt),
    Relation(Box<RelationStmt>),
    ClassDef {
        id: String,
        classes: String,
    },
    Style {
        id: String,
        style_class: String,
    },
    ApplyClass {
        id: String,
        style_class: String,
    },
    Dir(String),
    Click {
        id: String,
        url: String,
        tooltip: String,
    },
}
