use super::tokens::*;

pub struct Tokens {
    pub types: Vec<i32>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

const TEXT: u8 = 1;
const BIT_STYLE_TEXT: u8 = 2;
const ATTR_START: u8 = 4;
const ATTR_REST: u8 = 8;

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    Block,
    BlockBq,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
    Style,
}

fn class_bits(c: u16) -> u8 {
    if c >= 128 {
        return 0;
    }
    let b = c as u8;
    let word = b.is_ascii_alphanumeric() || b == b'_';
    let mut bits = 0;
    if word {
        bits |= TEXT | BIT_STYLE_TEXT | ATTR_REST;
    }
    if word && !b.is_ascii_digit() {
        bits |= ATTR_START;
    }
    if b == b'-' || b == b'*' {
        bits |= TEXT | BIT_STYLE_TEXT | ATTR_REST;
    }
    if b == b'.' {
        bits |= TEXT | ATTR_REST;
    }
    if b == b'*' {
        bits |= ATTR_START;
    }
    if matches!(b, b'[' | b']' | b'(' | b')' | b',') {
        bits |= ATTR_REST;
    }
    bits
}

fn is_attr(c: u16, kind: u8) -> bool {
    if c < 128 {
        class_bits(c) & kind != 0
    } else {
        c >= 0xc0 || c == 0xb5
    }
}

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

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn is_word(c: u16) -> bool {
    is_digit(c) || (65..=90).contains(&c) || (97..=122).contains(&c) || c == 95
}

fn is_line_end(c: u16) -> bool {
    matches!(c, 10 | 13 | 0x2028 | 0x2029)
}

const DIRECTION_WORDS: [&str; 4] = ["tb", "bt", "rl", "lr"];

struct Lexer {
    units: Vec<u16>,
    n: i64,
    state: State,
    p: i64,
    types: Vec<i32>,
    texts: Vec<String>,
    starts: Vec<usize>,
    dir_next: i64,
    dir_failed: i64,
    line_end: i64,
    tilde_next: i64,
    tilde_line: i64,
    tilde_last: i64,
    ws_next: i64,
}

impl Lexer {
    fn ch(&self, q: i64) -> u16 {
        if q < 0 || q >= self.n {
            0
        } else {
            self.units[q as usize]
        }
    }

    fn index_of(&self, c: u16, from: i64) -> i64 {
        let mut i = from.max(0);
        while i < self.n {
            if self.units[i as usize] == c {
                return i;
            }
            i += 1;
        }
        -1
    }

    fn last_index_of(&self, c: u16, from: i64) -> i64 {
        let mut i = from.min(self.n - 1);
        while i >= 0 {
            if self.units[i as usize] == c {
                return i;
            }
            i -= 1;
        }
        -1
    }

    fn emit(&mut self, t: i32, s: i64, e: i64) {
        let text = String::from_utf16_lossy(&self.units[s as usize..e.min(self.n).max(s) as usize]);
        self.types.push(t);
        self.texts.push(text);
        self.starts.push(s as usize);
        self.p = e;
    }

    fn push_raw(&mut self, t: i32, text: &str, start: i64) {
        self.types.push(t);
        self.texts.push(text.to_owned());
        self.starts.push(start as usize);
    }

    fn kw_at(&self, q: i64, word: &str) -> bool {
        for (i, k) in word.bytes().enumerate() {
            let k = u16::from(k);
            let c = self.ch(q + i as i64);
            if c != k && (!(97..=122).contains(&k) || (c | 32) != k) {
                return false;
            }
        }
        true
    }

    fn kw(&self, word: &str) -> bool {
        self.kw_at(self.p, word)
    }

    fn kwb(&self, word: &str) -> bool {
        self.kw(word) && !is_word(self.ch(self.p + word.len() as i64))
    }

