use super::tokens::*;
use crate::diagrams::common::spans::{Span, Spans};

pub struct Tokens {
    pub types: Vec<usize>,
    pub spans: Vec<Span>,
    pub starts: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    Struct,
    Attribute,
    String,
    StringKv,
    StringKvKey,
    StringKvValue,
    Acc,
    AccDescrMultiline,
}

const KEYWORDS: &[(&str, usize)] = &[
    ("C4Context", C4_CONTEXT),
    ("C4Container", C4_CONTAINER),
    ("C4Component", C4_COMPONENT),
    ("C4Dynamic", C4_DYNAMIC),
    ("C4Deployment", C4_DEPLOYMENT),
    ("Person", PERSON),
    ("Person_Ext", PERSON_EXT),
    ("System", SYSTEM),
    ("SystemDb", SYSTEM_DB),
    ("SystemQueue", SYSTEM_QUEUE),
    ("System_Ext", SYSTEM_EXT),
    ("SystemDb_Ext", SYSTEM_EXT_DB),
    ("SystemQueue_Ext", SYSTEM_EXT_QUEUE),
    ("Container", CONTAINER),
    ("ContainerDb", CONTAINER_DB),
    ("ContainerQueue", CONTAINER_QUEUE),
    ("Container_Ext", CONTAINER_EXT),
    ("ContainerDb_Ext", CONTAINER_EXT_DB),
    ("ContainerQueue_Ext", CONTAINER_EXT_QUEUE),
    ("Component", COMPONENT),
    ("ComponentDb", COMPONENT_DB),
    ("ComponentQueue", COMPONENT_QUEUE),
    ("Component_Ext", COMPONENT_EXT),
    ("ComponentDb_Ext", COMPONENT_EXT_DB),
    ("ComponentQueue_Ext", COMPONENT_EXT_QUEUE),
    ("Enterprise_Boundary", ENTERPRISE_BOUNDARY),
    ("System_Boundary", SYSTEM_BOUNDARY),
    ("Boundary", BOUNDARY),
    ("Container_Boundary", CONTAINER_BOUNDARY),
    ("Deployment_Node", NODE),
    ("Node", NODE),
    ("Node_L", NODE_L),
    ("Node_R", NODE_R),
    ("Rel", REL),
    ("BiRel", BIREL),
    ("Rel_Up", REL_U),
    ("Rel_U", REL_U),
    ("Rel_Down", REL_D),
    ("Rel_D", REL_D),
    ("Rel_Left", REL_L),
    ("Rel_L", REL_L),
    ("Rel_Right", REL_R),
    ("Rel_R", REL_R),
    ("Rel_Back", REL_B),
    ("RelIndex", REL_INDEX),
    ("UpdateElementStyle", UPDATE_EL_STYLE),
    ("UpdateRelStyle", UPDATE_REL_STYLE),
    ("UpdateLayoutConfig", UPDATE_LAYOUT_CONFIG),
];

const DIRECTION_WORDS: [&str; 4] = ["TB", "BT", "RL", "LR"];

fn is_space(c: u16) -> bool {
    if c < 128 {
        c == 32 || (9..=13).contains(&c)
    } else {
        matches!(
            c,
            0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff
        )
    }
}

fn is_word(c: u16) -> bool {
    (48..=57).contains(&c) || (65..=90).contains(&c) || (97..=122).contains(&c) || c == 95
}

fn is_line_end(c: u16) -> bool {
    matches!(c, 10 | 13 | 0x2028 | 0x2029)
}

struct Lexer {
    units: Vec<u16>,
    spans_of: Spans,
    n: i64,
    p: i64,
    state: State,
    acc_value: usize,
    types: Vec<usize>,
    spans: Vec<Span>,
    starts: Vec<usize>,
    dir_next: i64,
    line_end: i64,
    dir_failed: i64,
}

impl Lexer {
    fn ch(&self, q: i64) -> u16 {
        if q < 0 || q >= self.n {
            0
        } else {
            self.units[q as usize]
        }
    }

    fn keyword_at(&self, s: i64, e: i64) -> Option<usize> {
        let word = &self.units[s as usize..e as usize];
        KEYWORDS
            .iter()
            .find(|(name, _)| {
                name.len() == word.len() && name.bytes().zip(word).all(|(b, &u)| u16::from(b) == u)
            })
            .map(|&(_, t)| t)
    }

    fn emit(&mut self, t: usize, s: i64, e: i64) {
        let span = self.spans_of.span(&self.units, s as usize, e as usize);
        self.types.push(t);
        self.spans.push(span);
        self.starts.push(s as usize);
        self.p = e;
    }

    fn starts_with(&self, q: i64, word: &str) -> bool {
        word.bytes()
            .enumerate()
            .all(|(i, b)| self.ch(q + i as i64) == u16::from(b))
    }

