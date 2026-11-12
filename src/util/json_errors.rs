#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    End,
    Punct(u16),
    Str,
    Num,
    Literal,
    Ident,
}

type Lexed = (Token, usize);
type Parsed<T> = Result<T, JsonError>;

const KEYWORDS: [&str; 3] = ["true", "false", "null"];

fn fail(offset: usize, message: String) -> JsonError {
    JsonError { offset, message }
}

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn is_hex(c: u16) -> bool {
    is_digit(c) || (65..=70).contains(&c) || (97..=102).contains(&c)
}

fn is_ident_start(c: u16) -> bool {
    u8::try_from(c).is_ok_and(|b| b.is_ascii_alphabetic() || b == b'_' || b == b'$')
}

fn is_ident_part(c: u16) -> bool {
    is_ident_start(c) || is_digit(c)
}

fn char_text(c: u16) -> String {
    String::from_utf16_lossy(&[c])
}

struct Parser<'a> {
    u: &'a [u16],
    p: usize,
    buffered: Option<Lexed>,
}

impl Parser<'_> {
    fn at(&self, i: usize) -> Option<u16> {
        self.u.get(i).copied()
    }

    fn starts_with(&self, at: usize, word: &str) -> bool {
        word.encode_utf16()
            .enumerate()
            .all(|(k, unit)| self.at(at + k) == Some(unit))
    }

    fn skip_space(&mut self) {
        while matches!(self.at(self.p), Some(32 | 9 | 10 | 13)) {
            self.p += 1;
        }
    }

    fn lex(&mut self) -> Parsed<Lexed> {
        self.skip_space();
        let start = self.p;
        let Some(c) = self.at(start) else {
            return Ok((Token::End, start));
        };
        if matches!(c, 123 | 125 | 91 | 93 | 44 | 58 | 59 | 46) {
            self.p += 1;
            return Ok((Token::Punct(c), start));
        }
        if c == 34 {
            return self.string().map(|()| (Token::Str, start));
        }
        if c == 45 || is_digit(c) {
            return self.number().map(|()| (Token::Num, start));
        }
        for word in KEYWORDS {
            if self.starts_with(start, word) {
                self.p += word.len();
                return Ok((Token::Literal, start));
            }
        }
        if is_ident_start(c) {
            while self.at(self.p).is_some_and(is_ident_part) {
                self.p += 1;
            }
            return Ok((Token::Ident, start));
        }
        Err(fail(
            start,
            format!("Unrecognized token '{}'", char_text(c)),
        ))
    }

    fn string(&mut self) -> Parsed<()> {
        self.p += 1;
        loop {
            let Some(c) = self.at(self.p) else {
                return Err(fail(self.p, "Unterminated string".to_owned()));
            };
            if c == 34 {
                self.p += 1;
                return Ok(());
            }
            if c < 0x20 {
                return Err(fail(self.p, "Unterminated string".to_owned()));
            }
            if c != 92 {
                self.p += 1;
                continue;
            }
            let Some(escape) = self.at(self.p + 1) else {
                return Err(fail(self.p, "Unterminated string".to_owned()));
            };
            match escape {
                34 | 92 | 47 | 98 | 102 | 110 | 114 | 116 => self.p += 2,
                117 => {
                    let four: Vec<u16> = (self.p + 2..self.p + 6)
                        .filter_map(|i| self.at(i))
                        .collect();
                    if four.len() < 4 {
                        return Err(fail(
                            self.p,
                            "\\u must be followed by 4 hex digits".to_owned(),
                        ));
                    }
                    if four.iter().all(|&h| is_hex(h)) {
                        self.p += 6;
                        continue;
                    }
                    let raw = String::from_utf16_lossy(&self.u[self.p..self.p + 6]);
                    return Err(fail(
                        self.p,
                        format!("\"{raw}\" is not a valid unicode escape"),
                    ));
                }
                _ => {
                    return Err(fail(
                        self.p + 1,
                        format!("Invalid escape character {}", char_text(escape)),
                    ));
                }
            }
        }
    }

    fn number(&mut self) -> Parsed<()> {
        if self.at(self.p) == Some(45) {
            self.p += 1;
            if !self.at(self.p).is_some_and(is_digit) {
                return Err(fail(self.p, "Invalid number".to_owned()));
            }
        }
        if self.at(self.p) == Some(48) {
            self.p += 1;
        } else {
            while self.at(self.p).is_some_and(is_digit) {
                self.p += 1;
            }
        }
        if self.at(self.p) == Some(46) {
            self.p += 1;
            if !self.at(self.p).is_some_and(is_digit) {
                return Err(fail(
                    self.p,
                    "Invalid digits after decimal point".to_owned(),
                ));
            }
            while self.at(self.p).is_some_and(is_digit) {
                self.p += 1;
            }
        }
        if matches!(self.at(self.p), Some(101 | 69)) {
            let mut q = self.p + 1;
            if matches!(self.at(q), Some(43 | 45)) {
                q += 1;
            }
            if self.at(q).is_some_and(is_digit) {
                while self.at(q).is_some_and(is_digit) {
                    q += 1;
                }
                self.p = q;
            }
        }
        Ok(())
    }

    fn peek(&mut self) -> Parsed<Lexed> {
        if let Some(lexed) = self.buffered {
            return Ok(lexed);
        }
        let lexed = self.lex()?;
        self.buffered = Some(lexed);
        Ok(lexed)
    }

    fn take(&mut self) -> Parsed<Lexed> {
        let lexed = self.peek()?;
        self.buffered = None;
        Ok(lexed)
    }

    fn is_punct(lexed: Lexed, c: u16) -> bool {
        lexed.0 == Token::Punct(c)
    }

    fn value(&mut self) -> Parsed<()> {
        let (token, start) = self.peek()?;
        match token {
            Token::End => Err(fail(start, "Unexpected EOF".to_owned())),
            Token::Str | Token::Num | Token::Literal => {
                self.take()?;
                Ok(())
            }
            Token::Punct(123) => self.object(),
            Token::Punct(91) => self.array(),
            Token::Punct(c) => Err(fail(start, format!("Unexpected token '{}'", char_text(c)))),
            Token::Ident => {
                let word = self.word(start);
                Err(fail(start, format!("Unexpected identifier \"{word}\"")))
            }
        }
    }

    fn word(&self, start: usize) -> String {
        let mut end = start;
        while self.at(end).is_some_and(is_ident_part) {
            end += 1;
        }
        String::from_utf16_lossy(&self.u[start..end])
    }

    fn object(&mut self) -> Parsed<()> {
        self.take()?;
        let first = self.peek()?;
        if Self::is_punct(first, 125) {
            self.take()?;
            return Ok(());
        }
        if first.0 != Token::Str {
            return Err(fail(first.1, "Expected '}'".to_owned()));
        }
        loop {
            self.take()?;
            let colon = self.peek()?;
            if !Self::is_punct(colon, 58) {
                return Err(fail(
                    colon.1,
                    "Expected ':' before value in object property definition".to_owned(),
                ));
            }
            self.take()?;
            self.value()?;
            let next = self.peek()?;
            if Self::is_punct(next, 44) {
                self.take()?;
                let key = self.peek()?;
                if key.0 != Token::Str {
                    return Err(fail(
                        key.1,
                        "Property name must be a string literal".to_owned(),
                    ));
                }
                continue;
            }
            if Self::is_punct(next, 125) {
                self.take()?;
                return Ok(());
            }
            return Err(fail(next.1, "Expected '}'".to_owned()));
        }
    }

    fn array(&mut self) -> Parsed<()> {
        self.take()?;
        if Self::is_punct(self.peek()?, 93) {
            self.take()?;
            return Ok(());
        }
        loop {
            self.value()?;
            let next = self.peek()?;
            if Self::is_punct(next, 44) {
                self.take()?;
                let after = self.peek()?;
                if Self::is_punct(after, 93) {
                    return Err(fail(
                        after.1,
                        "Unexpected comma at the end of array expression".to_owned(),
                    ));
                }
                continue;
            }
            if Self::is_punct(next, 93) {
                self.take()?;
                return Ok(());
            }
            return Err(fail(next.1, "Expected ']'".to_owned()));
        }
    }

    fn document(&mut self) -> Parsed<()> {
        self.value()?;
        let (token, start) = self.peek()?;
        if token == Token::End {
            Ok(())
        } else {
            Err(fail(start, "Unable to parse JSON string".to_owned()))
        }
    }
}

pub fn parse_error(text: &str) -> Option<JsonError> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut parser = Parser {
        u: &units,
        p: 0,
        buffered: None,
    };
    parser.document().err()
}

pub fn error_message(text: &str) -> Option<String> {
    parse_error(text).map(|error| format!("JSON Parse error: {}", error.message))
}
