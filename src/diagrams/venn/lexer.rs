use crate::diagrams::common::scan::slice;
use crate::util::chars::is_js_space;

pub const END: i32 = 0;
pub const ERROR: i32 = 1;
pub const EOF: i32 = 2;
pub const NEWLINE: i32 = 3;
pub const VENN: i32 = 4;
pub const TITLE: i32 = 5;
pub const SET: i32 = 6;
pub const UNION: i32 = 7;
pub const TEXT: i32 = 8;
pub const INDENT_TEXT: i32 = 9;
pub const STYLE: i32 = 10;
pub const BRACKET_LABEL: i32 = 11;
pub const NUMERIC: i32 = 12;
pub const HEXCOLOR: i32 = 13;
pub const RGBCOLOR: i32 = 14;
pub const RGBACOLOR: i32 = 15;
pub const IDENTIFIER: i32 = 16;
pub const STRING: i32 = 17;
pub const COMMA: i32 = 18;
pub const COLON: i32 = 19;

pub const TOKEN_NAMES: [&str; 20] = [
    "$end",
    "INVALID",
    "EOF",
    "NEWLINE",
    "VENN",
    "TITLE",
    "SET",
    "UNION",
    "TEXT",
    "INDENT_TEXT",
    "STYLE",
    "BRACKET_LABEL",
    "NUMERIC",
    "HEXCOLOR",
    "RGBCOLOR",
    "RGBACOLOR",
    "IDENTIFIER",
    "STRING",
    "COMMA",
    "COLON",
];

const KEYWORDS: [(&str, i32); 5] = [
    ("venn-beta", VENN),
    ("set", SET),
    ("union", UNION),
    ("text", TEXT),
    ("style", STYLE),
];

fn is_word(c: u16) -> bool {
    (48..=57).contains(&c) || (65..=90).contains(&c) || (97..=122).contains(&c) || c == 95
}

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn is_hex(c: u16) -> bool {
    is_digit(c) || (65..=70).contains(&c) || (97..=102).contains(&c)
}

