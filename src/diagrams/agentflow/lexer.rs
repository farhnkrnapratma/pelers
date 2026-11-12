use super::tokens::*;
use crate::util::chars::{is_js_space, is_word};
use crate::util::unicode::is_unicode_letter;

pub struct Tokens {
    pub types: Vec<usize>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
    pub locs: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Initial,
    String,
    MdString,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
    Dir,
    Text,
    EllipseText,
    TrapText,
    EdgeText,
    Click,
    CallbackName,
    CallbackArgs,
    ShapeData,
    ShapeDataStr,
}

const NONE: u32 = u32::MAX;
const NODE_CHARS: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!\"#$%&'*+.`?\\_/";
const DIRECTIONS: [(&str, usize); 5] = [
    ("TB", DIRECTION_TB),
    ("BT", DIRECTION_BT),
    ("RL", DIRECTION_RL),
    ("LR", DIRECTION_LR),
    ("TD", DIRECTION_TD),
];
const DIRECTION_WORDS: [&str; 6] = ["LR", "RL", "TB", "BT", "TD", "BR"];

fn unit_at(u: &[u16], i: usize) -> u32 {
    u.get(i).map_or(NONE, |&c| u32::from(c))
}

fn is_space(c: u32) -> bool {
    char::from_u32(c).is_some_and(is_js_space)
}

fn is_word_unit(c: u32) -> bool {
    char::from_u32(c).is_some_and(is_word)
}

fn is_terminator(c: u16) -> bool {
    matches!(c, 10 | 13 | 0x2028 | 0x2029)
}

fn ascii_units(word: &str) -> impl Iterator<Item = u16> + '_ {
    word.bytes().map(u16::from)
}

struct Lexer {
    u: Vec<u16>,
    n: usize,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
    locs: Vec<usize>,
    stack: Vec<State>,
    state: State,
    p: usize,
    line: usize,
    col: usize,
    first_graph: bool,
    dir_next: isize,
    at_next: isize,
    line_end: isize,
    dir_failed: usize,
    run_end: usize,
    run_at: isize,
    ws_from: isize,
    ws_to: isize,
    blank_from: isize,
    blank_to: isize,
}

impl Lexer {
    fn new(src: &str) -> Self {
        let u: Vec<u16> = src.encode_utf16().collect();
        let n = u.len();
        Self {
            dir_next: index_of_str(&u, 0, "direction"),
            at_next: index_of_unit(&u, 0, 64).unwrap_or(-1),
            u,
            n,
            types: Vec::new(),
            texts: Vec::new(),
            starts: Vec::new(),
            locs: Vec::new(),
            stack: Vec::new(),
            state: State::Initial,
            p: 0,
            line: 1,
            col: 0,
            first_graph: true,
            line_end: -1,
            dir_failed: 0,
            run_end: 0,
            run_at: -1,
            ws_from: -1,
            ws_to: -1,
            blank_from: -1,
            blank_to: -1,
        }
    }

    fn unit(&self, i: usize) -> u32 {
        unit_at(&self.u, i)
    }

    fn slice(&self, from: usize, to: usize) -> String {
        String::from_utf16_lossy(&self.u[from.min(self.n)..to.min(self.n)])
    }

    fn advance(&mut self, e: usize) {
        let mut brk: Option<usize> = None;
        let mut k = self.p;
        while k < e {
            let ch = self.u[k];
            if ch == 10 {
                self.line += 1;
                brk = Some(k);
            } else if ch == 13 {
                if k + 1 < e && self.u[k + 1] == 10 {
                    k += 1;
                }
                self.line += 1;
                brk = Some(k);
            }
            k += 1;
        }
        match brk {
            None => self.col += e - self.p,
            Some(brk) => {
                let mut k = brk + 1;
                while k < e && !matches!(self.u[k], 0x2028 | 0x2029) {
                    k += 1;
                }
                self.col = k - brk - 1;
            }
        }
        self.p = e;
    }

