use super::common::{
    Grammar, acc_descr_match, acc_title_match, hidden_directives, literal, opening, parse_grammar,
    railroad_title,
};
use super::types::{RailroadNode, RailroadRule};
use crate::diagrams::common::tokens::{Pattern, Reader, TokenType};
use crate::errors::PelersError;

pub mod kind {
    pub const EQUALS: i32 = 1;
    pub const SEMICOLON: i32 = 2;
    pub const SLASH: i32 = 3;
    pub const GROUP: i32 = 4;
    pub const GROUP_END: i32 = 5;
    pub const OPTION: i32 = 6;
    pub const TITLE: i32 = 8;
    pub const NAME: i32 = 11;
    pub const STRING: i32 = 12;
    pub const NUMBER: i32 = 13;
    pub const REPEAT: i32 = 14;
    pub const EXACT: i32 = 15;
}

fn rule_name(src: &str, at: usize) -> Option<usize> {
    let first = src[at..]
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic())?;
    let mut end = at + first.len_utf8();
    for (offset, c) in src[end..].char_indices() {
        if !(c.is_ascii_alphanumeric() || c == '-') {
            break;
        }
        end = at + first.len_utf8() + offset + c.len_utf8();
    }
    Some(end)
}

fn comment(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with(';') {
        return None;
    }
    Some(src[at..].find(['\n', '\r']).map_or(src.len(), |i| at + i))
}

fn string_literal(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with('"') {
        return None;
    }
    src[at + 1..].find('"').map(|i| at + 1 + i + 1)
}

fn num_val(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    if bytes.get(at) != Some(&b'%') {
        return None;
    }
    if !matches!(
        bytes.get(at + 1),
        Some(b'x' | b'X' | b'd' | b'D' | b'b' | b'B')
    ) {
        return None;
    }
    let hex = |from: usize| -> usize {
        let mut i = from;
        while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
            i += 1;
        }
        i
    };
    let mut end = hex(at + 2);
    if end == at + 2 {
        return None;
    }
    loop {
        let sep = bytes.get(end).copied();
        if !matches!(sep, Some(b'-') | Some(b'.')) {
            break;
        }
        let next = hex(end + 1);
        if next == end + 1 {
            break;
        }
        end = next;
    }
    Some(end)
}

fn repeat(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut i = at;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if bytes.get(i) != Some(&b'*') {
        return None;
    }
    i += 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    Some(i)
}

fn exact(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut i = at;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    (i > at).then_some(i)
}

fn abnf_tokens() -> Vec<TokenType> {
    let mut list = vec![
        opening("railroad-abnf-beta", &["ABNF_RULENAME"]),
        literal("=", &[]),
        literal(";", &["ABNF_COMMENT"]),
        literal("/", &[]),
        literal("(", &[]),
        literal(")", &[]),
        literal("[", &[]),
        literal("]", &[]),
        TokenType::new("TITLE", Pattern::Match(title_abnf), Some("t")),
        TokenType::new("ACC_TITLE", Pattern::Match(acc_title_match), Some("a")),
        TokenType::new("ACC_DESCR", Pattern::Match(acc_descr_match), Some("a")),
        TokenType::new(
            "ABNF_RULENAME",
            Pattern::Match(rule_name),
            Some("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"),
        ),
        TokenType::new("ABNF_STRING", Pattern::Match(string_literal), Some("\"")),
        TokenType::new("ABNF_NUMVAL", Pattern::Match(num_val), Some("%")),
        TokenType::new("ABNF_REPEAT", Pattern::Match(repeat), Some("0123456789*")),
        TokenType::new(
            "ABNF_EXACT_REPEAT",
            Pattern::Match(exact),
            Some("0123456789"),
        ),
    ];
    list.extend(hidden_directives("ABNF"));
    list.push(TokenType {
        hidden: true,
        ..TokenType::new("ABNF_COMMENT", Pattern::Match(comment), Some(";"))
    });
    list
}

