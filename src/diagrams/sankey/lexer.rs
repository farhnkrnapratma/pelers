use crate::diagrams::common::scan::slice;
use crate::util::chars::is_word;

pub const END: i32 = 0;
pub const STUCK: i32 = 1;
pub const SANKEY: i32 = 2;
pub const NEWLINE: i32 = 3;
pub const COMMA: i32 = 4;
pub const DQUOTE: i32 = 5;
pub const NON_ESCAPED_TEXT: i32 = 6;
pub const ESCAPED_TEXT: i32 = 7;
pub const EOF: i32 = 8;

pub const TOKEN_NAMES: [&str; 9] = [
    "$end",
    "STUCK",
    "SANKEY",
    "NEWLINE",
    "COMMA",
    "DQUOTE",
    "NON_ESCAPED_TEXT",
    "ESCAPED_TEXT",
    "EOF",
];

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    Csv,
    EscapedText,
}

fn is_text_data(d: u32) -> bool {
    (32..=126).contains(&d) && d != 34 && d != 44
        || (0xa0..=0xd7ff).contains(&d)
        || (0xe000..=0xfffd).contains(&d)
        || (0x10000..=0x10ffff).contains(&d)
}

fn code_point_at(src: &[u16], q: usize) -> Option<(u32, usize)> {
    let first = u32::from(*src.get(q)?);
    if (0xd800..0xdc00).contains(&first)
        && let Some(second) = src.get(q + 1).map(|&s| u32::from(s))
        && (0xdc00..0xe000).contains(&second)
    {
        return Some((0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00), 2));
    }
    Some((first, 1))
}

pub struct Tokens {
    pub types: Vec<i32>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

pub fn tokenize(src: &[u16]) -> Tokens {
    let n = src.len();
    let mut out = Tokens {
        types: Vec::new(),
        texts: Vec::new(),
        starts: Vec::new(),
    };
    let mut stack: Vec<State> = Vec::new();
    let mut state = State::Initial;
    let mut p = 0usize;

    let emit = |out: &mut Tokens, kind: i32, s: usize, e: usize| {
        out.types.push(kind);
        out.texts.push(slice(src, s, e));
        out.starts.push(s);
    };
    let keyword = |p: usize, word: &str| -> bool {
        let end = (p + word.len()).min(n);
        let text = String::from_utf16_lossy(&src[p.min(n)..end]).to_lowercase();
        let after_word = src.get(p + word.len()).copied().unwrap_or(0);
        text == word && !(after_word < 128 && is_word(after_word as u8 as char))
    };

    while p < n {
        let c = src[p];
        let start = p;
        let mut q = p;
        if state == State::EscapedText {
            if c == 34 && src.get(p + 1).copied() != Some(34) {
                if let Some(s) = stack.pop() {
                    state = s;
                }
                emit(&mut out, DQUOTE, p, p + 1);
                p += 1;
                continue;
            }
            while q < n {
                let (d, width) = code_point_at(src, q).unwrap_or((0, 1));
                if is_text_data(d) || d == 44 || d == 13 || d == 10 {
                    q += width;
                } else if d == 34 && src.get(q + 1).copied() == Some(34) {
                    q += 2;
                } else {
                    break;
                }
            }
            emit(&mut out, ESCAPED_TEXT, p, q);
            p = q;
        } else {
            if state == State::Initial && (c | 32) == 115 {
                let length = if keyword(p, "sankey-beta") {
                    11
                } else if keyword(p, "sankey") {
                    6
                } else {
                    0
                };
                if length > 0 {
                    stack.push(state);
                    state = State::Csv;
                    emit(&mut out, SANKEY, p, p + length);
                    p += length;
                    continue;
                }
            }
            if c == 10 || (c == 13 && src.get(p + 1).copied() == Some(10)) {
                let e = if c == 10 { p + 1 } else { p + 2 };
                emit(&mut out, NEWLINE, p, e);
                p = e;
                continue;
            }
            if c == 44 {
                emit(&mut out, COMMA, p, p + 1);
                p += 1;
                continue;
            }
            if c == 34 {
                stack.push(state);
                state = State::EscapedText;
                emit(&mut out, DQUOTE, p, p + 1);
                p += 1;
                continue;
            }
            while q < n {
                let (d, width) = code_point_at(src, q).unwrap_or((0, 1));
                if !is_text_data(d) {
                    break;
                }
                q += width;
            }
            emit(&mut out, NON_ESCAPED_TEXT, p, q);
            p = q;
        }
        if q == start {
            emit(&mut out, STUCK, p, p);
            return out;
        }
    }

    let kind = if state == State::EscapedText {
        ESCAPED_TEXT
    } else {
        EOF
    };
    emit(&mut out, kind, n, n);
    emit(&mut out, END, n, n);
    out
}
