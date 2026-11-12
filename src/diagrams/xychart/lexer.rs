use crate::diagrams::common::scan::{Tokens, slice, word_at};
use crate::util::chars::{is_js_space, is_word};

pub const END: i32 = 0;
pub const ERROR: i32 = 1;
pub const NEWLINE: i32 = 2;
pub const TITLE: i32 = 3;
pub const ACC_TITLE: i32 = 4;
pub const ACC_TITLE_VALUE: i32 = 5;
pub const ACC_DESCR: i32 = 6;
pub const ACC_DESCR_VALUE: i32 = 7;
pub const ACC_DESCR_MULTILINE_VALUE: i32 = 8;
pub const XYCHART: i32 = 9;
pub const CHART_ORIENTATION: i32 = 10;
pub const X_AXIS: i32 = 11;
pub const Y_AXIS: i32 = 12;
pub const ARROW_DELIMITER: i32 = 13;
pub const LINE: i32 = 14;
pub const BAR: i32 = 15;
pub const NUMBER_WITH_DECIMAL: i32 = 16;
pub const SQUARE_BRACES_START: i32 = 17;
pub const SQUARE_BRACES_END: i32 = 18;
pub const STR: i32 = 19;
pub const ALPHA: i32 = 20;
pub const COLON: i32 = 21;
pub const PLUS: i32 = 22;
pub const COMMA: i32 = 23;
pub const EQUALS: i32 = 24;
pub const MULT: i32 = 25;
pub const BRKT: i32 = 26;
pub const UNDERSCORE: i32 = 27;
pub const DOT: i32 = 28;
pub const AMP: i32 = 29;
pub const MINUS: i32 = 30;
pub const NUM: i32 = 31;
pub const SEMI: i32 = 32;
pub const EOF: i32 = 33;

pub const TOKEN_NAMES: [&str; 34] = [
    "$end",
    "INVALID",
    "NEWLINE",
    "title",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "XYCHART",
    "CHART_ORIENTATION",
    "X_AXIS",
    "Y_AXIS",
    "ARROW_DELIMITER",
    "LINE",
    "BAR",
    "NUMBER_WITH_DECIMAL",
    "SQUARE_BRACES_START",
    "SQUARE_BRACES_END",
    "STR",
    "ALPHA",
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
    "SEMI",
    "EOF",
];

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    AxisData,
    AxisBandData,
    Data,
    DataInner,
    Str,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
}

fn at_end(state: State) -> i32 {
    match state {
        State::Initial | State::AxisData | State::AxisBandData | State::Data | State::DataInner => {
            EOF
        }
        State::Str => STR,
        State::AccTitle => ACC_TITLE_VALUE,
        State::AccDescr => ACC_DESCR_VALUE,
        State::AccDescrMultiline => ACC_DESCR_MULTILINE_VALUE,
    }
}

const BROKEN_RULE: &str = concat!(
    "`)",
    "                                    ",
    "{ this.pushstate(md_string); }\n<md_string>(?:(?!`\").)+",
    "                  ",
    "{ return md_str; }\n<md_string>(?:`"
);

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
        59 => SEMI,
        _ => 0,
    }
}

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn is_letter(c: u16) -> bool {
    (65..=90).contains(&c) || (97..=122).contains(&c)
}

fn is_word_unit(c: u16) -> bool {
    c < 128 && is_word(c as u8 as char)
}