fn title_abnf(src: &str, at: usize) -> Option<usize> {
    railroad_title(|c| c.is_ascii_alphanumeric() || c == '-')(src, at)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Bracket {
    Group,
    Optional,
}

#[derive(Clone)]
pub enum Primary {
    StringLiteral(String),
    NumVal(String),
    RuleName(String),
    Bracket(Bracket, Alternation),
}

#[derive(Clone)]
pub struct Element {
    pub repeat: Option<String>,
    pub primary: Primary,
}

#[derive(Clone, Default)]
pub struct Concatenation {
    pub elements: Vec<Element>,
}

#[derive(Clone, Default)]
pub struct Alternation {
    pub alternatives: Vec<Concatenation>,
}

pub struct Rule {
    pub name: String,
    pub definition: Alternation,
}

struct Frame {
    alternation: Alternation,
    bracket: Option<Bracket>,
    close: i32,
    repeat: Option<String>,
}

impl Frame {
    fn new(bracket: Option<Bracket>, close: i32, repeat: Option<String>) -> Frame {
        Frame {
            alternation: Alternation {
                alternatives: vec![Concatenation::default()],
            },
            bracket,
            close,
            repeat,
        }
    }
}

fn alternation(r: &mut Reader<'_>) -> Result<Alternation, PelersError> {
    let mut frames: Vec<Frame> = vec![Frame::new(None, -2, None)];
    loop {
        let mut repeat = if r.kind() == kind::REPEAT || r.kind() == kind::EXACT {
            Some(r.take())
        } else {
            None
        };
        let mut primary = match r.kind() {
            kind::STRING => {
                let text = r.take();
                Primary::StringLiteral(text[1..text.len() - 1].to_owned())
            }
            kind::NUMBER => Primary::NumVal(r.take()),
            kind::NAME => Primary::RuleName(r.take()),
            k if k == kind::GROUP || k == kind::OPTION => {
                r.i += 1;
                let bracket = if k == kind::GROUP {
                    Bracket::Group
                } else {
                    Bracket::Optional
                };
                frames.push(Frame::new(Some(bracket), k + 1, repeat.take()));
                continue;
            }
            _ => return Err(r.fail("a string, a number, a name, or an opening bracket")),
        };
        loop {
            let depth = frames.len();
            let top = frames.last_mut().expect("the root frame is never popped");
            let concatenation = top
                .alternation
                .alternatives
                .last_mut()
                .expect("a frame always holds one concatenation");
            concatenation.elements.push(Element {
                repeat: repeat.clone(),
                primary: primary.clone(),
            });
            let k = r.kind();
            if k == top.close {
                r.i += 1;
                let closed = frames.pop().expect("a bracket frame is open");
                primary = Primary::Bracket(
                    closed.bracket.expect("only a bracket frame can close"),
                    closed.alternation,
                );
                repeat = closed.repeat;
                continue;
            }
            if k == kind::SLASH {
                r.i += 1;
                top.alternation.alternatives.push(Concatenation::default());
            } else if k != kind::STRING
                && k != kind::NUMBER
                && k != kind::NAME
                && k != kind::GROUP
                && k != kind::OPTION
                && k != kind::REPEAT
                && k != kind::EXACT
            {
                if depth == 1 {
                    return Ok(frames.pop().expect("root").alternation);
                }
                let expected = if top.close == kind::GROUP_END {
                    ")"
                } else {
                    "]"
                };
                return Err(r.fail(&format!("token of type '{expected}'")));
            }
            break;
        }
    }
}

pub fn parse_abnf(src: &str) -> Result<Grammar<Rule>, PelersError> {
    let tokens = abnf_tokens();
    parse_grammar(src, &tokens, kind::TITLE, unescape_quoted, |r| {
        let name = r.expect(kind::NAME)?;
        r.expect(kind::EQUALS)?;
        let definition = alternation(r)?;
        r.expect(kind::SEMICOLON)?;
        Ok(Rule { name, definition })
    })
}

fn unescape_quoted(text: &str) -> String {
    text[1..text.len() - 1].to_owned()
}

fn parse_int_prefix(text: &str) -> f64 {
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        f64::NAN
    } else {
        digits.parse::<f64>().unwrap_or(f64::NAN)
    }
}

fn primary_node(primary: &Primary) -> RailroadNode {
    match primary {
        Primary::StringLiteral(value) | Primary::NumVal(value) => super::common::terminal(value),
        Primary::RuleName(name) => super::common::nonterminal(name),
        Primary::Bracket(Bracket::Group, alternation) => alternation_node(alternation),
        Primary::Bracket(Bracket::Optional, alternation) => {
            super::common::optional(alternation_node(alternation))
        }
    }
}

fn element_node(element: &Element) -> RailroadNode {
    let base = primary_node(&element.primary);
    let Some(repeat) = &element.repeat else {
        return base;
    };
    let star = repeat.find('*');
    let exact_form = star.is_none();
    let min = if star.is_some_and(|s| s > 0) || exact_form {
        parse_int_prefix(repeat)
    } else {
        0.0
    };
    let max = if exact_form {
        min
    } else {
        let star = star.unwrap_or(0);
        if star + 1 < repeat.len() {
            parse_int_prefix(&repeat[star + 1..])
        } else {
            f64::INFINITY
        }
    };
    if min == 0.0 && max == 1.0 {
        super::common::optional(base)
    } else {
        super::common::repetition(base, min, max)
    }
}

fn concatenation_node(concatenation: &Concatenation) -> RailroadNode {
    super::common::sequence(concatenation.elements.iter().map(element_node).collect())
}

fn alternation_node(alternation: &Alternation) -> RailroadNode {
    super::common::choice(
        alternation
            .alternatives
            .iter()
            .map(concatenation_node)
            .collect(),
    )
}

pub fn abnf_rules(grammar: &Grammar<Rule>) -> Vec<RailroadRule> {
    grammar
        .rules
        .iter()
        .map(|rule| RailroadRule {
            name: rule.name.clone(),
            definition: alternation_node(&rule.definition),
        })
        .collect()
}
