use super::tokens::*;

pub struct Tokens {
    pub types: Vec<usize>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    Str,
    MdString,
    Node,
    BlockArrow,
    ArrowDir,
    LLabel,
    ClassDef,
    ClassDefId,
    Class,
    ClassStyle,
    StyleStmnt,
    StyleDefinition,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
}

const DIRECTIONS: [&str; 6] = ["right", "left", "x", "y", "up", "down"];

fn is_id_stop(c: u16) -> bool {
    c < 128
        && matches!(
            c,
            9..=13 | 32 | 40 | 41 | 45 | 58 | 60 | 61 | 62 | 91 | 123 | 125
        )
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

fn is_word(c: u16) -> bool {
    (48..=57).contains(&c) || (65..=90).contains(&c) || (97..=122).contains(&c) || c == 95
}

struct Lexer {
    units: Vec<u16>,
    n: i64,
    p: i64,
    state: State,
    stack: Vec<State>,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
}

impl Lexer {
    fn ch(&self, q: i64) -> u16 {
        if q < 0 || q >= self.n {
            0
        } else {
            self.units[q as usize]
        }
    }

    fn slice(&self, s: i64, e: i64) -> String {
        String::from_utf16_lossy(&self.units[s as usize..e.clamp(s, self.n) as usize])
    }

    fn index_of(&self, c: u16, from: i64) -> i64 {
        let mut q = from.max(0);
        while q < self.n {
            if self.units[q as usize] == c {
                return q;
            }
            q += 1;
        }
        -1
    }

    fn put(&mut self, t: usize, text: String, end: i64) -> bool {
        self.types.push(t);
        self.texts.push(text);
        self.starts.push(self.p as usize);
        self.p = end;
        true
    }

    fn emit(&mut self, t: usize, end: i64) -> bool {
        let text = self.slice(self.p, end);
        self.put(t, text, end)
    }

    fn push(&mut self, s: State) {
        self.stack.push(self.state);
        self.state = s;
    }

    fn pop(&mut self) {
        if let Some(s) = self.stack.pop() {
            self.state = s;
        }
    }

    fn ws(&self, mut q: i64) -> i64 {
        while q < self.n && is_space(self.ch(q)) {
            q += 1;
        }
        q
    }

    fn digits(&self, mut q: i64) -> i64 {
        while q < self.n {
            let c = self.ch(q);
            if !(48..=57).contains(&c) {
                break;
            }
            q += 1;
        }
        q
    }

    fn word(&self, mut q: i64) -> i64 {
        while q < self.n && is_word(self.ch(q)) {
            q += 1;
        }
        q
    }

    fn starts_with(&self, q: i64, text: &str) -> bool {
        text.bytes()
            .enumerate()
            .all(|(i, b)| self.ch(q + i as i64) == u16::from(b))
    }

    fn keyword(&self, text: &str) -> bool {
        self.starts_with(self.p, text) && !is_word(self.ch(self.p + text.len() as i64))
    }

    fn quote(&mut self) -> bool {
        if self.ch(self.p + 1) == 96 {
            self.push(State::MdString);
            self.p += 2;
        } else {
            self.push(State::Str);
            self.p += 1;
        }
        true
    }

    fn dotted(&self, mut q: i64) -> i64 {
        if self.ch(q) != 46 {
            return -1;
        }
        loop {
            q += 1;
            if self.ch(q) != 46 {
                break;
            }
        }
        if self.ch(q) != 45 {
            return -1;
        }
        let c = self.ch(q + 1);
        if c == 120 || c == 111 || c == 62 {
            q + 2
        } else {
            q + 1
        }
    }

    fn link_end(&self, mut q: i64) -> i64 {
        let c = self.ch(q);
        if c == 120 || c == 111 || c == 60 {
            q += 1;
        }
        let line = self.ch(q);
        if line == 46 {
            return self.dotted(q);
        }
        if line != 45 && line != 61 {
            return -1;
        }
        let mut k = q + 1;
        while self.ch(k) == line {
            k += 1;
        }
        if k - q == 1 {
            return if line == 45 { self.dotted(k) } else { -1 };
        }
        let head = self.ch(k);
        if head == 120 || head == 111 || head == 62 {
            return k + 1;
        }
        if k - q >= 3 { k } else { -1 }
    }

    fn link(&mut self) -> bool {
        let e = self.link_end(self.p);
        if e >= 0 {
            let end = self.ws(e);
            return self.emit(LINK, end);
        }
        let q = if self.ch(self.p) == 60 {
            self.p + 1
        } else {
            self.p
        };
        let a = self.ch(q);
        let b = self.ch(q + 1);
        if (a == 45 && (b == 45 || b == 46)) || (a == 61 && b == 61) {
            self.push(State::LLabel);
            let end = self.ws(q + 2);
            return self.emit(START_LINK, end);
        }
        false
    }

    fn line_state(&mut self, t: usize) -> bool {
        let mut e = self.index_of(10, self.p);
        if e == -1 {
            e = self.n;
        }
        self.pop();
        self.emit(t, e)
    }

    fn id_list(&mut self, t: usize, next: State) -> bool {
        let mut k = self.word(self.p);
        if k == self.p {
            return false;
        }
        while self.ch(k) == 44 {
            let from = self.ws(k + 1);
            let to = self.word(from);
            if to == from {
                break;
            }
            k = to;
        }
        self.state = next;
        self.emit(t, k)
    }

    fn initial(&mut self, c: u16) -> bool {
        let p = self.p;
        if is_space(c) {
            self.p = self.ws(p + 1);
            return true;
        }
        match c {
            98 => {
                if self.starts_with(p, "block") {
                    if self.keyword("block-beta") {
                        return self.emit(KEY, p + 10);
                    }
                    if self.ch(p + 5) == 58 {
                        return self.emit(ID_BLOCK, p + 6);
                    }
                    if !is_word(self.ch(p + 5)) {
                        return self.emit(KEY, p + 5);
                    }
                }
            }
            99 => {
                if self.starts_with(p, "columns") {
                    let q = self.ws(p + 7);
                    if q > p + 7 {
                        if self.starts_with(q, "auto") && !is_word(self.ch(q + 4)) {
                            return self.put(COLUMNS, "-1".to_owned(), q + 4);
                        }
                        let e = self.digits(q);
                        if e > q {
                            let text = self.slice(q, e);
                            return self.put(COLUMNS, text, e);
                        }
                    }
                } else if self.starts_with(p, "class") {
                    let def = self.starts_with(p + 5, "Def");
                    let from = if def { p + 8 } else { p + 5 };
                    let q = self.ws(from);
                    if q > from {
                        self.push(if def { State::ClassDef } else { State::Class });
                        return self.emit(if def { CLASSDEF } else { CLASS }, q);
                    }
                }
            }
            34 => return self.quote(),
            115 => {
                if self.starts_with(p, "space") {
                    if self.ch(p + 5) == 58 {
                        let e = self.digits(p + 6);
                        if e > p + 6 {
                            let text = self.slice(p + 6, e);
                            return self.put(SPACE_BLOCK, text, e);
                        }
                    }
                    if !is_word(self.ch(p + 5)) {
                        return self.put(SPACE_BLOCK, "1".to_owned(), p + 5);
                    }
                } else if self.starts_with(p, "style") {
                    let q = self.ws(p + 5);
                    if q > p + 5 {
                        self.push(State::StyleStmnt);
                        return self.emit(STYLE, q);
                    }
                }
            }
            100 => {
                if self.keyword("default") {
                    return self.emit(DEFAULT, p + 7);
                }
            }
            108 => {
                if self.keyword("linkStyle") {
                    return self.emit(LINKSTYLE, p + 9);
                }
            }
            105 => {
                if self.keyword("interpolate") {
                    return self.emit(INTERPOLATE, p + 11);
                }
            }
            97 => {
                if self.starts_with(p, "accTitle") {
                    let q = self.ws(p + 8);
                    if self.ch(q) == 58 {
                        self.push(State::AccTitle);
                        let end = self.ws(q + 1);
                        return self.emit(ACC_TITLE, end);
                    }
                } else if self.starts_with(p, "accDescr") {
                    let q = self.ws(p + 8);
                    let d = self.ch(q);
                    if d == 58 {
                        self.push(State::AccDescr);
                        let end = self.ws(q + 1);
                        return self.emit(ACC_DESCR, end);
                    }
                    if d == 123 {
                        self.push(State::AccDescrMultiline);
                        self.p = self.ws(q + 1);
                        return true;
                    }
                }
            }
            101 => {
                if self.keyword("end") {
                    let end = self.ws(p + 3);
                    return self.emit(END_KW, end);
                }
            }
            45 => {
                if self.ch(p + 1) == 41 {
                    self.push(State::Node);
                    return self.emit(NODE_DSTART, p + 2);
                }
                return self.link();
            }
            61 => return self.link(),
            60 => {
                if self.ch(p + 1) == 91 {
                    self.push(State::BlockArrow);
                    return self.emit(BLOCK_ARROW_START, p + 2);
                }
                return self.link();
            }
            40 => {
                let d = self.ch(p + 1);
                let len = if d == 45 || d == 91 {
                    2
                } else if d == 40 {
                    if self.ch(p + 2) == 40 { 3 } else { 2 }
                } else {
                    1
                };
                self.push(State::Node);
                return self.emit(NODE_DSTART, p + len);
            }
            41 | 123 => {
                self.push(State::Node);
                let end = if self.ch(p + 1) == c { p + 2 } else { p + 1 };
                return self.emit(NODE_DSTART, end);
            }
            62 => {
                self.push(State::Node);
                return self.emit(NODE_DSTART, p + 1);
            }
            91 => {
                let d = self.ch(p + 1);
                self.push(State::Node);
                let end = if matches!(d, 91 | 124 | 40 | 92 | 47) {
                    p + 2
                } else {
                    p + 1
                };
                return self.emit(NODE_DSTART, end);
            }
            58 => {
                let e = self.digits(p + 1);
                if e > p + 1 {
                    let text = self.slice(p + 1, e);
                    return self.put(SIZE, text, e);
                }
                return false;
            }
            _ => {}
        }

        let mut q = p;
        while q < self.n {
            let ch = self.ch(q);
            let stop = if ch < 128 {
                is_id_stop(ch)
            } else {
                is_space(ch)
            };
            if stop {
                break;
            }
            q += 1;
        }
        q > p && self.emit(NODE_ID, q)
    }

    fn node_end(&mut self, c: u16) -> bool {
        let p = self.p;
        let d = self.ch(p + 1);
        let len: i64 = match c {
            34 => return self.quote(),
            40 => {
                if d == 45 {
                    2
                } else if d == 40 {
                    if self.ch(p + 2) == 40 { 3 } else { 2 }
                } else {
                    1
                }
            }
            41 => {
                if d == 41 {
                    if self.ch(p + 2) == 41 { 3 } else { 2 }
                } else if d == 93 {
                    2
                } else {
                    1
                }
            }
            125 => {
                if d == 125 {
                    2
                } else {
                    1
                }
            }
            45 => {
                if d == 41 {
                    2
                } else {
                    0
                }
            }
            93 => {
                if matches!(d, 93 | 41 | 62) {
                    2
                } else {
                    1
                }
            }
            92 | 47 => i64::from(d == 93) * 2,
            _ => 0,
        };
        if len == 0 {
            return false;
        }
        self.pop();
        self.emit(NODE_DEND, p + len)
    }

    fn arrow_dir(&mut self, c: u16) -> bool {
        if c == 41 {
            self.pop();
            self.pop();
            let end = self.ws(self.p + 1);
            return self.put(BLOCK_ARROW_END, "]>".to_owned(), end);
        }
        let from = self.ws(if c == 44 { self.p + 1 } else { self.p });
        for dir in DIRECTIONS {
            if self.starts_with(from, dir) {
                let e = self.ws(from + dir.len() as i64);
                let text = self.slice(if c == 44 { from } else { self.p }, e);
                return self.put(DIR, text, e);
            }
        }
        false
    }
}

pub fn tokenize(src: &str) -> Tokens {
    let units: Vec<u16> = src.encode_utf16().collect();
    let n = units.len() as i64;
    let mut lx = Lexer {
        units,
        n,
        p: 0,
        state: State::Initial,
        stack: Vec::new(),
        types: Vec::new(),
        texts: Vec::new(),
        starts: Vec::new(),
    };
    loop {
        if lx.p >= n {
            let ty = match lx.state {
                State::Initial => Some(EOF),
                State::Str => Some(STR),
                State::ClassDefId => Some(CLASSDEF_STYLEOPTS),
                State::ClassStyle => Some(STYLECLASS),
                State::StyleDefinition => Some(STYLE_DEFINITION_DATA),
                State::AccTitle => Some(ACC_TITLE_VALUE),
                State::AccDescr => Some(ACC_DESCR_VALUE),
                State::AccDescrMultiline => Some(ACC_DESCR_MULTILINE_VALUE),
                _ => None,
            };
            lx.p = n;
            if let Some(t) = ty {
                lx.put(t, String::new(), n);
            }
            break;
        }

        let p = lx.p;
        let c = lx.ch(p);
        let ok = match lx.state {
            State::Initial => lx.initial(c),
            State::Str => {
                if c == 34 {
                    lx.pop();
                    lx.p += 1;
                } else {
                    let e = lx.index_of(34, p);
                    lx.emit(STR, if e == -1 { n } else { e });
                }
                true
            }
            State::MdString => {
                if c == 96 {
                    let ok = lx.ch(p + 1) == 34;
                    if ok {
                        lx.pop();
                        lx.p += 2;
                    }
                    ok
                } else if c == 34 {
                    false
                } else {
                    let mut q = p + 1;
                    while q < n {
                        let ch = lx.ch(q);
                        if ch == 96 || ch == 34 {
                            break;
                        }
                        q += 1;
                    }
                    lx.emit(MD_STR, q);
                    true
                }
            }
            State::Node => lx.node_end(c),
            State::BlockArrow => {
                if c == 34 {
                    lx.quote();
                    true
                } else if c == 93 && lx.ch(p + 1) == 62 {
                    let q = lx.ws(p + 2);
                    let ok = lx.ch(q) == 40;
                    if ok {
                        lx.push(State::ArrowDir);
                        lx.p = q + 1;
                    }
                    ok
                } else {
                    false
                }
            }
            State::ArrowDir => lx.arrow_dir(c),
            State::LLabel => {
                if c == 34 {
                    if lx.ch(p + 1) == 96 {
                        lx.quote();
                    } else {
                        lx.emit(LINK_LABEL, p + 1);
                        lx.push(State::Str);
                    }
                    true
                } else {
                    let e = lx.link_end(lx.ws(p));
                    let ok = e >= 0;
                    if ok {
                        lx.pop();
                        let end = lx.ws(e);
                        lx.emit(LINK, end);
                    }
                    ok
                }
            }
            State::ClassDef => {
                let mut ty = DEFAULT_CLASSDEF_ID;
                let mut k = if lx.starts_with(p, "DEFAULT") {
                    p + 7
                } else {
                    p
                };
                let mut q = lx.ws(k);
                if k == p || q == k {
                    ty = CLASSDEF_ID;
                    k = lx.word(p);
                    q = lx.ws(k);
                }
                let ok = k > p && q > k;
                if ok {
                    lx.state = State::ClassDefId;
                    lx.emit(ty, q);
                }
                ok
            }
            State::ClassDefId => {
                lx.line_state(CLASSDEF_STYLEOPTS);
                true
            }
            State::Class => lx.id_list(CLASSENTITY_IDS, State::ClassStyle),
            State::ClassStyle => {
                lx.line_state(STYLECLASS);
                true
            }
            State::StyleStmnt => lx.id_list(STYLE_ENTITY_IDS, State::StyleDefinition),
            State::StyleDefinition => {
                lx.line_state(STYLE_DEFINITION_DATA);
                true
            }
            State::AccTitle => {
                lx.line_state(ACC_TITLE_VALUE);
                true
            }
            State::AccDescr => {
                lx.line_state(ACC_DESCR_VALUE);
                true
            }
            State::AccDescrMultiline => {
                if c == 125 {
                    lx.pop();
                    lx.p += 1;
                } else {
                    let e = lx.index_of(125, p);
                    lx.emit(ACC_DESCR_MULTILINE_VALUE, if e == -1 { n } else { e });
                }
                true
            }
        };
        if !ok {
            lx.types.push(ERROR);
            lx.texts.push(String::new());
            lx.starts.push(lx.p as usize);
            return Tokens {
                types: lx.types,
                texts: lx.texts,
                starts: lx.starts,
            };
        }
    }

    lx.types.push(END);
    lx.texts.push(String::new());
    lx.starts.push(n as usize);
    Tokens {
        types: lx.types,
        texts: lx.texts,
        starts: lx.starts,
    }
}
