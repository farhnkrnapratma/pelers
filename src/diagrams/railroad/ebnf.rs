use super::common::{
    Grammar, hidden_block_comment, hidden_directives, ident, iso_comment_match, literal, opening,
    parse_grammar, quoted, railroad_title, unescape,
};
use super::types::RailroadNode;
use crate::diagrams::common::tokens::{Pattern, Reader, TokenType};
use crate::errors::PelersError;

pub mod kind {
    pub const DEFINES: i32 = 1;
    pub const EQUALS: i32 = 2;
    pub const SEMICOLON: i32 = 3;
    pub const BAR: i32 = 4;
    pub const COMMA: i32 = 5;
    pub const GROUP: i32 = 6;
    pub const GROUP_END: i32 = 7;
    pub const OPTION: i32 = 8;
    pub const OPTION_END: i32 = 9;
    pub const REPEAT: i32 = 10;
    pub const QUESTION: i32 = 12;
    pub const STAR: i32 = 13;
    pub const PLUS: i32 = 14;
    pub const MINUS: i32 = 15;
    pub const TITLE: i32 = 16;
    pub const ID: i32 = 19;
    pub const STRING: i32 = 20;
    pub const SPECIAL: i32 = 21;
}

fn special_sequence(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with('?') {
        return None;
    }
    let mut solid = false;
    for (offset, c) in src[at + 1..].char_indices() {
        if c == '?' {
            return if solid {
                Some(at + 1 + offset + 1)
            } else {
                None
            };
        }
        if c == ';' {
            return None;
        }
        if !super::common::is_space_unit(c as u16) {
            solid = true;
        }
    }
    None
}

fn ebnf_tokens() -> Vec<TokenType> {
    let mut list = vec![
        opening("railroad-ebnf-beta", &["EBNF_ID"]),
        literal("::=", &[]),
        literal("=", &[]),
        literal(";", &[]),
        literal("|", &[]),
        literal(",", &[]),
        literal("(", &["EBNF_ISO_COMMENT"]),
        literal(")", &[]),
        literal("[", &[]),
        literal("]", &[]),
        literal("{", &[]),
        literal("}", &[]),
        literal("?", &["EBNF_SPECIAL_SEQUENCE"]),
        literal("*", &[]),
        literal("+", &[]),
        literal("-", &["EBNF_YAML"]),
        TokenType::new("TITLE", Pattern::Match(railroad_title_ebnf), Some("t")),
        TokenType::new(
            "ACC_TITLE",
            Pattern::Match(super::common::acc_title_match),
            Some("a"),
        ),
        TokenType::new(
            "ACC_DESCR",
            Pattern::Match(super::common::acc_descr_match),
            Some("a"),
        ),
        ident("EBNF_ID"),
        quoted("EBNF_STRING"),
        TokenType::new(
            "EBNF_SPECIAL_SEQUENCE",
            Pattern::Match(special_sequence),
            Some("?"),
        ),
    ];
    list.extend(hidden_directives("EBNF"));
    list.push(hidden_block_comment("EBNF_BLOCK_COMMENT"));
    list.push(TokenType {
        hidden: true,
        ..TokenType::new(
            "EBNF_ISO_COMMENT",
            Pattern::Match(iso_comment_match),
            Some("("),
        )
    });
    list
}

