use super::common::{
    Grammar, acc_descr_match, acc_title_match, hidden_directives, ident, literal, opening,
    parse_grammar, quoted, railroad_title, unescape,
};
use super::types::{RailroadNode, RailroadRule};
use crate::diagrams::common::tokens::{Pattern, Reader, TokenType};
use crate::errors::PelersError;

pub mod kind {
    pub const ARROW: i32 = 1;
    pub const SEMICOLON: i32 = 2;
    pub const SLASH: i32 = 3;
    pub const AND: i32 = 4;
    pub const NOT: i32 = 5;
    pub const QUESTION: i32 = 6;
    pub const STAR: i32 = 7;
    pub const PLUS: i32 = 8;
    pub const GROUP: i32 = 9;
    pub const GROUP_END: i32 = 10;
    pub const DOT: i32 = 11;
    pub const TITLE: i32 = 12;
    pub const ID: i32 = 15;
    pub const STRING: i32 = 16;
}

const ID_NAME: &str = "PEG_ID";

pub fn peg_tokens() -> Vec<TokenType> {
    let mut list = vec![
        opening("railroad-peg-beta", &[ID_NAME]),
        literal("<-", &[]),
        literal(";", &[]),
        literal("/", &[]),
        literal("&", &[]),
        literal("!", &[]),
        literal("?", &[]),
        literal("*", &[]),
        literal("+", &[]),
        literal("(", &[]),
        literal(")", &[]),
        literal(".", &[]),
        TokenType::new("TITLE", Pattern::Match(title_peg), Some("t")),
        TokenType::new("ACC_TITLE", Pattern::Match(acc_title_match), Some("a")),
        TokenType::new("ACC_DESCR", Pattern::Match(acc_descr_match), Some("a")),
        ident(ID_NAME),
        quoted("PEG_STRING"),
    ];
    list.extend(hidden_directives("PEG"));
    list.push(TokenType {
        hidden: true,
        ..TokenType::new("PEG_LINE_COMMENT", Pattern::Match(line_comment), Some("#"))
    });
    list
}

fn title_peg(src: &str, at: usize) -> Option<usize> {
    railroad_title(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')(src, at)
}

fn line_comment(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with('#') {
        return None;
    }
    Some(src[at..].find(['\n', '\r']).map_or(src.len(), |i| at + i))
}

#[derive(Clone)]
pub enum Primary {
    Literal(String),
    Identifier(String),
    Any,
    Group(Choice),
}

#[derive(Clone)]
pub struct Suffix {
    pub primary: Primary,
    pub operator: Option<String>,
}

#[derive(Clone)]
pub struct Prefix {
    pub operator: Option<String>,
    pub suffix: Suffix,
}

#[derive(Clone, Default)]
pub struct Sequence {
    pub elements: Vec<Prefix>,
}

#[derive(Clone, Default)]
pub struct Choice {
    pub alternatives: Vec<Sequence>,
}

pub struct Rule {
    pub name: String,
    pub definition: Choice,
}

struct Frame {
    choice: Choice,
    operator: Option<String>,
}

impl Frame {
    fn new(operator: Option<String>) -> Frame {
        Frame {
            choice: Choice {
                alternatives: vec![Sequence::default()],
            },
            operator,
        }
    }
}

fn definition(r: &mut Reader<'_>) -> Result<Choice, PelersError> {
    let mut frames: Vec<Frame> = vec![Frame::new(None)];
    loop {
        let mut operator = if r.kind() == kind::AND || r.kind() == kind::NOT {
            Some(r.take())
        } else {
            None
        };
        let mut primary = match r.kind() {
            kind::STRING => Primary::Literal(unescape(&r.take())),
            kind::ID => Primary::Identifier(r.take()),
            kind::DOT => {
                r.take();
                Primary::Any
            }
            kind::GROUP => {
                r.i += 1;
                let outer = operator.take();
                frames.push(Frame::new(outer));
                continue;
            }
            _ => return Err(r.fail("a literal, a name, a dot, or an opening parenthesis")),
        };
        loop {
            let suffix_operator = match r.kind() {
                kind::QUESTION | kind::STAR | kind::PLUS => Some(r.take()),
                _ => None,
            };
            let depth = frames.len();
            let top = frames.last_mut().expect("the root frame is never popped");
            let sequence = top
                .choice
                .alternatives
                .last_mut()
                .expect("a frame always holds one sequence");
            sequence.elements.push(Prefix {
                operator: operator.clone(),
                suffix: Suffix {
                    primary: primary.clone(),
                    operator: suffix_operator,
                },
            });
            let k = r.kind();
            if k == kind::GROUP_END && depth > 1 {
                r.i += 1;
                let closed = frames.pop().expect("a group frame is open");
                primary = Primary::Group(closed.choice);
                operator = closed.operator;
                continue;
            }
            if k == kind::SLASH {
                r.i += 1;
                top.choice.alternatives.push(Sequence::default());
            } else if k != kind::AND
                && k != kind::NOT
                && k != kind::STRING
                && k != kind::ID
                && k != kind::DOT
                && k != kind::GROUP
            {
                if depth == 1 {
                    return Ok(frames.pop().expect("root").choice);
                }
                return Err(r.fail("token of type ')'"));
            }
            break;
        }
    }
}

pub fn parse_peg(src: &str) -> Result<Grammar<Rule>, PelersError> {
    let tokens = peg_tokens();
    parse_grammar(src, &tokens, kind::TITLE, unescape, |r| {
        let name = r.expect(kind::ID)?;
        r.expect(kind::ARROW)?;
        let definition = definition(r)?;
        r.expect(kind::SEMICOLON)?;
        Ok(Rule { name, definition })
    })
}

fn label(node: &RailroadNode) -> String {
    match node {
        RailroadNode::Terminal(value) => format!("\"{value}\""),
        RailroadNode::Nonterminal(name) => name.clone(),
        RailroadNode::Special(text) => text.clone(),
        _ => "(...)".to_owned(),
    }
}

fn primary_node(primary: &Primary) -> RailroadNode {
    match primary {
        Primary::Literal(value) => super::common::terminal(value),
        Primary::Identifier(name) => super::common::nonterminal(name),
        Primary::Any => super::common::special("."),
        Primary::Group(choice) => choice_node(choice),
    }
}

fn suffix_node(suffix: &Suffix) -> RailroadNode {
    let kid = primary_node(&suffix.primary);
    match suffix.operator.as_deref() {
        None => kid,
        Some("?") => super::common::optional(kid),
        Some(op) => {
            super::common::repetition(kid, if op == "+" { 1.0 } else { 0.0 }, f64::INFINITY)
        }
    }
}

fn prefix_node(prefix: &Prefix) -> RailroadNode {
    let kid = suffix_node(&prefix.suffix);
    match prefix.operator.as_deref() {
        Some(op) if !op.is_empty() => super::common::special(&format!("{op}{}", label(&kid))),
        _ => kid,
    }
}

fn sequence_node(sequence: &Sequence) -> RailroadNode {
    super::common::sequence(sequence.elements.iter().map(prefix_node).collect())
}

fn choice_node(choice: &Choice) -> RailroadNode {
    super::common::choice(choice.alternatives.iter().map(sequence_node).collect())
}

pub fn peg_rules(grammar: &Grammar<Rule>) -> Vec<RailroadRule> {
    grammar
        .rules
        .iter()
        .map(|rule| RailroadRule {
            name: rule.name.clone(),
            definition: choice_node(&rule.definition),
        })
        .collect()
}
