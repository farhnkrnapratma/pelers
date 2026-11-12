use crate::diagrams::common::tokens::{
    ACC_TITLE, DIRECTIVE, NEWLINE, Pattern, SINGLE_LINE_COMMENT, STRING, TITLE_BEFORE_WORD,
    TokenType, WHITESPACE, YAML, acc_descr, keyword,
};

pub const RADARCOLON: i32 = 0;
pub const RADAR: i32 = 1;
pub const SHOW_LEGEND: i32 = 2;
pub const GRATICULE: i32 = 3;
pub const CURVE: i32 = 4;
pub const TICKS: i32 = 5;
pub const AXIS: i32 = 6;
pub const MAX: i32 = 7;
pub const MIN: i32 = 8;
pub const COLON: i32 = 9;
pub const COMMA: i32 = 10;
pub const OPEN: i32 = 11;
pub const CLOSE: i32 = 12;
pub const OPEN_BRACE: i32 = 13;
pub const CLOSE_BRACE: i32 = 14;
pub const GRATICULE_VALUE: i32 = 15;
pub const BOOLEAN: i32 = 16;
pub const ACC_DESCR_KIND: i32 = 17;
pub const ACC_TITLE_KIND: i32 = 18;
pub const TITLE: i32 = 19;
pub const NUMBER: i32 = 20;
pub const STRING_KIND: i32 = 21;
pub const ID: i32 = 22;
pub const NEWLINE_KIND: i32 = 23;

fn is_word(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn name_byte(b: u8) -> bool {
    b == b'-' || is_word(b)
}

fn digits(src: &[u8], from: usize) -> usize {
    from + src[from..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count()
}

pub fn number(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let whole = digits(bytes, at);
    if whole == at {
        return None;
    }
    let dot = bytes.get(whole) == Some(&b'.');
    if dot {
        let fraction = digits(bytes, whole + 1);
        let length = fraction - whole - 1;
        if length > 0 && bytes.get(fraction) != Some(&b'.') {
            return Some(fraction);
        }
        if length > 1 {
            return Some(fraction - 1);
        }
    }
    if bytes[at] == b'0' {
        return Some(at + 1);
    }
    if !dot {
        return Some(whole);
    }
    (whole - 1 > at).then_some(whole - 1)
}

fn acc_descr_block(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let mut p = at;
    while bytes.get(p).is_some_and(|&b| b == b'\t' || b == b' ') {
        p += 1;
    }
    if !src[p..].starts_with("accDescr") {
        return None;
    }
    p += "accDescr".len();
    while let Some(c) = src[p..]
        .chars()
        .next()
        .filter(|c| crate::util::chars::is_js_space(*c))
    {
        p += c.len_utf8();
    }
    (bytes.get(p) == Some(&b'{')).then_some(p + 1)
}

fn match_acc_descr(src: &str, at: usize) -> Option<usize> {
    if let Some(end) = acc_descr_block(src, at) {
        let last_brace = src.rfind('}').map_or(-1, |b| b as isize);
        if end as isize > last_brace {
            return None;
        }
    }
    acc_descr(src, at)
}

pub fn id(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    if !bytes.get(at).is_some_and(|&b| is_word(b)) {
        return None;
    }
    let mut end = at + 1;
    while bytes.get(end).is_some_and(|&b| name_byte(b)) {
        end += 1;
    }
    while end > at + 1 && bytes[end - 1] == b'-' {
        end -= 1;
    }
    Some(end)
}

fn name_rest(src: &str, at: usize) -> bool {
    src.as_bytes()[at.min(src.len())..]
        .iter()
        .take_while(|&&b| name_byte(b))
        .any(|&b| is_word(b))
}

fn option_value(src: &str, at: usize, previous: i32, option: i32, words: &[&str]) -> Option<usize> {
    let end = words
        .iter()
        .find(|word| src[at..].starts_with(*word))
        .map(|word| at + word.len())?;
    if previous == option {
        return Some(end);
    }
    if name_rest(src, end) { None } else { Some(end) }
}

fn graticule_value(src: &str, at: usize, previous: i32) -> Option<usize> {
    option_value(src, at, previous, GRATICULE, &["circle", "polygon"])
}

fn boolean_value(src: &str, at: usize, previous: i32) -> Option<usize> {
    option_value(src, at, previous, SHOW_LEGEND, &["true", "false"])
}

const fn mark(text: &'static str) -> TokenType {
    TokenType::new(text, Pattern::Literal(text), None)
}

const fn word(text: &'static str) -> TokenType {
    TokenType {
        longer: &["ID"],
        ..TokenType::new(text, Pattern::Literal(text), None)
    }
}

pub const RADAR_TOKENS: [TokenType; 28] = [
    mark("radar-beta:"),
    TokenType {
        longer: &["ID"],
        ..keyword("radar-beta")
    },
    word("showLegend"),
    word("graticule"),
    word("curve"),
    word("ticks"),
    word("axis"),
    word("max"),
    word("min"),
    mark(":"),
    mark(","),
    mark("["),
    mark("]"),
    mark("{"),
    mark("}"),
    TokenType::new("GRATICULE", Pattern::Scan(graticule_scan), None),
    TokenType::new("BOOLEAN", Pattern::Scan(boolean_scan), None),
    TokenType {
        pattern: Pattern::Match(match_acc_descr),
        ..ACC_DESCR_TOKEN
    },
    ACC_TITLE,
    TITLE_BEFORE_WORD,
    TokenType::new("NUMBER", Pattern::Match(number), Some("-0123456789")),
    STRING,
    TokenType::new("ID", Pattern::Match(id), None),
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
];

const ACC_DESCR_TOKEN: TokenType = crate::diagrams::common::tokens::ACC_DESCR;

fn graticule_scan(src: &str, at: usize, previous: i32) -> Option<usize> {
    graticule_value(src, at, previous)
}

fn boolean_scan(src: &str, at: usize, previous: i32) -> Option<usize> {
    boolean_value(src, at, previous)
}
