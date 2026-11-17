use super::tokens::*;
use crate::errors::{PelersError, PelersErrorCode, written};
use crate::util::chars::{trim, utf16_to_byte};

pub struct Tokens {
    pub types: Vec<usize>,
    pub starts: Vec<usize>,
    pub ends: Vec<usize>,
}

const WORD: u8 = 1;
const PUNCTUATION: u8 = 2;

fn class_bits(c: u16) -> u8 {
    if c >= 128 {
        return 0;
    }
    match c as u8 {
        b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_' => WORD,
        b'!' | b'#' | b'$' | b'&' | b'*' | b'+' | b'/' | b'=' | b'?' | b'^' | b'|' | b'~' => {
            PUNCTUATION
        }
        _ => 0,
    }
}

fn is_word(c: u16) -> bool {
    c < 128 && class_bits(c) & WORD != 0
}

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn is_letter(c: u16) -> bool {
    (65..=90).contains(&c) || (97..=122).contains(&c)
}

fn is_hex(c: u16) -> bool {
    is_digit(c) || (65..=70).contains(&c) || (97..=102).contains(&c)
}

fn is_blank(c: u16) -> bool {
    c == 32 || c == 9
}

const KEYWORD_LENGTH: [usize; 21] = [
    0, 12, 5, 14, 3, 9, 2, 2, 2, 2, 2, 4, 3, 4, 8, 5, 5, 7, 6, 4, 5,
];

struct Units<'a> {
    u: &'a [u16],
}

impl Units<'_> {
    fn at(&self, i: usize) -> u16 {
        self.u.get(i).copied().unwrap_or(0)
    }

    fn starts_with(&self, at: usize, word: &str) -> bool {
        word.bytes()
            .enumerate()
            .all(|(k, w)| self.at(at + k) == u16::from(w))
    }

    fn starts_with_no_case(&self, at: usize, word: &str) -> bool {
        word.bytes()
            .enumerate()
            .all(|(k, w)| (self.at(at + k) | 32) == u16::from(w))
    }

    fn keyword_at(&self, at: usize, c: u16) -> usize {
        match c {
            97 => {
                if self.starts_with(at, "actor") {
                    ACTOR
                } else {
                    0
                }
            }
            115 => {
                if self.starts_with(at, "systemBoundary") {
                    SYSTEM_BOUNDARY
                } else if self.starts_with(at, "style") {
                    STYLE
                } else {
                    0
                }
            }
            101 => {
                if self.starts_with(at, "end") {
                    END
                } else if self.starts_with_no_case(at, "extend") {
                    EXTEND
                } else {
                    0
                }
            }
            69 => {
                if self.starts_with_no_case(at, "extend") {
                    EXTEND
                } else {
                    0
                }
            }
            100 => {
                if self.starts_with(at, "direction") {
                    DIRECTION
                } else {
                    0
                }
            }
            84 => {
                if self.starts_with(at, "TD") {
                    TD
                } else if self.starts_with(at, "TB") {
                    TB
                } else {
                    0
                }
            }
            66 => {
                if self.starts_with(at, "BT") {
                    BT
                } else {
                    0
                }
            }
            76 => {
                if self.starts_with(at, "LR") {
                    LR
                } else {
                    0
                }
            }
            82 => {
                if self.starts_with(at, "RL") {
                    RL
                } else {
                    0
                }
            }
            110 => {
                if self.starts_with(at, "note") {
                    NOTE
                } else {
                    0
                }
            }
            102 => {
                if self.starts_with(at, "for") {
                    FOR
                } else if self.starts_with(at, "false") {
                    FALSE
                } else {
                    0
                }
            }
            106 => {
                if self.starts_with(at, "json") {
                    JSON
                } else {
                    0
                }
            }
            99 => {
                if self.starts_with(at, "classDef") {
                    CLASS_DEF
                } else if self.starts_with(at, "class") {
                    CLASS
                } else {
                    0
                }
            }
            105 | 73 => {
                if self.starts_with_no_case(at, "include") {
                    INCLUDE
                } else {
                    0
                }
            }
            116 => {
                if self.starts_with(at, "true") {
                    TRUE
                } else {
                    0
                }
            }
            117 if self.starts_with(at, "usecase-beta") => USECASE,
            _ => 0,
        }
    }

    fn string_end(&self, at: usize, quote: u16) -> Option<usize> {
        let mut k = at + 1;
        while k < self.u.len() {
            let c = self.u[k];
            if c == quote {
                return Some(k + 1);
            }
            if c == 10 || c == 13 {
                return None;
            }
            k += 1;
        }
        None
    }

    fn starts_token(&self, at: usize) -> bool {
        let c = self.at(at);
        if c != 34 && c != 39 {
            return true;
        }
        (c == 34 && self.at(at + 1) == 96) || self.string_end(at, c).is_some()
    }
}

