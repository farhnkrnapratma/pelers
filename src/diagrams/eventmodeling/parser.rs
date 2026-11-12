use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, Pattern, Reader, SINGLE_LINE_COMMENT, TokenType, YAML,
    keyword, title_before, tokenize,
};
use crate::errors::PelersError;
use crate::util::chars::is_js_space;

fn is_ascii_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn em_id(src: &str, at: usize) -> Option<usize> {
    let mut chars = src[at..].char_indices();
    let (_, first) = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let mut end = at + first.len_utf8();
    for (offset, c) in chars {
        if !is_ascii_word(c) {
            break;
        }
        end = at + offset + c.len_utf8();
    }
    Some(end)
}

fn fid(src: &str, at: usize) -> Option<usize> {
    let digits = src.as_bytes()[at..]
        .iter()
        .take(3)
        .take_while(|b| b.is_ascii_digit())
        .count();
    (digits > 0).then_some(at + digits)
}

fn line_run_end(src: &str, from: usize) -> usize {
    src[from..]
        .find(['\n', '\r', '\u{2028}', '\u{2029}'])
        .map_or(src.len(), |i| from + i)
}

fn delimited(src: &str, at: usize, open: char, close: char) -> Option<usize> {
    let first = src[at..].chars().next()?;
    if first != open {
        return None;
    }
    let run_end = line_run_end(src, at + 1);
    src[at + 1..run_end]
        .rfind(close)
        .map(|i| at + 1 + i + close.len_utf8())
}

fn inline_data(src: &str, at: usize) -> Option<usize> {
    delimited(src, at, '{', '}')
        .or_else(|| delimited(src, at, '"', '"'))
        .or_else(|| delimited(src, at, '\'', '\''))
}

fn closes_block(src: &str, after: usize) -> Option<usize> {
    let rest = &src[after..];
    if rest.starts_with("\r\n") {
        return Some(after + 2);
    }
    if rest.starts_with('\n') {
        return Some(after + 1);
    }
    match rest.chars().next() {
        None => Some(after),
        Some(c) if is_js_space(c) => Some(after),
        _ => None,
    }
}

fn data_block(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with('{') {
        return None;
    }
    let bytes = src.as_bytes();
    let mut p = at + 1;
    while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
        p += 1;
    }
    let header_end = if src[p..].starts_with("\r\n") {
        p + 2
    } else if src[p..].starts_with('\n') {
        p + 1
    } else {
        return None;
    };
    let mut q = header_end + 1;
    while q < bytes.len() {
        if bytes[q] == b'}'
            && bytes[q - 1] == b'\n'
            && let Some(end) = closes_block(src, q + 1)
        {
            return Some(end);
        }
        q += 1;
    }
    if bytes.get(header_end) == Some(&b'}') {
        return closes_block(src, header_end + 1);
    }
    None
}

fn ws(src: &str, at: usize) -> Option<usize> {
    let mut end = at;
    for c in src[at..].chars() {
        if !is_js_space(c) {
            break;
        }
        end += c.len_utf8();
    }
    (end > at).then_some(end)
}

fn ml_comment(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with("/*") {
        return None;
    }
    src[at + 2..].find("*/").map(|i| at + 2 + i + 2)
}

fn sl_comment(src: &str, at: usize) -> Option<usize> {
    src[at..]
        .starts_with("//")
        .then(|| line_run_end(src, at + 2))
}

fn em_title(src: &str, at: usize) -> Option<usize> {
    title_before(src, at, is_ascii_word, is_ascii_word)
}

fn em_fid_pattern(src: &str, at: usize) -> Option<usize> {
    fid(src, at)
}

const EM_ID: TokenType = TokenType::new("EM_ID", Pattern::Match(em_id), None);

const fn word(text: &'static str) -> TokenType {
    TokenType {
        longer: &["EM_ID"],
        ..TokenType::new(text, Pattern::Literal(text), None)
    }
}

const fn sign(text: &'static str) -> TokenType {
    TokenType::new(text, Pattern::Literal(text), None)
}

