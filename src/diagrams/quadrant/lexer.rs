use crate::diagrams::common::scan::slice;
use crate::util::chars::{is_js_space, is_word};

pub const END: i32 = 0;
pub const ERROR: i32 = 1;
pub const NEWLINE: i32 = 2;
pub const TITLE: i32 = 3;
pub const TITLE_VALUE: i32 = 4;
pub const ACC_TITLE: i32 = 5;
pub const ACC_TITLE_VALUE: i32 = 6;
pub const ACC_DESCR: i32 = 7;
pub const ACC_DESCR_VALUE: i32 = 8;
pub const ACC_DESCR_MULTILINE_VALUE: i32 = 9;
pub const X_AXIS: i32 = 10;
pub const Y_AXIS: i32 = 11;
pub const DELIMITER: i32 = 12;
pub const QUADRANT_1: i32 = 13;
pub const QUADRANT_4: i32 = 16;
pub const CLASSDEF: i32 = 17;
pub const MD_STR: i32 = 18;
pub const STR: i32 = 19;
pub const CLASS_NAME: i32 = 20;
pub const POINT_START: i32 = 21;
pub const POINT_X: i32 = 22;
pub const POINT_Y: i32 = 23;
pub const QUADRANT: i32 = 24;
pub const ALPHA: i32 = 25;
pub const UNICODE_TEXT: i32 = 26;
pub const COLON: i32 = 27;
pub const PLUS: i32 = 28;
pub const COMMA: i32 = 29;
pub const EQUALS: i32 = 30;
pub const MULT: i32 = 31;
pub const BRKT: i32 = 32;
pub const UNDERSCORE: i32 = 33;
pub const DOT: i32 = 34;
pub const AMP: i32 = 35;
pub const MINUS: i32 = 36;
pub const NUM: i32 = 37;
pub const SPACE: i32 = 38;
pub const SEMI: i32 = 39;
pub const PUNCTUATION: i32 = 40;
pub const EOF: i32 = 41;

pub const TOKEN_NAMES: [&str; 42] = [
    "$end",
    "INVALID",
    "NEWLINE",
    "title",
    "title_value",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "X-AXIS",
    "Y-AXIS",
    "AXIS-TEXT-DELIMITER",
    "QUADRANT_1",
    "QUADRANT_2",
    "QUADRANT_3",
    "QUADRANT_4",
    "CLASSDEF",
    "MD_STR",
    "STR",
    "class_name",
    "point_start",
    "point_x",
    "point_y",
    "QUADRANT",
    "ALPHA",
    "UNICODE_TEXT",
    "COLON",
    "PLUS",
    "COMMA",
    "EQUALS",
    "MULT",
    "BRKT",
    "UNDERSCORE",
    "DOT",
    "AMP",
    "MINUS",
    "NUM",
    "SPACE",
    "SEMI",
    "PUNCTUATION",
    "EOF",
];

#[derive(Debug, Default)]
pub struct Tokens {
    pub types: Vec<i32>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    Title,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
    Str,
    MdStr,
    ClassName,
    PointStart,
    PointX,
    PointY,
}

fn at_end(state: State) -> Option<i32> {
    match state {
        State::Initial => Some(EOF),
        State::Title => Some(TITLE_VALUE),
        State::AccTitle => Some(ACC_TITLE_VALUE),
        State::AccDescr => Some(ACC_DESCR_VALUE),
        State::AccDescrMultiline => Some(ACC_DESCR_MULTILINE_VALUE),
        State::Str => Some(STR),
        _ => None,
    }
}

fn single(c: u16) -> i32 {
    match c {
        58 => COLON,
        43 => PLUS,
        44 => COMMA,
        61 => EQUALS,
        42 => MULT,
        35 => BRKT,
        95 => UNDERSCORE,
        46 => DOT,
        38 => AMP,
        45 => MINUS,
        32 | 9 | 11 | 12 => SPACE,
        59 => SEMI,
        33 | 36 | 37 | 39 | 96 | 63 | 92 | 47 => PUNCTUATION,
        _ => 0,
    }
}

fn is_line_end(c: u16) -> bool {
    c == 10 || c == 13 || c == 0x2028 || c == 0x2029
}

fn is_letter(c: u16) -> bool {
    (65..=90).contains(&c) || (97..=122).contains(&c)
}

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn is_word_unit(c: Option<u16>) -> bool {
    c.is_some_and(|u| u < 128 && is_word(u as u8 as char))
}

