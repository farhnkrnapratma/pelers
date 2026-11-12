use crate::diagrams::common::scan::{
    Tokens, accessibility, keyword_at, line_end, skip_trivia, slice, space, word,
};

pub const END: i32 = 0;
pub const EOF: i32 = 1;
pub const INVALID: i32 = 2;
pub const NEWLINE: i32 = 3;
pub const TIMELINE: i32 = 4;
pub const TIMELINE_LR: i32 = 5;
pub const TIMELINE_TD: i32 = 6;
pub const TITLE: i32 = 7;
pub const ACC_TITLE: i32 = 8;
pub const ACC_DESCR: i32 = 10;
pub const ACC_DESCR_MULTILINE_VALUE: i32 = 12;
pub const SECTION: i32 = 13;
pub const PERIOD: i32 = 14;
pub const EVENT: i32 = 15;

pub const TOKEN_NAMES: [&str; 16] = [
    "$end",
    "EOF",
    "INVALID",
    "NEWLINE",
    "timeline",
    "timeline_lr",
    "timeline_td",
    "title",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "section",
    "period",
    "event",
];

fn period_end(src: &[u16], from: usize) -> usize {
    let mut e = from;
    while let Some(&k) = src.get(e) {
        if k == 35 || k == 58 || k == 10 {
            break;
        }
        e += 1;
    }
    e
}

pub fn tokenize(source: &str) -> Tokens {
    let src: Vec<u16> = source.encode_utf16().collect();
    let n = src.len();
    let mut out = Tokens::default();
    let mut p = 0usize;
    let emit = |out: &mut Tokens, p: &mut usize, kind: i32, end: usize| {
        out.types.push(kind);
        out.texts.push(slice(&src, *p, end));
        out.starts.push(*p);
        *p = end;
    };
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
            emit(&mut out, &mut p, NEWLINE, e);
            continue;
        }
        if keyword_at(&src, p, "timeline") {
            e = p + 8;
            while src.get(e) == Some(&32) || src.get(e) == Some(&9) {
                e += 1;
            }
            let lr = keyword_at(&src, e, "lr");
            if e > p + 8 && (lr || keyword_at(&src, e, "td")) && !word(src.get(e + 2)) {
                emit(
                    &mut out,
                    &mut p,
                    if lr { TIMELINE_LR } else { TIMELINE_TD },
                    e + 2,
                );
                continue;
            }
            if !word(src.get(p + 8)) {
                let end = p + 8;
                emit(&mut out, &mut p, TIMELINE, end);
                continue;
            }
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
                let end = line_end(&src, p + 6);
                emit(&mut out, &mut p, TITLE, end);
                continue;
            }
        }
        e = accessibility(&src, p, &mut out, ACC_TITLE);
        if e != p {
            p = e;
            continue;
        }
        if keyword_at(&src, p, "section") && src.get(p + 7).is_some_and(|&u| space(u)) {
            e = p + 8;
            while e < n && src[e] != 58 && src[e] != 10 {
                e += 1;
            }
            if e > p + 8 {
                emit(&mut out, &mut p, SECTION, e);
                continue;
            }
        }
        if c != 58 {
            let end = period_end(&src, p + 1);
            emit(&mut out, &mut p, PERIOD, end);
            continue;
        }
        if src.get(p + 1).is_some_and(|&u| space(u)) {
            e = p + 2;
            while e < n {
                let k = src[e];
                if k == 10 || (k == 58 && src.get(e + 1).is_some_and(|&u| space(u))) {
                    break;
                }
                e += 1;
            }
            if e > p + 2 {
                emit(&mut out, &mut p, EVENT, e);
                continue;
            }
        }
        let end = p + 1;
        emit(&mut out, &mut p, INVALID, end);
    }

    if out.starts.last().is_none_or(|&s| s < n) {
        emit(&mut out, &mut p, EOF, n);
    }
    emit(&mut out, &mut p, END, n);
    out
}
