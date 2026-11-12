use super::tokens::*;
use crate::util::chars::{is_js_space, trim};

pub struct Tokens {
    pub types: Vec<usize>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum S {
    Initial,
    Id,
    Alias,
    Line,
    Config,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
}

const ACTOR_START: u8 = 1;
const ACTOR_REST: u8 = 2;
const ID: u8 = 4;
const ID_WIDE: u8 = 8;
const LINE_BIT: u8 = 16;
const MENU: u8 = 32;

fn class_bits(c: u8) -> u8 {
    let ch = c as char;
    let ws = c == 32 || (9..=13).contains(&c);
    let mut bits = 0;
    if !"/\\+()<->:\n,;".contains(ch) {
        bits |= ACTOR_START;
    }
    if !"+<->:\n,;".contains(ch) {
        bits |= ACTOR_REST;
    }
    if !"<>:\n,;@".contains(ch) {
        bits |= ID_WIDE;
        if !ws {
            bits |= ID;
        }
    }
    if !"#\n;".contains(ch) {
        bits |= LINE_BIT;
    }
    if !"/\\+()<>:\n,;-".contains(ch) && !ws {
        bits |= MENU;
    }
    bits
}

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn is_space(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

fn is_word(c: u16) -> bool {
    c < 128 && (char::from(c as u8).is_ascii_alphanumeric() || c == u16::from(b'_'))
}

struct Lexer {
    u: Vec<u16>,
    n: usize,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
    stack: Vec<S>,
    state: S,
    p: usize,
    next_nl: Option<usize>,
}

impl Lexer {
    fn code(&self, i: usize) -> u16 {
        self.u.get(i).copied().unwrap_or(0)
    }

    fn text(&self, s: usize, e: usize) -> String {
        let e = e.min(self.n);
        if s >= e {
            return String::new();
        }
        String::from_utf16_lossy(&self.u[s..e])
    }

    fn emit(&mut self, t: usize, s: usize, e: usize) {
        self.types.push(t);
        self.texts.push(self.text(s, e));
        self.starts.push(s);
        self.p = e;
    }

    fn emit_text(&mut self, t: usize, s: usize, e: usize, text: String) {
        self.types.push(t);
        self.texts.push(text);
        self.starts.push(s);
        self.p = e;
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

    fn eol(&mut self, q: usize) -> usize {
        let needs = match self.next_nl {
            None => true,
            Some(found) => found < q,
        };
        if needs {
            let found = self.u[q.min(self.n)..]
                .iter()
                .position(|&c| c == 10)
                .map_or(self.n, |k| q + k);
            self.next_nl = Some(found);
        }
        self.next_nl.unwrap_or(self.n)
    }

    fn has(&self, q: usize, bit: u8) -> bool {
        let c = self.code(q);
        if c < 128 {
            return class_bits(c as u8) & bit != 0;
        }
        if bit == ID || bit == MENU {
            !is_space(c)
        } else {
            true
        }
    }

    fn ci(&self, q: usize, word: &str) -> bool {
        word.bytes().enumerate().all(|(k, w)| {
            let c = self.code(q + k);
            if w.is_ascii_lowercase() {
                (c | 32) == u16::from(w)
            } else {
                c == u16::from(w)
            }
        })
    }

    fn word(&self, lit: &str) -> bool {
        self.ci(self.p, lit) && !is_word(self.code(self.p + lit.len()))
    }

    fn skip_ws(&self, q: usize) -> usize {
        let mut q = q;
        while q < self.n && is_space(self.u[q]) {
            q += 1;
        }
        q
    }

    fn skip_same_line(&mut self) -> bool {
        let mut q = self.p;
        while q < self.n {
            let c = self.u[q];
            if c == 10 || !is_space(c) {
                break;
            }
            q += 1;
        }
        if q == self.p {
            return false;
        }
        self.p = q;
        true
    }

    fn line_run(&self, q: usize) -> usize {
        let mut q = q;
        while q < self.n && self.has(q, LINE_BIT) {
            q += 1;
        }
        q
    }

    fn begin(&mut self, t: usize, len: usize, next: S) {
        let p = self.p;
        self.emit(t, p, p + len);
        self.push(next);
    }

    fn keyword(&mut self, lit: &str, t: usize, next: Option<S>) -> bool {
        if !self.word(lit) {
            return false;
        }
        let p = self.p;
        self.emit(t, p, p + lit.len());
        if let Some(next) = next {
            self.push(next);
        }
        true
    }

    fn menu(&mut self, lit: &str, t: usize) -> bool {
        if !self.ci(self.p, lit) {
            return false;
        }
        let mut q = self.p + lit.len();
        let mut c = self.code(q);
        if c != 32 && c != 9 {
            return false;
        }
        while c == 32 || c == 9 {
            q += 1;
            c = self.code(q);
        }
        if q >= self.n || !self.has(q, MENU) {
            return false;
        }
        let p = self.p;
        self.emit(t, p, p + lit.len());
        true
    }

    fn arrow_ahead(&self, q: usize) -> bool {
        let c = self.code(q);
        let d = self.code(q + 1);
        if c == u16::from(b'-') {
            if b"xX)\\/-".iter().any(|&x| d == u16::from(x)) {
                return true;
            }
            let e = self.code(q + 2);
            return d == u16::from(b'|') && (e == u16::from(b'\\') || e == u16::from(b'/'));
        }
        if c == u16::from(b'/') || c == u16::from(b'\\') {
            return (d == u16::from(b'|') || d == c) && self.code(q + 2) == u16::from(b'-');
        }
        c == u16::from(b'(') && d == u16::from(b')')
    }

    fn actor(&mut self) {
        let p = self.p;
        let mut q = p + 1;
        while q < self.n && self.has(q, ACTOR_START) {
            q += 1;
        }
        while q < self.n && !self.arrow_ahead(q) {
            let mut r = q;
            while self.code(r) == u16::from(b'-') {
                r += 1;
            }
            if r >= self.n || !self.has(r, ACTOR_REST) {
                break;
            }
            r += 1;
            while r < self.n && self.has(r, ACTOR_REST) {
                r += 1;
            }
            q = r;
        }
        let text = trim(&self.text(p, q)).to_owned();
        self.emit_text(ACTOR, p, q, text);
    }

    fn number(&mut self) -> bool {
        let p = self.p;
        let mut q = p;
        while is_digit(self.code(q)) {
            q += 1;
        }
        let whole = q > p;
        if self.code(q) == u16::from(b'.') && is_digit(self.code(q + 1)) {
            let r = if is_digit(self.code(q + 2)) {
                q + 3
            } else {
                q + 2
            };
            let after = self.code(r);
            if after == 32 || after == 10 {
                self.emit(NUM, p, r);
                return true;
            }
        }
        let after = self.code(q);
        if whole && (after == 32 || after == 10) {
            self.emit(NUM, p, q);
            return true;
        }
        false
    }

    fn dash(&mut self) {
        let p = self.p;
        let d = self.code(p + 1);
        let e = self.code(p + 2);
        let b = |ch: u8| u16::from(ch);
        if d == b(b'>') {
            if e == b(b'>') {
                self.emit(SOLID_ARROW, p, p + 3);
            } else {
                self.emit(SOLID_OPEN_ARROW, p, p + 2);
            }
        } else if d == b(b'x') || d == b(b'X') {
            self.emit(SOLID_CROSS, p, p + 2);
        } else if d == b(b')') {
            self.emit(SOLID_POINT, p, p + 2);
        } else if d == b(b'|') && e == b(b'\\') {
            self.emit(SOLID_ARROW_TOP, p, p + 3);
        } else if d == b(b'|') && e == b(b'/') {
            self.emit(SOLID_ARROW_BOTTOM, p, p + 3);
        } else if d == b(b'\\') && e == b(b'\\') {
            self.emit(STICK_ARROW_TOP, p, p + 3);
        } else if d == b(b'/') && e == b(b'/') {
            self.emit(STICK_ARROW_BOTTOM, p, p + 3);
        } else if d == b(b'-') {
            let f = self.code(p + 3);
            if e == b(b'>') {
                if f == b(b'>') {
                    self.emit(DOTTED_ARROW, p, p + 4);
                } else {
                    self.emit(DOTTED_OPEN_ARROW, p, p + 3);
                }
            } else if e == b(b'x') || e == b(b'X') {
                self.emit(DOTTED_CROSS, p, p + 3);
            } else if e == b(b')') {
                self.emit(DOTTED_POINT, p, p + 3);
            } else if e == b(b'|') && f == b(b'\\') {
                self.emit(SOLID_ARROW_TOP_DOTTED, p, p + 4);
            } else if e == b(b'|') && f == b(b'/') {
                self.emit(SOLID_ARROW_BOTTOM_DOTTED, p, p + 4);
            } else if e == b(b'\\') && f == b(b'\\') {
                self.emit(STICK_ARROW_TOP_DOTTED, p, p + 4);
            } else if e == b(b'/') && f == b(b'/') {
                self.emit(STICK_ARROW_BOTTOM_DOTTED, p, p + 4);
            } else {
                self.emit(MINUS, p, p + 1);
            }
        } else {
            self.emit(MINUS, p, p + 1);
        }
    }

    fn reverse(&mut self, c: u16) -> bool {
        let p = self.p;
        let d = self.code(p + 1);
        if (d != u16::from(b'|') && d != c) || self.code(p + 2) != u16::from(b'-') {
            return false;
        }
        let dotted = self.code(p + 3) == u16::from(b'-');
        let top = c == u16::from(b'/');
        let t = if d == u16::from(b'|') {
            match (dotted, top) {
                (true, true) => SOLID_ARROW_TOP_REVERSE_DOTTED,
                (true, false) => SOLID_ARROW_BOTTOM_REVERSE_DOTTED,
                (false, true) => SOLID_ARROW_TOP_REVERSE,
                (false, false) => SOLID_ARROW_BOTTOM_REVERSE,
            }
        } else {
            match (dotted, top) {
                (true, true) => STICK_ARROW_TOP_REVERSE_DOTTED,
                (true, false) => STICK_ARROW_BOTTOM_REVERSE_DOTTED,
                (false, true) => STICK_ARROW_TOP_REVERSE,
                (false, false) => STICK_ARROW_BOTTOM_REVERSE,
            }
        };
        self.emit(t, p, p + if dotted { 4 } else { 3 });
        true
    }

    fn title(&mut self) -> bool {
        if !self.ci(self.p, "title") {
            return false;
        }
        let p = self.p;
        let mut q = p + 5;
        let legacy = self.code(q) == u16::from(b':');
        if legacy {
            q += 1;
        }
        let mut e = q;
        while e < self.n && self.u[e] != 10 && is_space(self.u[e]) {
            e += 1;
        }
        if e == self.n || self.code(e) == 10 {
            self.p = e;
            return true;
        }
        let sep = self.code(q);
        if !is_space(sep) || sep == 13 || !self.has(q + 1, LINE_BIT) {
            return false;
        }
        let end = self.line_run(q + 1);
        self.emit(if legacy { LEGACY_TITLE } else { TITLE }, p, end);
        true
    }

    fn acc(&mut self) -> bool {
        let descr = self.ci(self.p, "accdescr");
        if !descr && !self.ci(self.p, "acctitle") {
            return false;
        }
        let p = self.p;
        let q = self.skip_ws(p + 8);
        let c = self.code(q);
        if c == u16::from(b':') {
            let e = self.skip_ws(q + 1);
            self.emit(if descr { ACC_DESCR } else { ACC_TITLE }, p, e);
            self.push(if descr { S::AccDescr } else { S::AccTitle });
            return true;
        }
        if descr && c == u16::from(b'{') {
            self.p = self.skip_ws(q + 1);
            self.push(S::AccDescrMultiline);
            return true;
        }
        false
    }

    fn keywords(&mut self, c: u16) -> bool {
        match c | 32 {
            97 => {
                self.keyword("actor", PARTICIPANT_ACTOR, Some(S::Id))
                    || self.keyword("alt", ALT, Some(S::Line))
                    || self.keyword("and", AND, Some(S::Line))
                    || self.keyword("activate", ACTIVATE, Some(S::Id))
                    || self.acc()
                    || self.keyword("autonumber", AUTONUMBER, None)
            }
            98 => {
                self.keyword("box", BOX, Some(S::Line))
                    || self.keyword("break", BREAK, Some(S::Line))
            }
            99 => {
                self.keyword("create", CREATE, None)
                    || self.keyword("critical", CRITICAL, Some(S::Line))
            }
            100 => {
                self.keyword("destroy", DESTROY, Some(S::Id))
                    || self.menu("details", DETAILS)
                    || self.keyword("deactivate", DEACTIVATE, Some(S::Id))
            }
            101 => self.keyword("else", ELSE, Some(S::Line)) || self.keyword("end", END_KW, None),
            108 => {
                self.keyword("loop", LOOP, Some(S::Line))
                    || self.keyword("left of", LEFT_OF, None)
                    || self.menu("links", LINKS)
                    || self.menu("link", LINK)
            }
            110 => self.keyword("note", NOTE, None),
            111 => {
                self.keyword("opt", OPT, Some(S::Line))
                    || self.keyword("option", OPTION, Some(S::Line))
                    || self.keyword("over", OVER, None)
                    || self.keyword("off", OFF, None)
            }
            112 => {
                self.keyword("participant", PARTICIPANT, Some(S::Id))
                    || self.keyword("par", PAR, Some(S::Line))
                    || self.keyword("par_over", PAR_OVER, Some(S::Line))
                    || self.menu("properties", PROPERTIES)
            }
            114 => {
                self.keyword("rect", RECT, Some(S::Line))
                    || self.keyword("right of", RIGHT_OF, None)
            }
            115 => self.keyword("sequencediagram", SD, None),
            116 => self.title(),
            _ => false,
        }
    }

    fn initial(&mut self) {
        let p = self.p;
        let c = self.code(p);
        if c == 10 {
            let mut q = p + 1;
            while self.code(q) == 10 {
                q += 1;
            }
            self.emit(NEWLINE, p, q);
            return;
        }
        if is_space(c) {
            self.p = self.skip_ws(p + 1);
            return;
        }
        if c == u16::from(b'#') || (c == u16::from(b'%') && self.code(p + 1) != u16::from(b'{')) {
            self.p = self.eol(p);
            return;
        }
        if (is_digit(c) || c == u16::from(b'.')) && self.number() {
            return;
        }
        if c < 128 && ((c | 32) as u8).is_ascii_lowercase() && self.keywords(c) {
            return;
        }
        if c == u16::from(b',') {
            self.emit(COMMA, p, p + 1);
        } else if c == u16::from(b';') {
            self.emit(NEWLINE, p, p + 1);
        } else if self.has(p, ACTOR_START) {
            self.actor();
        } else if c == u16::from(b'-') {
            self.dash();
        } else if c == u16::from(b'<') && self.ci(p, "<<->>") {
            self.emit(BIDIRECTIONAL_SOLID_ARROW, p, p + 5);
        } else if c == u16::from(b'<') && self.ci(p, "<<-->>") {
            self.emit(BIDIRECTIONAL_DOTTED_ARROW, p, p + 6);
        } else if (c == u16::from(b'/') || c == u16::from(b'\\')) && self.reverse(c) {
        } else if c == u16::from(b':') {
            let end = self.line_run(p + 1);
            self.emit(TXT, p, end);
        } else if c == u16::from(b'+') {
            self.emit(PLUS, p, p + 1);
        } else if c == u16::from(b'(') && self.code(p + 1) == u16::from(b')') {
            self.emit(CENTRAL, p, p + 2);
        } else {
            self.emit(INVALID, p, p + 1);
        }
    }

    fn id(&mut self) -> bool {
        if self.skip_same_line() {
            return true;
        }
        let p = self.p;
        let c = self.code(p);
        if c == u16::from(b'#') {
            self.p = self.eol(p);
            return true;
        }
        if c == u16::from(b'@') && self.code(p + 1) == u16::from(b'{') {
            self.begin(CONFIG_START, 2, S::Config);
            return true;
        }
        let mut a = p;
        while a < self.n && self.has(a, ID) {
            a += 1;
        }
        if a > p {
            let w = self.skip_ws(a);
            if self.code(w) == u16::from(b'@') && self.code(w + 1) == u16::from(b'{') {
                self.emit(ACTOR, p, a);
                return true;
            }
            if w > a && self.ci(w, "as") && is_space(self.code(w + 2)) {
                self.emit(ACTOR, p, a);
                self.push(S::Alias);
                return true;
            }
        }
        let mut b = a;
        while b < self.n && self.has(b, ID_WIDE) {
            b += 1;
        }
        if b > p {
            let end = self.code(b);
            let mut cut = b;
            if b < self.n && end != 10 && end != u16::from(b';') {
                cut -= 1;
                while cut > p && self.code(cut) != u16::from(b'#') {
                    cut -= 1;
                }
            }
            if cut > p {
                let text = trim(&self.text(p, cut)).to_owned();
                self.emit_text(ACTOR, p, cut, text);
                self.pop();
                return true;
            }
        }
        if self.code(b) == u16::from(b'<') {
            let e = self.eol(p);
            self.emit(INVALID, p, e);
            self.pop();
            return true;
        }
        if c != 10 {
            let e = self.eol(p);
            let text = trim(&self.text(p, e)).to_owned();
            self.emit_text(INVALID, p, e, text);
            self.pop();
            return true;
        }
        false
    }

    fn step(&mut self) -> bool {
        match self.state {
            S::Initial => {
                self.initial();
                true
            }
            S::Id => self.id(),
            S::Alias => {
                if self.skip_same_line() {
                    return true;
                }
                let p = self.p;
                if self.code(p) == u16::from(b'#') {
                    self.p = self.eol(p);
                    return true;
                }
                self.pop();
                self.pop();
                if self.word("as") {
                    self.begin(AS, 2, S::Line);
                } else {
                    self.emit(NEWLINE, p, p);
                }
                true
            }
            S::Line => {
                if self.skip_same_line() {
                    return true;
                }
                let p = self.p;
                if self.code(p) == u16::from(b'#') {
                    self.p = self.eol(p);
                    return true;
                }
                let end = self.line_run(p);
                self.emit(RESTOFLINE, p, end);
                self.pop();
                true
            }
            S::Config => {
                let p = self.p;
                if self.code(p) != u16::from(b'}') {
                    let q = self.u[p..]
                        .iter()
                        .position(|&c| c == u16::from(b'}'))
                        .map_or(self.n, |k| p + k);
                    self.emit(CONFIG_CONTENT, p, q);
                    true
                } else {
                    let w = self.skip_ws(p + 1);
                    let alias = w > p + 1 && self.ci(w, "as") && is_space(self.code(w + 2));
                    self.emit(CONFIG_END, p, p + 1);
                    self.pop();
                    if alias {
                        self.push(S::Alias);
                    } else {
                        self.pop();
                    }
                    true
                }
            }
            S::AccTitle | S::AccDescr => {
                let p = self.p;
                let t = if self.state == S::AccTitle {
                    ACC_TITLE_VALUE
                } else {
                    ACC_DESCR_VALUE
                };
                let e = self.eol(p);
                self.emit(t, p, e);
                self.pop();
                true
            }
            S::AccDescrMultiline => {
                let p = self.p;
                if self.code(p) == u16::from(b'}') {
                    self.p += 1;
                    self.pop();
                } else {
                    let q = self.u[p..]
                        .iter()
                        .position(|&c| c == u16::from(b'}'))
                        .map_or(self.n, |k| p + k);
                    self.emit(ACC_DESCR_MULTILINE_VALUE, p, q);
                }
                true
            }
        }
    }
}

pub fn tokenize(src: &str) -> Tokens {
    let u: Vec<u16> = src.encode_utf16().collect();
    let n = u.len();
    let mut lexer = Lexer {
        u,
        n,
        types: Vec::new(),
        texts: Vec::new(),
        starts: Vec::new(),
        stack: Vec::new(),
        state: S::Initial,
        p: 0,
        next_nl: None,
    };
    while lexer.p < n {
        if !lexer.step() {
            let at = lexer.p;
            lexer.types.push(ERROR);
            lexer.texts.push(String::new());
            lexer.starts.push(at);
            return Tokens {
                types: lexer.types,
                texts: lexer.texts,
                starts: lexer.starts,
            };
        }
    }
    let end = lexer.n;
    match lexer.state {
        S::Initial | S::Alias => lexer.emit(NEWLINE, end, end),
        S::Line => lexer.emit(RESTOFLINE, end, end),
        S::AccTitle => lexer.emit(ACC_TITLE_VALUE, end, end),
        S::AccDescr => lexer.emit(ACC_DESCR_VALUE, end, end),
        S::AccDescrMultiline => lexer.emit(ACC_DESCR_MULTILINE_VALUE, end, end),
        _ => {}
    }
    lexer.types.push(END);
    lexer.texts.push(String::new());
    lexer.starts.push(end);
    Tokens {
        types: lexer.types,
        texts: lexer.texts,
        starts: lexer.starts,
    }
}
