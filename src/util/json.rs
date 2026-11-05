use super::value::{Object, Value};

const MAX_DEPTH: usize = 512;

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

type Parsed<T> = Result<T, ()>;

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn skip_space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn eat(&mut self, literal: &str) -> bool {
        let found = self.text[self.at..].starts_with(literal);
        if found {
            self.at += literal.len();
        }
        found
    }

    fn value(&mut self, depth: usize) -> Parsed<Value> {
        if depth > MAX_DEPTH {
            return Err(());
        }
        self.skip_space();
        match self.peek().ok_or(())? {
            b'{' => self.object(depth),
            b'[' => self.array(depth),
            b'"' => self.string().map(Value::String),
            b't' if self.eat("true") => Ok(Value::Bool(true)),
            b'f' if self.eat("false") => Ok(Value::Bool(false)),
            b'n' if self.eat("null") => Ok(Value::Null),
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err(()),
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.at;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.at += 1;
        }
        self.at - start
    }

    fn number(&mut self) -> Parsed<Value> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(()),
        }
        if self.peek() == Some(b'.') {
            self.at += 1;
            if self.digits() == 0 {
                return Err(());
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            if self.digits() == 0 {
                return Err(());
            }
        }
        self.text[start..self.at]
            .parse()
            .map(Value::Number)
            .map_err(|_| ())
    }

    fn hex4(&mut self) -> Parsed<u32> {
        let digits = self.text.get(self.at..self.at + 4).ok_or(())?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(());
        }
        self.at += 4;
        u32::from_str_radix(digits, 16).map_err(|_| ())
    }

    fn string(&mut self) -> Parsed<String> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let rest = &self.text[self.at..];
            let c = rest.chars().next().ok_or(())?;
            self.at += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let e = self.peek().ok_or(())?;
                    self.at += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let unit = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&unit)
                                && self.text[self.at..].starts_with("\\u")
                            {
                                let save = self.at;
                                self.at += 2;
                                match self.hex4() {
                                    Ok(low) if (0xdc00..0xe000).contains(&low) => {
                                        0x10000 + ((unit - 0xd800) << 10) + (low - 0xdc00)
                                    }
                                    _ => {
                                        self.at = save;
                                        unit
                                    }
                                }
                            } else {
                                unit
                            };
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(()),
                    }
                }
                c if (c as u32) < 0x20 => return Err(()),
                c => out.push(c),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Parsed<Value> {
        self.at += 1;
        let mut items = Vec::new();
        self.skip_space();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(()),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Parsed<Value> {
        self.at += 1;
        let mut object = Object::new();
        self.skip_space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Value::Object(object));
        }
        loop {
            self.skip_space();
            if self.peek() != Some(b'"') {
                return Err(());
            }
            let key = self.string()?;
            self.skip_space();
            if self.peek() != Some(b':') {
                return Err(());
            }
            self.at += 1;
            let value = self.value(depth + 1)?;
            object.set(key, value);
            self.skip_space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Value::Object(object));
                }
                _ => return Err(()),
            }
        }
    }
}

pub fn parse(text: &str) -> Option<Value> {
    let mut parser = Parser { text, at: 0 };
    let value = parser.value(0).ok()?;
    parser.skip_space();
    (parser.at == text.len()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_json_parse_accepts() {
        let value = parse(
            " {\"a\": [1, -2.5e1, true, null, \"x\\u0041\\n\"], \"a\": 3, \"__proto__\": {}} ",
        )
        .unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.keys(), ["a", "__proto__"]);
        assert_eq!(object.get("a"), Some(&Value::Number(3.0)));
    }

    #[test]
    fn rejects_what_json_parse_rejects() {
        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\":1,}",
            "01",
            "1.",
            "\"\t\"",
            "'a'",
            "{a:1}",
            "[1] 2",
            "+1",
            "nul",
        ] {
            assert!(parse(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn joins_surrogate_pairs() {
        assert_eq!(
            parse("\"\\ud83d\\ude00\"").unwrap().as_str(),
            Some("\u{1F600}")
        );
    }

    #[test]
    fn refuses_absurd_nesting() {
        assert!(parse(&"[".repeat(100_000)).is_none());
    }
}
