use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, Pattern, Reader, TokenType, acc_descr_value, acc_title_value, keyword,
    title_value, tokenize,
};
use crate::errors::PelersError;
use crate::util::chars::trim;

pub mod kind {
    pub const KEYWORD: i32 = 0;
    pub const ACC_DESCR: i32 = 1;
    pub const ACC_TITLE: i32 = 2;
    pub const TITLE: i32 = 3;
    pub const CLASS_ANNOTATION: i32 = 4;
    pub const ICON_ANNOTATION: i32 = 5;
    pub const DESC_ANNOTATION: i32 = 6;
    pub const INDENTATION: i32 = 7;
    pub const QUOTED_NAME: i32 = 8;
    pub const BARE_NAME: i32 = 12;
}

fn is_blank(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn blank_run(bytes: &[u8], from: usize) -> usize {
    let mut p = from;
    while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
        p += 1;
    }
    p
}

fn word_run(bytes: &[u8], from: usize) -> usize {
    let mut p = from;
    while p < bytes.len()
        && (bytes[p].is_ascii_alphanumeric() || bytes[p] == b'_' || bytes[p] == b'-')
    {
        p += 1;
    }
    p
}

fn line_end(src: &str, from: usize) -> usize {
    src[from..]
        .find(['\n', '\r'])
        .map_or(src.len(), |i| from + i)
}

fn annotation_at(src: &str, at: usize) -> bool {
    let rest = &src[at..];
    if rest.starts_with("##") || rest.starts_with("icon(") {
        return true;
    }
    let Some(after) = rest.strip_prefix(":::") else {
        return false;
    };
    after
        .trim_start_matches(is_blank)
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
}

fn bare_name(src: &str, at: usize) -> Option<usize> {
    let first = src[at..].chars().next()?;
    if is_blank(first) || matches!(first, '\n' | '\r' | '"' | '\'') {
        return None;
    }
    let rest = &src[at..];
    if rest.starts_with(":::") || rest.starts_with("icon(") || rest.starts_with("##") {
        return None;
    }
    let bytes = src.as_bytes();
    let mut i = at + first.len_utf8();
    while i < src.len() {
        let c = src[i..].chars().next()?;
        if c == '\n' || c == '\r' {
            break;
        }
        if is_blank(c) {
            let j = blank_run(bytes, i + 1);
            if annotation_at(src, j) {
                break;
            }
            i = j;
        } else {
            i += c.len_utf8();
        }
    }
    Some(i)
}

fn class_annotation(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let p = blank_run(bytes, at);
    if p == at {
        return None;
    }
    let after = src[p..].strip_prefix(":::")?;
    let name = blank_run(bytes, src.len() - after.len());
    let first = src[name..].chars().next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    Some(word_run(bytes, name + 1))
}

fn icon_annotation(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let p = blank_run(bytes, at);
    if p == at {
        return None;
    }
    let after = src[p..].strip_prefix("icon(")?;
    let name_end = word_run(bytes, src.len() - after.len());
    match bytes.get(name_end) {
        Some(b')') => Some(name_end + 1),
        Some(b':') => {
            let part_end = word_run(bytes, name_end + 1);
            if part_end == name_end + 1 || bytes.get(part_end) != Some(&b')') {
                return None;
            }
            Some(part_end + 1)
        }
        _ => None,
    }
}

fn desc_annotation(src: &str, at: usize) -> Option<usize> {
    let p = blank_run(src.as_bytes(), at);
    if p == at {
        return None;
    }
    let after = src[p..].strip_prefix("##")?;
    Some(line_end(src, src.len() - after.len()))
}

fn indentation(src: &str, at: usize) -> Option<usize> {
    let p = blank_run(src.as_bytes(), at);
    (p > at).then_some(p)
}

fn quoted_name(src: &str, at: usize) -> Option<usize> {
    let quote = src[at..]
        .chars()
        .next()
        .filter(|&c| c == '"' || c == '\'')?;
    let close = src[at + 1..].find(quote)?;
    Some(at + 1 + close + 1)
}

fn ml_comment(src: &str, at: usize) -> Option<usize> {
    src[at..].starts_with("%%").then(|| line_end(src, at))
}

fn ws(src: &str, at: usize) -> Option<usize> {
    indentation(src, at)
}