    fn lower(&self, q: i64) -> u16 {
        let c = self.ch(q);
        if (65..=90).contains(&c) { c + 32 } else { c }
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

    fn skip_spaces(&self, mut q: i64) -> i64 {
        while is_space(self.ch(q)) {
            q += 1;
        }
        q
    }

    fn acc_title_end(&self, q: i64) -> Option<i64> {
        if !self.kw_at(q, "acctitle") {
            return None;
        }
        let colon = self.skip_spaces(q + 8);
        (self.ch(colon) == 58).then(|| self.skip_spaces(colon + 1))
    }

    fn acc_descr_end(&self, q: i64) -> Option<i64> {
        if !self.kw_at(q, "accdescr") {
            return None;
        }
        let colon = self.skip_spaces(q + 8);
        (self.ch(colon) == 58).then(|| self.skip_spaces(colon + 1))
    }

    fn acc_descr_block_end(&self, q: i64) -> Option<i64> {
        if !self.kw_at(q, "accdescr") {
            return None;
        }
        let brace = self.skip_spaces(q + 8);
        (self.ch(brace) == 123).then(|| self.skip_spaces(brace + 1))
    }

    fn direction_end(&self, q: i64, second: &str) -> Option<i64> {
        let mut k = self.next_line_end(q);
        while k >= q {
            if self.kw_at(k, "direction") && is_space(self.ch(k + 9)) {
                let j = self.skip_spaces(k + 9);
                if self.kw_at(j, second) {
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

    fn find_direction(&self, from: i64) -> Option<i64> {
        (from.max(0)..self.n).find(|&k| self.kw_at(k, "direction"))
    }

    fn direction(&mut self) -> bool {
        if self.dir_next == -1 {
            return false;
        }
        if self.dir_next < self.p {
            match self.find_direction(self.p) {
                Some(at) => self.dir_next = at,
                None => {
                    self.dir_next = -1;
                    return false;
                }
            }
        }
        if self.p < self.dir_failed {
            return false;
        }
        if self.line_end < self.p {
            self.line_end = self.next_line_end(self.p);
        }
        if self.dir_next >= self.line_end {
            return false;
        }
        for (i, word) in DIRECTION_WORDS.iter().enumerate() {
            if let Some(e) = self.direction_end(self.p, word) {
                let start = self.p;
                self.emit(DIRECTION_TB + i as i32, start, e);
                return true;
            }
        }
        self.dir_failed = self.line_end;
        false
    }

    fn quoted(&mut self) {
        let e = self.index_of(34, self.p + 1);
        if e == -1 {
            let p = self.p;
            self.emit(CHAR, p, p + 1);
            return;
        }
        let mut name = e > self.p + 1;
        let mut q = self.p + 1;
        while name && q < e {
            if matches!(self.units[q as usize], 37 | 92 | 13 | 10 | 11 | 8) {
                name = false;
            }
            q += 1;
        }
        let p = self.p;
        self.emit(if name { ENTITY_NAME } else { WORD }, p, e + 1);
    }

    fn digits(&self, mut q: i64) -> i64 {
        while is_digit(self.ch(q)) {
            q += 1;
        }
        q
    }

    fn number(&mut self, c: u16) {
        let p = self.p;
        let d = self.ch(p + 1);
        if d == 43 && (c == 49 || c == 48) {
            self.emit(if c == 49 { ONE_OR_MORE } else { ZERO_OR_MORE }, p, p + 2);
            return;
        }
        let q = self.digits(p + 1);
        if self.ch(q) == 46 && is_digit(self.ch(q + 1)) {
            let end = self.digits(q + 2);
            self.emit(DECIMAL_NUM, p, end);
            return;
        }
        if c == 49 {
            if is_space(d) {
                let mut r = p + 2;
                while is_space(self.ch(r)) {
                    r += 1;
                }
                let f = self.ch(r);
                if is_word(f) || f == 34 || f == 39 {
                    self.emit(ONLY_ONE, p, p + 1);
                    return;
                }
            } else if d == 45 || d == 46 {
                let f = self.ch(p + 2);
                if f == 45 || f == 46 {
                    self.emit(ONLY_ONE, p, p + 1);
                    return;
                }
            }
            if !is_word(d) {
                self.emit(ENTITY_ONE, p, p + 1);
                return;
            }
        }
        self.emit(NUM, p, q);
    }

    fn keyword(&mut self, lc: u16) -> bool {
        let p = self.p;
        let d = self.ch(p + 1);
        match lc {
            101 => {
                if self.kwb("erdiagram") {
                    self.emit(ER_DIAGRAM, p, p + 9);
                } else if self.kwb("end") {
                    let e = self.skip_spaces(p + 3);
                    self.emit(END_KW, p, e);
                } else {
                    return false;
                }
            }
            115 => {
                if self.kwb("style") {
                    self.state = State::Style;
                    self.emit(STYLE, p, p + 5);
                } else if self.kwb("subgraph") {
                    self.emit(SUBGRAPH, p, p + 8);
                } else {
                    return false;
                }
            }
            99 => {
                if self.kwb("classdef") {
                    self.state = State::Style;
                    self.emit(CLASSDEF, p, p + 8);
                } else if self.kwb("class") {
                    self.emit(CLASS, p, p + 5);
                } else {
                    return false;
                }
            }
            111 => {
                if self.kwb("one or zero") {
                    self.emit(ZERO_OR_ONE, p, p + 11);
                } else if self.kwb("one or more") || self.kwb("one or many") {
                    self.emit(ONE_OR_MORE, p, p + 11);
                } else if self.kwb("one") {
                    self.emit(ONLY_ONE, p, p + 3);
                } else if self.kwb("only one") {
                    self.emit(ONLY_ONE, p, p + 8);
                } else if d == 124 {
                    self.emit(ZERO_OR_ONE, p, p + 2);
                } else if d == 123 {
                    self.emit(ZERO_OR_MORE, p, p + 2);
                } else if self.kwb("optionally to") {
                    self.emit(NON_IDENTIFYING, p, p + 13);
                } else {
                    return false;
                }
            }
            122 => {
                if self.kwb("zero or one") {
                    self.emit(ZERO_OR_ONE, p, p + 11);
                } else if self.kwb("zero or more") || self.kwb("zero or many") {
                    self.emit(ZERO_OR_MORE, p, p + 12);
                } else {
                    return false;
                }
            }
            109 => {
                if self.kw("many(0)") {
                    self.emit(ZERO_OR_MORE, p, p + 7);
                } else if self.kw("many(1)") {
                    self.emit(ONE_OR_MORE, p, p + 7);
                } else if self.kwb("many") {
                    self.emit(ZERO_OR_MORE, p, p + 4);
                } else {
                    return false;
                }
            }
            116 => {
                if !self.kwb("to") {
                    return false;
                }
                self.emit(IDENTIFYING, p, p + 2);
            }
            117 => {
                if d != 46 && d != 45 && d != 124 {
                    return false;
                }
                self.emit(MD_PARENT, p, p + 1);
            }
            124 => {
                if self.lower(p + 1) == 111 && !is_word(self.ch(p + 2)) {
                    self.emit(ZERO_OR_ONE, p, p + 2);
                } else if d == 124 {
                    self.emit(ONLY_ONE, p, p + 2);
                } else if d == 123 {
                    self.emit(ONE_OR_MORE, p, p + 2);
                } else {
                    return false;
                }
            }
            125 => {
                if self.lower(p + 1) == 111 && !is_word(self.ch(p + 2)) {
                    self.emit(ZERO_OR_MORE, p, p + 2);
                } else if d == 124 {
                    self.emit(ONE_OR_MORE, p, p + 2);
                } else {
                    return false;
                }
            }
            46 => {
                if d != 46 && d != 45 {
                    return false;
                }
                self.emit(NON_IDENTIFYING, p, p + 2);
            }
            45 => {
                if d == 45 {
                    self.emit(IDENTIFYING, p, p + 2);
                } else if d == 46 {
                    self.emit(NON_IDENTIFYING, p, p + 2);
                } else {
                    return false;
                }
            }
            _ => return false,
        }
        true
    }

    fn initial(&mut self, c: u16) {
        let p = self.p;
        if (c == 97 || c == 65) && self.kw("acc") {
            if let Some(e) = self.acc_title_end(p) {
                self.state = State::AccTitle;
                self.emit(ACC_TITLE, p, e);
                return;
            }
            if let Some(e) = self.acc_descr_end(p) {
                self.state = State::AccDescr;
                self.emit(ACC_DESCR, p, e);
                return;
            }
            if let Some(e) = self.acc_descr_block_end(p) {
                self.state = State::AccDescrMultiline;
                self.p = e;
                return;
            }
        }
        if self.direction() {
            return;
        }
        match c {
            32 | 9 | 13 => {
                let mut e = p + 1;
                while matches!(self.ch(e), 32 | 9 | 13) {
                    e += 1;
                }
                self.p = e;
                return;
            }
            10 => {
                let mut e = p + 1;
                while self.ch(e) == 10 {
                    e += 1;
                }
                self.emit(NEWLINE, p, e);
                return;
            }
            34 => {
                self.quoted();
                return;
            }
            123 => {
                self.state = State::Block;
                self.emit(BLOCK_START, p, p + 1);
                return;
            }
            35 => {
                self.emit(BRKT, p, p + 1);
                return;
            }
            44 => {
                self.emit(COMMA, p, p + 1);
                return;
            }
            58 => {
                if self.ch(p + 1) == 58 && self.ch(p + 2) == 58 {
                    self.emit(STYLE_SEPARATOR, p, p + 3);
                } else {
                    self.emit(COLON, p, p + 1);
                }
                return;
            }
            91 => {
                self.emit(SQS, p, p + 1);
                return;
            }
            93 => {
                self.emit(SQE, p, p + 1);
                return;
            }
            _ => {}
        }
        if is_digit(c) {
            self.number(c);
            return;
        }
        let lc = if (65..=90).contains(&c) { c + 32 } else { c };
        if self.keyword(lc) {
            return;
        }
        if c >= 128 || class_bits(c) & TEXT != 0 {
            let mut e = p + 1;
            loop {
                let d = self.ch(e);
                if !(d >= 128 || class_bits(d) & TEXT != 0) {
                    break;
                }
                e += 1;
            }
            self.emit(UNICODE_TEXT, p, e);
            return;
        }
        self.emit(CHAR, p, p + 1);
    }

    fn generic(&mut self) -> bool {
        if self.tilde_next == -1 {
            return false;
        }
        if self.tilde_next < self.p {
            self.tilde_next = self.index_of(126, self.p);
            if self.tilde_next == -1 {
                return false;
            }
        }
        if self.ws_next < self.p {
            let mut q = self.p;
            while q < self.n && !is_space(self.ch(q)) {
                q += 1;
            }
            self.ws_next = q;
        }
        if self.tilde_next >= self.ws_next {
            return false;
        }
        if self.tilde_line < self.p {
            self.tilde_line = self.next_line_end(self.p);
            self.tilde_last = self.last_index_of(126, self.tilde_line - 1);
        }
        if self.tilde_last <= self.tilde_next {
            return false;
        }
        let mut e = self.tilde_last + 1;
        while e < self.n && !is_space(self.ch(e)) {
            e += 1;
        }
        let p = self.p;
        self.emit(ATTRIBUTE_WORD, p, e);
        true
    }

    fn block(&mut self, c: u16) {
        let p = self.p;
        if is_space(c) {
            self.p += 1;
            self.p = self.skip_spaces(self.p);
            return;
        }
        let lc = c | 32;
        if matches!(lc, 112 | 102 | 117) && (self.ch(p + 1) | 32) == 107 && !is_word(self.ch(p + 2))
        {
            self.emit(ATTRIBUTE_KEY, p, p + 2);
            return;
        }
        if self.generic() {
            return;
        }
        if is_attr(c, ATTR_START) {
            let mut e = p + 1;
            while e < self.n && is_attr(self.ch(e), ATTR_REST) {
                e += 1;
            }
            self.emit(ATTRIBUTE_WORD, p, e);
            return;
        }
        if c == 96 {
            self.state = State::BlockBq;
            self.p += 1;
            return;
        }
        if c == 34 {
            let e = self.index_of(34, p + 1);
            if e != -1 {
                self.emit(COMMENT, p, e + 1);
                return;
            }
        } else if c == 125 {
            self.state = State::Initial;
            self.emit(BLOCK_STOP, p, p + 1);
            return;
        }
        self.emit(CHAR, p, p + 1);
    }

    fn style(&mut self, c: u16) -> bool {
        let p = self.p;
        if c == 10 {
            let mut e = p + 1;
            while self.ch(e) == 10 {
                e += 1;
            }
            self.state = State::Initial;
            self.emit(NEWLINE, p, e);
        } else if is_space(c) {
            self.p = self.skip_spaces(p + 1);
        } else if c == 58 {
            self.emit(COLON, p, p + 1);
        } else if c == 44 {
            self.emit(COMMA, p, p + 1);
        } else if c == 35 {
            self.emit(BRKT, p, p + 1);
        } else if c == 59 {
            self.emit(SEMI, p, p + 1);
        } else if c >= 128 || class_bits(c) & BIT_STYLE_TEXT != 0 {
            let mut e = p + 1;
            loop {
                let d = self.ch(e);
                if !(d >= 128 || class_bits(d) & BIT_STYLE_TEXT != 0) {
                    break;
                }
                e += 1;
            }
            self.emit(STYLE_TEXT, p, e);
        } else {
            self.push_raw(ERROR, "", p);
            return false;
        }
        true
    }

    fn finish(self) -> Tokens {
        Tokens {
            types: self.types,
            texts: self.texts,
            starts: self.starts,
        }
    }
}

pub fn tokenize(src: &str) -> Tokens {
    let units: Vec<u16> = src.encode_utf16().collect();
    let n = units.len() as i64;
    let mut lx = Lexer {
        units,
        n,
        state: State::Initial,
        p: 0,
        types: Vec::new(),
        texts: Vec::new(),
        starts: Vec::new(),
        dir_next: -2,
        dir_failed: 0,
        line_end: -1,
        tilde_next: -2,
        tilde_line: -1,
        tilde_last: -1,
        ws_next: -1,
    };
    while lx.p < n {
        let p = lx.p;
        let c = lx.units[p as usize];
        match lx.state {
            State::Initial => lx.initial(c),
            State::Block => lx.block(c),
            State::BlockBq => {
                if c == 96 {
                    lx.state = State::Block;
                    lx.p += 1;
                } else {
                    let e = lx.index_of(96, p);
                    lx.emit(ATTRIBUTE_WORD, p, if e == -1 { n } else { e });
                }
            }
            State::AccTitle | State::AccDescr => {
                let e = lx.index_of(10, p);
                let t = if lx.state == State::AccTitle {
                    ACC_TITLE_VALUE
                } else {
                    ACC_DESCR_VALUE
                };
                lx.emit(t, p, if e == -1 { n } else { e });
                lx.state = State::Initial;
            }
            State::AccDescrMultiline => {
                if c == 125 {
                    lx.state = State::Initial;
                    lx.p += 1;
                } else {
                    let e = lx.index_of(125, p);
                    lx.emit(ACC_DESCR_MULTILINE_VALUE, p, if e == -1 { n } else { e });
                }
            }
            State::Style => {
                if !lx.style(c) {
                    return lx.finish();
                }
            }
        }
    }

    let last = match lx.state {
        State::Initial => EOF,
        State::AccTitle => ACC_TITLE_VALUE,
        State::AccDescr => ACC_DESCR_VALUE,
        State::AccDescrMultiline => ACC_DESCR_MULTILINE_VALUE,
        _ => -1,
    };
    if last != -1 {
        lx.push_raw(last, "", n);
    }
    lx.push_raw(END, "", n);
    lx.finish()
}
