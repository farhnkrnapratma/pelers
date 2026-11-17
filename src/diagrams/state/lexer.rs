use super::tokens::*;
use crate::util::chars::{is_js_space, trim};

pub struct Tokens {
    pub types: Vec<usize>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum S {
    Initial,
    Struct,
    State,
    StateString,
    StateId,
    Scale,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
    ClassDef,
    ClassDefId,
    Class,
    ClassStyle,
    Style,
    StyleDefStyles,
    Note,
    NoteId,
    NoteText,
    FloatingNote,
    FloatingNoteId,
}

const fn cu(ch: u8) -> u16 {
    ch as u16
}

fn is_space(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

fn is_word(c: u16) -> bool {
    c < 128 && crate::util::chars::is_word(char::from(c as u8))
}

fn word_at_ci(units: &[u16], at: usize, word: &str) -> bool {
    word.bytes().enumerate().all(|(k, w)| {
        units
            .get(at + k)
            .is_some_and(|&c| c < 128 && (c as u8).to_ascii_lowercase() == w)
    })
}

fn stereotype_kind(units: &[u16], d: usize) -> Option<usize> {
    let at = |k: usize| units.get(d + k).copied().unwrap_or(0);
    let (side, close) = if at(0) == cu(b'<') && at(1) == cu(b'<') {
        (0, cu(b'>'))
    } else if at(0) == cu(b'[') && at(1) == cu(b'[') {
        (3, cu(b']'))
    } else {
        return None;
    };
    for (k, name) in ["fork", "join", "choice"].iter().enumerate() {
        let end = 2 + name.len();
        if word_at_ci(units, d + 2, name) && at(end) == close && at(end + 1) == close {
            return Some(side + k);
        }
    }
    None
}

fn find_word_ci(units: &[u16], from: usize, word: &str) -> Option<usize> {
    (from..units.len()).find(|&i| word_at_ci(units, i, word))
}

fn find_direction(units: &[u16], from: usize) -> Option<usize> {
    find_word_ci(units, from, "direction")
}

fn find_stereotype(units: &[u16], from: usize) -> Option<usize> {
    (from..units.len()).find(|&d| stereotype_kind(units, d).is_some())
}

fn find_line_end(units: &[u16], from: usize) -> Option<usize> {
    (from..units.len()).find(|&i| matches!(units[i], 10 | 13 | 0x2028 | 0x2029))
}

fn find_percent(units: &[u16], from: usize) -> Option<usize> {
    (from..units.len().saturating_sub(1))
        .find(|&i| units[i] == cu(b'%') && units[i + 1] == cu(b'%'))
}

fn find_brace(units: &[u16], from: usize) -> Option<usize> {
    (from..units.len()).find(|&i| units[i] == cu(b'{'))
}

type Search = fn(&[u16], usize) -> Option<usize>;

struct Finder {
    search: Search,
    from: usize,
    next: Option<Option<usize>>,
}

impl Finder {
    fn new(search: Search) -> Self {
        Self {
            search,
            from: 0,
            next: None,
        }
    }

    fn at(&mut self, units: &[u16], q: usize) -> Option<usize> {
        let stale = match self.next {
            None => true,
            Some(None) => q < self.from,
            Some(Some(found)) => q < self.from || q > found,
        };
        if stale {
            self.next = Some((self.search)(units, q));
            self.from = q;
        }
        self.next.flatten()
    }
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
    error: Option<String>,
    next_direction: Finder,
    next_stereotype: Finder,
    next_line_end: Finder,
    next_percent: Finder,
    next_brace: Finder,
    dir_range: Option<(usize, usize)>,
    dir_at: [Option<usize>; 4],
    dir_end: [usize; 4],
    stereo_range: Option<(usize, usize)>,
    stereo_at: [Option<usize>; 6],
}

impl Lexer {
    fn new(src: &str) -> Self {
        let u: Vec<u16> = src.encode_utf16().collect();
        let n = u.len();
        Self {
            u,
            n,
            types: Vec::new(),
            texts: Vec::new(),
            starts: Vec::new(),
            stack: Vec::new(),
            state: S::Initial,
            p: 0,
            error: None,
            next_direction: Finder::new(find_direction),
            next_stereotype: Finder::new(find_stereotype),
            next_line_end: Finder::new(find_line_end),
            next_percent: Finder::new(find_percent),
            next_brace: Finder::new(find_brace),
            dir_range: None,
            dir_at: [None; 4],
            dir_end: [0; 4],
            stereo_range: None,
            stereo_at: [None; 6],
        }
    }

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

    fn emit_text(&mut self, t: usize, text: String, s: usize, e: usize) {
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

    fn word(&self, w: &str, q: usize) -> bool {
        word_at_ci(&self.u, q, w)
    }

    fn ws_end(&self, q: usize) -> usize {
        let mut j = q;
        while j < self.n && is_space(self.u[j]) {
            j += 1;
        }
        j
    }

    fn word_end(&self, q: usize) -> usize {
        let mut j = q;
        while j < self.n && is_word(self.u[j]) {
            j += 1;
        }
        j
    }

    fn eol(&self, q: usize) -> usize {
        if q >= self.n {
            return self.n;
        }
        self.u[q..]
            .iter()
            .position(|&c| c == 10)
            .map_or(self.n, |k| q + k)
    }

    fn line_end(&mut self, q: usize) -> usize {
        self.next_line_end.at(&self.u, q).unwrap_or(self.n)
    }

    fn direction(&mut self, q: usize) -> Option<usize> {
        let stale = match self.dir_range {
            Some((from, to)) => q < from || q > to,
            None => true,
        };
        if stale {
            let first = self.next_direction.at(&self.u, q);
            let end = match first {
                None => self.n,
                Some(_) => self.line_end(q),
            };
            self.dir_range = Some((q, end));
            self.dir_at = [None; 4];
            let mut d = first;
            while let Some(at) = d {
                if at >= end {
                    break;
                }
                let w = self.ws_end(at + 9);
                if w != at + 9 {
                    let kind = if self.word("tb", w) {
                        Some(0)
                    } else if self.word("bt", w) {
                        Some(1)
                    } else if self.word("rl", w) {
                        Some(2)
                    } else if self.word("lr", w) {
                        Some(3)
                    } else {
                        None
                    };
                    if let Some(kind) = kind {
                        self.dir_at[kind] = Some(at);
                        self.dir_end[kind] = w + 2;
                    }
                }
                d = self.next_direction.at(&self.u, at + 1);
            }
        }
        (0..4).find(|&k| matches!(self.dir_at[k], Some(at) if at >= q))
    }

    fn stereotype(&mut self, q: usize) -> Option<usize> {
        let stale = match self.stereo_range {
            Some((from, to)) => q < from || q > to,
            None => true,
        };
        if stale {
            let first = self.next_stereotype.at(&self.u, q);
            let end = match first {
                None => self.n,
                Some(_) => self.line_end(q),
            };
            self.stereo_range = Some((q, end));
            self.stereo_at = [None; 6];
            let mut d = first;
            while let Some(at) = d {
                if at >= end {
                    break;
                }
                if let Some(kind) = stereotype_kind(&self.u, at) {
                    self.stereo_at[kind] = Some(at);
                }
                d = self.next_stereotype.at(&self.u, at + 1);
            }
        }
        (0..6).find(|&k| matches!(self.stereo_at[k], Some(at) if at >= q))
    }

    fn emit_id(&mut self, s: usize, e: usize) -> bool {
        match self.next_percent.at(&self.u, s) {
            Some(at) if at + 2 <= e => {
                if at == s {
                    self.p = e;
                    return false;
                }
                self.emit(ID, s, at);
                true
            }
            _ => {
                self.emit(ID, s, e);
                true
            }
        }
    }

    fn finish(&mut self, t: usize) -> Tokens {
        self.types.push(t);
        self.texts.push(String::new());
        self.starts.push(self.p);
        Tokens {
            types: std::mem::take(&mut self.types),
            texts: std::mem::take(&mut self.texts),
            starts: std::mem::take(&mut self.starts),
            error: self.error.take(),
        }
    }

    fn end_of_input(&mut self) {
        let n = self.n;
        match self.state {
            S::Initial => self.emit(NL, n, n),
            S::AccTitle => self.emit(ACC_TITLE_VALUE, n, n),
            S::AccDescr => self.emit(ACC_DESCR_VALUE, n, n),
            S::AccDescrMultiline => self.emit(ACC_DESCR_MULTILINE_VALUE, n, n),
            S::ClassDefId => self.emit(CLASSDEF_STYLEOPTS, n, n),
            S::ClassStyle => self.emit(STYLECLASS, n, n),
            S::StyleDefStyles => self.emit(STYLEDEF_STYLEOPTS, n, n),
            S::StateString => self.emit(STATE_DESCR, n, n),
            S::FloatingNote => self.emit(NOTE_TEXT, n, n),
            S::StateId | S::FloatingNoteId => self.emit(ID, n, n),
            _ => {}
        }
    }

    fn run(mut self) -> Tokens {
        loop {
            if self.p >= self.n {
                self.end_of_input();
                return self.finish(END);
            }
            let c = self.code(self.p);
            let done = match self.state {
                S::Initial | S::Struct => self.structure(c),
                S::State => self.state_name(c),
                S::StateString => self.state_string(c),
                S::StateId => self.state_id(),
                S::Scale => self.scale(c),
                S::AccTitle => {
                    self.emit(ACC_TITLE_VALUE, self.p, self.eol(self.p));
                    self.pop();
                    None
                }
                S::AccDescr => {
                    self.emit(ACC_DESCR_VALUE, self.p, self.eol(self.p));
                    self.pop();
                    None
                }
                S::AccDescrMultiline => self.acc_descr_multiline(c),
                S::ClassDef => self.class_def(),
                S::ClassDefId => {
                    self.emit(CLASSDEF_STYLEOPTS, self.p, self.eol(self.p));
                    self.pop();
                    None
                }
                S::Class => self.class_ids(),
                S::ClassStyle => {
                    self.emit(STYLECLASS, self.p, self.eol(self.p));
                    self.pop();
                    None
                }
                S::Style => self.style_ids(),
                S::StyleDefStyles => {
                    self.emit(STYLEDEF_STYLEOPTS, self.p, self.eol(self.p));
                    self.pop();
                    None
                }
                S::Note => self.note(c),
                S::NoteId => self.note_id(),
                S::NoteText => self.note_text(),
                S::FloatingNote => self.floating_note(c),
                S::FloatingNoteId => {
                    if self.emit_id(self.p, self.eol(self.p)) {
                        self.pop();
                    }
                    None
                }
            };
            if let Some(tokens) = done {
                return tokens;
            }
        }
    }

    fn structure(&mut self, c: u16) -> Option<Tokens> {
        let p = self.p;
        let root = self.state == S::Initial;
        let lc = c | 32;
        if root {
            if lc == cu(b'c') && self.word("click", p) && !is_word(self.code(p + 5)) {
                self.emit(CLICK, p, p + 5);
                return None;
            }
            if lc == cu(b'h') && self.word("href", p) && !is_word(self.code(p + 4)) {
                self.emit(HREF, p, p + 4);
                return None;
            }
            if c == cu(b'"')
                && let Some(close) = self.u[p + 1..]
                    .iter()
                    .position(|&x| x == cu(b'"'))
                    .map(|k| p + 1 + k)
            {
                self.emit(STRING, p, close + 1);
                return None;
            }
            if lc == cu(b'd') && self.word("default", p) && !is_word(self.code(p + 7)) {
                self.emit(DEFAULT, p, p + 7);
                return None;
            }
            if let Some(kind) = self.direction(p) {
                let end = self.eol(self.dir_end[kind]);
                self.emit(DIRECTION_TB + kind, p, end);
                return None;
            }
            if c == 10 {
                let mut j = p + 1;
                while self.code(j) == 10 {
                    j += 1;
                }
                self.emit(NL, p, j);
                return None;
            }
            if is_space(c) {
                self.p = self.ws_end(p);
                return None;
            }
        } else if c != 10 && is_space(c) {
            let mut j = p + 1;
            while j < self.n && self.u[j] != 10 && is_space(self.u[j]) {
                j += 1;
            }
            self.p = j;
            return None;
        }
        if c == cu(b'#')
            || (c == cu(b'%') && self.code(p + 1) == cu(b'%') && self.code(p + 2) != cu(b'{'))
        {
            self.p = self.eol(p);
            return None;
        }
        if lc == cu(b's') {
            if root && self.word("scale", p) {
                let e = self.ws_end(p + 5);
                if e > p + 5 {
                    self.emit(SCALE, p, e);
                    self.push(S::Scale);
                    return None;
                }
            }
            if self.word("style", p) {
                let e = self.ws_end(p + 5);
                if e > p + 5 {
                    self.emit(STYLE, p, e);
                    self.push(S::Style);
                    return None;
                }
            }
            if self.word("state", p) {
                let e = self.ws_end(p + 5);
                if e > p + 5 {
                    self.p = e;
                    self.push(S::State);
                    return None;
                }
                if root && self.word("diagram", p + 5) {
                    let from = if self.word("-v2", p + 12) {
                        p + 15
                    } else {
                        p + 12
                    };
                    let e = self.ws_end(from);
                    if e > from {
                        self.emit(SD, p, e);
                        return None;
                    }
                }
            }
        } else if lc == cu(b'c') {
            if self.word("class", p) {
                let def = self.word("def", p + 5);
                let from = if def { p + 8 } else { p + 5 };
                let e = self.ws_end(from);
                if e > from {
                    self.emit(if def { CLASSDEF } else { CLASS }, p, e);
                    self.push(if def { S::ClassDef } else { S::Class });
                    return None;
                }
            }
        } else if lc == cu(b'a') && root && (self.word("acctitle", p) || self.word("accdescr", p)) {
            let title = (self.code(p + 3) | 32) == cu(b't');
            let q = self.ws_end(p + 8);
            let d = self.code(q);
            if d == cu(b':') {
                let e = self.ws_end(q + 1);
                self.emit(if title { ACC_TITLE } else { ACC_DESCR }, p, e);
                self.push(if title { S::AccTitle } else { S::AccDescr });
                return None;
            }
            if d == cu(b'{') && !title {
                self.p = self.ws_end(q + 1);
                self.push(S::AccDescrMultiline);
                return None;
            }
        }
        if root {
            if c == cu(b'{') {
                self.emit(STRUCT_START, p, p + 1);
                self.push(S::Struct);
                return None;
            }
        } else {
            if let Some(kind) = self.direction(p) {
                let end = self.eol(self.dir_end[kind]);
                self.emit(DIRECTION_TB + kind, p, end);
                return None;
            }
            if c == cu(b'}') {
                self.emit(STRUCT_STOP, p, p + 1);
                self.pop();
                return None;
            }
            if c == 10 {
                self.p += 1;
                return None;
            }
        }
        if lc == cu(b'n') && self.word("note", p) {
            let e = self.ws_end(p + 4);
            if e > p + 4 {
                self.emit(NOTE, p, e);
                self.push(S::Note);
                return None;
            }
        }
        if root
            && lc == cu(b'h')
            && self.word("hide empty description", p)
            && !is_word(self.code(p + 22))
        {
            self.emit(HIDE_EMPTY, p, p + 22);
            return None;
        }
        if c == cu(b'[') && self.code(p + 1) == cu(b'*') && self.code(p + 2) == cu(b']') {
            self.emit(EDGE_STATE, p, p + 3);
            return None;
        }
        let mut j = p;
        while j < self.n {
            let d = self.u[j];
            if d == cu(b':') || d == cu(b'-') || d == cu(b'{') || is_space(d) {
                break;
            }
            j += 1;
        }
        if j > p {
            self.emit_id(p, j);
            return None;
        }
        if c == cu(b':') {
            j = p + 1;
            while j < self.n {
                let d = self.u[j];
                if d == 10 || d == cu(b';') {
                    break;
                }
                if d == cu(b':') {
                    let after = self.code(j + 1);
                    if j + 1 >= self.n || after == cu(b':') || after == 10 || after == cu(b';') {
                        break;
                    }
                    j += 1;
                }
                j += 1;
            }
            if j > p + 1 {
                let text = trim(&self.text(p, j)).to_owned();
                self.emit_text(DESCR, text, p, j);
                return None;
            }
            if self.code(p + 1) == cu(b':') && self.code(p + 2) == cu(b':') {
                self.emit(STYLE_SEPARATOR, p, p + 3);
                return None;
            }
        } else if c == cu(b'-') && self.code(p + 1) == cu(b'-') {
            if self.code(p + 2) == cu(b'>') {
                self.emit(ARROW, p, p + 3);
                return None;
            }
            if !root {
                self.emit(CONCURRENT, p, p + 2);
                return None;
            }
        }
        if !root {
            return Some(self.finish(ERROR));
        }
        self.emit(INVALID, p, p + 1);
        None
    }

    fn state_name(&mut self, c: u16) -> Option<Tokens> {
        let p = self.p;
        if c != 10 && is_space(c) {
            let mut j = p + 1;
            while j < self.n && self.u[j] != 10 && is_space(self.u[j]) {
                j += 1;
            }
            self.p = j;
            return None;
        }
        if c == cu(b'#')
            || (c == cu(b'%') && self.code(p + 1) == cu(b'%') && self.code(p + 2) != cu(b'{'))
        {
            self.p = self.eol(p);
            return None;
        }
        if let Some(kind) = self.stereotype(p) {
            let at = self.stereo_at[kind].unwrap_or(p);
            let choice = kind == 2 || kind == 5;
            let text = trim(&self.text(p, at)).to_owned();
            let t = if choice {
                CHOICE
            } else if kind % 3 == 0 {
                FORK
            } else {
                JOIN
            };
            self.emit_text(t, text, p, at + if choice { 10 } else { 8 });
            self.pop();
            return None;
        }
        if c == cu(b'"') {
            self.p += 1;
            self.push(S::StateString);
            return None;
        }
        let q = if c == 10 { self.ws_end(p) } else { p };
        if self.word("as", q) {
            let e = self.ws_end(q + 2);
            if e > q + 2 {
                self.emit(AS, p, e);
                self.push(S::StateId);
                return None;
            }
        }
        if c == 10 {
            self.p += 1;
            self.pop();
            return None;
        }
        if c == cu(b'{') {
            self.emit(STRUCT_START, p, p + 1);
            self.pop();
            self.push(S::Struct);
            return None;
        }
        if is_word(c) {
            let first = self.word_end(p);
            let second = self.ws_end(first);
            if second > first && is_word(self.code(second)) {
                let brace = self.next_brace.at(&self.u, second + 1);
                if let Some(brace) = brace {
                    let line_end = self.line_end(second);
                    if brace < line_end {
                        self.error = Some(format!(
                            "Error: State name must be a single word. Found: \"{}\"",
                            trim(&self.text(p, brace + 1))
                        ));
                        return Some(self.finish(ERROR));
                    }
                }
            }
        }
        let mut j = p + 1;
        while j < self.n {
            let d = self.u[j];
            if d == cu(b'{') || is_space(d) {
                break;
            }
            j += 1;
        }
        self.emit(COMPOSIT_STATE, p, j);
        None
    }

    fn state_string(&mut self, c: u16) -> Option<Tokens> {
        let p = self.p;
        if c == cu(b'"') {
            self.p += 1;
            self.pop();
            return None;
        }
        let close = self.u[p..]
            .iter()
            .position(|&x| x == cu(b'"'))
            .map(|k| p + k);
        self.emit(STATE_DESCR, p, close.unwrap_or(self.n));
        None
    }

    fn state_id(&mut self) -> Option<Tokens> {
        let p = self.p;
        let mut j = p;
        while j < self.n {
            let d = self.u[j];
            if d == 10 || d == cu(b'{') {
                break;
            }
            j += 1;
        }
        if self.emit_id(p, j) {
            self.pop();
        }
        None
    }

    fn scale(&mut self, c: u16) -> Option<Tokens> {
        let p = self.p;
        if (48..=57).contains(&c) {
            let mut j = p + 1;
            while j < self.n && (48..=57).contains(&self.u[j]) {
                j += 1;
            }
            self.emit(WIDTH, p, j);
            return None;
        }
        let q = self.ws_end(p);
        if q > p && self.word("width", q) && !is_word(self.code(q + 5)) {
            self.p = q + 5;
            self.pop();
            return None;
        }
        Some(self.finish(ERROR))
    }

    fn acc_descr_multiline(&mut self, c: u16) -> Option<Tokens> {
        let p = self.p;
        if c == cu(b'}') {
            self.p += 1;
            self.pop();
            return None;
        }
        let close = self.u[p..]
            .iter()
            .position(|&x| x == cu(b'}'))
            .map(|k| p + k);
        self.emit(ACC_DESCR_MULTILINE_VALUE, p, close.unwrap_or(self.n));
        None
    }

    fn class_def(&mut self) -> Option<Tokens> {
        let p = self.p;
        let is_default = self.word("default", p) && self.ws_end(p + 7) > p + 7;
        let j = if is_default { p + 7 } else { self.word_end(p) };
        let e = self.ws_end(j);
        if j == p || e == j {
            return Some(self.finish(ERROR));
        }
        self.emit(
            if is_default {
                DEFAULT_CLASSDEF_ID
            } else {
                CLASSDEF_ID
            },
            p,
            e,
        );
        self.pop();
        self.push(S::ClassDefId);
        None
    }

    fn class_ids(&mut self) -> Option<Tokens> {
        let p = self.p;
        let mut j = self.word_end(p);
        if j == p {
            return Some(self.finish(ERROR));
        }
        while self.code(j) == cu(b',') {
            let k = self.ws_end(j + 1);
            let e = self.word_end(k);
            if e == k {
                break;
            }
            j = e;
        }
        self.emit(CLASSENTITY_IDS, p, j);
        self.pop();
        self.push(S::ClassStyle);
        None
    }

    fn style_ids(&mut self) -> Option<Tokens> {
        let p = self.p;
        let mut j = p;
        while j < self.n && (is_word(self.u[j]) || self.u[j] == cu(b',')) {
            j += 1;
        }
        let e = self.ws_end(j);
        if j == p || e == j {
            return Some(self.finish(ERROR));
        }
        self.emit(STYLE_IDS, p, e);
        self.pop();
        self.push(S::StyleDefStyles);
        None
    }

    fn note(&mut self, c: u16) -> Option<Tokens> {
        let p = self.p;
        if self.word("left of", p) && !is_word(self.code(p + 7)) {
            self.emit(LEFT_OF, p, p + 7);
            self.pop();
            self.push(S::NoteId);
            return None;
        }
        if self.word("right of", p) && !is_word(self.code(p + 8)) {
            self.emit(RIGHT_OF, p, p + 8);
            self.pop();
            self.push(S::NoteId);
            return None;
        }
        if c == cu(b'"') {
            self.p += 1;
            self.pop();
            self.push(S::FloatingNote);
            return None;
        }
        Some(self.finish(ERROR))
    }

    fn note_id(&mut self) -> Option<Tokens> {
        let p = self.p;
        let q = self.ws_end(p);
        let mut j = q;
        while j < self.n {
            let d = self.u[j];
            if d == cu(b':') || d == cu(b'-') || is_space(d) {
                break;
            }
            j += 1;
        }
        if j == q {
            return Some(self.finish(ERROR));
        }
        if self.emit_id(p, j) {
            self.pop();
            self.push(S::NoteText);
        }
        None
    }

    fn note_text(&mut self) -> Option<Tokens> {
        let p = self.p;
        let q = self.ws_end(p);
        if self.code(q) == cu(b':') {
            let mut j = q + 1;
            while j < self.n {
                let d = self.u[j];
                if d == cu(b':') || d == 10 || d == cu(b';') {
                    break;
                }
                j += 1;
            }
            if j > q + 1 {
                let text = trim(&self.text(p + 2, j)).to_owned();
                self.emit_text(NOTE_TEXT, text, p, j);
                self.pop();
                return None;
            }
        }
        let mut k = self.u[p..].iter().position(|&x| x == 10).map(|k| p + k);
        let mut e = None;
        while let Some(at) = k {
            let next = self.ws_end(at);
            e = Some(next);
            if self.word("end note", next) && !is_word(self.code(next + 8)) {
                break;
            }
            k = self.u[next..]
                .iter()
                .position(|&x| x == 10)
                .map(|k| next + k);
        }
        let (Some(e), Some(_)) = (e, k) else {
            return Some(self.finish(ERROR));
        };
        let text = trim(&self.text(p, e)).to_owned();
        self.emit_text(NOTE_TEXT, text, p, e + 8);
        self.pop();
        None
    }

    fn floating_note(&mut self, c: u16) -> Option<Tokens> {
        let p = self.p;
        let q = self.ws_end(p);
        if self.word("as", q) {
            let e = self.ws_end(q + 2);
            self.emit(AS, p, e);
            self.pop();
            self.push(S::FloatingNoteId);
            return None;
        }
        if c == cu(b'"') {
            self.p += 1;
            return None;
        }
        let close = self.u[p..]
            .iter()
            .position(|&x| x == cu(b'"'))
            .map(|k| p + k);
        self.emit(NOTE_TEXT, p, close.unwrap_or(self.n));
        None
    }
}

pub fn tokenize(src: &str) -> Tokens {
    Lexer::new(src).run()
}
