use std::sync::OnceLock;

use regex::Regex;

use super::tokens::{DIRECTIONS, T};
use crate::util::unicode::is_unicode_letter;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tokens {
    pub types: Vec<T>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum S {
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
    ThickEdgeText,
    DottedEdgeText,
    Click,
    CallbackName,
    CallbackArgs,
    ShapeData,
    ShapeDataStr,
}

struct Res {
    acc_title: Regex,
    acc_descr: Regex,
    acc_descr_ml: Regex,
    call: Regex,
    click: Regex,
    nodir: Regex,
    dir: Regex,
    dir_sym: Regex,
    dir_v: Regex,
    direction: [Regex; 5],
    link: Regex,
    start_link: Regex,
    thick_link: Regex,
    thick_start: Regex,
    dotted_link: Regex,
    dotted_start: Regex,
    invis_link: Regex,
    newline: Regex,
    space: Regex,
    empty_call: Regex,
    line_end: Regex,
    br: Regex,
}

fn anchored(pattern: &str) -> Regex {
    Regex::new(&format!("^(?:{pattern})")).expect("flowchart lexer pattern")
}

fn res() -> &'static Res {
    static RES: OnceLock<Res> = OnceLock::new();
    RES.get_or_init(|| Res {
        acc_title: anchored(r"accTitle\s*:\s*"),
        acc_descr: anchored(r"accDescr\s*:\s*"),
        acc_descr_ml: anchored(r"accDescr\s*\{\s*"),
        call: anchored(r"call\s+"),
        click: anchored(r"click\s+"),
        nodir: anchored(r"(?:\r?\n)*\s*\n"),
        dir: anchored(r"\s*(?:LR|RL|TB|BT|TD|BR)(?-u:\b)"),
        dir_sym: anchored(r"\s*[<>^]"),
        dir_v: anchored(r"\s*v(?-u:\b)"),
        direction: [
            anchored(r"[^\n\r\x{2028}\x{2029}]*direction\s+TB[^\n]*"),
            anchored(r"[^\n\r\x{2028}\x{2029}]*direction\s+BT[^\n]*"),
            anchored(r"[^\n\r\x{2028}\x{2029}]*direction\s+RL[^\n]*"),
            anchored(r"[^\n\r\x{2028}\x{2029}]*direction\s+LR[^\n]*"),
            anchored(r"[^\n\r\x{2028}\x{2029}]*direction\s+TD[^\n]*"),
        ],
        link: anchored(r"\s*[xo<]?--+[-xo>]\s*"),
        start_link: anchored(r"\s*[xo<]?--\s*"),
        thick_link: anchored(r"\s*[xo<]?==+[=xo>]\s*"),
        thick_start: anchored(r"\s*[xo<]?==\s*"),
        dotted_link: anchored(r"\s*[xo<]?-?\.+-[xo>]?\s*"),
        dotted_start: anchored(r"\s*[xo<]?-\.\s*"),
        invis_link: anchored(r"\s*~~~+\s*"),
        newline: anchored(r"(?:\r?\n)+"),
        space: anchored(r"[^\S\n\r]+"),
        empty_call: anchored(r"\(\s*\)"),
        line_end: Regex::new(r"[\n\r\x{2028}\x{2029}]").expect("line end"),
        br: Regex::new(r"\n\s*").expect("line break"),
    })
}

const NODE_CHAR: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!\"#$%&'*+.`?\\_/";

fn is_space(c: u32) -> bool {
    if c < 128 {
        return c == 32 || (9..=13).contains(&c);
    }
    matches!(
        c,
        0xa0 | 0x1680 | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
    ) || (0x2000..=0x200a).contains(&c)
}

fn is_word(c: u32) -> bool {
    (48..=57).contains(&c) || (65..=90).contains(&c) || (97..=122).contains(&c) || c == 95
}

const NAN: u32 = u32::MAX;

