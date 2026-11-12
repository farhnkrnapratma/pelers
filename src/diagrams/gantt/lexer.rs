use super::tokens::*;
use crate::util::chars::is_js_space;

pub struct Tokens {
    pub types: Vec<usize>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
    Href,
    CallbackName,
    CallbackArgs,
    Click,
}

const DAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

fn is_word(c: u16) -> bool {
    (48..=57).contains(&c) || (65..=90).contains(&c) || (97..=122).contains(&c) || c == 95
}

fn is_space(c: u16) -> bool {
    if c < 128 {
        c == 32 || (9..=13).contains(&c)
    } else {
        char::from_u32(u32::from(c)).is_some_and(is_js_space)
    }
}

struct Lexer {
    units: Vec<u16>,
    n: i64,
    p: i64,
    state: State,
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

    fn text(&self, s: i64, e: i64) -> String {
        String::from_utf16_lossy(&self.units[s as usize..e.clamp(s, self.n) as usize])
    }

    fn emit(&mut self, t: usize, s: i64, e: i64) {
        let text = self.text(s, e);
        self.types.push(t);
        self.texts.push(text);
        self.starts.push(s as usize);
        self.p = e;
    }

    fn until(&self, c: u16, from: i64) -> i64 {
        let mut q = from.max(0);
        while q < self.n {
            if self.units[q as usize] == c {
                return q;
            }
            q += 1;
        }
        self.n
    }

    fn word_at(&self, q: i64, word: &str) -> bool {
        word.bytes().enumerate().all(|(i, b)| {
            let c = self.ch(q + i as i64);
            let b = u16::from(b);
            c == b || ((97..=122).contains(&b) && (c | 32) == b)
        })
    }

    fn skip_spaces(&self, mut q: i64) -> i64 {
        while is_space(self.ch(q)) {
            q += 1;
        }
        q
    }

    fn run(&self, from: i64, stop: &[u16]) -> Option<i64> {
        let mut q = from;
        while q < self.n && !stop.contains(&self.units[q as usize]) {
            q += 1;
        }
        (q > from).then_some(q)
    }

    fn acc_title_end(&self, p: i64) -> Option<i64> {
        if !self.word_at(p, "acctitle") {
            return None;
        }
        let colon = self.skip_spaces(p + 8);
        (self.ch(colon) == 58).then(|| self.skip_spaces(colon + 1))
    }

    fn acc_descr_end(&self, p: i64) -> Option<i64> {
        if !self.word_at(p, "accdescr") {
            return None;
        }
        let colon = self.skip_spaces(p + 8);
        (self.ch(colon) == 58).then(|| self.skip_spaces(colon + 1))
    }

    fn acc_descr_block_end(&self, p: i64) -> Option<i64> {
        if !self.word_at(p, "accdescr") {
            return None;
        }
        let brace = self.skip_spaces(p + 8);
        (self.ch(brace) == 123).then(|| self.skip_spaces(brace + 1))
    }

    fn word_then_space(&self, p: i64, word: &str) -> Option<i64> {
        if !self.word_at(p, word) {
            return None;
        }
        let q = p + word.len() as i64;
        if !is_space(self.ch(q)) {
            return None;
        }
        Some(self.skip_spaces(q))
    }

    fn word_boundary(&self, p: i64, word: &str) -> Option<i64> {
        if !self.word_at(p, word) {
            return None;
        }
        let q = p + word.len() as i64;
        (!is_word(self.ch(q))).then_some(q)
    }

    fn word_space_run(&self, p: i64, word: &str, stop: &[u16]) -> Option<i64> {
        if !self.word_at(p, word) {
            return None;
        }
        let q = p + word.len() as i64;
        if !is_space(self.ch(q)) {
            return None;
        }
        self.run(q + 1, stop)
    }

    fn weekday_end(&self, p: i64) -> Option<(i64, usize)> {
        let after = self.word_then_space(p, "weekday")?;
        for (index, day) in DAYS.iter().enumerate() {
            if self.word_at(after, day) {
                let end = after + day.len() as i64;
                if !is_word(self.ch(end)) {
                    return Some((end, index));
                }
            }
        }
        None
    }

    fn weekend_end(&self, p: i64) -> Option<(i64, bool)> {
        let after = self.word_then_space(p, "weekend")?;
        if self.word_at(after, "friday") {
            let end = after + 6;
            if !is_word(self.ch(end)) {
                return Some((end, true));
            }
        }
        if self.word_at(after, "saturday") {
            let end = after + 8;
            if !is_word(self.ch(end)) {
                return Some((end, false));
            }
        }
        None
    }

