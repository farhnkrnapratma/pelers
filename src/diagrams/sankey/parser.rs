use std::collections::HashMap;

use super::lexer::{
    COMMA, DQUOTE, END, EOF, ESCAPED_TEXT, NEWLINE, NON_ESCAPED_TEXT, SANKEY, STUCK, TOKEN_NAMES,
    tokenize,
};
use crate::errors::{Edit, PelersError, input_offset, syntax_error};
use crate::text::entities::decode_entities;
use crate::util::chars::{is_js_space, utf16_to_byte};
use crate::util::jsnum::js_parse_float;

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNode {
    pub id: String,
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SankeyLink {
    pub source: usize,
    pub target: usize,
    pub value: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SankeyDb {
    pub title: Option<String>,
    pub nodes: Vec<SankeyNode>,
    pub links: Vec<SankeyLink>,
    lookup: HashMap<String, usize>,
}

impl SankeyDb {
    fn find_or_create_node(&mut self, id: &str) -> usize {
        if let Some(&index) = self.lookup.get(id) {
            return index;
        }
        let index = self.nodes.len();
        self.nodes.push(SankeyNode {
            id: id.to_owned(),
            index,
        });
        self.lookup.insert(id.to_owned(), index);
        index
    }
}

fn is_blank(c: u16) -> bool {
    if c == 10 || c == 13 {
        return false;
    }
    if c < 128 {
        return c == 32 || c == 9 || c == 11 || c == 12;
    }
    matches!(
        c,
        0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
    )
}

fn trim_rows(text: &[u16], edits: &mut Vec<Edit>) -> Vec<u16> {
    let mut end = text.len();
    while end > 0 && is_blank(text[end - 1]) {
        end -= 1;
    }
    let mut out: Vec<u16> = Vec::new();
    let mut kept = 0usize;
    let mut quoted = false;
    let mut i = 0usize;
    while i < end {
        let c = text[i];
        if quoted {
            if c == 34 {
                if text.get(i + 1) == Some(&34) {
                    i += 1;
                } else {
                    quoted = false;
                }
            }
            i += 1;
        } else if c == 34 {
            quoted = true;
            i += 1;
        } else if is_blank(c) {
            let mut j = i + 1;
            while text.get(j).is_some_and(|&u| is_blank(u)) {
                j += 1;
            }
            let p = if i == 0 { 0 } else { text[i - 1] };
            let q = text.get(j).copied().unwrap_or(0);
            if i == 0 || q == 10 || q == 13 || ((p == 10 || p == 13) && q != 44) {
                out.extend_from_slice(&text[kept..i]);
                edits.push(Edit::cut(i, j));
                kept = j;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    if end < text.len() {
        edits.push(Edit::cut(end, text.len()));
    }
    out.extend_from_slice(&text[kept..end]);
    out
}

pub fn prepare_text_for_parsing(text: &[u16]) -> (Vec<u16>, Vec<Vec<Edit>>) {
    let mut edits: Vec<Edit> = Vec::new();
    let rows = trim_rows(text, &mut edits);
    let mut collapsed: Vec<u16> = Vec::with_capacity(rows.len());
    let mut replacements: Vec<Edit> = Vec::new();
    let mut i = 0usize;
    while i < rows.len() {
        if rows[i] == 10 || rows[i] == 13 {
            let mut j = i;
            while j < rows.len() && (rows[j] == 10 || rows[j] == 13) {
                j += 1;
            }
            replacements.push(Edit::resized(i, j, 1));
            collapsed.push(10);
            i = j;
        } else {
            collapsed.push(rows[i]);
            i += 1;
        }
    }
    let lead = collapsed
        .iter()
        .take_while(|&&u| char::from_u32(u32::from(u)).is_some_and(is_js_space))
        .count();
    let mut end = collapsed.len();
    while end > lead && char::from_u32(u32::from(collapsed[end - 1])).is_some_and(is_js_space) {
        end -= 1;
    }
    let trimmed = collapsed[lead..end].to_vec();
    let steps = vec![
        edits,
        replacements,
        vec![
            Edit::cut(0, lead),
            Edit::cut(lead + trimmed.len(), collapsed.len()),
        ],
    ];
    (trimmed, steps)
}

pub fn parse_sankey(src: &[u16], db: &mut SankeyDb, source: &[u16]) -> Result<(), PelersError> {
    let steps = if source != src {
        prepare_text_for_parsing(source).1
    } else {
        Vec::new()
    };
    let source_text = String::from_utf16_lossy(source);
    let error = |at: usize, detail: &str| -> PelersError {
        let offset = if at > 0 && at >= src.len() {
            input_offset(&steps, src.len() - 1) + 1
        } else {
            input_offset(&steps, at)
        };
        syntax_error(
            "sankey",
            &source_text,
            utf16_to_byte(&source_text, offset),
            detail,
        )
    };

    let tokens = tokenize(src);
    let (types, texts, starts) = (tokens.types, tokens.texts, tokens.starts);
    let mut i = 0usize;

    let fail = |i: usize, expected: &str| -> PelersError {
        let t = types[i];
        let got = if t == STUCK {
            TOKEN_NAMES[types[i - 1] as usize]
        } else {
            TOKEN_NAMES[t as usize]
        };
        error(starts[i], &format!("Expecting {expected}, got '{got}'"))
    };

    macro_rules! expect {
        ($kind:expr, $also:expr) => {{
            if types[i] != $kind {
                return Err(fail(
                    i,
                    &format!("'{}'{}", TOKEN_NAMES[$kind as usize], $also),
                ));
            }
            i += 1;
        }};
    }

    macro_rules! field {
        () => {{
            if types[i] == NON_ESCAPED_TEXT {
                i += 1;
                texts[i - 1].clone()
            } else {
                if types[i] != DQUOTE {
                    return Err(fail(i, "'DQUOTE', 'NON_ESCAPED_TEXT'"));
                }
                i += 1;
                if types[i] != ESCAPED_TEXT {
                    return Err(fail(i, "'ESCAPED_TEXT'"));
                }
                i += 1;
                let text = texts[i - 1].clone();
                expect!(DQUOTE, "");
                text
            }
        }};
    }

    macro_rules! name {
        () => {{
            let start = starts[i];
            let text = field!().trim().replace("\"\"", "\"");
            let decoded = decode_entities(&text);
            if crate::svg::builder::has_invalid_xml(&decoded) {
                return Err(error(start, "Invalid XML character in node name"));
            }
            text
        }};
    }

    expect!(SANKEY, "");
    expect!(NEWLINE, "");
    loop {
        let from = name!();
        expect!(COMMA, ", 'NEWLINE', 'EOF'");
        let to = name!();
        expect!(COMMA, ", 'NEWLINE', 'EOF'");
        let start = starts[i];
        let value = js_parse_float(&field!());
        if !value.is_finite() {
            return Err(error(start, "Expecting a number"));
        }
        let source_index = db.find_or_create_node(&from);
        let target_index = db.find_or_create_node(&to);
        db.links.push(SankeyLink {
            source: source_index,
            target: target_index,
            value,
        });
        if types[i] != NEWLINE {
            break;
        }
        i += 1;
    }
    if types[i] == EOF {
        i += 1;
    }
    if types[i] != END {
        return Err(fail(i, "'NEWLINE', 'EOF'"));
    }
    Ok(())
}