pub const EVENTMODELING_TOKENS: &[TokenType] = &[
    TokenType {
        longer: &["EM_ID"],
        ..keyword("eventmodeling")
    },
    word("resetframe"),
    word("readmodel"),
    word("processor"),
    word("timeframe"),
    word("command"),
    word("entity"),
    word("event"),
    word("jsobj"),
    word("figma"),
    word("given"),
    word("json"),
    word("salt"),
    word("html"),
    word("text"),
    word("data"),
    word("note"),
    word("when"),
    word("then"),
    word("rmo"),
    word("cmd"),
    word("evt"),
    word("pcr"),
    word("uri"),
    sign("->>"),
    word("gwt"),
    word("ui"),
    word("md"),
    word("tf"),
    sign("[["),
    sign("]]"),
    word("rf"),
    sign("`"),
    sign("."),
    EM_ID,
    TokenType::new("EM_FID", Pattern::Match(em_fid_pattern), None),
    TokenType::new("EM_DATA_INLINE", Pattern::Match(inline_data), None),
    TokenType::new("EM_DATA_BLOCK", Pattern::Match(data_block), None),
    TokenType {
        name: "EM_ACC_DESCR",
        ..ACC_DESCR
    },
    TokenType {
        name: "EM_ACC_TITLE",
        ..ACC_TITLE
    },
    TokenType {
        name: "EM_TITLE",
        ..TokenType::new("EM_TITLE", Pattern::Match(em_title), Some("\t t"))
    },
    TokenType {
        hidden: true,
        ..TokenType::new("EM_WS", Pattern::Match(ws), None)
    },
    TokenType {
        name: "EM_YAML",
        ..YAML
    },
    TokenType {
        name: "EM_DIRECTIVE",
        ..DIRECTIVE
    },
    TokenType {
        name: "EM_SINGLE_LINE_COMMENT",
        ..SINGLE_LINE_COMMENT
    },
    TokenType {
        hidden: true,
        ..TokenType::new("EM_ML_COMMENT", Pattern::Match(ml_comment), None)
    },
    TokenType {
        hidden: true,
        ..TokenType::new("EM_SL_COMMENT", Pattern::Match(sl_comment), None)
    },
];