fn is_space_unit(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

fn word_at(src: &[u16], at: usize, word: &str) -> bool {
    word.bytes().enumerate().all(|(k, w)| {
        src.get(at + k).is_some_and(|&c| {
            let w = u16::from(w);
            c == w || ((97..=122).contains(&w) && c == w - 32)
        })
    })
}

struct Lexer {
    src: Vec<u16>,
    n: usize,
    out: Tokens,
    p: usize,
    state: State,
    stack: Vec<State>,
}

impl Lexer {
    fn at(&self, i: usize) -> u16 {
        self.src.get(i).copied().unwrap_or(0)
    }

    fn has(&self, i: usize) -> bool {
        i < self.n
    }

    fn emit(&mut self, kind: i32, start: usize, end: usize) {
        self.out.types.push(kind);
        self.out.texts.push(slice(&self.src, start, end));
        self.out.starts.push(start);
        self.p = end;
    }

    fn push(&mut self, state: State) {
        self.stack.push(self.state);
        self.state = state;
    }

    fn pop(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.state = state;
        }
    }

    fn line_end(&self, from: usize) -> usize {
        self.src
            .get(from..)
            .and_then(|rest| rest.iter().position(|&u| u == 10))
            .map_or(self.n, |at| from + at)
    }

    fn spaces(&self, mut q: usize) -> usize {
        while self.src.get(q) == Some(&32) {
            q += 1;
        }
        q
    }

    fn run(&self, mut q: usize) -> usize {
        while q < self.n && is_space_unit(self.src[q]) {
            q += 1;
        }
        q
    }

    fn coordinate(&self, q: usize) -> Option<usize> {
        let c = self.src.get(q).copied()?;
        if c == 49 {
            return Some(q + 1);
        }
        if c != 48 {
            return None;
        }
        let mut e = q + 1;
        if e < self.n && !is_line_end(self.at(e)) && is_digit(self.at(e + 1)) {
            e += 2;
            while is_digit(self.at(e)) {
                e += 1;
            }
        }
        Some(e)
    }
}