    fn skip_ws(&self, mut q: i64) -> i64 {
        while q < self.n && is_space(self.ch(q)) {
            q += 1;
        }
        q
    }

    fn upto(&self, c: u16, from: i64) -> i64 {
        let mut q = from.max(0);
        while q < self.n {
            if self.units[q as usize] == c {
                return q;
            }
            q += 1;
        }
        self.n
    }

    fn next_line_end(&self, q: i64) -> i64 {
        let mut i = q.max(0);
        while i < self.n {
            if is_line_end(self.units[i as usize]) {
                return i;
            }
            i += 1;
        }
        self.n
    }

    fn direction_end(&self, q: i64, second: &str) -> Option<i64> {
        let mut k = self.next_line_end(q);
        while k >= q {
            if self.starts_with(k, "direction") && is_space(self.ch(k + 9)) {
                let j = self.skip_ws(k + 9);
                if self.starts_with(j, second) {
                    let mut e = j + 2;
                    while e < self.n && self.units[e as usize] != 10 {
                        e += 1;
                    }
                    return Some(e);
                }
            }
            k -= 1;
        }
        None
    }

    fn bare_title_end(&self, q: i64) -> Option<i64> {
        if !self.starts_with(q, "title") {
            return None;
        }
        let mut at = q + 5;
        while at < self.n {
            let c = self.ch(at);
            if !is_space(c) || c == 10 || c == 13 {
                break;
            }
            at += 1;
        }
        let stop = at >= self.n || matches!(self.ch(at), 10 | 13);
        stop.then(|| self.skip_ws(at))
    }

    fn statement(&mut self, word: &str, t: usize) -> bool {
        let p = self.p;
        let mut q = p + word.len() as i64;
        if !self.starts_with(p, word) || !is_space(self.ch(q)) {
            return false;
        }
        q += 1;
        let from = q;
        while q < self.n {
            let c = self.ch(q);
            if c == 35 || c == 10 || c == 59 {
                break;
            }
            q += 1;
        }
        if q == from {
            return false;
        }
        self.emit(t, p, q);
        true
    }

    fn find_direction(&self, from: i64) -> i64 {
        let mut q = from.max(0);
        while q < self.n {
            if self.starts_with(q, "direction") {
                return q;
            }
            q += 1;
        }
        -1
    }

    fn into_tokens(self) -> Tokens {
        Tokens {
            types: self.types,
            spans: self.spans,
            starts: self.starts,
        }
    }
}

