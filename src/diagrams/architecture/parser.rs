use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, NEWLINE, Pattern, Reader, SINGLE_LINE_COMMENT, STRING,
    TokenType, WHITESPACE, YAML, acc_descr_value, acc_title_value, string_value, title_before,
    title_value, tokenize,
};
use crate::errors::PelersError;
use crate::util::chars::trim;

pub mod kind {
    pub const HEADER: i32 = 0;
    pub const JUNCTION: i32 = 1;
    pub const SERVICE: i32 = 2;
    pub const COLUMN: i32 = 3;
    pub const GROUP: i32 = 4;
    pub const ALIGN: i32 = 5;
    pub const ROW: i32 = 6;
    pub const DASHES: i32 = 7;
    pub const IN: i32 = 8;
    pub const COLON: i32 = 9;
    pub const DASH: i32 = 10;
    pub const DIRECTION: i32 = 11;
    pub const ARROW_GROUP: i32 = 12;
    pub const INTO: i32 = 13;
    pub const ACC_DESCR: i32 = 14;
    pub const ACC_TITLE: i32 = 15;
    pub const TITLE: i32 = 16;
    pub const STRING: i32 = 17;
    pub const ID: i32 = 18;
    pub const NEWLINE: i32 = 19;
    pub const ARCH_ICON: i32 = 24;
    pub const LABEL: i32 = 25;
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn name_run(c: char) -> bool {
    is_word(c) || c == '-'
}

fn word_run_end(src: &str, from: usize) -> usize {
    let mut found = from;
    for (offset, c) in src[from..].char_indices() {
        if !(is_word(c) || c == '-') {
            break;
        }
        if is_word(c) {
            found = from + offset + c.len_utf8();
        }
    }
    found
}

fn id(src: &str, at: usize) -> Option<usize> {
    let first = src[at..].chars().next().filter(|&c| is_word(c))?;
    Some(word_run_end(src, at + first.len_utf8()))
}

fn direction(src: &str, at: usize) -> Option<usize> {
    let c = src[at..].chars().next()?;
    matches!(c, 'L' | 'R' | 'T' | 'B').then(|| at + 1)
}

fn arrow_into(src: &str, at: usize) -> Option<usize> {
    let c = src[at..].chars().next()?;
    matches!(c, '<' | '>').then(|| at + 1)
}

fn arrow_group(src: &str, at: usize) -> Option<usize> {
    src[at..]
        .starts_with("{group}")
        .then(|| at + "{group}".len())
}

fn title(src: &str, at: usize) -> Option<usize> {
    title_before(src, at, name_run, is_word)
}

fn arch_icon(src: &str, at: usize) -> Option<usize> {
    let rest = src[at..].strip_prefix('(')?;
    let run = rest
        .chars()
        .take_while(|&c| is_word(c) || c == '-' || c == ':')
        .map(char::len_utf8)
        .sum::<usize>();
    (run > 0 && rest[run..].starts_with(')')).then(|| at + 1 + run + 1)
}

fn quoted_alt(src: &str, at: usize, quote: char) -> Option<usize> {
    let mut i = at + 2;
    while i < src.len() {
        let c = src[i..].chars().next()?;
        if c == quote {
            return src[i + 1..].starts_with(']').then(|| i + 2);
        }
        if c == '\\' {
            let next = src[i + 1..].chars().next()?;
            if matches!(next, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
                return None;
            }
            i += 1 + next.len_utf8();
        } else {
            i += c.len_utf8();
        }
    }
    None
}

fn arch_title(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with('[') {
        return None;
    }
    let next = src[at + 1..].chars().next();
    if next == Some('"')
        && let Some(end) = quoted_alt(src, at, '"')
    {
        return Some(end);
    }
    if next == Some('\'')
        && let Some(end) = quoted_alt(src, at, '\'')
    {
        return Some(end);
    }
    let run: usize = src[at + 1..]
        .chars()
        .take_while(|&c| !matches!(c, '[' | ']' | '\r' | '\n'))
        .map(char::len_utf8)
        .sum();
    (run > 0 && src[at + 1 + run..].starts_with(']')).then(|| at + 1 + run + 1)
}

const fn keyword(text: &'static str) -> TokenType {
    TokenType {
        longer: &["ID"],
        ..TokenType::new(text, Pattern::Literal(text), None)
    }
}

pub const ARCHITECTURE_TOKENS: &[TokenType] = &[
    keyword("architecture-beta"),
    keyword("junction"),
    keyword("service"),
    keyword("column"),
    keyword("group"),
    keyword("align"),
    keyword("row"),
    TokenType {
        longer: &["YAML"],
        ..TokenType::new("--", Pattern::Literal("--"), None)
    },
    keyword("in"),
    TokenType::new(":", Pattern::Literal(":"), None),
    TokenType {
        longer: &["YAML"],
        ..TokenType::new("-", Pattern::Literal("-"), None)
    },
    TokenType::new("ARROW_DIRECTION", Pattern::Match(direction), None),
    TokenType::new("ARROW_GROUP", Pattern::Match(arrow_group), None),
    TokenType::new("ARROW_INTO", Pattern::Match(arrow_into), None),
    ACC_DESCR,
    ACC_TITLE,
    TokenType::new("TITLE", Pattern::Match(title), Some("\t t")),
    STRING,
    TokenType::new("ID", Pattern::Match(id), None),
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
    TokenType::new("ARCH_ICON", Pattern::Match(arch_icon), None),
    TokenType::new("ARCH_TITLE", Pattern::Match(arch_title), None),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    L,
    R,
    T,
    B,
}

impl Side {
    fn parse(text: &str) -> Side {
        match text {
            "L" => Side::L,
            "R" => Side::R,
            "T" => Side::T,
            _ => Side::B,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GroupAst {
    pub id: String,
    pub icon: Option<String>,
    pub title: Option<String>,
    pub in_group: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ServiceAst {
    pub id: String,
    pub icon: Option<String>,
    pub icon_text: Option<String>,
    pub title: Option<String>,
    pub in_group: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JunctionAst {
    pub id: String,
    pub in_group: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdgeAst {
    pub lhs_id: String,
    pub lhs_group: bool,
    pub lhs_dir: Side,
    pub lhs_into: bool,
    pub title: Option<String>,
    pub rhs_into: bool,
    pub rhs_dir: Side,
    pub rhs_id: String,
    pub rhs_group: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AlignmentAst {
    pub row: bool,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArchitectureAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub groups: Vec<GroupAst>,
    pub services: Vec<ServiceAst>,
    pub junctions: Vec<JunctionAst>,
    pub edges: Vec<EdgeAst>,
    pub alignments: Vec<AlignmentAst>,
}

fn label_value(text: &str) -> String {
    let inner = text.get(1..text.len() - 1).unwrap_or("");
    let mut result = trim(inner).to_owned();
    let quoted = (result.starts_with('"') && result.ends_with('"'))
        || (result.starts_with('\'') && result.ends_with('\''));
    if quoted {
        let body = result.get(1..result.len().saturating_sub(1)).unwrap_or("");
        result = body.replace("\\\"", "\"").replace("\\'", "'");
    }
    trim(&result).to_owned()
}

fn end_of_line(r: &mut Reader<'_>) -> Result<(), PelersError> {
    if r.kind() == -1 {
        return Ok(());
    }
    if r.kind() != kind::NEWLINE {
        return Err(r.fail("a line break or the end of input"));
    }
    while r.kind() == kind::NEWLINE {
        r.i += 1;
    }
    Ok(())
}

fn strip_bracket_pair(text: String) -> String {
    text.get(1..text.len() - 1).unwrap_or("").to_owned()
}

pub fn parse_architecture(src: &str) -> Result<ArchitectureAst, PelersError> {
    let tokens = tokenize(src, ARCHITECTURE_TOKENS, "architecture")?;
    let mut r = Reader::new(tokens, ARCHITECTURE_TOKENS, "architecture");
    let mut ast = ArchitectureAst::default();

    while r.kind() == kind::NEWLINE {
        r.i += 1;
    }
    r.expect(kind::HEADER)?;
    while r.kind() != -1 {
        match r.kind() {
            kind::NEWLINE => {
                r.i += 1;
                continue;
            }
            kind::ACC_DESCR => {
                ast.acc_descr = Some(acc_descr_value(&r.take()));
            }
            kind::ACC_TITLE => {
                ast.acc_title = Some(acc_title_value(&r.take()));
            }
            kind::TITLE => {
                ast.title = Some(title_value(&r.take()));
            }
            kind::GROUP => {
                r.i += 1;
                let mut group = GroupAst {
                    id: r.expect(kind::ID)?,
                    ..GroupAst::default()
                };
                if r.kind() == kind::ARCH_ICON {
                    group.icon = Some(strip_bracket_pair(r.take()));
                }
                if r.kind() == kind::LABEL {
                    group.title = Some(label_value(&r.take()));
                }
                if r.accept(kind::IN) {
                    group.in_group = Some(r.expect(kind::ID)?);
                }
                ast.groups.push(group);
            }
            kind::SERVICE => {
                r.i += 1;
                let mut service = ServiceAst {
                    id: r.expect(kind::ID)?,
                    ..ServiceAst::default()
                };
                if r.kind() == kind::STRING {
                    service.icon_text = Some(string_value(&r.take()));
                } else if r.kind() == kind::ARCH_ICON {
                    service.icon = Some(strip_bracket_pair(r.take()));
                }
                if r.kind() == kind::LABEL {
                    service.title = Some(label_value(&r.take()));
                }
                if r.accept(kind::IN) {
                    service.in_group = Some(r.expect(kind::ID)?);
                }
                ast.services.push(service);
            }
            kind::JUNCTION => {
                r.i += 1;
                let mut junction = JunctionAst {
                    id: r.expect(kind::ID)?,
                    in_group: None,
                };
                if r.accept(kind::IN) {
                    junction.in_group = Some(r.expect(kind::ID)?);
                }
                ast.junctions.push(junction);
            }
            kind::ID => {
                let lhs_id = r.take();
                let lhs_group = r.accept(kind::ARROW_GROUP);
                r.expect(kind::COLON)?;
                let lhs_dir = Side::parse(&r.expect(kind::DIRECTION)?);
                let lhs_into = r.accept(kind::INTO);
                let mut title = None;
                if !r.accept(kind::DASHES) {
                    if r.kind() != kind::DASH {
                        return Err(r.fail("'--' or '-'"));
                    }
                    r.i += 1;
                    title = Some(label_value(&r.expect(kind::LABEL)?));
                    r.expect(kind::DASH)?;
                }
                let rhs_into = r.accept(kind::INTO);
                let rhs_dir = Side::parse(&r.expect(kind::DIRECTION)?);
                r.expect(kind::COLON)?;
                let rhs_id = r.expect(kind::ID)?;
                let rhs_group = r.accept(kind::ARROW_GROUP);
                ast.edges.push(EdgeAst {
                    lhs_id,
                    lhs_group,
                    lhs_dir,
                    lhs_into,
                    title,
                    rhs_into,
                    rhs_dir,
                    rhs_id,
                    rhs_group,
                });
            }
            kind::ALIGN => {
                r.i += 1;
                if r.kind() != kind::ROW && r.kind() != kind::COLUMN {
                    return Err(r.fail("'row' or 'column'"));
                }
                let row = r.kind() == kind::ROW;
                r.i += 1;
                let mut members = vec![r.expect(kind::ID)?, r.expect(kind::ID)?];
                while r.kind() == kind::ID {
                    members.push(r.take());
                }
                ast.alignments.push(AlignmentAst { row, members });
            }
            _ => {
                return Err(r.fail(
                    "a group, a service, a junction, an edge, an alignment, a title, or a line break",
                ));
            }
        }
        end_of_line(&mut r)?;
    }
    Ok(ast)
}
