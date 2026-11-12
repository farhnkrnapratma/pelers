use std::collections::HashMap;

use crate::errors::{PelersError, PelersErrorCode, locate};
use crate::util::chars::{is_js_space, trim, trim_end};

const INDENT_UNIT: &str = "    ";

fn is_box_char(c: char) -> bool {
    u16::try_from(u32::from(c)).is_ok_and(is_box)
}

fn is_box(u: u16) -> bool {
    matches!(
        u,
        0x2500 | 0x2501 | 0x2502 | 0x2503 | 0x2514 | 0x2517 | 0x251C | 0x2523
    )
}

fn is_branch(u: u16) -> bool {
    matches!(u, 0x2514 | 0x2517 | 0x251C | 0x2523)
}

fn is_dash(u: u16) -> bool {
    matches!(u, 0x2500 | 0x2501)
}

fn is_space(u: u16) -> bool {
    char::from_u32(u32::from(u)).is_some_and(is_js_space)
}

fn is_decoration(u: u16) -> bool {
    is_space(u) || matches!(u, 0x2502 | 0x2503)
}

fn is_box_only(u: u16) -> bool {
    is_space(u) || is_box(u)
}

fn all(units: &[u16], check: fn(u16) -> bool) -> bool {
    !units.is_empty() && units.iter().all(|&u| check(u))
}

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn position(units: &[u16], check: fn(u16) -> bool) -> Option<usize> {
    units.iter().position(|&u| check(u))
}

fn is_metadata_line(line: &str) -> bool {
    let rest = line.trim_start_matches(is_js_space);
    if let Some(after) = rest.strip_prefix("title") {
        return after.starts_with(['\t', ' ']);
    }
    if let Some(after) = rest.strip_prefix("accTitle") {
        return after.trim_start_matches(['\t', ' ']).starts_with(':');
    }
    if let Some(after) = rest.strip_prefix("accDescr") {
        let after = after.trim_start_matches(['\t', ' ']);
        return after.starts_with(':') || after.starts_with('{');
    }
    false
}

fn is_comment_line(line: &str) -> bool {
    line.trim_start_matches(is_js_space).starts_with("%%")
}

fn passes(line: &str) -> bool {
    trim(line).is_empty() || is_comment_line(line) || is_metadata_line(line)
}

fn decoration_only(line: &str) -> bool {
    all(&units(line), is_decoration)
}

fn before_tabs(line: &str, at: usize) -> usize {
    let text = units(line);
    let mut i = 0;
    let mut n = 0;
    while i < text.len() && n < at {
        n += if text[i] == u16::from(b'\t') { 4 } else { 1 };
        i += 1;
    }
    i
}

#[derive(Clone, Debug)]
pub struct ColumnMap {
    pub original: String,
    pub pos: usize,
    pub indent: usize,
}

impl ColumnMap {
    pub fn map(&self, at: usize) -> usize {
        before_tabs(&self.original, self.pos + at.saturating_sub(self.indent))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Preprocessed {
    pub text: String,
    pub line_map: HashMap<usize, usize>,
    pub columns: HashMap<usize, ColumnMap>,
}

fn fail(input: &str, index: usize, message: &str) -> PelersError {
    let (line, _, _) = locate(input, line_start(input, index + 1));
    let mut error = PelersError::new(format!("Line {line}: {message}"), PelersErrorCode::Syntax);
    error.diagram_type = Some("treeView".to_owned());
    error.line = line;
    error
}

pub fn preprocess_box_drawing(input: &str) -> Result<Preprocessed, PelersError> {
    let unchanged = Preprocessed {
        text: input.to_owned(),
        ..Preprocessed::default()
    };
    if !input.chars().any(is_box_char) {
        return Ok(unchanged);
    }
    let lines: Vec<&str> = input.split('\n').collect();
    let Some(keyword_idx) = lines.iter().position(|line| trim(line) == "treeView-beta") else {
        return Ok(unchanged);
    };

    let mut content: Vec<String> = Vec::new();
    for line in &lines[keyword_idx + 1..] {
        if passes(line) || decoration_only(line) {
            continue;
        }
        content.push(line.replace('\t', INDENT_UNIT));
    }
    if !content.iter().any(|line| line.chars().any(is_box_char)) {
        return Ok(unchanged);
    }

    let segment_width = content
        .iter()
        .find_map(|line| position(&units(line), is_branch).filter(|&index| index > 0))
        .unwrap_or(4);

    let mut out: Vec<String> = Vec::new();
    let mut result = Preprocessed::default();
    let keep = |out: &mut Vec<String>,
                result: &mut Preprocessed,
                text: String,
                index: usize,
                column: Option<ColumnMap>| {
        out.push(text);
        result.line_map.insert(out.len(), index + 1);
        if let Some(column) = column {
            result.columns.insert(out.len(), column);
        }
    };

    for (i, line) in lines[..=keyword_idx].iter().enumerate() {
        keep(&mut out, &mut result, (*line).to_owned(), i, None);
    }
    for (i, line) in lines.iter().enumerate().skip(keyword_idx + 1) {
        if passes(line) {
            keep(&mut out, &mut result, (*line).to_owned(), i, None);
            continue;
        }
        if decoration_only(line) {
            continue;
        }
        let normalized = line.replace('\t', INDENT_UNIT);
        let text = units(&normalized);
        if let Some(branch) = position(&text, is_branch) {
            let depth = js_round(branch as f64 / segment_width as f64) as usize + 1;
            let mut pos = branch + 1;
            while pos < text.len() && is_dash(text[pos]) {
                pos += 1;
            }
            while pos < text.len() && text[pos] == u16::from(b' ') {
                pos += 1;
            }
            let content = trim_end(&String::from_utf16_lossy(&text[pos..])).to_owned();
            if content.is_empty() {
                return Err(fail(
                    input,
                    i,
                    "Empty node — expected a filename or directory name after the box-drawing prefix",
                ));
            }
            let indent = INDENT_UNIT.repeat(depth);
            let indent_len = indent.len();
            let column = ColumnMap {
                original: (*line).to_owned(),
                pos,
                indent: indent_len,
            };
            keep(&mut out, &mut result, indent + &content, i, Some(column));
        } else if all(&text, is_box_only) {
            continue;
        } else if text.iter().any(|&u| is_box(u)) {
            keep(&mut out, &mut result, (*line).to_owned(), i, None);
        } else if text.first().is_some_and(|&u| is_space(u)) {
            return Err(fail(
                input,
                i,
                "Unexpected indentation without box-drawing characters. In box-drawing format, use ├── or └── prefixes for indented nodes.",
            ));
        } else {
            keep(&mut out, &mut result, (*line).to_owned(), i, None);
        }
    }
    result.text = out.join("\n");
    Ok(result)
}

fn js_round(value: f64) -> f64 {
    crate::util::jsnum::js_round(value)
}

pub fn line_start(text: &str, line: usize) -> usize {
    let mut at = 0;
    for _ in 1..line {
        at = text[at..].find('\n').map_or(text.len(), |i| at + i) + 1;
    }
    at
}
