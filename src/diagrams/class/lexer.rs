use super::tokens::*;
use crate::diagrams::common::spans::{Span, Spans, empty};
use crate::util::chars::is_js_space;
use crate::util::unicode::is_unicode_letter;

pub struct Tokens {
    pub types: Vec<usize>,
    pub spans: Vec<Span>,
    pub starts: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum S {
    Initial,
    String,
    BqString,
    Generic,
    CallbackName,
    CallbackArgs,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
    Cls,
    ClassBody,
    Namespace,
    NamespaceBody,
}

const AT_END: [usize; 13] = [
    EOF,
    STR,
    EOF,
    GENERICTYPE,
    CALLBACK_NAME,
    CALLBACK_ARGS,
    ACC_TITLE_VALUE,
    ACC_DESCR_VALUE,
    ACC_DESCR_MULTILINE_VALUE,
    EOF,
    EOF_IN_STRUCT,
    EOF,
    EOF_IN_STRUCT,
];

const fn cu(ch: u8) -> u16 {
    ch as u16
}

fn is_space(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

fn is_word(c: u16) -> bool {
    c < 128 && (char::from(c as u8).is_ascii_alphanumeric() || c == cu(b'_'))
}

fn is_line_terminator(c: u16) -> bool {
    matches!(c, 10 | 13 | 0x2028 | 0x2029)
}

fn starts_with(u: &[u16], at: usize, word: &str) -> bool {
    word.bytes()
        .enumerate()
        .all(|(k, w)| u.get(at + k).is_some_and(|&c| c == u16::from(w)))
}

struct Lexer {
    u: Vec<u16>,
    n: usize,
    spans_of: Spans,
    types: Vec<usize>,
    spans: Vec<Span>,
    starts: Vec<usize>,
    stack: Vec<S>,
    state: S,
    p: usize,
}

impl Lexer {
    fn code(&self, i: usize) -> u16 {
        self.u.get(i).copied().unwrap_or(0)
    }

    fn span(&self, s: usize, e: usize) -> Span {
        let e = e.min(self.n);
        if s >= e {
            return empty();
        }
        self.spans_of.span(&self.u, s, e)
    }

    fn emit(&mut self, t: usize, s: usize, e: usize) -> bool {
        self.types.push(t);
        let span = self.span(s, e);
        self.spans.push(span);
        self.starts.push(s);
        self.p = e;
        true
    }

    fn push(&mut self, s: S) {
        self.stack.push(self.state);
        self.state = s;
    }

    fn pop(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.state = state;
        }
    }

    fn kw(&self, word: &str) -> bool {
        starts_with(&self.u, self.p, word) && !is_word(self.code(self.p + word.len()))
    }

    fn skip_ws(&self, q: usize) -> usize {
        let mut q = q;
        while q < self.n && is_space(self.u[q]) {
            q += 1;
        }
        q
    }

    fn until(&self, ch: u16) -> usize {
        self.u[self.p..]
            .iter()
            .position(|&c| c == ch)
            .map_or(self.n, |k| self.p + k)
    }

    fn space(&mut self, leaves: bool) {
        let mut q = self.p;
        let mut k: Option<usize> = None;
        while q < self.n && is_space(self.u[q]) {
            if self.u[q] == 10 {
                k = Some(q);
            }
            q += 1;
        }
        let Some(k) = k else {
            self.p = q;
            return;
        };
        if leaves {
            self.pop();
        }
        let start = self.p;
        self.emit(NEWLINE, start, k + 1);
    }

    fn keyword(&mut self, c: u16) -> bool {
        match c {
            0x63 => {
                if self.kw("cssClass") {
                    return self.emit(CSSCLASS, self.p, self.p + 8);
                }
                if self.kw("callback") {
                    return self.emit(CALLBACK, self.p, self.p + 8);
                }
                if self.kw("click") {
                    return self.emit(CLICK, self.p, self.p + 5);
                }
            }
            0x6c => {
                if self.kw("link") {
                    return self.emit(LINK, self.p, self.p + 4);
                }
            }
            0x6e => {
                if self.kw("note for") {
                    return self.emit(NOTE_FOR, self.p, self.p + 8);
                }
                if self.kw("note") {
                    return self.emit(NOTE, self.p, self.p + 4);
                }
            }
            0x3c => {
                if self.code(self.p + 1) == cu(b'<') {
                    return self.emit(ANNOTATION_START, self.p, self.p + 2);
                }
            }
            0x3e => {
                if self.code(self.p + 1) == cu(b'>') {
                    return self.emit(ANNOTATION_END, self.p, self.p + 2);
                }
            }
            0x68 if self.kw("href") => return self.emit(HREF, self.p, self.p + 4),
            _ => {}
        }
        false
    }

    fn common(&mut self, c: u16) -> bool {
        if self.keyword(c) {
            return true;
        }
        let p = self.p;
        let d = self.code(p + 1);
        match c {
            0x22 => {
                self.push(S::String);
                self.p += 1;
                return true;
            }
            0x7e => {
                self.push(S::Generic);
                self.p += 1;
                return true;
            }
            0x60 => {
                self.push(S::BqString);
                self.p += 1;
                return true;
            }
            0x5f => {
                if self.kw("_self") {
                    return self.emit(LINK_TARGET, p, p + 5);
                }
                if self.kw("_blank") {
                    return self.emit(LINK_TARGET, p, p + 6);
                }
                if self.kw("_parent") {
                    return self.emit(LINK_TARGET, p, p + 7);
                }
                if self.kw("_top") {
                    return self.emit(LINK_TARGET, p, p + 4);
                }
            }
            0x3c => {
                return if d == cu(b'|') {
                    self.emit(EXTENSION, p, p + 2)
                } else {
                    self.emit(DEPENDENCY, p, p + 1)
                };
            }
            0x7c => {
                return d == cu(b'>') && self.emit(EXTENSION, p, p + 2);
            }
            0x3e => return self.emit(DEPENDENCY, p, p + 1),
            0x2a => return self.emit(COMPOSITION, p, p + 1),
            0x6f => {
                if !is_word(d) {
                    return self.emit(AGGREGATION, p, p + 1);
                }
            }
            0x28 => return d == cu(b')') && self.emit(LOLLIPOP, p, p + 2),
            0x2d => {
                return if d == cu(b'-') {
                    self.emit(LINE, p, p + 2)
                } else {
                    self.emit(MINUS, p, p + 1)
                };
            }
            0x2e => {
                return if d == cu(b'.') {
                    self.emit(DOTTED_LINE, p, p + 2)
                } else {
                    self.emit(DOT, p, p + 1)
                };
            }
            0x3a => {
                let mut q = p + 1;
                while q < self.n {
                    let ch = self.u[q];
                    if ch == cu(b':') || ch == 10 || ch == cu(b';') {
                        break;
                    }
                    q += 1;
                }
                if q > p + 1 {
                    return self.emit(LABEL, p, q);
                }
                return d == cu(b':')
                    && self.code(p + 2) == cu(b':')
                    && self.emit(STYLE_SEPARATOR, p, p + 3);
            }
            0x2b => return self.emit(PLUS, p, p + 1),
            0x25 => return self.emit(PCT, p, p + 1),
            0x3d => return self.emit(EQUALS, p, p + 1),
            0x5b => return self.emit(SQS, p, p + 1),
            0x5d => return self.emit(SQE, p, p + 1),
            0x21 | 0x23 | 0x24 | 0x26 | 0x27 | 0x2c | 0x3f | 0x5c | 0x2f => {
                return self.emit(PUNCTUATION, p, p + 1);
            }
            _ => {}
        }
        if is_word(c) {
            let mut q = p + 1;
            while is_word(self.code(q)) {
                q += 1;
            }
            return self.emit(ALPHA, p, q);
        }
        is_unicode_letter(u32::from(c)) && self.emit(UNICODE_TEXT, p, p + 1)
    }

    fn direction_end(&self, keyword: &str) -> Option<usize> {
        let p = self.p;
        let first_terminator = self.u[p..]
            .iter()
            .position(|&c| is_line_terminator(c))
            .map_or(self.n, |k| p + k);
        for d in (p..first_terminator).rev() {
            if !starts_with(&self.u, d, "direction") {
                continue;
            }
            let mut after = d + 9;
            while after < self.n && is_space(self.u[after]) {
                after += 1;
            }
            if after == d + 9 || !starts_with(&self.u, after, keyword) {
                continue;
            }
            let tb_end = after + 2;
            let end = self.u[tb_end.min(self.n)..]
                .iter()
                .position(|&c| c == 10)
                .map_or(self.n, |k| tb_end + k);
            return Some(end);
        }
        None
    }

    fn direction(&mut self) -> bool {
        const KEYWORDS: [&str; 4] = ["TB", "BT", "RL", "LR"];
        for (i, keyword) in KEYWORDS.iter().enumerate() {
            if let Some(end) = self.direction_end(keyword) {
                let p = self.p;
                return self.emit(DIRECTION_TB + i, p, end);
            }
        }
        false
    }

    fn initial(&mut self, c: u16) -> bool {
        if self.direction() {
            return true;
        }
        if is_space(c) {
            self.space(false);
            return true;
        }
        let p = self.p;
        match c {
            0x25 => {
                if self.code(p + 1) == cu(b'%') {
                    let mut e = self.until(10);
                    while e < self.n && (self.u[e] == 10 || self.u[e] == 13) {
                        e += 1;
                    }
                    self.p = e;
                    return true;
                }
            }
            0x61 => {
                let title = starts_with(&self.u, p, "accTitle");
                if title || starts_with(&self.u, p, "accDescr") {
                    let q = self.skip_ws(p + 8);
                    let d = self.code(q);
                    if d == cu(b':') {
                        let e = self.skip_ws(q + 1);
                        self.emit(if title { ACC_TITLE } else { ACC_DESCR }, p, e);
                        self.push(if title { S::AccTitle } else { S::AccDescr });
                        return true;
                    }
                    if d == cu(b'{') && !title {
                        self.p = self.skip_ws(q + 1);
                        self.push(S::AccDescrMultiline);
                        return true;
                    }
                }
            }
            0x63 => {
                if self.kw("classDiagram-v2") {
                    return self.emit(CLASS_DIAGRAM, p, p + 15);
                }
                if self.kw("classDiagram") {
                    return self.emit(CLASS_DIAGRAM, p, p + 12);
                }
                if starts_with(&self.u, p, "call") && is_space(self.code(p + 4)) {
                    self.p = self.skip_ws(p + 4);
                    self.push(S::CallbackName);
                    return true;
                }
                if self.kw("classDef") {
                    return self.emit(CLASSDEF, p, p + 8);
                }
                if self.kw("class") {
                    self.push(S::Cls);
                    return self.emit(CLASS, p, p + 5);
                }
            }
            0x5b => {
                if starts_with(&self.u, p + 1, "*]") {
                    return self.emit(EDGE_STATE, p, p + 3);
                }
            }
            0x73 => {
                if self.kw("style") {
                    return self.emit(STYLE, p, p + 5);
                }
            }
            0x6e => {
                if self.kw("namespace") {
                    self.push(S::Namespace);
                    return self.emit(NAMESPACE, p, p + 9);
                }
            }
            0x2c => return self.emit(COMMA, p, p + 1),
            0x23 => return self.emit(BRKT, p, p + 1),
            _ => {}
        }
        self.common(c) || (c == cu(b':') && self.emit(COLON, p, p + 1))
    }

    fn run(mut self) -> Tokens {
        loop {
            if self.p >= self.n {
                let at_end = AT_END[self.state as usize];
                let n = self.n;
                self.types.push(at_end);
                self.spans.push(empty());
                self.starts.push(n);
                break;
            }
            let c = self.code(self.p);
            let p = self.p;
            let ok = match self.state {
                S::Initial => self.initial(c),
                S::Cls => {
                    if is_space(c) {
                        self.space(true);
                        true
                    } else if c == cu(b'}') {
                        self.pop();
                        self.pop();
                        self.emit(STRUCT_STOP, p, p + 1)
                    } else if c == cu(b'{') {
                        self.push(S::ClassBody);
                        self.emit(STRUCT_START, p, p + 1)
                    } else {
                        self.common(c)
                    }
                }
                S::ClassBody => {
                    if c == cu(b'"') {
                        self.push(S::String);
                        self.p += 1;
                    } else if c == cu(b'}') {
                        self.pop();
                        self.emit(STRUCT_STOP, p, p + 1);
                    } else if c == cu(b'[') && starts_with(&self.u, p + 1, "*]") {
                        self.emit(EDGE_STATE, p, p + 3);
                    } else if c == cu(b'{') {
                        self.emit(OPEN_IN_STRUCT, p, p + 1);
                    } else if c == 10 {
                        self.p += 1;
                    } else {
                        let mut q = p + 1;
                        while q < self.n {
                            let ch = self.u[q];
                            if ch == cu(b'{') || ch == cu(b'}') || ch == 10 {
                                break;
                            }
                            q += 1;
                        }
                        self.emit(MEMBER, p, q);
                    }
                    true
                }
                S::Namespace => {
                    if is_space(c) {
                        self.space(true);
                        true
                    } else if c == cu(b'{') {
                        self.push(S::NamespaceBody);
                        self.emit(STRUCT_START, p, p + 1)
                    } else if c == cu(b'}') {
                        self.pop();
                        true
                    } else if c == cu(b'n') && self.kw("namespace") {
                        self.push(S::Namespace);
                        self.emit(NAMESPACE, p, p + 9)
                    } else {
                        self.common(c)
                    }
                }
                S::NamespaceBody => {
                    if is_space(c) {
                        self.space(false);
                        true
                    } else if c == cu(b'}') {
                        self.pop();
                        self.emit(STRUCT_STOP, p, p + 1)
                    } else if c == cu(b'n') && self.kw("namespace") {
                        self.push(S::Namespace);
                        self.emit(NAMESPACE, p, p + 9)
                    } else if c == cu(b'[') && starts_with(&self.u, p + 1, "*]") {
                        self.emit(EDGE_STATE, p, p + 3)
                    } else if c == cu(b'c') && self.kw("class") {
                        self.push(S::Cls);
                        self.emit(CLASS, p, p + 5)
                    } else {
                        self.common(c)
                    }
                }
                S::String => {
                    if c == cu(b'"') {
                        self.pop();
                        self.p += 1;
                    } else {
                        let e = self.until(cu(b'"'));
                        self.emit(STR, p, e);
                    }
                    true
                }
                S::Generic => {
                    if c == cu(b'"') {
                        self.push(S::String);
                        self.p += 1;
                        true
                    } else if self.keyword(c) {
                        true
                    } else if c == cu(b'~') {
                        self.pop();
                        self.p += 1;
                        true
                    } else {
                        let e = self.until(cu(b'~'));
                        self.emit(GENERICTYPE, p, e);
                        true
                    }
                }
                S::BqString => {
                    if c == cu(b'"') {
                        self.push(S::String);
                        self.p += 1;
                        true
                    } else if self.keyword(c) {
                        true
                    } else if c == cu(b'~') {
                        self.push(S::Generic);
                        self.p += 1;
                        true
                    } else if c == cu(b'`') {
                        self.pop();
                        self.p += 1;
                        true
                    } else {
                        let e = self.until(cu(b'`'));
                        self.emit(BQUOTE_STR, p, e);
                        true
                    }
                }
                S::CallbackName => {
                    if c == cu(b'(') {
                        let q = self.skip_ws(p + 1);
                        self.pop();
                        if self.code(q) == cu(b')') {
                            self.p = q + 1;
                        } else {
                            self.push(S::CallbackArgs);
                            self.p += 1;
                        }
                    } else {
                        let e = self.until(cu(b'('));
                        self.emit(CALLBACK_NAME, p, e);
                    }
                    true
                }
                S::CallbackArgs => {
                    if c == cu(b')') {
                        self.pop();
                        self.p += 1;
                    } else {
                        let e = self.until(cu(b')'));
                        self.emit(CALLBACK_ARGS, p, e);
                    }
                    true
                }
                S::AccTitle | S::AccDescr => {
                    let t = if self.state == S::AccTitle {
                        ACC_TITLE_VALUE
                    } else {
                        ACC_DESCR_VALUE
                    };
                    let e = self.until(10);
                    self.emit(t, p, e);
                    self.pop();
                    true
                }
                S::AccDescrMultiline => {
                    if c == cu(b'}') {
                        self.pop();
                        self.p += 1;
                    } else {
                        let e = self.until(cu(b'}'));
                        self.emit(ACC_DESCR_MULTILINE_VALUE, p, e);
                    }
                    true
                }
            };
            if !ok {
                self.types.push(ERROR);
                self.spans.push(empty());
                self.starts.push(p);
                return Tokens {
                    types: self.types,
                    spans: self.spans,
                    starts: self.starts,
                };
            }
        }
        let n = self.n;
        self.types.push(END);
        self.spans.push(empty());
        self.starts.push(n);
        Tokens {
            types: self.types,
            spans: self.spans,
            starts: self.starts,
        }
    }
}

pub fn tokenize(src: &str) -> Tokens {
    let u: Vec<u16> = src.encode_utf16().collect();
    let n = u.len();
    Lexer {
        spans_of: Spans::new(src),
        u,
        n,
        types: Vec::new(),
        spans: Vec::new(),
        starts: Vec::new(),
        stack: Vec::new(),
        state: S::Initial,
        p: 0,
    }
    .run()
}
