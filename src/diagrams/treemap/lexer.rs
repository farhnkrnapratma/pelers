use crate::diagrams::common::tokens::{ACC_DESCR, ACC_TITLE, Pattern, TokenType, title_before};

pub const ACC_DESCR_KIND: i32 = 0;
pub const ACC_TITLE_KIND: i32 = 1;
pub const TITLE: i32 = 2;
pub const KEYWORD: i32 = 3;
pub const CLASS_DEF: i32 = 4;
pub const STYLE_SEPARATOR: i32 = 5;
pub const SEPARATOR: i32 = 6;
pub const COMMA: i32 = 7;
pub const INDENTATION: i32 = 8;
pub const NUMBER: i32 = 13;
pub const STRING: i32 = 14;
pub const ID: i32 = 12;

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

fn run_of(src: &str, at: usize, keep: impl Fn(u8) -> bool) -> Option<usize> {
    let bytes = src.as_bytes();
    let end = at
        + bytes[at.min(bytes.len())..]
            .iter()
            .take_while(|&&b| keep(b))
            .count();
    (end > at).then_some(end)
}

fn title_word(src: &str, at: usize) -> Option<usize> {
    title_before(src, at, is_word, is_word)
}

fn keyword(src: &str, at: usize) -> Option<usize> {
    if src[at..].starts_with("treemap-beta") {
        Some(at + "treemap-beta".len())
    } else if src[at..].starts_with("treemap") {
        Some(at + "treemap".len())
    } else {
        None
    }
}

fn class_def(src: &str, at: usize) -> Option<usize> {
    let rest = src[at..].strip_prefix("classDef")?;
    let mut p = at + "classDef".len();
    let after_space = rest.len()
        - rest
            .trim_start_matches(crate::util::chars::is_js_space)
            .len();
    if after_space == 0 {
        return None;
    }
    p += after_space;
    let name_end = run_of(src, p, |b| b.is_ascii_alphanumeric() || b == b'_')?;
    let first = src.as_bytes()[p];
    if name_end - p < 2 || !(first.is_ascii_alphabetic() || first == b'_') {
        return None;
    }
    p = name_end;
    let spaced = src[p..]
        .chars()
        .take_while(|&c| crate::util::chars::is_js_space(c))
        .map(char::len_utf8)
        .sum::<usize>();
    if spaced > 0 {
        let body_start = p + spaced;
        let body_end = src[body_start..]
            .find([';', '\r', '\n'])
            .map_or(src.len(), |at| body_start + at);
        p = body_end;
    }
    Some(if src.as_bytes().get(p) == Some(&b';') {
        p + 1
    } else {
        p
    })
}

fn number2(src: &str, at: usize) -> Option<usize> {
    run_of(src, at, |b| {
        b.is_ascii_digit() || b == b'_' || b == b'.' || b == b','
    })
}

fn id2(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    if !bytes
        .get(at)
        .is_some_and(|&b| b.is_ascii_alphabetic() || b == b'_')
    {
        return None;
    }
    run_of(src, at, |b| b.is_ascii_alphanumeric() || b == b'_')
}

fn string2(src: &str, at: usize) -> Option<usize> {
    let quote = *src.as_bytes().get(at)?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let close = src[at + 1..].find(quote as char)?;
    Some(at + 1 + close + 1)
}

fn indentation(src: &str, at: usize) -> Option<usize> {
    run_of(src, at, blank)
}

fn whitespace(src: &str, at: usize) -> Option<usize> {
    run_of(src, at, blank)
}

fn comment(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with("%%") {
        return None;
    }
    Some(src[at..].find('\n').map_or(src.len(), |n| at + n))
}

fn newline(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    match bytes.get(at)? {
        b'\n' => Some(at + 1),
        b'\r' if bytes.get(at + 1) == Some(&b'\n') => Some(at + 2),
        _ => None,
    }
}

const fn literal(text: &'static str) -> TokenType {
    TokenType::new(text, Pattern::Literal(text), None)
}

const fn hidden(token: TokenType) -> TokenType {
    TokenType {
        hidden: true,
        ..token
    }
}

pub const TREEMAP_TOKENS: [TokenType; 15] = [
    ACC_DESCR,
    ACC_TITLE,
    TokenType::new("TITLE", Pattern::Match(title_word), Some("\t t")),
    TokenType::new("TREEMAP_KEYWORD", Pattern::Match(keyword), None),
    TokenType::new("CLASS_DEF", Pattern::Match(class_def), None),
    literal(":::"),
    literal(":"),
    literal(","),
    TokenType::new("INDENTATION", Pattern::Match(indentation), Some("\t ")),
    hidden(TokenType::new(
        "WS",
        Pattern::Match(whitespace),
        Some("\t "),
    )),
    hidden(TokenType::new("ML_COMMENT", Pattern::Match(comment), None)),
    hidden(TokenType::new("NL", Pattern::Match(newline), Some("\r\n"))),
    TokenType::new("ID2", Pattern::Match(id2), None),
    TokenType::new("NUMBER2", Pattern::Match(number2), None),
    TokenType::new("STRING2", Pattern::Match(string2), Some("\"'")),
];

pub fn class_def_parts(text: &str) -> (String, Option<String>) {
    let rest = text.strip_prefix("classDef").unwrap_or(text);
    let rest = rest.trim_start_matches(crate::util::chars::is_js_space);
    let name_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
        .count();
    let name = rest[..name_len].to_owned();
    let after = &rest[name_len..];
    let spaced = after.len()
        - after
            .trim_start_matches(crate::util::chars::is_js_space)
            .len();
    if spaced == 0 {
        return (name, None);
    }
    let body = &after[spaced..];
    let end = body.find([';', '\r', '\n']).unwrap_or(body.len());
    (name, Some(body[..end].to_owned()))
}