    fn push_token(&mut self, kind: usize, text: String, start: usize) {
        self.types.push(kind);
        self.texts.push(text);
        self.starts.push(start);
        self.locs.push(self.line);
        self.locs.push(self.col);
    }

    fn close_token(&mut self) {
        self.locs.push(self.line);
        self.locs.push(self.col);
    }

    fn emit(&mut self, kind: usize, e: usize) {
        let text = self.slice(self.p, e);
        let start = self.p;
        self.push_token(kind, text, start);
        self.advance(e);
        self.close_token();
    }

    fn emit_flat(&mut self, kind: usize, e: usize) {
        let text = self.slice(self.p, e);
        let start = self.p;
        self.push_token(kind, text, start);
        self.col += e - self.p;
        self.p = e;
        self.close_token();
    }

    fn emit_chars(&mut self, kind: usize, e: usize) {
        let text = self.slice(self.p, e);
        let start = self.p;
        self.push_token(kind, text, start);
        for k in self.p..e {
            let ch = self.u[k];
            if ch == 10 || ch == 13 {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
        }
        self.p = e;
        self.close_token();
    }

    fn emit_text(&mut self, kind: usize, text: String, e: usize) {
        let start = self.p;
        self.push_token(kind, text, start);
        self.advance(e);
        self.close_token();
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

    fn keyword(&self, word: &str) -> bool {
        let len = word.len();
        starts_with_units(&self.u, self.p, word) && !is_word_unit(self.unit(self.p + len))
    }

    fn skip_space(&self, mut q: usize) -> usize {
        while q < self.n && is_space(self.u[q] as u32) {
            q += 1;
        }
        q
    }

    fn ws_end(&mut self, q: usize) -> usize {
        let q_signed = q as isize;
        if q_signed >= self.ws_from && q_signed <= self.ws_to {
            return self.ws_to as usize;
        }
        let j = self.skip_space(q);
        self.ws_from = q_signed;
        self.ws_to = j as isize;
        j
    }

    fn blank_end(&mut self, q: usize) -> usize {
        let q_signed = q as isize;
        if q_signed >= self.blank_from && q_signed <= self.blank_to {
            return self.blank_to as usize;
        }
        let mut j = q;
        while j < self.n {
            let ch = self.u[j];
            if ch != 32 && ch != 9 {
                break;
            }
            j += 1;
        }
        self.blank_from = q_signed;
        self.blank_to = j as isize;
        j
    }

    fn arrow_end(&mut self, q: usize) -> Option<usize> {
        let j = self.ws_end(q);
        if self.unit(j) != 45 || self.unit(j + 1) != 45 {
            return None;
        }
        let mut k = j + 2;
        while self.unit(k) == 45 {
            k += 1;
        }
        let t = self.unit(k);
        if t == 120 || t == 62 {
            Some(self.ws_end(k + 1))
        } else {
            None
        }
    }

    fn link(&mut self) -> bool {
        let j = self.ws_end(self.p);
        if self.unit(j) != 45 {
            return false;
        }
        let d = self.unit(j + 1);
        let mut k = j + 2;
        if d == 45 {
            while self.unit(k) == 45 {
                k += 1;
            }
            let t = self.unit(k);
            if t == 120 || t == 62 {
                let e = self.ws_end(k + 1);
                self.emit(LINK, e);
            } else {
                self.push(State::EdgeText);
                let e = self.ws_end(j + 2);
                self.emit(START_LINK, e);
            }
            return true;
        }
        if d == 46 {
            while self.unit(k) == 46 {
                k += 1;
            }
            if self.unit(k) == 45 {
                let e = self.ws_end(k + 1);
                self.emit(LINK, e);
                return true;
            }
        }
        false
    }

    fn comment_end(&self, q: usize) -> Option<usize> {
        if self.unit(q) != 37 || self.unit(q + 1) != 37 || self.unit(q + 2) == 123 {
            return None;
        }
        Some(index_of_unit(&self.u, q + 2, 10).map_or(self.n, |e| e as usize))
    }

    fn quote(&mut self) {
        if self.unit(self.p + 1) == 96 {
            self.push(State::MdString);
            self.advance(self.p + 2);
        } else {
            self.push(State::String);
            self.advance(self.p + 1);
        }
    }

    fn opener(&mut self, c: u32) -> bool {
        let p = self.p;
        let d = self.unit(p + 1);
        if c == 40 {
            if d == 45 {
                self.push(State::EllipseText);
                self.emit_flat(ELLIPSE_START, p + 2);
            } else if d == 91 {
                self.push(State::Text);
                self.emit_flat(STADIUMSTART, p + 2);
            } else if d == 40 && self.unit(p + 2) == 40 {
                self.push(State::Text);
                self.emit_flat(DOUBLECIRCLESTART, p + 3);
            } else {
                self.push(State::Text);
                self.emit_flat(PS, p + 1);
            }
            return true;
        }
        if c == 91 {
            if d == 91 {
                self.push(State::Text);
                self.emit_flat(SUBROUTINESTART, p + 2);
            } else if d == 40 {
                self.push(State::Text);
                self.emit_flat(CYLINDERSTART, p + 2);
            } else if d == 47 {
                self.push(State::TrapText);
                self.emit_flat(TRAPSTART, p + 2);
            } else if d == 92 {
                self.push(State::TrapText);
                self.emit_flat(INVTRAPSTART, p + 2);
            } else {
                self.push(State::Text);
                self.emit_flat(SQS, p + 1);
            }
            return true;
        }
        if c == 124 {
            self.push(State::Text);
            self.emit_flat(PIPE, p + 1);
            return true;
        }
        if c == 123 {
            self.push(State::Text);
            self.emit_flat(DIAMOND_START, p + 1);
            return true;
        }
        false
    }

    fn node_string(&mut self) -> bool {
        let mut q = self.p;
        while q < self.n {
            let ch = self.u[q];
            if ch < 128 && NODE_CHARS.contains(&(ch as u8)) {
                q += 1;
            } else if ch == 45 && q + 1 < self.n {
                let d = self.u[q + 1];
                if d == 62 || d == 45 || d == 46 {
                    break;
                }
                q += 1;
            } else {
                break;
            }
        }
        if q == self.p {
            return false;
        }
        self.emit_flat(NODE_STRING, q);
        true
    }

    fn blanks(&self, from: usize) -> usize {
        let mut e = from;
        while e < self.n {
            let ch = self.u[e];
            if ch != 32 && ch != 9 {
                break;
            }
            e += 1;
        }
        e
    }

    fn acc_match(&self, p: usize, name: &str, colon: bool) -> Option<usize> {
        if !starts_with_units(&self.u, p, name) {
            return None;
        }
        let mut j = self.skip_space(p + name.len());
        if colon {
            if self.unit(j) != 58 {
                return None;
            }
            j += 1;
        } else {
            if self.unit(j) != 123 {
                return None;
            }
            j += 1;
        }
        Some(self.skip_space(j))
    }

    fn word_then_space(&self, p: usize, word: &str) -> Option<usize> {
        if !starts_with_units(&self.u, p, word) {
            return None;
        }
        let j = self.skip_space(p + word.len());
        if j == p + word.len() { None } else { Some(j) }
    }

    fn initial(&mut self, c: u32) -> bool {
        let p = self.p;
        match c {
            97 => {
                if starts_with_units(&self.u, p, "acc") {
                    if let Some(e) = self.acc_match(p, "accTitle", true) {
                        self.push(State::AccTitle);
                        self.emit(ACC_TITLE, e);
                        return true;
                    }
                    if let Some(e) = self.acc_match(p, "accDescr", true) {
                        self.push(State::AccDescr);
                        self.emit(ACC_DESCR, e);
                        return true;
                    }
                    if let Some(e) = self.acc_match(p, "accDescr", false) {
                        self.push(State::AccDescrMultiline);
                        self.advance(e);
                        return true;
                    }
                }
                if self.keyword("agentflow-beta") {
                    if self.first_graph {
                        self.first_graph = false;
                        self.push(State::Dir);
                    }
                    self.emit_flat(GRAPH, p + 14);
                    return true;
                }
            }
            64 => {
                if self.unit(p + 1) == 123 {
                    self.push(State::ShapeData);
                    self.emit_text(SHAPE_DATA, String::new(), p + 2);
                    return true;
                }
            }
            99 => {
                if let Some(e) = self.word_then_space(p, "call") {
                    self.push(State::CallbackName);
                    self.advance(e);
                    return true;
                }
                if self.keyword("classDef") {
                    self.emit_flat(CLASSDEF, p + 8);
                    return true;
                }
                if self.keyword("class") {
                    self.emit_flat(CLASS, p + 5);
                    return true;
                }
                if let Some(e) = self.word_then_space(p, "click") {
                    self.push(State::Click);
                    self.advance(e);
                    return true;
                }
                if self.keyword("connector") {
                    self.emit_flat(CONNECTOR, p + 9);
                    return true;
                }
            }
            34 => {
                self.quote();
                return true;
            }
            115 => {
                if self.keyword("style") {
                    self.emit_flat(STYLE, p + 5);
                    return true;
                }
            }
            100 => {
                if self.keyword("default") {
                    self.emit_flat(DEFAULT, p + 7);
                    return true;
                }
            }
            108 => {
                if self.keyword("linkStyle") {
                    self.emit_flat(LINKSTYLE, p + 9);
                    return true;
                }
            }
            105 => {
                if self.keyword("interpolate") {
                    self.emit_flat(INTERPOLATE, p + 11);
                    return true;
                }
            }
            104 => {
                if starts_with_units(&self.u, p, "href") && is_space(self.unit(p + 4)) {
                    self.emit(HREF, p + 5);
                    return true;
                }
            }
            102 => {
                if self.keyword("flow") {
                    self.emit_flat(FLOW, p + 4);
                    return true;
                }
            }
            103 => {
                if self.keyword("global") {
                    let e = self.blanks(p + 6);
                    self.emit_flat(GLOBAL, e);
                    return true;
                }
            }
            101 => {
                if self.keyword("end") {
                    let e = self.blanks(p + 3);
                    self.emit_flat(END_KW, e);
                    return true;
                }
            }
            95 => {
                for (word, len) in [("_self", 5), ("_blank", 6), ("_parent", 7), ("_top", 4)] {
                    if self.keyword(word) {
                        self.emit_flat(LINK_TARGET, p + len);
                        return true;
                    }
                }
            }
            _ => {}
        }

        if self.dir_next != -1 && p >= self.dir_failed {
            if self.dir_next < p as isize {
                self.dir_next = index_of_str(&self.u, p, "direction");
            }
            if self.dir_next != -1 {
                if self.line_end < p as isize {
                    self.line_end =
                        index_of_terminator(&self.u, p).map_or(self.n as isize, |e| e as isize);
                }
                if self.dir_next < self.line_end {
                    if let Some((index, e)) = self.direction_statement(p) {
                        self.emit(DIRECTION_TB + index, e);
                        return true;
                    }
                    self.dir_failed = self.line_end as usize;
                }
            }
        }

        if self.at_next != -1 && c != 34 && !is_space(c) {
            if p >= self.run_end {
                if self.at_next < p as isize {
                    self.at_next = index_of_unit(&self.u, p, 64).unwrap_or(-1);
                }
                let mut q = p;
                while q < self.n {
                    let ch = self.u[q];
                    if ch == 34 || is_space(ch as u32) {
                        break;
                    }
                    q += 1;
                }
                self.run_end = q;
                self.run_at = -1;
                if self.at_next != -1 && (self.at_next as usize) < q {
                    let mut k = q.min(self.n - 1) as isize - 1;
                    while k >= self.at_next {
                        let ku = k as usize;
                        if self.u[ku] == 64 {
                            let after = self.unit(ku + 1);
                            if after != 123 && after != 34 {
                                self.run_at = k;
                                break;
                            }
                        }
                        k -= 1;
                    }
                }
            }
            if self.run_at > p as isize {
                self.emit_flat(LINK_ID, self.run_at as usize + 1);
                return true;
            }
        }

        if (48..=57).contains(&c) {
            let mut q = p + 1;
            while q < self.n && (48..=57).contains(&self.u[q]) {
                q += 1;
            }
            self.emit_flat(NUM, q);
            return true;
        }

        match c {
            35 => {
                self.emit_flat(BRKT, p + 1);
                return true;
            }
            58 => {
                if self.unit(p + 1) == 58 && self.unit(p + 2) == 58 {
                    self.emit_flat(STYLE_SEPARATOR, p + 3);
                } else {
                    self.emit_flat(COLON, p + 1);
                }
                return true;
            }
            38 => {
                self.emit_flat(AMP, p + 1);
                return true;
            }
            59 => {
                self.emit_flat(SEMI, p + 1);
                return true;
            }
            44 => {
                self.emit_flat(COMMA, p + 1);
                return true;
            }
            42 => {
                self.emit_flat(MULT, p + 1);
                return true;
            }
            60 => {
                self.emit_flat(TAGSTART, p + 1);
                return true;
            }
            45 => {
                if self.link() || self.node_string() {
                    return true;
                }
                self.emit_flat(MINUS, p + 1);
                return true;
            }
            40 => return self.opener(c),
            91 => {
                if self.unit(p + 1) == 124 {
                    self.emit_flat(VERTEX_WITH_PROPS_START, p + 2);
                    return true;
                }
                return self.opener(c);
            }
            124 | 123 => return self.opener(c),
            62 => {
                self.push(State::Text);
                self.emit_flat(TAGEND, p + 1);
                return true;
            }
            94 => {
                self.emit_flat(UP, p + 1);
                return true;
            }
            92 => {
                if self.unit(p + 1) == 124 {
                    self.emit_flat(SEP, p + 2);
                    return true;
                }
                return self.node_string();
            }
            118 => {
                if !is_word_unit(self.unit(p + 1)) {
                    self.emit_flat(DOWN, p + 1);
                    return true;
                }
                return self.node_string();
            }
            37 => {
                if let Some(e) = self.comment_end(p) {
                    self.emit(COMMENT, e);
                    return true;
                }
                return self.node_string();
            }
            _ => {}
        }

        if is_space(c) {
            if self.link() {
                return true;
            }
            if c == 32 || c == 9 {
                let blank = self.blank_end(p);
                if let Some(e) = self.comment_end(blank) {
                    self.emit(COMMENT, e);
                    return true;
                }
            }
            let mut e = p;
            loop {
                let ch = self.unit(e);
                if ch == 10 {
                    e += 1;
                } else if ch == 13 && self.unit(e + 1) == 10 {
                    e += 2;
                } else {
                    break;
                }
            }
            if e > p {
                self.emit(NEWLINE, e);
            } else {
                self.emit(SPACE, p + 1);
            }
            return true;
        }
        if c < 128 {
            return self.node_string();
        }
        if is_unicode_letter(c) {
            self.emit_flat(UNICODE_TEXT, p + 1);
            return true;
        }
        false
    }

    fn direction_statement(&self, p: usize) -> Option<(usize, usize)> {
        let line_end = self.line_end as usize;
        for (index, (word, _)) in DIRECTIONS.iter().enumerate() {
            let mut d = line_end.min(self.n);
            while d > p {
                d -= 1;
                if !starts_with_units(&self.u, d, "direction") {
                    continue;
                }
                let k = self.skip_space(d + 9);
                if k == d + 9 || !starts_with_units(&self.u, k, word) {
                    continue;
                }
                let end = index_of_unit(&self.u, k + 2, 10).map_or(self.n, |e| e as usize);
                return Some((index, end));
            }
        }
        None
    }

    fn step(&mut self, c: u32) -> bool {
        let p = self.p;
        let d = self.unit(p + 1);
        match self.state {
            State::Initial => return self.initial(c),
            State::String => {
                if c == 34 {
                    if self.unit(p + 1) == 96 {
                        self.push(State::MdString);
                        self.advance(p + 2);
                    } else {
                        self.pop();
                        self.advance(p + 1);
                    }
                } else {
                    let e = index_of_unit(&self.u, p, 34).map_or(self.n, |e| e as usize);
                    self.emit(STR, e);
                }
            }
            State::MdString => {
                if c == 96 {
                    if self.unit(p + 1) == 34 {
                        self.pop();
                        self.advance(p + 2);
                    } else {
                        return false;
                    }
                } else if c == 34 {
                    self.quote();
                } else {
                    let mut q = p + 1;
                    while q < self.n && self.u[q] != 96 && self.u[q] != 34 {
                        q += 1;
                    }
                    self.emit(MD_STR, q);
                }
            }
            State::AccTitle | State::AccDescr => {
                let e = index_of_unit(&self.u, p, 10).map_or(self.n, |e| e as usize);
                let kind = if self.state == State::AccTitle {
                    ACC_TITLE_VALUE
                } else {
                    ACC_DESCR_VALUE
                };
                self.pop();
                self.emit(kind, e);
            }
            State::AccDescrMultiline => {
                if c == 125 {
                    self.pop();
                    self.advance(p + 1);
                } else {
                    let e = index_of_unit(&self.u, p, 125).map_or(self.n, |e| e as usize);
                    self.emit(ACC_DESCR_MULTILINE_VALUE, e);
                }
            }
            State::Dir => {
                if c == 34 {
                    self.quote();
                } else if let Some(e) = self.nodir_end(p) {
                    self.pop();
                    self.emit(NODIR, e);
                } else if let Some(e) = self.dir_end(p) {
                    self.pop();
                    self.emit(DIR, e);
                } else {
                    return self.opener(c);
                }
            }
            State::Text => match c {
                34 => self.quote(),
                40 | 91 | 123 => {
                    self.opener(c);
                }
                93 => {
                    self.pop();
                    if d == 41 {
                        self.emit_flat(STADIUMEND, p + 2);
                    } else if d == 93 {
                        self.emit_flat(SUBROUTINEEND, p + 2);
                    } else {
                        self.emit_flat(SQE, p + 1);
                    }
                }
                41 => {
                    self.pop();
                    if d == 93 {
                        self.emit_flat(CYLINDEREND, p + 2);
                    } else if d == 41 && self.unit(p + 2) == 41 {
                        self.emit_flat(DOUBLECIRCLEEND, p + 3);
                    } else {
                        self.emit_flat(PE, p + 1);
                    }
                }
                124 => {
                    self.pop();
                    self.emit_flat(PIPE, p + 1);
                }
                125 => {
                    self.pop();
                    self.emit_flat(DIAMOND_STOP, p + 1);
                }
                37 => match self.comment_end(p) {
                    Some(e) => self.advance(e),
                    None => return false,
                },
                _ => {
                    let mut q = p + 1;
                    while q < self.n
                        && !matches!(self.u[q], 91 | 93 | 40 | 41 | 123 | 125 | 124 | 34 | 37)
                    {
                        q += 1;
                    }
                    self.emit(TEXT, q);
                }
            },
            State::EllipseText => {
                if c == 34 {
                    self.quote();
                } else if c == 37 && self.comment_end(p).is_some() {
                    let e = self.comment_end(p).unwrap_or(self.n);
                    self.advance(e);
                } else if matches!(c, 45 | 47 | 41) && d == 41 {
                    self.pop();
                    self.emit_flat(ELLIPSE_END, p + 2);
                } else if matches!(c, 40 | 91 | 123) {
                    self.opener(c);
                } else if matches!(c, 41 | 93 | 125) {
                    return false;
                } else {
                    let mut q = p + 1;
                    while q < self.n {
                        let ch = self.u[q];
                        if matches!(ch, 34 | 40 | 41 | 91 | 93 | 123 | 125) {
                            break;
                        }
                        if matches!(ch, 45 | 47) && self.unit(q + 1) == 41 {
                            break;
                        }
                        if ch == 37 && self.unit(q + 1) == 37 && self.unit(q + 2) != 123 {
                            break;
                        }
                        q += 1;
                    }
                    self.emit_chars(TEXT, q);
                }
            }
            State::TrapText => {
                if c == 34 {
                    self.quote();
                    return true;
                }
                match c {
                    40 => {
                        if d == 45 || d == 91 || (d == 40 && self.unit(p + 2) == 40) {
                            self.opener(c);
                        } else if d == 93 {
                            self.pop();
                            self.emit_flat(TRAPEND, p + 2);
                        } else {
                            self.opener(c);
                        }
                    }
                    91 | 123 => {
                        self.opener(c);
                    }
                    92 => {
                        if d == 93 {
                            self.pop();
                            self.emit_flat(TRAPEND, p + 2);
                        } else {
                            self.emit_flat(TEXT, p + 1);
                        }
                    }
                    47 => {
                        if d == 93 {
                            self.pop();
                            self.emit_flat(INVTRAPEND, p + 2);
                        } else {
                            self.emit_flat(TEXT, p + 1);
                        }
                    }
                    93 | 41 => {
                        if d == 93 {
                            self.pop();
                            self.emit_flat(TRAPEND, p + 2);
                        } else {
                            return false;
                        }
                    }
                    125 => return false,
                    37 => match self.comment_end(p) {
                        Some(e) => self.advance(e),
                        None => return false,
                    },
                    _ => {
                        if matches!(c, 63 | 61) && d == 93 {
                            self.pop();
                            self.emit_flat(TRAPEND, p + 2);
                            return true;
                        }
                        let mut q = p + 1;
                        while q < self.n
                            && !matches!(self.u[q], 92 | 91 | 93 | 40 | 41 | 123 | 125 | 47 | 37)
                        {
                            q += 1;
                        }
                        self.emit(TEXT, q);
                    }
                }
            }
            State::EdgeText => {
                if c == 34 {
                    self.quote();
                    return true;
                }
                if let Some(e) = self.arrow_end(p) {
                    self.pop();
                    self.emit(LINK, e);
                    return true;
                }
                if c == 45 && d == 45 {
                    return false;
                }
                let mut q = p + 1;
                while q < self.n {
                    let ch = self.u[q];
                    if ch == 34 {
                        break;
                    }
                    if ch == 45 {
                        if self.unit(q + 1) == 45 {
                            break;
                        }
                    } else if is_space(ch as u32) && self.arrow_end(q).is_some() {
                        break;
                    }
                    q += 1;
                }
                self.emit_chars(EDGE_TEXT, q);
            }
            State::Click => {
                if c == 34 {
                    self.quote();
                } else if is_space(c) {
                    self.pop();
                    self.advance(p + 1);
                } else {
                    let mut q = p + 1;
                    while q < self.n && !is_space(self.u[q] as u32) {
                        q += 1;
                    }
                    self.emit_flat(CLICK, q);
                }
            }
            State::CallbackName => {
                if c == 40 {
                    let e = self.empty_call_end(p);
                    self.pop();
                    match e {
                        Some(e) => self.advance(e),
                        None => {
                            self.push(State::CallbackArgs);
                            self.advance(p + 1);
                        }
                    }
                } else {
                    let e = index_of_unit(&self.u, p, 40).map_or(self.n, |e| e as usize);
                    self.emit(CALLBACKNAME, e);
                }
            }
            State::CallbackArgs => {
                if c == 41 {
                    self.pop();
                    self.advance(p + 1);
                } else {
                    let e = index_of_unit(&self.u, p, 41).map_or(self.n, |e| e as usize);
                    self.emit(CALLBACKARGS, e);
                }
            }
            State::ShapeData => {
                if c == 34 {
                    self.push(State::ShapeDataStr);
                    self.emit_flat(SHAPE_DATA, p + 1);
                } else if c == 125 {
                    self.pop();
                    self.advance(p + 1);
                } else if c == 94 {
                    return false;
                } else {
                    let mut q = p + 1;
                    while q < self.n && !matches!(self.u[q], 125 | 94 | 34) {
                        q += 1;
                    }
                    self.emit(SHAPE_DATA, q);
                }
            }
            State::ShapeDataStr => {
                if c == 34 {
                    self.pop();
                    self.emit_flat(SHAPE_DATA, p + 1);
                } else {
                    let e = index_of_unit(&self.u, p, 34).map_or(self.n, |e| e as usize);
                    let text = line_breaks(&self.slice(p, e));
                    self.emit_text(SHAPE_DATA, text, e);
                }
            }
        }
        true
    }

    fn nodir_end(&self, p: usize) -> Option<usize> {
        let w_end = self.skip_space(p);
        let mut last = None;
        for (k, &ch) in self.u[p..w_end].iter().enumerate() {
            if ch == 10 {
                last = Some(p + k + 1);
            }
        }
        last
    }

    fn dir_end(&self, p: usize) -> Option<usize> {
        let j = self.skip_space(p);
        for word in DIRECTION_WORDS {
            if starts_with_units(&self.u, j, word) && !is_word_unit(self.unit(j + 2)) {
                return Some(j + 2);
            }
        }
        if matches!(self.unit(j), 60 | 62 | 94) {
            return Some(j + 1);
        }
        if self.unit(j) == 118 && !is_word_unit(self.unit(j + 1)) {
            return Some(j + 1);
        }
        None
    }

    fn empty_call_end(&self, p: usize) -> Option<usize> {
        if self.unit(p) != 40 {
            return None;
        }
        let j = self.skip_space(p + 1);
        if self.unit(j) == 41 {
            Some(j + 1)
        } else {
            None
        }
    }

    fn run(mut self) -> Tokens {
        loop {
            if self.p >= self.n {
                let kind = match self.state {
                    State::Initial => Some(EOF),
                    State::AccTitle => Some(ACC_TITLE_VALUE),
                    State::AccDescr => Some(ACC_DESCR_VALUE),
                    State::AccDescrMultiline => Some(ACC_DESCR_MULTILINE_VALUE),
                    State::Click => Some(CLICK),
                    State::CallbackName => Some(CALLBACKNAME),
                    State::CallbackArgs => Some(CALLBACKARGS),
                    _ => None,
                };
                if let Some(kind) = kind {
                    let n = self.n;
                    self.emit_flat(kind, n);
                }
                break;
            }
            let c = self.unit(self.p);
            if !self.step(c) {
                let start = self.p;
                self.push_token(ERROR, String::new(), start);
                self.close_token();
                return self.finish();
            }
        }
        let start = self.n;
        self.push_token(END, String::new(), start);
        self.close_token();
        self.finish()
    }

    fn finish(self) -> Tokens {
        Tokens {
            types: self.types,
            texts: self.texts,
            starts: self.starts,
            locs: self.locs,
        }
    }
}

fn starts_with_units(u: &[u16], at: usize, word: &str) -> bool {
    for (offset, unit) in ascii_units(word).enumerate() {
        if u.get(at + offset) != Some(&unit) {
            return false;
        }
    }
    true
}

fn index_of_unit(u: &[u16], from: usize, ch: u16) -> Option<isize> {
    (from..u.len()).find(|&k| u[k] == ch).map(|k| k as isize)
}

fn index_of_str(u: &[u16], from: usize, word: &str) -> isize {
    (from..u.len())
        .find(|&k| starts_with_units(u, k, word))
        .map_or(-1, |k| k as isize)
}

fn index_of_terminator(u: &[u16], from: usize) -> Option<usize> {
    (from..u.len()).find(|&k| is_terminator(u[k]))
}

fn line_breaks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\n' {
            out.push_str("<br/>");
            while chars.peek().is_some_and(|&next| is_js_space(next)) {
                chars.next();
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn tokenize(src: &str) -> Tokens {
    Lexer::new(src).run()
}
