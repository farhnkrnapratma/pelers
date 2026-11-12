use super::common::{
    Grammar, acc_descr_match, acc_title_match, hidden_block_comment, hidden_directives, ident,
    literal, opening, parse_grammar, quoted, railroad_title, unescape,
};
use super::types::{RailroadNode, RailroadRule};
use crate::diagrams::common::tokens::{Pattern, Reader, TokenType};
use crate::errors::PelersError;

pub mod kind {
    pub const TERMINAL: i32 = 6;
    pub const NONTERMINAL: i32 = 1;
    pub const SPECIAL: i32 = 7;
    pub const SEQUENCE: i32 = 4;
    pub const CHOICE: i32 = 8;
    pub const OPTIONAL: i32 = 5;
    pub const ONE_OR_MORE: i32 = 3;
    pub const ZERO_OR_MORE: i32 = 2;
    pub const EQUALS: i32 = 9;
    pub const SEMICOLON: i32 = 10;
    pub const OPEN: i32 = 11;
    pub const COMMA: i32 = 12;
    pub const CLOSE: i32 = 13;
    pub const TITLE: i32 = 14;
    pub const ID: i32 = 17;
    pub const STRING: i32 = 18;
}

const ID_NAME: &str = "RR_ID";

pub fn ir_tokens() -> Vec<TokenType> {
    let mut list = vec![
        opening("railroad-beta", &[ID_NAME]),
        literal("nonterminal", &[ID_NAME]),
        literal("zeroOrMore", &[ID_NAME]),
        literal("oneOrMore", &[ID_NAME]),
        literal("sequence", &[ID_NAME]),
        literal("optional", &[ID_NAME]),
        literal("terminal", &[ID_NAME]),
        literal("special", &[ID_NAME]),
        literal("choice", &[ID_NAME]),
        literal("=", &[]),
        literal(";", &[]),
        literal("(", &[]),
        literal(",", &[]),
        literal(")", &[]),
        TokenType::new("TITLE", Pattern::Match(title_ir), Some("t")),
        TokenType::new("ACC_TITLE", Pattern::Match(acc_title_match), Some("a")),
        TokenType::new("ACC_DESCR", Pattern::Match(acc_descr_match), Some("a")),
        ident(ID_NAME),
        quoted("RR_STRING"),
    ];
    list.extend(hidden_directives("RR"));
    list.push(hidden_block_comment("RR_BLOCK_COMMENT"));
    list
}

fn title_ir(src: &str, at: usize) -> Option<usize> {
    railroad_title(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')(src, at)
}

#[derive(Clone)]
pub enum Expr {
    Sequence(Vec<Expr>),
    Choice(Vec<Expr>),
    Optional(Box<Expr>),
    OneOrMore(Box<Expr>),
    ZeroOrMore(Box<Expr>),
    Terminal(String),
    NonTerminal(String),
    Special(String),
}

pub struct Rule {
    pub name: String,
    pub definition: Expr,
}

fn argument(r: &mut Reader<'_>) -> Result<String, PelersError> {
    r.i += 1;
    r.expect(kind::OPEN)?;
    let value = unescape(&r.expect(kind::STRING)?);
    r.expect(kind::CLOSE)?;
    Ok(value)
}

#[derive(Clone, Copy)]
enum Wrap {
    Optional,
    OneOrMore,
    ZeroOrMore,
}

enum Open {
    Sequence(Vec<Expr>),
    Choice(Vec<Expr>),
    Wrapper(Wrap, Option<Box<Expr>>),
}

enum Pending {
    Done(Expr),
    Open(Open),
}

fn finish(opened: Open) -> Expr {
    match opened {
        Open::Sequence(items) => Expr::Sequence(items),
        Open::Choice(items) => Expr::Choice(items),
        Open::Wrapper(wrap, element) => {
            let element = element.expect("a wrapper always holds its element");
            match wrap {
                Wrap::Optional => Expr::Optional(element),
                Wrap::OneOrMore => Expr::OneOrMore(element),
                Wrap::ZeroOrMore => Expr::ZeroOrMore(element),
            }
        }
    }
}

fn expression(r: &mut Reader<'_>) -> Result<Expr, PelersError> {
    let mut open: Vec<Open> = Vec::new();
    loop {
        let pending = match r.kind() {
            kind::TERMINAL => Pending::Done(Expr::Terminal(argument(r)?)),
            kind::NONTERMINAL => Pending::Done(Expr::NonTerminal(argument(r)?)),
            kind::SPECIAL => Pending::Done(Expr::Special(argument(r)?)),
            kind::SEQUENCE => Pending::Open(Open::Sequence(Vec::new())),
            kind::CHOICE => Pending::Open(Open::Choice(Vec::new())),
            kind::OPTIONAL => Pending::Open(Open::Wrapper(Wrap::Optional, None)),
            kind::ONE_OR_MORE => Pending::Open(Open::Wrapper(Wrap::OneOrMore, None)),
            kind::ZERO_OR_MORE => Pending::Open(Open::Wrapper(Wrap::ZeroOrMore, None)),
            _ => return Err(r.fail("an expression")),
        };
        let mut value = match pending {
            Pending::Open(opened) => {
                r.i += 1;
                r.expect(kind::OPEN)?;
                open.push(opened);
                continue;
            }
            Pending::Done(expr) => expr,
        };
        loop {
            let Some(parent) = open.last_mut() else {
                return Ok(value);
            };
            match parent {
                Open::Sequence(items) | Open::Choice(items) => {
                    items.push(value);
                    if r.accept(kind::COMMA) {
                        break;
                    }
                }
                Open::Wrapper(_, element) => {
                    *element = Some(Box::new(value));
                }
            }
            r.expect(kind::CLOSE)?;
            let closed = open.pop().expect("a parent exists");
            value = finish(closed);
        }
    }
}

pub fn parse_ir(src: &str) -> Result<Grammar<Rule>, PelersError> {
    let tokens = ir_tokens();
    parse_grammar(src, &tokens, kind::TITLE, unescape, |r| {
        let name = r.expect(kind::ID)?;
        r.expect(kind::EQUALS)?;
        let definition = expression(r)?;
        r.expect(kind::SEMICOLON)?;
        Ok(Rule { name, definition })
    })
}

pub fn ir_rules(grammar: &Grammar<Rule>) -> Vec<RailroadRule> {
    grammar
        .rules
        .iter()
        .map(|rule| RailroadRule {
            name: rule.name.clone(),
            definition: node(&rule.definition),
        })
        .collect()
}

fn node(expr: &Expr) -> RailroadNode {
    match expr {
        Expr::Terminal(value) => super::common::terminal(value),
        Expr::NonTerminal(name) => super::common::nonterminal(name),
        Expr::Special(text) => super::common::special(text),
        Expr::Sequence(elements) => super::common::sequence(elements.iter().map(node).collect()),
        Expr::Choice(alternatives) => {
            super::common::choice(alternatives.iter().map(node).collect())
        }
        Expr::Optional(element) => super::common::optional(node(element)),
        Expr::OneOrMore(element) => super::common::repetition(node(element), 1.0, f64::INFINITY),
        Expr::ZeroOrMore(element) => super::common::repetition(node(element), 0.0, f64::INFINITY),
    }
}