pub mod kind {
    pub const KEYWORD: i32 = 0;
    pub const RESETFRAME: i32 = 1;
    pub const READMODEL: i32 = 2;
    pub const PROCESSOR: i32 = 3;
    pub const TIMEFRAME: i32 = 4;
    pub const COMMAND: i32 = 5;
    pub const ENTITY: i32 = 6;
    pub const EVENT: i32 = 7;
    pub const JSOBJ: i32 = 8;
    pub const FIGMA: i32 = 9;
    pub const GIVEN: i32 = 10;
    pub const JSON: i32 = 11;
    pub const SALT: i32 = 12;
    pub const HTML: i32 = 13;
    pub const TEXT: i32 = 14;
    pub const DATA: i32 = 15;
    pub const NOTE: i32 = 16;
    pub const WHEN: i32 = 17;
    pub const THEN: i32 = 18;
    pub const RMO: i32 = 19;
    pub const CMD: i32 = 20;
    pub const EVT: i32 = 21;
    pub const PCR: i32 = 22;
    pub const URI: i32 = 23;
    pub const ARROW: i32 = 24;
    pub const GWT: i32 = 25;
    pub const UI: i32 = 26;
    pub const MD: i32 = 27;
    pub const TF: i32 = 28;
    pub const OPEN: i32 = 29;
    pub const CLOSE: i32 = 30;
    pub const RF: i32 = 31;
    pub const TICK: i32 = 32;
    pub const DOT: i32 = 33;
    pub const ID: i32 = 34;
    pub const FRAME_ID: i32 = 35;
    pub const INLINE: i32 = 36;
    pub const BLOCK: i32 = 37;
    pub const ACC_DESCR: i32 = 38;
    pub const ACC_TITLE: i32 = 39;
    pub const TITLE: i32 = 40;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ref {
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmFrame {
    pub reset: bool,
    pub name: String,
    pub model_entity_type: String,
    pub entity_identifier: String,
    pub source_frames: Vec<Ref>,
    pub data_reference: Option<Ref>,
    pub data_type: Option<String>,
    pub data_inline_value: Option<String>,
    pub offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmDataEntity {
    pub name: String,
    pub data_type: Option<String>,
    pub data_block_value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmNoteEntity {
    pub source_frame: Ref,
    pub data_type: Option<String>,
    pub data_block_value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmGwt {
    pub source_frame: Ref,
    pub given: Vec<String>,
    pub when: Vec<String>,
    pub then: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventModelAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub model_entities: Vec<String>,
    pub frames: Vec<EmFrame>,
    pub data_entities: Vec<EmDataEntity>,
    pub note_entities: Vec<EmNoteEntity>,
    pub gwt_entities: Vec<EmGwt>,
    pub source: String,
}

pub fn parse_eventmodel(src: &str) -> Result<EventModelAst, PelersError> {
    let tokens = tokenize(src, EVENTMODELING_TOKENS, "eventmodeling")?;
    let mut r = Reader::new(tokens, EVENTMODELING_TOKENS, "eventmodeling");
    let mut ast = EventModelAst {
        source: src.to_owned(),
        ..EventModelAst::default()
    };
    r.expect(kind::KEYWORD)?;
    while r.kind() != -1 {
        let current = r.kind();
        if current == kind::ACC_DESCR {
            ast.acc_descr = Some(r.take());
        } else if current == kind::ACC_TITLE {
            ast.acc_title = Some(r.take());
        } else if current == kind::TITLE {
            ast.title = Some(r.take());
        } else if current == kind::ENTITY {
            r.i += 1;
            let name = qualified_name(&mut r)?;
            ast.model_entities.push(name);
        } else if is_time_frame(current) || is_reset_frame(current) {
            let offset = r.start_at(r.i);
            let reset = is_reset_frame(current);
            r.i += 1;
            let mut frame = EmFrame {
                reset,
                name: r.expect(kind::FRAME_ID)?,
                model_entity_type: one(&mut r, is_entity_type, "an entity type")?,
                entity_identifier: qualified_name(&mut r)?,
                source_frames: Vec::new(),
                data_reference: None,
                data_type: None,
                data_inline_value: None,
                offset,
            };
            while r.accept(kind::ARROW) {
                frame.source_frames.push(Ref {
                    text: r.expect(kind::FRAME_ID)?,
                });
            }
            if r.accept(kind::OPEN) {
                frame.data_reference = Some(Ref {
                    text: r.expect(kind::ID)?,
                });
                r.expect(kind::CLOSE)?;
            }
            if r.kind() == kind::TICK || r.kind() == kind::INLINE {
                if let Some(data_type) = data_type(&mut r)? {
                    frame.data_type = Some(data_type);
                }
                frame.data_inline_value = Some(r.expect(kind::INLINE)?);
            }
            ast.frames.push(frame);
        } else if current == kind::DATA {
            r.i += 1;
            let name = r.expect(kind::ID)?;
            let data_type = data_type(&mut r)?;
            let block = r.expect(kind::BLOCK)?;
            ast.data_entities.push(EmDataEntity {
                name,
                data_type,
                data_block_value: block,
            });
        } else if current == kind::NOTE {
            r.i += 1;
            let frame = Ref {
                text: r.expect(kind::FRAME_ID)?,
            };
            let data_type = data_type(&mut r)?;
            let block = r.expect(kind::BLOCK)?;
            ast.note_entities.push(EmNoteEntity {
                source_frame: frame,
                data_type,
                data_block_value: block,
            });
        } else if current == kind::GWT {
            r.i += 1;
            let source_frame = Ref {
                text: r.expect(kind::FRAME_ID)?,
            };
            r.expect(kind::GIVEN)?;
            let given = statements(&mut r)?;
            let when = if r.accept(kind::WHEN) {
                statements(&mut r)?
            } else {
                Vec::new()
            };
            r.expect(kind::THEN)?;
            let then = statements(&mut r)?;
            ast.gwt_entities.push(EmGwt {
                source_frame,
                given,
                when,
                then,
            });
        } else {
            return Err(r.fail("a frame, an entity, a data block, a note, or a specification"));
        }
    }
    Ok(ast)
}

fn is_time_frame(k: i32) -> bool {
    k == kind::TIMEFRAME || k == kind::TF
}

fn is_reset_frame(k: i32) -> bool {
    k == kind::RESETFRAME || k == kind::RF
}

fn is_entity_type(k: i32) -> bool {
    matches!(
        k,
        kind::RMO
            | kind::READMODEL
            | kind::UI
            | kind::CMD
            | kind::COMMAND
            | kind::EVT
            | kind::EVENT
            | kind::PCR
            | kind::PROCESSOR
    )
}

fn is_data_type(k: i32) -> bool {
    matches!(
        k,
        kind::JSON
            | kind::JSOBJ
            | kind::FIGMA
            | kind::SALT
            | kind::URI
            | kind::MD
            | kind::HTML
            | kind::TEXT
    )
}

fn one(r: &mut Reader<'_>, check: fn(i32) -> bool, expected: &str) -> Result<String, PelersError> {
    if check(r.kind()) {
        Ok(r.take())
    } else {
        Err(r.fail(expected))
    }
}

fn qualified_name(r: &mut Reader<'_>) -> Result<String, PelersError> {
    let mut name = r.expect(kind::ID)?;
    while r.accept(kind::DOT) {
        name.push('.');
        name.push_str(&r.expect(kind::ID)?);
    }
    Ok(name)
}

fn data_type(r: &mut Reader<'_>) -> Result<Option<String>, PelersError> {
    if !r.accept(kind::TICK) {
        return Ok(None);
    }
    let found = one(r, is_data_type, "a data type")?;
    r.expect(kind::TICK)?;
    Ok(Some(found))
}

fn statements(r: &mut Reader<'_>) -> Result<Vec<String>, PelersError> {
    let mut out = Vec::new();
    loop {
        one(r, is_entity_type, "an entity type")?;
        out.push(r.expect(kind::ID)?);
        if !is_entity_type(r.kind()) {
            break;
        }
    }
    Ok(out)
}