fn nl(src: &str, at: usize) -> Option<usize> {
    let rest = &src[at..];
    if rest.starts_with("\r\n") {
        Some(at + 2)
    } else if rest.starts_with('\n') {
        Some(at + 1)
    } else {
        None
    }
}

fn title(src: &str, at: usize) -> Option<usize> {
    let p = blank_run(src.as_bytes(), at);
    let after = src[p..].strip_prefix("title")?;
    let r = src.len() - after.len();
    match src[r..].chars().next() {
        Some(c) if is_blank(c) => {
            let from = r + 1;
            let line = line_end(src, from);
            Some(src[from..line].find("%%").map_or(line, |i| from + i))
        }
        Some(c) if !matches!(c, '\n' | '\r') && !src[r..].starts_with("%%") => None,
        _ => Some(r),
    }
}

pub const TREEVIEW_TOKENS: &[TokenType] = &[
    TokenType {
        longer: &["BARE_NAME"],
        ..keyword("treeView-beta")
    },
    ACC_DESCR,
    ACC_TITLE,
    TokenType::new("TITLE", Pattern::Match(title), Some("\t t")),
    TokenType::new("CLASS_ANNOTATION", Pattern::Match(class_annotation), None),
    TokenType::new("ICON_ANNOTATION", Pattern::Match(icon_annotation), None),
    TokenType::new("DESC_ANNOTATION", Pattern::Match(desc_annotation), None),
    TokenType::new("INDENTATION", Pattern::Match(indentation), None),
    TokenType::new("QUOTED_NAME", Pattern::Match(quoted_name), None),
    TokenType {
        hidden: true,
        ..TokenType::new("WS", Pattern::Match(ws), None)
    },
    TokenType {
        hidden: true,
        ..TokenType::new("ML_COMMENT", Pattern::Match(ml_comment), None)
    },
    TokenType {
        hidden: true,
        ..TokenType::new("NL", Pattern::Match(nl), None)
    },
    TokenType::new("BARE_NAME", Pattern::Match(bare_name), None),
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeNodeAst {
    pub indent: Option<usize>,
    pub name: String,
    pub class_annotation: Option<String>,
    pub icon_annotation: Option<String>,
    pub desc_annotation: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeViewAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub nodes: Vec<TreeNodeAst>,
}

fn trim_blanks_end(text: &str) -> &str {
    text.trim_end_matches(is_blank)
}

pub fn convert_class(input: &str) -> String {
    trim(trim(input).get(3..).unwrap_or("")).to_owned()
}

pub fn convert_icon(input: &str) -> String {
    let trimmed = trim(input);
    trimmed
        .get(5..trimmed.len().saturating_sub(1))
        .unwrap_or("")
        .to_owned()
}

pub fn convert_desc(input: &str) -> String {
    trim(trim(input).get(2..).unwrap_or("")).to_owned()
}

pub fn parse_treeview(src: &str) -> Result<TreeViewAst, PelersError> {
    let tokens = tokenize(src, TREEVIEW_TOKENS, "treeView")?;
    let mut r = Reader::new(tokens, TREEVIEW_TOKENS, "treeView");
    let mut ast = TreeViewAst::default();
    r.expect(kind::KEYWORD)?;
    loop {
        match r.kind() {
            kind::ACC_DESCR => ast.acc_descr = Some(acc_descr_value(&r.take())),
            kind::ACC_TITLE => ast.acc_title = Some(acc_title_value(&r.take())),
            kind::TITLE => ast.title = Some(title_value(&r.take())),
            _ => break,
        }
    }
    while r.kind() != -1 {
        let mut node = TreeNodeAst::default();
        if r.kind() == kind::INDENTATION {
            node.indent = Some(r.take().len());
        }
        if r.kind() == kind::QUOTED_NAME {
            let text = r.take();
            node.name = text[1..text.len() - 1].to_owned();
        } else if r.kind() == kind::BARE_NAME {
            node.name = trim_blanks_end(&r.take()).to_owned();
        } else {
            return Err(r.fail("a name"));
        }
        loop {
            match r.kind() {
                kind::CLASS_ANNOTATION => node.class_annotation = Some(convert_class(&r.take())),
                kind::ICON_ANNOTATION => node.icon_annotation = Some(convert_icon(&r.take())),
                kind::DESC_ANNOTATION => node.desc_annotation = Some(convert_desc(&r.take())),
                _ => break,
            }
        }
        ast.nodes.push(node);
    }
    Ok(ast)
}