pub fn tokenize(source: &str) -> Tokens {
    let src: Vec<u16> = source.encode_utf16().collect();
    let n = src.len();
    let mut lx = Lexer {
        src,
        n,
        out: Tokens::default(),
        p: 0,
        state: State::Initial,
        stack: Vec::new(),
    };

    'lexing: while lx.p < n {
        let p = lx.p;
        let c = lx.at(p);
        let next = lx.src.get(p + 1).copied();
        match lx.state {
            State::Initial => {}
            State::Title | State::AccTitle | State::AccDescr => {
                let kind = at_end(lx.state).unwrap_or(EOF);
                let end = lx.line_end(p);
                lx.emit(kind, p, end);
                lx.pop();
                continue;
            }
            State::AccDescrMultiline => {
                if c == 125 {
                    lx.p += 1;
                    lx.pop();
                } else {
                    let e = lx.src[p..]
                        .iter()
                        .position(|&u| u == 125)
                        .map_or(n, |at| p + at);
                    lx.emit(ACC_DESCR_MULTILINE_VALUE, p, e);
                }
                continue;
            }
            State::Str => {
                if c == 34 {
                    lx.p += 1;
                    lx.pop();
                } else {
                    let e = lx.src[p..]
                        .iter()
                        .position(|&u| u == 34)
                        .map_or(n, |at| p + at);
                    lx.emit(STR, p, e);
                }
                continue;
            }
            State::MdStr => {
                if c == 96 && next == Some(34) {
                    lx.p += 2;
                    lx.pop();
                    continue;
                }
                if c != 96 && c != 34 {
                    let mut e = p + 1;
                    while e < n && lx.src[e] != 96 && lx.src[e] != 34 {
                        e += 1;
                    }
                    lx.emit(MD_STR, p, e);
                    continue;
                }
                break 'lexing;
            }
            State::ClassName => {
                if is_word_unit(Some(c)) {
                    let mut e = p + 1;
                    while is_word_unit(lx.src.get(e).copied()) {
                        e += 1;
                    }
                    lx.emit(CLASS_NAME, p, e);
                    lx.pop();
                    continue;
                }
                break 'lexing;
            }
            State::PointStart => {
                if let Some(e) = lx.coordinate(p) {
                    lx.emit(POINT_X, p, e);
                    lx.push(State::PointX);
                    continue;
                }
                let k = lx.run(p);
                if lx.at(k) == 93 && lx.has(k) {
                    lx.p = lx.spaces(k + 1);
                    lx.pop();
                    continue;
                }
                break 'lexing;
            }
            State::PointX => {
                let k = lx.run(p);
                if lx.has(k) && lx.at(k) == 44 {
                    lx.p = lx.run(k + 1);
                    lx.state = State::PointY;
                    continue;
                }
                break 'lexing;
            }
            State::PointY => {
                if let Some(e) = lx.coordinate(p) {
                    lx.emit(POINT_Y, p, e);
                    lx.pop();
                    continue;
                }
                break 'lexing;
            }
        }

        if c == 37 && next == Some(37) {
            lx.p = lx.line_end(p + 2);
            continue;
        }
        if c == 10 || c == 13 {
            let mut e = p + 1;
            while lx.at(e) == 10 || lx.at(e) == 13 {
                e += 1;
            }
            lx.emit(NEWLINE, p, e);
            continue;
        }

        let lower = c | 32;
        if lower == 116 && word_at(&lx.src, p, "title") && !is_word_unit(lx.src.get(p + 5).copied())
        {
            lx.emit(TITLE, p, p + 5);
            lx.push(State::Title);
            continue;
        }
        if lower == 97 && word_at(&lx.src, p, "acc") {
            let descr = word_at(&lx.src, p + 3, "descr");
            if descr || word_at(&lx.src, p + 3, "title") {
                let k = lx.run(p + 8);
                let d = lx.at(k);
                if lx.has(k) && d == 58 {
                    let end = lx.run(k + 1);
                    lx.emit(if descr { ACC_DESCR } else { ACC_TITLE }, p, end);
                    lx.push(if descr {
                        State::AccDescr
                    } else {
                        State::AccTitle
                    });
                    continue;
                }
                if lx.has(k) && d == 123 && descr {
                    lx.p = lx.run(k + 1);
                    lx.push(State::AccDescrMultiline);
                    continue;
                }
            }
        }

        let mut j = p;
        if c == 32 {
            j = lx.spaces(p);
        }
        let d = lx.at(j);
        let dl = d | 32;
        let mut kind = 0;
        let mut end = 0;
        if lx.has(j) && dl == 120 && word_at(&lx.src, j, "x-axis") {
            kind = X_AXIS;
            end = j + 6;
        } else if lx.has(j) && dl == 121 && word_at(&lx.src, j, "y-axis") {
            kind = Y_AXIS;
            end = j + 6;
        } else if lx.has(j) && d == 45 {
            let mut dash_to = j;
            while lx.at(dash_to) == 45 {
                dash_to += 1;
            }
            if dash_to - j >= 2 && lx.at(dash_to) == 62 {
                kind = DELIMITER;
                end = dash_to + 1;
            }
        } else if lx.has(j) && dl == 113 && word_at(&lx.src, j, "quadrant") {
            let q = lx.at(j + 9);
            if lx.at(j + 8) == 45 && (49..=52).contains(&q) {
                kind = QUADRANT_1 + i32::from(q) - 49;
                end = j + 10;
            } else if word_at(&lx.src, j + 8, "chart") {
                kind = QUADRANT;
                end = j + 13;
            }
        }
        if kind != 0 {
            let to = lx.spaces(end);
            lx.emit(kind, p, to);
            continue;
        }

        if lower == 99
            && word_at(&lx.src, p, "classdef")
            && !is_word_unit(lx.src.get(p + 8).copied())
        {
            lx.emit(CLASSDEF, p, p + 8);
            continue;
        }
        if c == 34 {
            if next == Some(96) {
                lx.p += 2;
                lx.push(State::MdStr);
            } else {
                lx.p += 1;
                lx.push(State::Str);
            }
            continue;
        }
        if c == 58 && next == Some(58) && lx.at(p + 2) == 58 && lx.has(p + 2) {
            lx.p += 3;
            lx.push(State::ClassName);
            continue;
        }
        if c == 58 || is_space_unit(c) {
            let k = if c == 58 { p } else { lx.run(p) };
            if lx.has(k) && lx.at(k) == 58 {
                let gap = lx.run(k + 1);
                if lx.has(gap) && lx.at(gap) == 91 {
                    let end = lx.run(gap + 1);
                    lx.emit(POINT_START, p, end);
                    lx.push(State::PointStart);
                    continue;
                }
            }
        }

        if is_letter(c) {
            let mut e = p + 1;
            while is_letter(lx.at(e)) && lx.has(e) {
                e += 1;
            }
            lx.emit(ALPHA, p, e);
            continue;
        }
        if c >= 128 {
            let mut e = p + 1;
            while lx.has(e) && lx.at(e) >= 128 {
                e += 1;
            }
            lx.emit(UNICODE_TEXT, p, e);
            continue;
        }
        if is_digit(c) {
            let mut e = p + 1;
            while is_digit(lx.at(e)) && lx.has(e) {
                e += 1;
            }
            lx.emit(NUM, p, e);
            continue;
        }
        let kind = single(c);
        if kind != 0 {
            lx.emit(kind, p, p + 1);
            continue;
        }
        break;
    }

    let Lexer {
        mut out,
        p,
        state,
        n,
        ..
    } = lx;
    if p < n {
        out.types.push(ERROR);
        out.texts.push(String::new());
        out.starts.push(p);
        return out;
    }
    if let Some(kind) = at_end(state) {
        out.types.push(kind);
        out.texts.push(String::new());
        out.starts.push(n);
    }
    out.types.push(END);
    out.texts.push(String::new());
    out.starts.push(n);
    out
}
