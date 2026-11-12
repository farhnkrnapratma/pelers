use crate::diagrams::common::scan::{
    Tokens, accessibility, keyword_at, skip_trivia, slice, space, word,
};

pub const END: i32 = 0;
pub const EOF: i32 = 1;
pub const INVALID: i32 = 2;
pub const NEWLINE: i32 = 3;
pub const JOURNEY: i32 = 4;
pub const TITLE: i32 = 5;
pub const ACC_TITLE: i32 = 6;
pub const ACC_DESCR: i32 = 8;
pub const ACC_DESCR_MULTILINE_VALUE: i32 = 10;
pub const SECTION: i32 = 11;
pub const TASK_NAME: i32 = 12;
pub const TASK_DATA: i32 = 13;
pub const COLON: i32 = 14;

pub const TOKEN_NAMES: [&str; 15] = [
    "$end",
    "EOF",
    "INVALID",
    "NEWLINE",
    "journey",
    "title",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "section",
    "taskName",
    "taskData",
    ":",
];

fn text_end(src: &[u16], from: usize, colon: bool) -> usize {
    let mut e = from;
    while let Some(&c) = src.get(e) {
        if c == 35 || c == 10 || c == 59 || (colon && c == 58) {
            break;
        }
        e += 1;
    }
    e
}

fn emit(out: &mut Tokens, src: &[u16], start: usize, kind: i32, end: usize) -> usize {
    out.types.push(kind);
    out.texts.push(slice(src, start, end));
    out.starts.push(start);
    end
}

pub fn tokenize(source: &str) -> Tokens {
    let src: Vec<u16> = source.encode_utf16().collect();
    let n = src.len();
    let mut out = Tokens::default();
    let mut p = 0;
    loop {
        p = skip_trivia(&src, p);
        if p >= n {
            break;
        }
        let c = src[p];
        let mut e = p + 1;
        if c == 10 {
            while src.get(e) == Some(&10) {
                e += 1;
            }
            p = emit(&mut out, &src, p, NEWLINE, e);
            continue;
        }
        if keyword_at(&src, p, "journey") && !word(src.get(p + 7)) {
            p = emit(&mut out, &src, p, JOURNEY, p + 7);
            continue;
        }
        if keyword_at(&src, p, "title") {
            e = p + 5;
            while src.get(e).is_some_and(|&u| u != 10 && space(u)) {
                e += 1;
            }
            if e == n || src[e] == 10 {
                p = e;
                continue;
            }
            let sep = src[p + 5];
            if sep != 13 && space(sep) {
                e = text_end(&src, p + 6, false);
                if e > p + 6 {
                    p = emit(&mut out, &src, p, TITLE, e);
                    continue;
                }
            }
        }
        e = accessibility(&src, p, &mut out, ACC_TITLE);
        if e != p {
            p = e;
            continue;
        }
        if keyword_at(&src, p, "section") && src.get(p + 7).is_some_and(|&u| space(u)) {
            e = text_end(&src, p + 8, true);
            if e > p + 8 {
                p = emit(&mut out, &src, p, SECTION, e);
                continue;
            }
        }
        if c == 58 {
            e = text_end(&src, p + 1, false);
            if e > p + 1 {
                p = emit(&mut out, &src, p, TASK_DATA, e);
            } else {
                p = emit(&mut out, &src, p, COLON, p + 1);
            }
        } else if c == 59 {
            p = emit(&mut out, &src, p, INVALID, p + 1);
        } else {
            let end = text_end(&src, p + 1, true);
            p = emit(&mut out, &src, p, TASK_NAME, end);
        }
    }
    if out.starts.last().is_none_or(|&s| s < n) {
        p = emit(&mut out, &src, p, EOF, n);
    }
    emit(&mut out, &src, p, END, n);
    out
}