fn is_space(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

pub struct Lexer {
    src: Vec<u16>,
    at: usize,
    line_start: bool,
    ended: bool,
    indent: bool,
    pub text: String,
    pub start: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            src: source.encode_utf16().collect(),
            at: 0,
            line_start: false,
            ended: false,
            indent: false,
            text: String::new(),
            start: 0,
        }
    }

    pub fn set_indent(&mut self, enabled: bool) {
        self.indent = enabled;
    }

    fn unit(&self, i: usize) -> u16 {
        self.src.get(i).copied().unwrap_or(0)
    }

    fn word(&self, at: usize, word: &str) -> bool {
        word.bytes().enumerate().all(|(k, w)| {
            let c = self.unit(at + k);
            let w = u16::from(w);
            !(c != w && (c.wrapping_add(32) != w || !(65..=90).contains(&c)))
        })
    }

    fn keyword(&self, at: usize, word: &str) -> bool {
        self.word(at, word) && !is_word(self.unit(at + word.len()))
    }

    fn color(&self, at: usize, parts: usize) -> Option<usize> {
        let mut i = at + 1;
        for part in 0..parts {
            while is_space(self.unit(i)) {
                i += 1;
            }
            let from = i;
            while is_digit(self.unit(i)) || self.unit(i) == 46 {
                i += 1;
            }
            if i == from {
                return None;
            }
            while is_space(self.unit(i)) {
                i += 1;
            }
            let want = if part == parts - 1 { 41 } else { 44 };
            if self.unit(i) != want {
                return None;
            }
            i += 1;
        }
        Some(i)
    }

    fn after_line_feed(&self, at: usize) -> bool {
        let mut k = at as isize - 1;
        while k >= 0 && (self.unit(k as usize) == 32 || self.unit(k as usize) == 9) {
            k -= 1;
        }
        k >= 0 && self.unit(k as usize) == 10
    }

    fn line_end(&self, from: usize) -> usize {
        self.src
            .get(from..)
            .and_then(|rest| rest.iter().position(|&u| u == 10))
            .map_or(self.src.len(), |at| from + at)
    }

    fn token(&mut self, kind: i32, end: usize) -> i32 {
        self.text = slice(&self.src, self.at, end);
        self.at = end;
        kind
    }

    pub fn next(&mut self) -> i32 {
        let n = self.src.len();
        loop {
            let mut i = self.at;
            self.start = i;
            self.text.clear();
            if i >= n {
                if self.ended {
                    return END;
                }
                self.ended = true;
                return EOF;
            }
            let c = self.unit(i);
            if c == 37 && self.unit(i + 1) == 37 && self.unit(i + 2) != 123 {
                self.at = self.line_end(i + 2);
                continue;
            }
            if self.line_start
                && self.word(i, "title")
                && self.after_line_feed(i)
                && slice(&self.src, (i + 5).min(n), self.line_end(i + 5))
                    .trim_matches(is_js_space)
                    .is_empty()
            {
                let mut j = self.line_end(i + 5);
                while self.unit(j) == 10 || self.unit(j) == 13 {
                    j += 1;
                }
                self.at = j;
                continue;
            }
            if self.line_start {
                if c == 32 || c == 9 {
                    let mut j = i + 1;
                    while self.unit(j) == 32 || self.unit(j) == 9 {
                        j += 1;
                    }
                    self.at = j;
                    if self.keyword(j, "text") && self.indent {
                        self.at = j + 4;
                        self.line_start = false;
                        self.text = slice(&self.src, i, j);
                        return INDENT_TEXT;
                    }
                    continue;
                }
                if c != 10 && c != 13 {
                    self.indent = false;
                    self.line_start = false;
                }
            }
            if c == 10 || c == 13 {
                i += 1;
                while self.unit(i) == 10 || self.unit(i) == 13 {
                    i += 1;
                }
                self.line_start = true;
                return self.token(NEWLINE, i);
            }
            if c == 37 && self.unit(i + 1) == 37 {
                self.at = self.line_end(i);
                continue;
            }
            if c == 32 || c == 9 {
                i += 1;
                while self.unit(i) == 32 || self.unit(i) == 9 {
                    i += 1;
                }
                self.at = i;
                continue;
            }

            if ((c | 32) >= 97 && (c | 32) <= 122) || c == 95 {
                let sep = self.unit(i + 5);
                if self.word(i, "title") && is_space(sep) && sep != 10 && sep != 13 {
                    let mut end = i + 6;
                    while end < n {
                        let ch = self.unit(end);
                        if ch == 35 || ch == 10 || ch == 59 {
                            break;
                        }
                        end += 1;
                    }
                    if end > i + 6 {
                        return self.token(TITLE, end);
                    }
                }
                for (word, kind) in KEYWORDS {
                    if self.keyword(i, word) && self.unit(i + word.len()) != 45 {
                        return self.token(kind, i + word.len());
                    }
                }
                if self.word(i, "rgb") {
                    let alpha = (self.unit(i + 3) | 32) == 97;
                    let open = i + if alpha { 4 } else { 3 };
                    let end = if self.unit(open) == 40 {
                        self.color(open, if alpha { 4 } else { 3 })
                    } else {
                        None
                    };
                    if let Some(end) = end {
                        return self.token(if alpha { RGBACOLOR } else { RGBCOLOR }, end);
                    }
                }
                i += 1;
                while is_word(self.unit(i)) || self.unit(i) == 45 {
                    i += 1;
                }
                return self.token(IDENTIFIER, i);
            }
            if c == 91 {
                if self.unit(i + 1) == 34 {
                    let Some(quote) = self.src[(i + 2).min(n)..]
                        .iter()
                        .position(|&u| u == 34)
                        .map(|at| i + 2 + at)
                    else {
                        return ERROR;
                    };
                    if self.unit(quote + 1) != 93 {
                        return ERROR;
                    }
                    self.at = quote + 2;
                    self.text = slice(&self.src, i + 2, quote);
                    return BRACKET_LABEL;
                }
                let mut end = i + 1;
                while end < n && self.unit(end) != 93 && self.unit(end) != 34 {
                    end += 1;
                }
                if end == i + 1 || self.unit(end) != 93 {
                    return ERROR;
                }
                self.at = end + 1;
                self.text = crate::util::chars::trim(&slice(&self.src, i + 1, end)).to_owned();
                return BRACKET_LABEL;
            }
            if is_digit(c) || c == 46 || c == 43 || c == 45 {
                if c == 43 || c == 45 {
                    i += 1;
                }
                let from = i;
                while is_digit(self.unit(i)) {
                    i += 1;
                }
                if i > from {
                    if self.unit(i) == 46 && is_digit(self.unit(i + 1)) {
                        i += 2;
                        while is_digit(self.unit(i)) {
                            i += 1;
                        }
                    }
                } else if self.unit(i) == 46 && is_digit(self.unit(i + 1)) {
                    i += 2;
                    while is_digit(self.unit(i)) {
                        i += 1;
                    }
                } else {
                    return ERROR;
                }
                return self.token(NUMERIC, i);
            }
            if c == 35 {
                let mut end = i + 1;
                while end < i + 9 && is_hex(self.unit(end)) {
                    end += 1;
                }
                return if end >= i + 4 {
                    self.token(HEXCOLOR, end)
                } else {
                    ERROR
                };
            }
            if c == 34 {
                return match self.src[(i + 1).min(n)..].iter().position(|&u| u == 34) {
                    None => ERROR,
                    Some(at) => self.token(STRING, i + 1 + at + 1),
                };
            }
            if c == 44 {
                return self.token(COMMA, i + 1);
            }
            if c == 58 {
                return self.token(COLON, i + 1);
            }
            return ERROR;
        }
    }
}