struct Lx {
    src: Vec<u16>,
    n: usize,
    text: String,
    u2b: Vec<usize>,
    b2u: Vec<usize>,
    out: Tokens,
    stack: Vec<S>,
    state: S,
    p: usize,
    first_graph: bool,
    dir_next: isize,
    at_next: isize,
    line_end: isize,
    dir_failed: isize,
    run_end: isize,
    run_at: isize,
    ws_from: isize,
    ws_to: isize,
}

impl Lx {
    fn cc(&self, i: usize) -> u32 {
        if i < self.n { self.src[i] as u32 } else { NAN }
    }

    fn slice(&self, s: usize, e: usize) -> String {
        String::from_utf16_lossy(&self.src[s..e])
    }

    fn index_of(&self, pat: &str, from: usize) -> isize {
        let pat: Vec<u16> = pat.encode_utf16().collect();
        if pat.is_empty() {
            return from.min(self.n) as isize;
        }
        let mut i = from;
        while i + pat.len() <= self.n {
            if self.src[i..i + pat.len()] == pat[..] {
                return i as isize;
            }
            i += 1;
        }
        -1
    }

    fn index_of_unit(&self, unit: u16, from: usize) -> isize {
        (from..self.n)
            .find(|&i| self.src[i] == unit)
            .map_or(-1, |i| i as isize)
    }

    fn starts_with(&self, word: &str, at: usize) -> bool {
        for (offset, u) in word.encode_utf16().enumerate() {
            let i = at + offset;
            if i >= self.n || self.src[i] != u {
                return false;
            }
        }
        true
    }

    fn emit(&mut self, t: T, s: usize, e: usize) {
        self.out.types.push(t);
        self.out.texts.push(self.slice(s, e));
        self.out.starts.push(s);
        self.p = e;
    }

    fn push(&mut self, s: S) {
        self.stack.push(self.state);
        self.state = s;
    }