fn mapped(src: &str, unit: usize) -> (String, usize) {
    let (text, offset) = written(src, utf16_to_byte(src, unit));
    let units = text[..offset.min(text.len())].encode_utf16().count();
    (text, units)
}

pub fn line_column(src: &str, unit: usize) -> (usize, usize) {
    let (text, offset) = mapped(src, unit);
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut line = 1usize;
    let mut line_start = 0usize;
    let mut i = 0usize;
    while i < offset {
        let c = units[i];
        if c != 10 && c != 13 {
            i += 1;
            continue;
        }
        if c == 13 && i + 1 < offset && units[i + 1] == 10 {
            i += 1;
        }
        line += 1;
        line_start = i + 1;
        i += 1;
    }
    (line, offset - line_start + 1)
}

pub fn location_text(src: &str, start: usize, end: usize) -> String {
    let (line, column) = line_column(src, start);
    format!(
        "line {line}, column {column} [{},{})",
        mapped(src, start).1,
        mapped(src, end).1
    )
}

fn lex_error(src: &str, units: &[u16], offset: usize, length: usize) -> PelersError {
    let (line, column) = line_column(src, offset);
    let from = mapped(src, offset).1;
    let to = mapped(src, offset + length).1;
    let ch =
        String::from_utf16_lossy(&units[offset.min(units.len())..(offset + 1).min(units.len())]);
    let mut error = PelersError::new(
        format!(
            "Error lexing usecase diagram: unexpected character: ->{ch}<- at offset: {from}, skipped {} characters. at line {line}, column {column} [{from},{to})",
            to - from
        ),
        PelersErrorCode::Syntax,
    );
    error.diagram_type = Some("usecase".to_owned());
    error.line = line;
    error.column = column;
    error
}