pub fn tokenize(src: &str) -> Tokens {
    let units: Vec<u16> = src.encode_utf16().collect();
    let n = units.len() as i64;
    let mut lx = Lexer {
        units,
        spans_of: Spans::new(src),
        n,
        p: 0,
        state: State::Initial,
        acc_value: ACC_TITLE_VALUE,
        types: Vec::new(),
        spans: Vec::new(),
        starts: Vec::new(),
        dir_next: -2,
        line_end: -1,
        dir_failed: 0,
    };
    let mut done = false;
    loop {
        if done {
            lx.emit(END, n, n);
            break;
        }
        if lx.p >= n {
            done = true;
        }
        let p = lx.p;
        let c = lx.ch(p);

        if lx.state == State::Initial {
            if p >= n {
                lx.emit(EOF, n, n);
                continue;
            }
            if lx.dir_next != -1 && p >= lx.dir_failed {
                if lx.dir_next < p {
                    lx.dir_next = lx.find_direction(p);
                }
                if lx.dir_next != -1 {
                    if lx.line_end < p {
                        lx.line_end = lx.next_line_end(p);
                    }
                    if lx.dir_next < lx.line_end {
                        let mut matched = false;
                        for (i, word) in DIRECTION_WORDS.iter().enumerate() {
                            if let Some(e) = lx.direction_end(p, word) {
                                lx.emit(DIRECTION_TB + i, p, e);
                                matched = true;
                                break;
                            }
                        }
                        if matched {
                            continue;
                        }
                        lx.dir_failed = lx.line_end;
                    }
                }
            }
            if c == 116 {
                if let Some(end) = lx.bare_title_end(p) {
                    lx.p = end;
                    continue;
                }
                if lx.statement("title", TITLE) {
                    continue;
                }
            }
            if c == 97 {
                if lx.statement("accDescription", ACC_DESCRIPTION) {
                    continue;
                }
                let title = lx.starts_with(p, "accTitle");
                if title || lx.starts_with(p, "accDescr") {
                    let q = lx.skip_ws(p + 8);
                    let d = lx.ch(q);
                    if d == 58 {
                        let end = lx.skip_ws(q + 1);
                        lx.emit(if title { ACC_TITLE } else { ACC_DESCR }, p, end);
                        lx.acc_value = if title {
                            ACC_TITLE_VALUE
                        } else {
                            ACC_DESCR_VALUE
                        };
                        lx.state = State::Acc;
                        continue;
                    }
                    if d == 123 && !title {
                        lx.p = lx.skip_ws(q + 1);
                        lx.state = State::AccDescrMultiline;
                        continue;
                    }
                }
            }
            if c == 37 && lx.ch(p + 1) == 37 {
                let mut q = lx.upto(10, p + 2);
                while q < n && matches!(lx.ch(q), 10 | 13) {
                    q += 1;
                }
                lx.p = q;
                continue;
            }
            if is_space(c) {
                let mut q = p;
                let mut last = -1;
                while q < n {
                    let d = lx.ch(q);
                    if !is_space(d) {
                        break;
                    }
                    if d == 10 {
                        last = q;
                    }
                    q += 1;
                }
                if last != -1 {
                    lx.emit(NEWLINE, p, last + 1);
                } else {
                    lx.p = q;
                }
                continue;
            }
            if c == 123 {
                lx.emit(LBRACE, p, p + 1);
                continue;
            }
            if c == 125 {
                lx.emit(RBRACE, p, p + 1);
                continue;
            }
            if is_word(c) {
                let mut q = p + 1;
                while q < n && is_word(lx.ch(q)) {
                    q += 1;
                }
                let found = if q - p <= 19 {
                    lx.keyword_at(p, q)
                } else {
                    None
                };
                if let Some(t) = found {
                    if t > C4_DEPLOYMENT {
                        lx.state = State::Struct;
                    }
                    lx.emit(t, p, q);
                    continue;
                }
            }
            lx.emit(ERROR, p, p);
            break;
        }

        if lx.state == State::Struct {
            if p >= n {
                lx.emit(EOF_IN_STRUCT, n, n);
            } else if c == 40 {
                let mut q = p + 1;
                while lx.ch(q) == 32 {
                    q += 1;
                }
                if lx.ch(q) == 44 {
                    lx.emit(ATTRIBUTE_EMPTY, p, q + 1);
                } else {
                    lx.p += 1;
                }
                lx.state = State::Attribute;
            } else if c == 41 {
                lx.p += 1;
                lx.state = State::Initial;
            } else {
                lx.emit(ERROR, p, p);
                break;
            }
            continue;
        }

        if lx.state == State::Attribute {
            if p >= n {
                continue;
            }
            if c == 41 {
                lx.p += 1;
                lx.state = State::Initial;
            } else if c == 44 {
                if lx.ch(p + 1) == 44 {
                    lx.emit(ATTRIBUTE_EMPTY, p, p + 2);
                } else {
                    lx.p += 1;
                }
            } else {
                let mut q = p;
                while lx.ch(q) == 32 {
                    q += 1;
                }
                let d = lx.ch(q);
                if d == 34 {
                    if lx.ch(q + 1) == 34 {
                        lx.emit(ATTRIBUTE_EMPTY, p, q + 2);
                    } else {
                        lx.p = q + 1;
                        lx.state = State::String;
                    }
                } else if d == 36 {
                    lx.p = q + 1;
                    lx.state = State::StringKv;
                } else {
                    let end = lx.upto(44, p);
                    lx.emit(STR, p, end);
                }
            }
            continue;
        }

        if lx.state == State::String {
            if c == 34 {
                lx.p += 1;
                lx.state = State::Attribute;
            } else {
                let end = lx.upto(34, p);
                lx.emit(STR, p, end);
            }
            continue;
        }

        if lx.state == State::StringKv {
            let end = lx.upto(61, p);
            lx.emit(STR_KEY, p, end);
            lx.state = State::StringKvKey;
            continue;
        }

        if lx.state == State::StringKvKey {
            if p >= n {
                continue;
            }
            let mut q = p + 1;
            if c == 61 {
                while lx.ch(q) == 32 {
                    q += 1;
                }
            }
            if c != 61 || lx.ch(q) != 34 {
                lx.emit(ERROR, p, p);
                break;
            }
            lx.p = q + 1;
            lx.state = State::StringKvValue;
            continue;
        }

        if lx.state == State::StringKvValue {
            if p >= n {
                continue;
            }
            if c == 34 {
                lx.p += 1;
                lx.state = State::Attribute;
            } else {
                let end = lx.upto(34, p);
                lx.emit(STR_VALUE, p, end);
            }
            continue;
        }

        if lx.state == State::Acc {
            let end = lx.upto(10, p);
            let value = lx.acc_value;
            lx.emit(value, p, end);
            lx.state = State::Initial;
            continue;
        }

        if c == 125 {
            lx.p += 1;
            lx.state = State::Initial;
        } else {
            let end = lx.upto(125, p);
            lx.emit(ACC_DESCR_MULTILINE_VALUE, p, end);
        }
    }

    lx.into_tokens()
}