fn railroad_title_ebnf(src: &str, at: usize) -> Option<usize> {
    railroad_title(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')(src, at)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Bracket {
    Group,
    Optional,
    Repetition,
}

#[derive(Clone)]
pub enum Primary {
    Terminal(String),
    NonTerminal(String),
    Special(String),
    Bracket(Bracket, Choice),
}

#[derive(Clone)]
pub enum Postfix {
    Optional,
    ZeroOrMore,
    OneOrMore,
    Except(Primary),
}

#[derive(Clone)]
pub struct Term {
    pub base: Primary,
    pub postfixes: Vec<Postfix>,
}

#[derive(Clone, Default)]
pub struct Sequence {
    pub elements: Vec<Term>,
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
    term: Option<(usize, usize)>,
    bracket: Option<Bracket>,
    close: i32,
    except: bool,
}

impl Frame {
    fn new(bracket: Option<Bracket>, close: i32, except: bool) -> Frame {
        Frame {
            choice: Choice {
                alternatives: vec![Sequence::default()],
            },
            term: None,
            bracket,
            close,
            except,
        }
    }

    fn current_sequence(&mut self) -> &mut Sequence {
        self.choice
            .alternatives
            .last_mut()
            .expect("a frame always holds one sequence")
    }
}

fn definition(r: &mut Reader<'_>) -> Result<Choice, PelersError> {
    let mut frames: Vec<Frame> = vec![Frame::new(None, -2, false)];
    let mut except = false;
    loop {
        let mut primary = match r.kind() {
            kind::STRING => Primary::Terminal(unescape(&r.take())),
            kind::ID => Primary::NonTerminal(r.take()),
            kind::SPECIAL => {
                let text = r.take();
                Primary::Special(crate::util::chars::trim(&text[1..text.len() - 1]).to_owned())
            }
            k if k == kind::GROUP || k == kind::OPTION || k == kind::REPEAT => {
                r.i += 1;
                let bracket = match (k - kind::GROUP) >> 1 {
                    0 => Bracket::Group,
                    1 => Bracket::Optional,
                    _ => Bracket::Repetition,
                };
                let outer = std::mem::replace(&mut except, false);
                frames.push(Frame::new(Some(bracket), k + 1, outer));
                continue;
            }
            _ => return Err(r.fail("a terminal, a name, or an opening bracket")),
        };
        loop {
            let depth = frames.len();
            let top = frames.last_mut().expect("the root frame is never popped");
            if except {
                let (a, e) = top.term.expect("an exception follows a term");
                top.choice.alternatives[a].elements[e]
                    .postfixes
                    .push(Postfix::Except(primary));
                except = false;
            } else {
                let index = top.current_sequence().elements.len();
                let alternative = top.choice.alternatives.len() - 1;
                top.current_sequence().elements.push(Term {
                    base: primary,
                    postfixes: Vec::new(),
                });
                top.term = Some((alternative, index));
            }
            let (a, e) = top.term.expect("a term was just recorded");
            loop {
                match r.kind() {
                    kind::QUESTION => {
                        r.take();
                        top.choice.alternatives[a].elements[e]
                            .postfixes
                            .push(Postfix::Optional);
                    }
                    kind::STAR => {
                        r.take();
                        top.choice.alternatives[a].elements[e]
                            .postfixes
                            .push(Postfix::ZeroOrMore);
                    }
                    kind::PLUS => {
                        r.take();
                        top.choice.alternatives[a].elements[e]
                            .postfixes
                            .push(Postfix::OneOrMore);
                    }
                    _ => break,
                }
            }
            let k = r.kind();
            if k == top.close {
                r.i += 1;
                let closed = frames.pop().expect("a bracket frame is open");
                let bracket = closed.bracket.expect("only a bracket frame can close");
                except = closed.except;
                primary = Primary::Bracket(bracket, closed.choice);
                continue;
            }
            if k == kind::MINUS {
                r.i += 1;
                except = true;
            } else if k == kind::COMMA {
                r.i += 1;
            } else if k == kind::BAR {
                r.i += 1;
                top.choice.alternatives.push(Sequence::default());
            } else if k != kind::STRING
                && k != kind::ID
                && k != kind::SPECIAL
                && k != kind::GROUP
                && k != kind::OPTION
                && k != kind::REPEAT
            {
                if depth == 1 {
                    return Ok(frames.pop().expect("root").choice);
                }
                let expected = match top.close {
                    kind::GROUP_END => ")",
                    kind::OPTION_END => "]",
                    _ => "}",
                };
                return Err(r.fail(&format!("token of type '{expected}'")));
            }
            break;
        }
    }
}

pub fn parse_ebnf(src: &str) -> Result<Grammar<Rule>, PelersError> {
    let tokens = ebnf_tokens();
    parse_grammar(src, &tokens, kind::TITLE, unescape, |r| {
        let name = r.expect(kind::ID)?;
        if !r.accept(kind::EQUALS) {
            r.expect(kind::DEFINES)?;
        }
        let definition = definition(r)?;
        r.expect(kind::SEMICOLON)?;
        Ok(Rule { name, definition })
    })
}

fn sequence_node(elements: &[Term]) -> RailroadNode {
    let kids: Vec<RailroadNode> = elements.iter().map(term_node).collect();
    super::common::sequence(kids)
}

fn choice_node(choice: &Choice) -> RailroadNode {
    let kids: Vec<RailroadNode> = choice
        .alternatives
        .iter()
        .map(|s| sequence_node(&s.elements))
        .collect();
    super::common::choice(kids)
}

fn primary_node(primary: &Primary) -> RailroadNode {
    match primary {
        Primary::Terminal(value) => super::common::terminal(value),
        Primary::NonTerminal(name) => super::common::nonterminal(name),
        Primary::Special(text) => super::common::special(text),
        Primary::Bracket(Bracket::Group, choice) => choice_node(choice),
        Primary::Bracket(Bracket::Optional, choice) => super::common::optional(choice_node(choice)),
        Primary::Bracket(Bracket::Repetition, choice) => {
            super::common::repetition(choice_node(choice), 0.0, f64::INFINITY)
        }
    }
}

fn term_node(term: &Term) -> RailroadNode {
    let mut result = primary_node(&term.base);
    for postfix in &term.postfixes {
        result = match postfix {
            Postfix::Optional => super::common::optional(result),
            Postfix::Except(except) => RailroadNode::Sequence(vec![
                result,
                super::common::terminal("-"),
                primary_node(except),
            ]),
            Postfix::ZeroOrMore => super::common::repetition(result, 0.0, f64::INFINITY),
            Postfix::OneOrMore => super::common::repetition(result, 1.0, f64::INFINITY),
        };
    }
    result
}

pub fn ebnf_rules(grammar: &Grammar<Rule>) -> Vec<super::types::RailroadRule> {
    grammar
        .rules
        .iter()
        .map(|rule| super::types::RailroadRule {
            name: rule.name.clone(),
            definition: choice_node(&rule.definition),
        })
        .collect()
}