pub fn tokenize(src: &str) -> Result<Tokens, PelersError> {
    let u: Vec<u16> = src.encode_utf16().collect();
    let units = Units { u: &u };
    let n = u.len();
    let mut types = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut i = 0usize;
    let mut line_start = true;
    let mut no_brace = false;

    'main: while i < n {
        let c = units.at(i);
        if c == 32 || c == 9 {
            i += 1;
            continue;
        }
        let mut token: usize;
        let mut end = i + 1;
        let mut fresh = false;

        if c < 128 && class_bits(c) & WORD != 0 {
            token = 0;
            if c == 97 && line_start && units.starts_with(i, "acc") {
                let descr = units.starts_with(i + 3, "Descr");
                if descr || units.starts_with(i + 3, "Title") {
                    let mut j = i + 8;
                    while is_blank(units.at(j)) {
                        j += 1;
                    }
                    let next = units.at(j);
                    if next == 58 {
                        token = if descr {
                            ACC_DESCR_LINE
                        } else {
                            ACC_TITLE_LINE
                        };
                        end = j + 1;
                        while end < n && units.at(end) != 10 && units.at(end) != 13 {
                            end += 1;
                        }
                    } else if next == 123 && descr && !no_brace {
                        match u[j + 1..].iter().position(|&x| x == u16::from(b'}')) {
                            None => no_brace = true,
                            Some(offset) => {
                                let close = j + 1 + offset;
                                token = ACC_DESCR_BLOCK;
                                end = close + 1;
                            }
                        }
                    }
                }
            } else if c == 106 && units.starts_with(i, "json") && is_blank(units.at(i + 4)) {
                let mut j = i + 5;
                while is_blank(units.at(j)) {
                    j += 1;
                }
                let id = j;
                while is_word(units.at(j)) {
                    j += 1;
                }
                if j > id {
                    while is_blank(units.at(j)) {
                        j += 1;
                    }
                    if units.at(j) == 64 {
                        j += 1;
                        while is_blank(units.at(j)) {
                            j += 1;
                        }
                        if units.at(j) == 123 {
                            types.push(JSON_DECLARATION_START);
                            starts.push(i);
                            ends.push(j);
                            let mut depth = 0i64;
                            let mut quoted = false;
                            let mut escaped = false;
                            let mut k = j;
                            while k < n {
                                let ch = units.at(k);
                                if quoted {
                                    if escaped {
                                        escaped = false;
                                    } else if ch == 92 {
                                        escaped = true;
                                    } else if ch == 34 {
                                        quoted = false;
                                    }
                                } else if ch == 34 {
                                    quoted = true;
                                } else if ch == 123 {
                                    depth += 1;
                                } else if ch == 125 {
                                    depth -= 1;
                                    if depth == 0 {
                                        break;
                                    }
                                }
                                k += 1;
                            }
                            types.push(if k < n {
                                JSON_OBJECT_LITERAL
                            } else {
                                UNCLOSED_JSON_OBJECT_LITERAL
                            });
                            starts.push(j);
                            i = if k < n { k + 1 } else { n };
                            ends.push(i);
                            line_start = false;
                            continue 'main;
                        }
                    }
                }
            }
            if token == 0 {
                while end < n && is_word(units.at(end)) {
                    end += 1;
                }
                let keyword = units.keyword_at(i, c);
                if keyword != 0 {
                    if keyword == USECASE || end - i == KEYWORD_LENGTH[keyword] {
                        token = keyword;
                        end = i + KEYWORD_LENGTH[keyword];
                    } else {
                        token = IDENTIFIER;
                    }
                } else if (c == 111 || c == 120) && units.at(i + 1) == 45 && units.at(i + 2) == 45 {
                    token = if c == 111 {
                        BACKWARD_CIRCLE
                    } else {
                        BACKWARD_CROSS
                    };
                    end = i + 3;
                } else if is_digit(c) {
                    let mut digits = i + 1;
                    while is_digit(units.at(digits)) {
                        digits += 1;
                    }
                    if units.at(digits) == 46 && is_digit(units.at(digits + 1)) {
                        token = NUMBER;
                        end = digits + 2;
                        while is_digit(units.at(end)) {
                            end += 1;
                        }
                        while is_letter(units.at(end)) {
                            end += 1;
                        }
                    } else {
                        token = IDENTIFIER;
                    }
                } else {
                    token = IDENTIFIER;
                    while units.at(end) == 45 && is_word(units.at(end + 1)) {
                        token = CSS_IDENTIFIER;
                        end += 2;
                        while is_word(units.at(end)) {
                            end += 1;
                        }
                    }
                }
            }
        } else if c == 10 {
            token = NEWLINE;
            fresh = true;
        } else if c == 13 {
            token = NEWLINE;
            fresh = true;
            if units.at(end) == 10 {
                end += 1;
            }
        } else if c == 34 || c == 39 {
            if c == 34 && units.at(i + 1) == 96 {
                let close = u[i + 2..]
                    .windows(2)
                    .position(|w| w == [u16::from(b'`'), u16::from(b'"')])
                    .map(|k| i + 2 + k);
                token = if close.is_none() {
                    UNCLOSED_MARKDOWN_STRING
                } else {
                    MARKDOWN_STRING
                };
                end = close.map_or(n, |close| close + 2);
            } else {
                token = PLAIN_STRING;
                match units.string_end(i, c) {
                    Some(close) => end = close,
                    None => {
                        let mut resume = i + 1;
                        while resume < n && !units.starts_token(resume) {
                            resume += 1;
                        }
                        return Err(lex_error(src, &u, i, resume - i));
                    }
                }
            }
        } else if c == 37 {
            if line_start && units.at(end) == 37 {
                token = COMMENT;
                while end < n && units.at(end) != 10 && units.at(end) != 13 {
                    end += 1;
                }
            } else {
                token = PERCENT;
            }
        } else if c == 45 {
            if units.at(end) != 45 {
                token = DASH;
            } else {
                while units.at(end) == 45 {
                    end += 1;
                }
                let next = units.at(end);
                let two = end - i == 2;
                if two && next == 124 && units.at(end + 1) == 62 {
                    token = GENERALIZATION;
                    end += 2;
                } else if next == 62 {
                    token = FORWARD_SOLID;
                    end += 1;
                } else if two && (next == 111 || next == 120) {
                    token = if next == 111 {
                        FORWARD_CIRCLE
                    } else {
                        FORWARD_CROSS
                    };
                    end += 1;
                } else {
                    token = MARKERLESS_SOLID;
                }
            }
        } else if c == 46 {
            let next = units.at(end);
            if next == 46 && units.at(end + 1) == 62 {
                token = DEPENDENCY_ARROW;
                end += 2;
            } else if is_digit(next) {
                token = NUMBER;
                while is_digit(units.at(end)) {
                    end += 1;
                }
                while is_letter(units.at(end)) {
                    end += 1;
                }
            } else {
                token = DOT;
            }
        } else if c == 60 {
            if units.at(end) == 60 {
                types.push(STEREOTYPE_START);
                starts.push(i);
                ends.push(i + 2);
                i += 2;
                if i >= n {
                    break 'main;
                }
                if units.at(i) == 62 && units.at(i + 1) == 62 {
                    token = STEREOTYPE_END;
                    end = i + 2;
                } else {
                    let mut close = i;
                    let mut found = false;
                    while close < n {
                        let ch = units.at(close);
                        if ch == 10 || ch == 13 {
                            break;
                        }
                        if ch == 62 && units.at(close + 1) == 62 {
                            found = true;
                            break;
                        }
                        close += 1;
                    }
                    if found && !trim(&String::from_utf16_lossy(&u[i..close])).is_empty() {
                        types.push(STEREOTYPE_TEXT);
                        starts.push(i);
                        ends.push(close);
                        token = STEREOTYPE_END;
                        i = close;
                        end = close + 2;
                    } else {
                        while close < n && units.at(close) != 10 && units.at(close) != 13 {
                            close += 1;
                        }
                        if close == i {
                            while close < n && (units.at(close) == 10 || units.at(close) == 13) {
                                close += 1;
                            }
                            return Err(lex_error(src, &u, i, close - i));
                        }
                        token = UNCLOSED_STEREOTYPE_TEXT;
                        end = close;
                    }
                }
            } else if units.at(end) == 45 && units.at(end + 1) == 45 {
                token = BACKWARD_SOLID;
                end += 2;
                while units.at(end) == 45 {
                    end += 1;
                }
            } else {
                token = LABEL_PUNCTUATION;
            }
        } else if c == 58 {
            if units.at(end) == 58 && units.at(end + 1) == 58 {
                token = CLASS_SEPARATOR;
                end += 2;
            } else {
                token = COLON;
            }
        } else if c == 64 {
            if units.at(end) == 123 {
                token = METADATA_START;
                end += 1;
            } else {
                token = AT;
            }
        } else if c == 123 {
            token = LBRACE;
        } else if c == 125 {
            token = RBRACE;
        } else if c == 91 {
            token = LBRACKET;
        } else if c == 93 {
            token = RBRACKET;
        } else if c == 40 {
            token = LPAREN;
        } else if c == 41 {
            token = RPAREN;
        } else if c == 44 {
            token = COMMA;
        } else if c == 35 {
            while is_hex(units.at(end)) {
                end += 1;
            }
            token = if end > i + 1 {
                HASH_COLOR
            } else {
                CSS_PUNCTUATION
            };
        } else if c == 92 {
            if units.at(end) == 44 {
                token = CSS_ESCAPED_COMMA;
                end += 1;
            } else {
                token = LABEL_PUNCTUATION;
            }
        } else if c < 127 && c > 32 {
            token = if class_bits(c) & PUNCTUATION != 0 {
                CSS_PUNCTUATION
            } else {
                LABEL_PUNCTUATION
            };
        } else {
            token = LABEL_SYMBOL;
            while end < n {
                let ch = units.at(end);
                if (32..127).contains(&ch) || ch == 9 || ch == 10 || ch == 13 {
                    break;
                }
                end += 1;
            }
        }

        types.push(token);
        starts.push(i);
        ends.push(end);
        i = end;
        line_start = fresh;
    }

    for _ in 0..3 {
        types.push(EOF);
        starts.push(n);
        ends.push(n);
    }
    Ok(Tokens {
        types,
        starts,
        ends,
    })
}
