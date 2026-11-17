use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, NEWLINE, Pattern, SINGLE_LINE_COMMENT, STRING, TokenType,
    WHITESPACE, YAML, keyword, title_before,
};
use crate::util::chars::is_js_space;

pub mod kind {
    pub const CHERRY_PICK: i32 = 0;
    pub const GIT_GRAPH_COLON: i32 = 1;
    pub const HIGHLIGHT: i32 = 2;
    pub const GIT_GRAPH: i32 = 3;
    pub const CHECKOUT: i32 = 4;
    pub const REVERSE: i32 = 5;
    pub const PARENT: i32 = 6;
    pub const COMMIT: i32 = 7;
    pub const NORMAL: i32 = 8;
    pub const BRANCH: i32 = 9;
    pub const ORDER: i32 = 10;
    pub const SWITCH: i32 = 11;
    pub const TYPE: i32 = 12;
    pub const MERGE: i32 = 13;
    pub const MSG: i32 = 14;
    pub const TAG: i32 = 15;
    pub const ID: i32 = 16;
    pub const LR: i32 = 17;
    pub const TB: i32 = 18;
    pub const BT: i32 = 19;
    pub const COLON: i32 = 20;
    pub const ACC_DESCR: i32 = 21;
    pub const ACC_TITLE: i32 = 22;
    pub const TITLE: i32 = 23;
    pub const INT: i32 = 24;
    pub const STRING: i32 = 25;
    pub const NEWLINE: i32 = 26;
    pub const REFERENCE: i32 = 31;
}

fn word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn name_run(c: char) -> bool {
    word_char(c) || matches!(c, '-' | '.' | '/')
}

fn name_last(c: char) -> bool {
    word_char(c) || c == '-'
}

fn run_end(src: &str, from: usize) -> usize {
    let mut found = from;
    for (offset, c) in src[from..].char_indices() {
        if !name_run(c) {
            break;
        }
        if name_last(c) {
            found = from + offset + c.len_utf8();
        }
    }
    found
}

fn reference(src: &str, at: usize) -> Option<usize> {
    let first = src[at..].chars().next().filter(|&c| word_char(c))?;
    Some(run_end(src, at + first.len_utf8()))
}

fn integer(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    match bytes.get(at)? {
        b'0' => Some(at + 1),
        b'1'..=b'9' => {
            let mut max = at + 1;
            while bytes.get(max).is_some_and(u8::is_ascii_digit) {
                max += 1;
            }
            (at + 1..=max)
                .rev()
                .find(|&end| bytes.get(end) != Some(&b'.'))
        }
        _ => None,
    }
}

fn title(src: &str, at: usize) -> Option<usize> {
    title_before(src, at, name_run, name_last)
}

fn acc_descr_opener(src: &str, at: usize) -> Option<usize> {
    let rest = src[at..].trim_start_matches(['\t', ' ']);
    let rest = rest
        .strip_prefix("accDescr")?
        .trim_start_matches(is_js_space);
    rest.starts_with('{').then_some(at)
}

const NAME: &[&str] = &["REFERENCE"];

const fn word(name: &'static str) -> TokenType {
    TokenType {
        longer: NAME,
        ..TokenType::new(name, Pattern::Literal(name), None)
    }
}

const fn literal(name: &'static str) -> TokenType {
    TokenType::new(name, Pattern::Literal(name), None)
}

pub const GIT_TOKENS: &[TokenType] = &[
    word("cherry-pick"),
    literal("gitGraph:"),
    word("HIGHLIGHT"),
    TokenType {
        longer: NAME,
        ..keyword("gitGraph")
    },
    word("checkout"),
    word("REVERSE"),
    literal("parent:"),
    word("commit"),
    word("NORMAL"),
    word("branch"),
    literal("order:"),
    word("switch"),
    literal("type:"),
    word("merge"),
    literal("msg:"),
    literal("tag:"),
    literal("id:"),
    word("LR"),
    word("TB"),
    word("BT"),
    literal(":"),
    TokenType {
        opener: Some(acc_descr_opener),
        ..ACC_DESCR
    },
    ACC_TITLE,
    TokenType::new("TITLE", Pattern::Match(title), Some("\t t")),
    TokenType::new("INT", Pattern::Match(integer), None),
    STRING,
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
    TokenType::new("REFERENCE", Pattern::Match(reference), None),
];