fn is_space_unit(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

pub fn tokenize(source: &str) -> Tokens {
    let src: Vec<u16> = source.encode_utf16().collect();
    let n = src.len();
    let mut out = Tokens::default();
    let mut state = State::Initial;
    let mut stack: Vec<State> = Vec::new();
    let mut p = 0usize;

    let unit = |i: usize| src.get(i).copied().unwrap_or(0);
    let has = |i: usize| i < n;
    let line_end = |q: usize| -> usize {
        src.get(q..)
            .and_then(|rest| rest.iter().position(|&u| u == 10))
            .map_or(n, |at| q + at)
    };
    let run = |mut q: usize| -> usize {
        while q < n && is_space_unit(src[q]) {
            q += 1;
        }
        q
    };
    let digits = |mut q: usize| -> usize {
        while is_digit(unit(q)) && has(q) {
            q += 1;
        }
        q
    };

    macro_rules! emit {
        ($kind:expr, $s:expr, $e:expr) => {{
            let (s, e): (usize, usize) = ($s, $e);
            out.types.push($kind);
            out.texts.push(slice(&src, s, e));
            out.starts.push(s);
            p = e;
        }};
    }

    'lexing: while p < n {
        let c = unit(p);
        if state == State::Str {
            if c == 34 {
                p += 1;
                state = stack.pop().unwrap_or(State::Initial);
            } else {
                let e = src[p..]
                    .iter()
                    .position(|&u| u == 34)
                    .map_or(n, |at| p + at);
                emit!(STR, p, e);
            }
            continue;
        }
        if state == State::AccTitle || state == State::AccDescr {
            let kind = at_end(state);
            let e = line_end(p);
            emit!(kind, p, e);
            state = stack.pop().unwrap_or(State::Initial);
            continue;
        }
        if state == State::AccDescrMultiline {
            if c == 125 {
                p += 1;
                state = stack.pop().unwrap_or(State::Initial);
            } else {
                let e = src[p..]
                    .iter()
                    .position(|&u| u == 125)
                    .map_or(n, |at| p + at);
                emit!(ACC_DESCR_MULTILINE_VALUE, p, e);
            }
            continue;
        }

        let next = unit(p + 1);
        if c == 37 && next == 37 {
            p = line_end(p + 2);
            continue;
        }
        if c == 10 || c == 13 {
            if (state == State::AxisData || state == State::Data) && (c == 10 || next == 10) {
                let e = if c == 10 { p + 1 } else { p + 2 };
                emit!(NEWLINE, p, e);
                state = stack.pop().unwrap_or(State::Initial);
                continue;
            }
            let mut e = p + 1;
            while unit(e) == 10 || unit(e) == 13 {
                e += 1;
            }
            emit!(NEWLINE, p, e);
            continue;
        }

        let lower = c | 32;
        let keyword = |word: &str, p: usize| -> bool {
            word_at(&src, p, word) && !is_word_unit(unit(p + word.len()))
        };
        match lower {
            116 => {
                if keyword("title", p) {
                    emit!(TITLE, p, p + 5);
                    continue;
                }
            }
            97 => {
                if word_at(&src, p, "acc") {
                    let descr = word_at(&src, p + 3, "descr");
                    if descr || word_at(&src, p + 3, "title") {
                        let k = run(p + 8);
                        let d = unit(k);
                        if d == 58 {
                            let e = run(k + 1);
                            emit!(if descr { ACC_DESCR } else { ACC_TITLE }, p, e);
                            stack.push(state);
                            state = if descr {
                                State::AccDescr
                            } else {
                                State::AccTitle
                            };
                            continue;
                        }
                        if d == 123 && descr {
                            p = run(k + 1);
                            stack.push(state);
                            state = State::AccDescrMultiline;
                            continue;
                        }
                    }
                }
            }
            120 => {
                if keyword("xychart-beta", p) {
                    emit!(XYCHART, p, p + 12);
                    continue;
                }
                if keyword("xychart", p) {
                    emit!(XYCHART, p, p + 7);
                    continue;
                }
                if keyword("x-axis", p) {
                    emit!(X_AXIS, p, p + 6);
                    stack.push(state);
                    state = State::AxisData;
                    continue;
                }
            }
            121 => {
                if keyword("y-axis", p) {
                    emit!(Y_AXIS, p, p + 6);
                    stack.push(state);
                    state = State::AxisData;
                    continue;
                }
            }
            118 => {
                if word_at(&src, p, "vertical") {
                    emit!(CHART_ORIENTATION, p, p + 8);
                    continue;
                }
            }
            104 => {
                if word_at(&src, p, "horizontal") {
                    emit!(CHART_ORIENTATION, p, p + 10);
                    continue;
                }
            }
            108 => {
                if keyword("line", p) {
                    emit!(LINE, p, p + 4);
                    stack.push(state);
                    state = State::Data;
                    continue;
                }
            }
            98 if keyword("bar", p) => {
                emit!(BAR, p, p + 3);
                stack.push(state);
                state = State::Data;
                continue;
            }
            _ => {}
        }

        if c == 91 {
            if state == State::AxisData {
                stack.push(state);
                state = State::AxisBandData;
            } else if state == State::Data {
                stack.push(state);
                state = State::DataInner;
            }
            emit!(SQUARE_BRACES_START, p, p + 1);
            continue;
        }
        if c == 93 {
            if state == State::DataInner || state == State::AxisBandData {
                state = stack.pop().unwrap_or(State::Initial);
            }
            emit!(SQUARE_BRACES_END, p, p + 1);
            continue;
        }
        if state == State::AxisData && c == 45 && next == 45 && unit(p + 2) == 62 {
            emit!(ARROW_DELIMITER, p, p + 3);
            continue;
        }
        if state == State::AxisData || state == State::DataInner {
            let q = if c == 43 || c == 45 { p + 1 } else { p };
            let d = unit(q);
            let mut e: Option<usize> = None;
            if is_digit(d) {
                let mut end = digits(q + 1);
                if unit(end) == 46 && is_digit(unit(end + 1)) {
                    end = digits(end + 2);
                }
                e = Some(end);
            } else if d == 46 && is_digit(unit(q + 1)) {
                e = Some(digits(q + 2));
            }
            if let Some(e) = e {
                emit!(NUMBER_WITH_DECIMAL, p, e);
                continue;
            }
        }
        if c == 96 && word_at(&src, p, BROKEN_RULE) {
            p += BROKEN_RULE.encode_utf16().count();
            state = stack.pop().unwrap_or(State::Initial);
            continue;
        }
        if c == 34 {
            p += 1;
            stack.push(state);
            state = State::Str;
            continue;
        }
        if is_letter(c) {
            let mut e = p + 1;
            while is_letter(unit(e)) && has(e) {
                e += 1;
            }
            emit!(ALPHA, p, e);
            continue;
        }
        if is_digit(c) {
            let e = digits(p + 1);
            emit!(NUM, p, e);
            continue;
        }
        if is_space_unit(c) {
            p = run(p + 1);
            continue;
        }
        if c < 128 && single(c) != 0 {
            emit!(single(c), p, p + 1);
            continue;
        }
        break 'lexing;
    }

    if p < n {
        out.types.push(ERROR);
        out.texts.push(String::new());
        out.starts.push(p);
        return out;
    }
    out.types.push(at_end(state));
    out.texts.push(String::new());
    out.starts.push(n);
    out.types.push(END);
    out.texts.push(String::new());
    out.starts.push(n);
    out
}
