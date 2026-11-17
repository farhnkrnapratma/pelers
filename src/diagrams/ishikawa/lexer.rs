use crate::diagrams::common::scan::{space, word};

pub const END: i32 = 0;
pub const EOF: i32 = 1;
pub const SPACELINE: i32 = 2;
pub const NL: i32 = 3;
pub const ISHIKAWA: i32 = 4;
pub const SPACELIST: i32 = 5;
pub const TEXT: i32 = 6;

pub const TOKEN_NAMES: [&str; 7] = [
    "$end",
    "EOF",
    "SPACELINE",
    "NL",
    "ISHIKAWA",
    "SPACELIST",
    "TEXT",
];

#[derive(Debug, Default)]
pub struct Tokens {
    pub types: Vec<i32>,
    pub starts: Vec<usize>,
    pub ends: Vec<usize>,
}

fn keyword(src: &[u16], at: usize) -> usize {
    const WORD: &[u8] = b"ishikawa-beta";
    let mut k = 0;
    while k < 13 {
        let Some(&c) = src.get(at + k) else {
            break;
        };
        let w = u16::from(WORD[k]);
        if c != w && (c.wrapping_add(32) != w || !(65..=90).contains(&c)) {
            break;
        }
        k += 1;
    }
    if k == 13 && !word(src.get(at + 13)) {
        return 13;
    }
    if k >= 8 && !word(src.get(at + 8)) {
        8
    } else {
        0
    }
}

pub fn tokenize(source: &str) -> Tokens {
    let src: Vec<u16> = source.encode_utf16().collect();
    let n = src.len();
    let mut out = Tokens::default();
    let mut i = 0;
    while i < n {
        let mut j = i;
        let mut last_break: Option<usize> = None;
        while j < n && space(src[j]) {
            if src[j] == 10 {
                last_break = Some(j);
            }
            j += 1;
        }
        let (kind, end) = if src.get(j) == Some(&37) && src.get(j + 1) == Some(&37) {
            let mut end = j + 2;
            while end < n {
                let c = src[end];
                if c == 10 || c == 13 || c == 0x2028 || c == 0x2029 {
                    break;
                }
                end += 1;
            }
            (SPACELINE, end)
        } else if j == i {
            let length = keyword(&src, i);
            if length > 0 {
                (ISHIKAWA, i + length)
            } else {
                let end = src[i..]
                    .iter()
                    .position(|&u| u == 10)
                    .map_or(n, |at| i + at);
                (TEXT, end)
            }
        } else if last_break.is_some_and(|b| b > i) {
            (SPACELINE, last_break.unwrap() + 1)
        } else if last_break == Some(i) {
            (NL, i + 1)
        } else {
            (SPACELIST, j)
        };
        out.types.push(kind);
        out.starts.push(i);
        out.ends.push(end);
        i = end;
    }
    out.types.extend([EOF, END]);
    out.starts.extend([n, n]);
    out.ends.extend([n, n]);
    out
}