    fn pop(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
        }
    }

    fn at(&self, re: &Regex, q: usize) -> Option<usize> {
        if q > self.n {
            return None;
        }
        let bs = self.u2b[q];
        let rest = &self.text[bs..];
        let m = re.find(rest)?;
        Some(self.b2u[bs + m.end()])
    }

    fn kw(&self, word: &str) -> bool {
        self.starts_with(word, self.p) && !is_word(self.cc(self.p + word.encode_utf16().count()))
    }

    fn link_char(&mut self, q: usize) -> u32 {
        let mut j = q;
        let qi = q as isize;
        if qi >= self.ws_from && qi <= self.ws_to {
            j = self.ws_to as usize;
        } else if is_space(self.cc(q)) {
            while j < self.n && is_space(self.cc(j)) {
                j += 1;
            }
            self.ws_from = qi;
            self.ws_to = j as isize;
        }
        let d = self.cc(j);
        if d == 120 || d == 111 || d == 60 {
            self.cc(j + 1)
        } else {
            d
        }
    }

    fn invis_ahead(&mut self, q: usize) -> bool {
        let mut j = q;
        let qi = q as isize;
        if qi >= self.ws_from && qi <= self.ws_to {
            j = self.ws_to as usize;
        } else if is_space(self.cc(q)) {
            while j < self.n && is_space(self.cc(j)) {
                j += 1;
            }
            self.ws_from = qi;
            self.ws_to = j as isize;
        }
        self.cc(j) == 126
    }

    fn quote(&mut self) {
        if self.cc(self.p + 1) == 96 {
            self.push(S::MdString);
            self.p += 2;
        } else {
            self.push(S::String);
            self.p += 1;
        }
    }

    fn invis_link(&mut self) -> bool {
        if !self.invis_ahead(self.p) {
            return false;
        }
        match self.at(&res().invis_link, self.p) {
            None => false,
            Some(e) => {
                let p = self.p;
                self.emit(T::LINK, p, e);
                true
            }
        }
    }

    fn opener(&mut self, c: u32) -> bool {
        let d = self.cc(self.p + 1);
        let p = self.p;
        if c == 40 {
            if d == 45 {
                self.push(S::EllipseText);
                self.emit(T::ELLIPSE_START, p, p + 2);
            } else if d == 91 {
                self.push(S::Text);
                self.emit(T::STADIUMSTART, p, p + 2);
            } else if d == 40 && self.cc(p + 2) == 40 {
                self.push(S::Text);
                self.emit(T::DOUBLECIRCLESTART, p, p + 3);
            } else {
                self.push(S::Text);
                self.emit(T::PS, p, p + 1);
            }
            return true;
        }
        if c == 91 {
            if d == 91 {
                self.push(S::Text);
                self.emit(T::SUBROUTINESTART, p, p + 2);
            } else if d == 40 {
                self.push(S::Text);
                self.emit(T::CYLINDERSTART, p, p + 2);
            } else if d == 47 {
                self.push(S::TrapText);
                self.emit(T::TRAPSTART, p, p + 2);
            } else if d == 92 {
                self.push(S::TrapText);
                self.emit(T::INVTRAPSTART, p, p + 2);
            } else {
                self.push(S::Text);
                self.emit(T::SQS, p, p + 1);
            }
            return true;
        }
        if c == 124 {
            self.push(S::Text);
            self.emit(T::PIPE, p, p + 1);
            return true;
        }
        if c == 123 {
            self.push(S::Text);
            self.emit(T::DIAMOND_START, p, p + 1);
            return true;
        }
        false
    }

    fn graph(&mut self, len: usize) {
        if self.first_graph {
            self.first_graph = false;
            self.push(S::Dir);
        }
        let p = self.p;
        self.emit(T::GRAPH, p, p + len);
    }

    fn node_string(&mut self) -> bool {
        let mut q = self.p;
        while q < self.n {
            let ch = self.cc(q);
            if ch < 128 && NODE_CHAR.contains(&(ch as u8)) {
                q += 1;
            } else if ch == 45 && q + 1 < self.n {
                let d = self.cc(q + 1);
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
        let p = self.p;
        self.emit(T::NODE_STRING, p, q);
        true
    }

    fn link(&mut self, c: u32) -> bool {
        let d = self.link_char(self.p);
        let p = self.p;
        if d == 45 {
            if let Some(e) = self.at(&res().link, p) {
                self.emit(T::LINK, p, e);
                return true;
            }
            if let Some(e) = self.at(&res().start_link, p) {
                self.push(S::EdgeText);
                self.emit(T::START_LINK, p, e);
                return true;
            }
            if let Some(e) = self.at(&res().dotted_link, p) {
                self.emit(T::LINK, p, e);
                return true;
            }
            if let Some(e) = self.at(&res().dotted_start, p) {
                self.push(S::DottedEdgeText);
                self.emit(T::START_LINK, p, e);
                return true;
            }
        } else if d == 61 {
            if let Some(e) = self.at(&res().thick_link, p) {
                self.emit(T::LINK, p, e);
                return true;
            }
            if let Some(e) = self.at(&res().thick_start, p) {
                self.push(S::ThickEdgeText);
                self.emit(T::START_LINK, p, e);
                return true;
            }
        } else if d == 46
            && let Some(e) = self.at(&res().dotted_link, p)
        {
            self.emit(T::LINK, p, e);
            return true;
        }
        c != 45 && c != 61 && c != 46 && self.invis_link()
    }

    fn initial(&mut self, c: u32) -> bool {
        let p = self.p;
        match c {
            97 => {
                if self.starts_with("acc", p) {
                    if let Some(e) = self.at(&res().acc_title, p) {
                        self.push(S::AccTitle);
                        self.emit(T::acc_title, p, e);
                        return true;
                    }
                    if let Some(e) = self.at(&res().acc_descr, p) {
                        self.push(S::AccDescr);
                        self.emit(T::acc_descr, p, e);
                        return true;
                    }
                    if let Some(e) = self.at(&res().acc_descr_ml, p) {
                        self.push(S::AccDescrMultiline);
                        self.p = e;
                        return true;
                    }
                }
            }
            64 => {
                if self.cc(p + 1) == 123 {
                    self.push(S::ShapeData);
                    self.out.types.push(T::SHAPE_DATA);
                    self.out.texts.push(String::new());
                    self.out.starts.push(p);
                    self.p += 2;
                    return true;
                }
            }
            99 => {
                if let Some(e) = self.at(&res().call, p) {
                    self.push(S::CallbackName);
                    self.p = e;
                    return true;
                }
                if self.kw("classDef") {
                    self.emit(T::CLASSDEF, p, p + 8);
                    return true;
                }
                if self.kw("class") {
                    self.emit(T::CLASS, p, p + 5);
                    return true;
                }
                if let Some(e) = self.at(&res().click, p) {
                    self.push(S::Click);
                    self.p = e;
                    return true;
                }
            }
            34 => {
                self.quote();
                return true;
            }
            115 => {
                if self.kw("style") {
                    self.emit(T::STYLE, p, p + 5);
                    return true;
                }
                if self.kw("swimlane-beta") {
                    self.graph(13);
                    return true;
                }
                if self.kw("subgraph") {
                    self.emit(T::subgraph, p, p + 8);
                    return true;
                }
            }
            100 => {
                if self.kw("default") {
                    self.emit(T::DEFAULT, p, p + 7);
                    return true;
                }
            }
            108 => {
                if self.kw("linkStyle") {
                    self.emit(T::LINKSTYLE, p, p + 9);
                    return true;
                }
            }
            105 => {
                if self.kw("interpolate") {
                    self.emit(T::INTERPOLATE, p, p + 11);
                    return true;
                }
            }
            104 => {
                if self.starts_with("href", p) && is_space(self.cc(p + 4)) {
                    self.emit(T::HREF, p, p + 5);
                    return true;
                }
            }
            102 => {
                if self.kw("flowchart-elk") {
                    self.graph(13);
                    return true;
                }
                if self.kw("flowchart") {
                    self.graph(9);
                    return true;
                }
            }
            103 => {
                if self.kw("graph") {
                    self.graph(5);
                    return true;
                }
            }
            101 => {
                if self.kw("end") {
                    let mut e = p + 3;
                    while e < self.n && is_space(self.cc(e)) {
                        e += 1;
                    }
                    self.emit(T::end, p, e);
                    return true;
                }
            }
            95 => {
                if self.kw("_self") {
                    self.emit(T::LINK_TARGET, p, p + 5);
                    return true;
                }
                if self.kw("_blank") {
                    self.emit(T::LINK_TARGET, p, p + 6);
                    return true;
                }
                if self.kw("_parent") {
                    self.emit(T::LINK_TARGET, p, p + 7);
                    return true;
                }
                if self.kw("_top") {
                    self.emit(T::LINK_TARGET, p, p + 4);
                    return true;
                }
            }
            _ => {}
        }

        if self.dir_next != -1 && p as isize >= self.dir_failed {
            if self.dir_next < p as isize {
                self.dir_next = self.index_of("direction", p);
            }
            if self.dir_next != -1 {
                if self.line_end < p as isize {
                    let rest = &self.text[self.u2b[p]..];
                    self.line_end = match res().line_end.find(rest) {
                        Some(m) => self.b2u[self.u2b[p] + m.start()] as isize,
                        None => self.n as isize,
                    };
                }
                if self.dir_next < self.line_end {
                    for (i, re) in res().direction.iter().enumerate() {
                        if let Some(e) = self.at(re, p) {
                            self.emit(DIRECTIONS[i], p, e);
                            return true;
                        }
                    }
                    self.dir_failed = self.line_end;
                }
            }
        }

        if self.at_next != -1 && c != 34 && !is_space(c) {
            if p as isize >= self.run_end {
                if self.at_next < p as isize {
                    self.at_next = self.index_of("@", p);
                }
                let mut q = p;
                while q < self.n {
                    let ch = self.cc(q);
                    if ch == 34 || is_space(ch) {
                        break;
                    }
                    q += 1;
                }
                self.run_end = q as isize;
                self.run_at = -1;
                if self.at_next != -1 && self.at_next < q as isize {
                    let mut k = (q.min(self.n.saturating_sub(1)) as isize) - 1;
                    while k >= self.at_next {
                        if self.src[k as usize] == 64 {
                            let after = self.cc(k as usize + 1);
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
                let end = self.run_at as usize + 1;
                self.emit(T::LINK_ID, p, end);
                return true;
            }
        }

        if (48..=57).contains(&c) {
            let mut q = p + 1;
            while q < self.n {
                let ch = self.cc(q);
                if !(48..=57).contains(&ch) {
                    break;
                }
                q += 1;
            }
            self.emit(T::NUM, p, q);
            return true;
        }

        match c {
            35 => {
                self.emit(T::BRKT, p, p + 1);
                return true;
            }
            58 => {
                if self.cc(p + 1) == 58 && self.cc(p + 2) == 58 {
                    self.emit(T::STYLE_SEPARATOR, p, p + 3);
                } else {
                    self.emit(T::COLON, p, p + 1);
                }
                return true;
            }
            38 => {
                self.emit(T::AMP, p, p + 1);
                return true;
            }
            59 => {
                self.emit(T::SEMI, p, p + 1);
                return true;
            }
            44 => {
                self.emit(T::COMMA, p, p + 1);
                return true;
            }
            42 => {
                self.emit(T::MULT, p, p + 1);
                return true;
            }
            120 | 111 => return self.link(c) || self.node_string(),
            60 => {
                if self.link(c) {
                    return true;
                }
                self.emit(T::TAGSTART, p, p + 1);
                return true;
            }
            45 => {
                if self.link(c) || self.node_string() {
                    return true;
                }
                self.emit(T::MINUS, p, p + 1);
                return true;
            }
            61 => return self.link(c),
            46 => return self.link(c) || self.node_string(),
            126 => return self.invis_link(),
            40 => return self.opener(c),
            91 => {
                if self.cc(p + 1) == 124 {
                    self.emit(T::VERTEX_WITH_PROPS_START, p, p + 2);
                    return true;
                }
                return self.opener(c);
            }
            124 | 123 => return self.opener(c),
            62 => {
                self.push(S::Text);
                self.emit(T::TAGEND, p, p + 1);
                return true;
            }
            94 => {
                self.emit(T::UP, p, p + 1);
                return true;
            }
            92 => {
                if self.cc(p + 1) == 124 {
                    self.emit(T::SEP, p, p + 2);
                    return true;
                }
                return self.node_string();
            }
            118 => {
                if !is_word(self.cc(p + 1)) {
                    self.emit(T::DOWN, p, p + 1);
                    return true;
                }
                return self.node_string();
            }
            _ => {}
        }

        if is_space(c) {
            if self.link(c) {
                return true;
            }
            if let Some(e) = self.at(&res().newline, p) {
                self.emit(T::NEWLINE, p, e);
                return true;
            }
            if let Some(e) = self.at(&res().space, p) {
                self.emit(T::SPACE, p, e);
                return true;
            }
            return false;
        }

        if c < 128 {
            return self.node_string();
        }
        if is_unicode_letter(c) {
            self.emit(T::UNICODE_TEXT, p, p + 1);
            return true;
        }
        false
    }

    fn edge_text(&mut self, c: u32, re: &Regex, a: i64, b: i64) -> bool {
        if c == 34 {
            self.quote();
            return true;
        }
        let p = self.p;
        let d = self.link_char(p) as i64;
        if (d == a || d == b)
            && let Some(e) = self.at(re, p)
        {
            self.pop();
            self.emit(T::LINK, p, e);
            return true;
        }
        if c as i64 == a && (a != 45 || self.cc(p + 1) == 45) {
            return false;
        }
        let mut q = p + 1;
        while q < self.n {
            let ch = self.cc(q);
            if ch == 34 {
                break;
            }
            if ch as i64 == a {
                if a != 45 || self.cc(q + 1) == 45 {
                    break;
                }
            } else if ch == 120 || ch == 111 || ch == 60 || ch as i64 == b || is_space(ch) {
                let dd = self.link_char(q) as i64;
                if (dd == a || dd == b) && self.at(re, q).is_some() {
                    break;
                }
            }
            q += 1;
        }
        self.emit(T::EDGE_TEXT, p, q);
        true
    }

    fn lex(&mut self) {
        loop {
            let p = self.p;
            if p >= self.n {
                let ty = match self.state {
                    S::Initial => Some(T::EOF),
                    S::AccTitle => Some(T::acc_title_value),
                    S::AccDescr => Some(T::acc_descr_value),
                    S::AccDescrMultiline => Some(T::acc_descr_multiline_value),
                    S::Click => Some(T::CLICK),
                    S::CallbackName => Some(T::CALLBACKNAME),
                    S::CallbackArgs => Some(T::CALLBACKARGS),
                    _ => None,
                };
                if let Some(t) = ty {
                    let n = self.n;
                    self.emit(t, n, n);
                }
                break;
            }
            let c = self.cc(p);
            let mut ok = true;
            match self.state {
                S::Initial => ok = self.initial(c),
                S::String => {
                    if c == 34 {
                        if self.cc(p + 1) == 96 {
                            self.push(S::MdString);
                            self.p += 2;
                        } else {
                            self.pop();
                            self.p += 1;
                        }
                    } else {
                        let e = self.index_of_unit(34, p);
                        let e = if e == -1 { self.n } else { e as usize };
                        self.emit(T::STR, p, e);
                    }
                }
                S::MdString => {
                    if c == 96 {
                        if self.cc(p + 1) == 34 {
                            self.pop();
                            self.p += 2;
                        } else {
                            ok = false;
                        }
                    } else if c == 34 {
                        self.quote();
                    } else {
                        let mut q = p + 1;
                        while q < self.n {
                            let ch = self.cc(q);
                            if ch == 96 || ch == 34 {
                                break;
                            }
                            q += 1;
                        }
                        self.emit(T::MD_STR, p, q);
                    }
                }
                S::AccTitle | S::AccDescr => {
                    let mut e = self.index_of_unit(10, p);
                    if e == -1 {
                        e = self.n as isize;
                    }
                    let t = if self.state == S::AccTitle {
                        T::acc_title_value
                    } else {
                        T::acc_descr_value
                    };
                    self.pop();
                    self.emit(t, p, e as usize);
                }
                S::AccDescrMultiline => {
                    if c == 125 {
                        self.pop();
                        self.p += 1;
                    } else {
                        let mut e = self.index_of_unit(125, p);
                        if e == -1 {
                            e = self.n as isize;
                        }
                        self.emit(T::acc_descr_multiline_value, p, e as usize);
                    }
                }
                S::Dir => {
                    if c == 34 {
                        self.quote();
                    } else if let Some(e) = self.at(&res().nodir, p) {
                        self.pop();
                        self.emit(T::NODIR, p, e);
                    } else if let Some(e) = self
                        .at(&res().dir, p)
                        .or_else(|| self.at(&res().dir_sym, p))
                        .or_else(|| self.at(&res().dir_v, p))
                    {
                        self.pop();
                        self.emit(T::DIR, p, e);
                    } else {
                        ok = self.invis_link() || self.opener(c);
                    }
                }
                S::Text => {
                    let d = self.cc(p + 1);
                    match c {
                        34 => self.quote(),
                        40 | 91 | 123 => {
                            self.opener(c);
                        }
                        93 => {
                            self.pop();
                            if d == 41 {
                                self.emit(T::STADIUMEND, p, p + 2);
                            } else if d == 93 {
                                self.emit(T::SUBROUTINEEND, p, p + 2);
                            } else {
                                self.emit(T::SQE, p, p + 1);
                            }
                        }
                        41 => {
                            self.pop();
                            if d == 93 {
                                self.emit(T::CYLINDEREND, p, p + 2);
                            } else if d == 41 && self.cc(p + 2) == 41 {
                                self.emit(T::DOUBLECIRCLEEND, p, p + 3);
                            } else {
                                self.emit(T::PE, p, p + 1);
                            }
                        }
                        124 => {
                            self.pop();
                            self.emit(T::PIPE, p, p + 1);
                        }
                        125 => {
                            self.pop();
                            self.emit(T::DIAMOND_STOP, p, p + 1);
                        }
                        _ => {
                            if (c == 126 || is_space(c)) && self.invis_link() {
                                continue;
                            }
                            let mut q = p + 1;
                            while q < self.n {
                                let ch = self.cc(q);
                                if matches!(ch, 91 | 93 | 40 | 41 | 123 | 125 | 124 | 34) {
                                    break;
                                }
                                q += 1;
                            }
                            self.emit(T::TEXT, p, q);
                        }
                    }
                }
                S::EllipseText => {
                    let d = self.cc(p + 1);
                    if c == 34 {
                        self.quote();
                    } else if (c == 126 || is_space(c)) && self.invis_link() {
                    } else if (c == 45 || c == 47 || c == 41) && d == 41 {
                        self.pop();
                        self.emit(T::ELLIPSE_END, p, p + 2);
                    } else if c == 40 || c == 91 || c == 123 {
                        self.opener(c);
                    } else if c == 41 || c == 93 || c == 125 {
                        ok = false;
                    } else {
                        let mut q = p + 1;
                        while q < self.n {
                            let ch = self.cc(q);
                            if matches!(ch, 34 | 40 | 41 | 91 | 93 | 123 | 125) {
                                break;
                            }
                            if (ch == 45 || ch == 47) && self.cc(q + 1) == 41 {
                                break;
                            }
                            if (ch == 126 || is_space(ch))
                                && self.invis_ahead(q)
                                && self.at(&res().invis_link, q).is_some()
                            {
                                break;
                            }
                            q += 1;
                        }
                        self.emit(T::TEXT, p, q);
                    }
                }
                S::TrapText => {
                    let d = self.cc(p + 1);
                    if c == 34 {
                        self.quote();
                    } else if (c == 126 || is_space(c)) && self.invis_link() {
                    } else {
                        match c {
                            40 => {
                                if d == 45 || d == 91 || (d == 40 && self.cc(p + 2) == 40) {
                                    self.opener(c);
                                } else if d == 93 {
                                    self.pop();
                                    self.emit(T::TRAPEND, p, p + 2);
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
                                    self.emit(T::TRAPEND, p, p + 2);
                                } else {
                                    self.emit(T::TEXT, p, p + 1);
                                }
                            }
                            47 => {
                                if d == 93 {
                                    self.pop();
                                    self.emit(T::INVTRAPEND, p, p + 2);
                                } else {
                                    self.emit(T::TEXT, p, p + 1);
                                }
                            }
                            93 | 41 => {
                                if d == 93 {
                                    self.pop();
                                    self.emit(T::TRAPEND, p, p + 2);
                                } else {
                                    ok = false;
                                }
                            }
                            125 => ok = false,
                            _ => {
                                if (c == 63 || c == 61) && d == 93 {
                                    self.pop();
                                    self.emit(T::TRAPEND, p, p + 2);
                                } else {
                                    let mut q = p + 1;
                                    while q < self.n {
                                        let ch = self.cc(q);
                                        if matches!(ch, 92 | 91 | 93 | 40 | 41 | 123 | 125 | 47) {
                                            break;
                                        }
                                        q += 1;
                                    }
                                    self.emit(T::TEXT, p, q);
                                }
                            }
                        }
                    }
                }
                S::EdgeText => ok = self.edge_text(c, &res().link, 45, -1),
                S::ThickEdgeText => ok = self.edge_text(c, &res().thick_link, 61, -1),
                S::DottedEdgeText => ok = self.edge_text(c, &res().dotted_link, 46, 45),
                S::Click => {
                    if c == 34 {
                        self.quote();
                    } else if is_space(c) {
                        self.pop();
                        self.p += 1;
                    } else {
                        let mut q = p + 1;
                        while q < self.n && !is_space(self.cc(q)) {
                            q += 1;
                        }
                        self.emit(T::CLICK, p, q);
                    }
                }
                S::CallbackName => {
                    if c == 40 {
                        let e = self.at(&res().empty_call, p);
                        self.pop();
                        match e {
                            Some(e) => self.p = e,
                            None => {
                                self.push(S::CallbackArgs);
                                self.p += 1;
                            }
                        }
                    } else {
                        let mut e = self.index_of_unit(40, p);
                        if e == -1 {
                            e = self.n as isize;
                        }
                        self.emit(T::CALLBACKNAME, p, e as usize);
                    }
                }
                S::CallbackArgs => {
                    if c == 41 {
                        self.pop();
                        self.p += 1;
                    } else {
                        let mut e = self.index_of_unit(41, p);
                        if e == -1 {
                            e = self.n as isize;
                        }
                        self.emit(T::CALLBACKARGS, p, e as usize);
                    }
                }
                S::ShapeData => {
                    if c == 34 {
                        self.push(S::ShapeDataStr);
                        self.emit(T::SHAPE_DATA, p, p + 1);
                    } else if c == 125 {
                        self.pop();
                        self.p += 1;
                    } else if c == 94 {
                        ok = false;
                    } else {
                        let mut q = p + 1;
                        while q < self.n {
                            let ch = self.cc(q);
                            if ch == 125 || ch == 94 || ch == 34 {
                                break;
                            }
                            q += 1;
                        }
                        self.emit(T::SHAPE_DATA, p, q);
                    }
                }
                S::ShapeDataStr => {
                    if c == 34 {
                        self.pop();
                        self.emit(T::SHAPE_DATA, p, p + 1);
                    } else {
                        let mut e = self.index_of_unit(34, p);
                        if e == -1 {
                            e = self.n as isize;
                        }
                        let e = e as usize;
                        let raw = self.slice(p, e);
                        self.out.types.push(T::SHAPE_DATA);
                        self.out
                            .texts
                            .push(res().br.replace_all(&raw, "<br/>").into_owned());
                        self.out.starts.push(p);
                        self.p = e;
                    }
                }
            }

            if !ok {
                self.out.types.push(T::ERROR);
                self.out.texts.push(String::new());
                self.out.starts.push(self.p);
                return;
            }
        }
        self.out.types.push(T::END);
        self.out.texts.push(String::new());
        self.out.starts.push(self.n);
    }
}

pub fn tokenize(src: &str) -> Tokens {
    let units: Vec<u16> = src.encode_utf16().collect();
    let n = units.len();
    let mut u2b = Vec::with_capacity(n + 1);
    let mut b2u = vec![0usize; src.len() + 1];
    let mut unit = 0usize;
    for (byte, ch) in src.char_indices() {
        let width = ch.len_utf16();
        for k in 0..width {
            u2b.push(byte);
            let _ = k;
        }
        b2u[byte] = unit;
        unit += width;
    }
    u2b.push(src.len());
    b2u[src.len()] = n;
    let mut lx = Lx {
        src: units,
        n,
        text: src.to_owned(),
        u2b,
        b2u,
        out: Tokens::default(),
        stack: Vec::new(),
        state: S::Initial,
        p: 0,
        first_graph: true,
        dir_next: -1,
        at_next: -1,
        line_end: -1,
        dir_failed: 0,
        run_end: 0,
        run_at: -1,
        ws_from: -1,
        ws_to: -1,
    };
    lx.dir_next = lx.index_of("direction", 0);
    lx.at_next = lx.index_of("@", 0);
    lx.lex();
    lx.out
}
