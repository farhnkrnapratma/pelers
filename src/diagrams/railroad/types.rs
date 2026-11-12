#[derive(Clone, Debug, PartialEq)]
pub enum RailroadNode {
    Terminal(String),
    Nonterminal(String),
    Special(String),
    Sequence(Vec<RailroadNode>),
    Choice(Vec<RailroadNode>),
    Optional(Box<RailroadNode>),
    Repetition {
        element: Box<RailroadNode>,
        min: f64,
        max: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RailroadRule {
    pub name: String,
    pub definition: RailroadNode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RailroadModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub rules: Vec<RailroadRule>,
}