    fn bare_title_end(&self, p: i64) -> Option<i64> {
        if !self.word_at(p, "title") {
            return None;
        }
        let mut q = p + 5;
        while is_space(self.ch(q)) && self.ch(q) != 10 && self.ch(q) != 13 {
            q += 1;
        }
        (q >= self.n || matches!(self.ch(q), 10 | 13)).then_some(q)
    }

    fn keyword(&mut self, c: u16) -> bool {
        let p = self.p;
        let lc = c | 32;
        match lc {
            97 => {
                if let Some(e) = self.acc_title_end(p) {
                    self.state = State::AccTitle;
                    self.emit(ACC_TITLE, p, e);
                    return true;
                }
                if let Some(e) = self.acc_descr_end(p) {
                    self.state = State::AccDescr;
                    self.emit(ACC_DESCR, p, e);
                    return true;
                }
                if let Some(e) = self.acc_descr_block_end(p) {
                    self.state = State::AccDescrMultiline;
                    self.p = e;
                    return true;
                }
            }
            104 => {
                if let Some(e) = self.word_then_space(p, "href")
                    && self.ch(e) == 34
                {
                    self.state = State::Href;
                    self.p = e + 1;
                    return true;
                }
            }
            99 => {
                if let Some(e) = self.word_then_space(p, "call") {
                    self.state = State::CallbackName;
                    self.p = e;
                    return true;
                }
                if let Some(e) = self.word_then_space(p, "click") {
                    self.state = State::Click;
                    self.p = e;
                    return true;
                }
            }
            116 => {
                if let Some(e) = self.bare_title_end(p) {
                    self.p = e;
                    return true;
                }
            }
            119 => {
                if let Some((end, index)) = self.weekday_end(p) {
                    self.emit(WEEKDAY_MONDAY + index, p, end);
                    return true;
                }
                if let Some((end, friday)) = self.weekend_end(p) {
                    self.emit(
                        if friday {
                            WEEKEND_FRIDAY
                        } else {
                            WEEKEND_SATURDAY
                        },
                        p,
                        end,
                    );
                    return true;
                }
            }
            _ => {}
        }
        let rules: &[(&str, usize)] = match lc {
            97 => &[
                ("axisformat", AXISFORMAT),
                ("accdescription", ACCDESCRIPTION),
            ],
            103 => &[("gantt", GANTT)],
            100 => &[("dateformat", DATEFORMAT)],
            105 => &[
                ("inclusiveenddates", INCLUSIVEENDDATES),
                ("includes", INCLUDES),
            ],
            116 => &[
                ("topaxis", TOPAXIS),
                ("tickinterval", TICKINTERVAL),
                ("todaymarker", TODAYMARKER),
                ("title", TITLE),
            ],
            101 => &[("excludes", EXCLUDES)],
            115 => &[("section", SECTION)],
            _ => &[],
        };
        for &(word, t) in rules {
            let end = match (word, t) {
                ("gantt", _) | ("inclusiveenddates", _) | ("topaxis", _) => {
                    self.word_boundary(p, word)
                }
                ("todaymarker", _) => self.word_space_run(p, word, &[10, 59]),
                ("title", _) => {
                    if self.word_at(p, word)
                        && is_space(self.ch(p + 5))
                        && !matches!(self.ch(p + 5), 10 | 13)
                    {
                        self.run(p + 6, &[10]).map(|end| end.max(p + 6))
                    } else {
                        None
                    }
                }
                ("section", _) => {
                    if self.word_at(p, word) && is_space(self.ch(p + 7)) {
                        self.run(p + 8, &[10])
                    } else {
                        None
                    }
                }
                _ => self.word_space_run(p, word, &[35, 10, 59]),
            };
            if let Some(end) = end {
                self.emit(t, p, end);
                return true;
            }
        }
        false
    }

