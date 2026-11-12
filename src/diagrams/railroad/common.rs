use crate::diagrams::common::tokens::{
    DIRECTIVE, Pattern, Reader, SINGLE_LINE_COMMENT, TokenType, YAML, acc_descr_value,
    acc_title_value, title_value,
};
use crate::errors::PelersError;
use crate::util::chars::is_js_space;

pub fn is_space_unit(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

fn class_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

pub fn identifier_match(src: &str, at: usize) -> Option<usize> {
    let first = src[at..]
        .chars()
        .next()
        .filter(|c| c.is_ascii_alphabetic() || *c == '_')?;
    let mut end = at + first.len_utf8();
    for (offset, c) in src[end..].char_indices() {
        if !class_char(c) {
            break;
        }
        end = at + first.len_utf8() + offset + c.len_utf8();
    }
    Some(end)
}

pub fn quoted_match(src: &str, at: usize) -> Option<usize> {
    let quote = src[at..]
        .chars()
        .next()
        .filter(|&c| c == '"' || c == '\'')?;
    let mut i = at + 1;
    while i < src.len() {
        let c = src[i..].chars().next()?;
        if c == quote {
            return Some(i + 1);
        }
        if c == '\\' {
            let next = src[i + 1..].chars().next()?;
            i += 1 + next.len_utf8();
        } else {
            i += c.len_utf8();
        }
    }
    None
}

fn line_or_comment_end(src: &str, from: usize) -> usize {
    let mut i = from;
    for (offset, c) in src[from..].char_indices() {
        if c == '\n' || c == '\r' {
            return from + offset;
        }
        if c == '%' && src[from + offset..].starts_with("%%") {
            return from + offset;
        }
        i = from + offset + c.len_utf8();
    }
    i
}

fn line_end(src: &str, from: usize) -> usize {
    src[from..]
        .find(['\n', '\r'])
        .map_or(src.len(), |i| from + i)
}

pub fn railroad_title(rest: fn(char) -> bool) -> impl Fn(&str, usize) -> Option<usize> {
    move |src: &str, at: usize| {
        let after = src[at..].strip_prefix("title")?;
        let r = src.len() - after.len();
        match src[r..].chars().next() {
            Some(c) if c == ' ' || c == '\t' => {
                let from = r + 1;
                let end = line_end(src, from);
                let stop = src[from..end].find("%%").map_or(end, |i| from + i);
                Some(stop)
            }
            Some(c) if rest(c) => None,
            _ => Some(r),
        }
    }
}

pub fn acc_title_match(src: &str, at: usize) -> Option<usize> {
    let after = src[at..].strip_prefix("accTitle")?;
    let mut i = src.len() - after.len();
    while src
        .as_bytes()
        .get(i)
        .is_some_and(|&b| b == b' ' || b == b'\t')
    {
        i += 1;
    }
    if src.as_bytes().get(i) != Some(&b':') {
        return None;
    }
    Some(line_or_comment_end(src, i + 1))
}

pub fn acc_descr_match(src: &str, at: usize) -> Option<usize> {
    let after = src[at..].strip_prefix("accDescr")?;
    let mut i = src.len() - after.len();
    let bytes = src.as_bytes();
    while bytes.get(i).is_some_and(|&b| b == b' ' || b == b'\t') {
        i += 1;
    }
    if bytes.get(i) == Some(&b':') {
        return Some(line_or_comment_end(src, i + 1));
    }
    while let Some(c) = src[i..].chars().next().filter(|&c| is_space_unit(c as u16)) {
        i += c.len_utf8();
    }
    if bytes.get(i) != Some(&b'{') {
        return None;
    }
    src[i + 1..].find('}').map(|close| i + 1 + close + 1)
}

pub fn block_comment_match(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with("/*") {
        return None;
    }
    src[at + 2..].find("*/").map(|i| at + 2 + i + 2)
}

pub fn iso_comment_match(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with("(*") {
        return None;
    }
    src[at + 2..].find("*)").map(|i| at + 2 + i + 2)
}

pub fn ident(name: &'static str) -> TokenType {
    TokenType::new(
        name,
        Pattern::Match(identifier_match),
        Some("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_"),
    )
}

pub fn quoted(name: &'static str) -> TokenType {
    TokenType::new(name, Pattern::Match(quoted_match), Some("\"'"))
}

pub const fn literal(name: &'static str, longer: &'static [&'static str]) -> TokenType {
    TokenType {
        longer,
        ..TokenType::new(name, Pattern::Literal(name), None)
    }
}

pub const fn opening(word: &'static str, longer: &'static [&'static str]) -> TokenType {
    TokenType {
        longer,
        ..TokenType::new(word, Pattern::Literal(word), None)
    }
}

pub fn hidden_whitespace(name: &'static str) -> TokenType {
    TokenType {
        hidden: true,
        ..TokenType::new(name, Pattern::Match(whitespace_match), Some("\t \r\n"))
    }
}

fn whitespace_match(src: &str, at: usize) -> Option<usize> {
    let run = src.as_bytes()[at..]
        .iter()
        .take_while(|&&b| matches!(b, b'\t' | b' ' | b'\r' | b'\n'))
        .count();
    (run > 0).then_some(at + run)
}

pub fn hidden_block_comment(name: &'static str) -> TokenType {
    TokenType {
        hidden: true,
        ..TokenType::new(name, Pattern::Match(block_comment_match), Some("/"))
    }
}

pub fn hidden_directives(prefix: &'static str) -> [TokenType; 4] {
    let names = match prefix {
        "EBNF" => [
            "EBNF_WHITESPACE",
            "EBNF_YAML",
            "EBNF_DIRECTIVE",
            "EBNF_SINGLE_LINE_COMMENT",
        ],
        "RR" => [
            "RR_WHITESPACE",
            "RR_YAML",
            "RR_DIRECTIVE",
            "RR_SINGLE_LINE_COMMENT",
        ],
        "ABNF" => [
            "ABNF_WHITESPACE",
            "ABNF_YAML",
            "ABNF_DIRECTIVE",
            "ABNF_SINGLE_LINE_COMMENT",
        ],
        _ => [
            "PEG_WHITESPACE",
            "PEG_YAML",
            "PEG_DIRECTIVE",
            "PEG_SINGLE_LINE_COMMENT",
        ],
    };
    [
        hidden_whitespace(names[0]),
        TokenType {
            name: names[1],
            ..YAML
        },
        TokenType {
            name: names[2],
            ..DIRECTIVE
        },
        TokenType {
            name: names[3],
            ..SINGLE_LINE_COMMENT
        },
    ]
}

pub struct Grammar<Rule> {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub rules: Vec<Rule>,
}

pub fn parse_grammar<Rule>(
    src: &str,
    types: &[TokenType],
    meta: i32,
    unquote: fn(&str) -> String,
    mut rule: impl FnMut(&mut Reader<'_>) -> Result<Rule, PelersError>,
) -> Result<Grammar<Rule>, PelersError> {
    let tokens = crate::diagrams::common::tokens::tokenize(src, types, "railroad")?;
    let mut r = Reader::new(tokens, types, "railroad");
    let mut ast = Grammar {
        title: None,
        acc_title: None,
        acc_descr: None,
        rules: Vec::new(),
    };
    r.expect(0)?;
    loop {
        let kind = r.kind();
        if kind == meta {
            let title = title_value(&r.take());
            let quote = title.chars().next();
            ast.title = Some(
                if matches!(quote, Some('"') | Some('\'')) && title.ends_with(quote.unwrap_or('"'))
                {
                    unquote(&title)
                } else {
                    title
                },
            );
        } else if kind == meta + 1 {
            ast.acc_title = Some(acc_title_value(&r.take()));
        } else if kind == meta + 2 {
            ast.acc_descr = Some(acc_descr_value(&r.take()));
        } else {
            break;
        }
    }
    while r.kind() != -1 {
        ast.rules.push(rule(&mut r)?);
    }
    Ok(ast)
}

pub fn unescape(text: &str) -> String {
    if !text.contains('\\') {
        return text[1..text.len() - 1].to_owned();
    }
    let inner: Vec<char> = text[1..text.len() - 1].chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < inner.len() {
        let c = inner[i];
        if c == '\\' && i + 1 < inner.len() {
            i += 1;
            out.push(match inner[i] {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

pub fn terminal(value: &str) -> RailroadNode {
    RailroadNode::Terminal(value.to_owned())
}

pub fn nonterminal(name: &str) -> RailroadNode {
    RailroadNode::Nonterminal(name.to_owned())
}

pub fn special(text: &str) -> RailroadNode {
    RailroadNode::Special(text.to_owned())
}

pub fn optional(element: RailroadNode) -> RailroadNode {
    RailroadNode::Optional(Box::new(element))
}

pub fn repetition(element: RailroadNode, min: f64, max: f64) -> RailroadNode {
    RailroadNode::Repetition {
        element: Box::new(element),
        min,
        max,
    }
}

pub fn sequence(mut elements: Vec<RailroadNode>) -> RailroadNode {
    if elements.len() == 1 {
        return elements.remove(0);
    }
    RailroadNode::Sequence(elements)
}

pub fn choice(mut alternatives: Vec<RailroadNode>) -> RailroadNode {
    if alternatives.len() == 1 {
        return alternatives.remove(0);
    }
    RailroadNode::Choice(alternatives)
}

use super::types::RailroadNode;