    fn quote_free_task(&mut self) {
        let p = self.p;
        let mut e = p + 1;
        while e < self.n {
            let ch = self.ch(e);
            if ch == 58 || ch == 10 {
                break;
            }
            e += 1;
        }
        self.emit(TASKTXT, p, e);
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
        types: Vec::new(),
        texts: Vec::new(),
        starts: Vec::new(),
    };
    loop {
        if lx.p >= n {
            let t = match lx.state {
                State::Initial => EOF,
                State::AccTitle => ACC_TITLE_VALUE,
                State::AccDescr => ACC_DESCR_VALUE,
                State::AccDescrMultiline => ACC_DESCR_MULTILINE_VALUE,
                State::Href => HREF,
                State::CallbackName => CALLBACKNAME,
                State::CallbackArgs => CALLBACKARGS,
                State::Click => CLICK,
            };
            lx.emit(t, n, n);
            break;
        }
        let p = lx.p;
        let c = lx.ch(p);
        match lx.state {
            State::Initial => {
                let d = lx.ch(p + 1);
                if c == 37 {
                    if d != 37 {
                        let mut e = lx.until(10, p + 1);
                        while lx.ch(e) == 10 {
                            e += 1;
                        }
                        lx.p = e;
                    } else if lx.ch(p + 2) == 123 {
                        lx.emit(OPEN_DIRECTIVE, p, p + 3);
                        lx.types.push(ERROR);
                        lx.texts.push(String::new());
                        lx.starts.push(p as usize);
                        return Tokens {
                            types: lx.types,
                            texts: lx.texts,
                            starts: lx.starts,
                        };
                    } else {
                        lx.p = lx.until(10, p + 2);
                    }
                } else if c == 10 {
                    let mut e = p + 1;
                    while lx.ch(e) == 10 {
                        e += 1;
                    }
                    lx.emit(NL, p, e);
                } else if is_space(c) {
                    let mut q = p + 1;
                    while q < n && is_space(lx.ch(q)) {
                        q += 1;
                    }
                    lx.p = q;
                } else if lx.keyword(c) {
                } else if (48..=57).contains(&c) && date_at(&lx, p) {
                    lx.emit(DATE, p, p + 10);
                } else if c != 58 {
                    lx.quote_free_task();
                } else {
                    let mut e = p + 1;
                    while e < n {
                        let ch = lx.ch(e);
                        if ch == 35 || ch == 10 || ch == 59 {
                            break;
                        }
                        e += 1;
                    }
                    let t = if e > p + 1 { TASKDATA } else { COLON };
                    lx.emit(t, p, e);
                }
            }
            State::AccTitle | State::AccDescr => {
                let t = if lx.state == State::AccTitle {
                    ACC_TITLE_VALUE
                } else {
                    ACC_DESCR_VALUE
                };
                lx.state = State::Initial;
                let e = lx.until(10, p);
                lx.emit(t, p, e);
            }
            State::AccDescrMultiline => {
                if c == 125 {
                    lx.state = State::Initial;
                    lx.p += 1;
                } else {
                    let e = lx.until(125, p);
                    lx.emit(ACC_DESCR_MULTILINE_VALUE, p, e);
                }
            }
            State::Href => {
                if c == 34 {
                    lx.state = State::Initial;
                    lx.p += 1;
                } else {
                    let e = lx.until(34, p);
                    lx.emit(HREF, p, e);
                }
            }
            State::CallbackName => {
                if c == 40 {
                    let close = lx.skip_spaces(p + 1);
                    if lx.ch(close) == 41 {
                        lx.state = State::Initial;
                        lx.p = close + 1;
                    } else {
                        lx.state = State::CallbackArgs;
                        lx.p += 1;
                    }
                } else {
                    let e = lx.until(40, p);
                    lx.emit(CALLBACKNAME, p, e);
                }
            }
            State::CallbackArgs => {
                if c == 41 {
                    lx.state = State::Initial;
                    lx.p += 1;
                } else {
                    let e = lx.until(41, p);
                    lx.emit(CALLBACKARGS, p, e);
                }
            }
            State::Click => {
                if is_space(c) {
                    lx.state = State::Initial;
                    lx.p += 1;
                } else {
                    let mut e = p + 1;
                    while e < n && !is_space(lx.ch(e)) {
                        e += 1;
                    }
                    lx.emit(CLICK, p, e);
                }
            }
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

fn date_at(lx: &Lexer, p: i64) -> bool {
    let pattern = [4, 1, 2, 1, 2];
    let mut q = p;
    for (k, digits) in pattern.iter().enumerate() {
        if k > 0 {
            if lx.ch(q) != 45 {
                return false;
            }
            q += 1;
        }
        for _ in 0..*digits {
            if !(48..=57).contains(&lx.ch(q)) {
                return false;
            }
            q += 1;
        }
    }
    !is_word(lx.ch(q))
}
