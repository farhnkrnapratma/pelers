use super::jsdate::JsDate;
use crate::util::chars::is_js_space;

const NONE: i64 = i64::MIN;
const MAX_SIGNIFICANT_DIGITS: usize = 9;
const SMI_MAX: i64 = (1 << 30) - 1;
const MAX_TIME: f64 = 8.64e15;
const MAX_TIME_BEFORE_UTC: f64 = MAX_TIME + 864e5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeywordType {
    Invalid,
    Month,
    AmPm,
    TimeZone,
    TimeSeparator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    End,
    Number {
        value: i64,
        length: usize,
    },
    Symbol(char),
    Keyword {
        kind: KeywordType,
        value: i64,
        length: usize,
    },
    WhiteSpace,
    Unknown,
}

impl Token {
    fn is_number(self) -> bool {
        matches!(self, Self::Number { .. })
    }

    fn is_fixed(self, digits: usize) -> bool {
        matches!(self, Self::Number { length, .. } if length == digits)
    }

    fn number(self) -> i64 {
        match self {
            Self::Number { value, .. } => value,
            _ => 0,
        }
    }

    fn length(self) -> usize {
        match self {
            Self::Number { length, .. } => length,
            _ => 0,
        }
    }

    fn is_symbol(self, c: char) -> bool {
        self == Self::Symbol(c)
    }

    fn is_sign(self) -> bool {
        self.is_symbol('+') || self.is_symbol('-')
    }

    fn sign(self) -> i64 {
        if self.is_symbol('-') { -1 } else { 1 }
    }

    fn is_keyword_type(self, want: KeywordType) -> bool {
        matches!(self, Self::Keyword { kind, .. } if kind == want)
    }

    fn is_keyword_z(self) -> bool {
        matches!(
            self,
            Self::Keyword {
                kind: KeywordType::TimeZone,
                value: 0,
                length: 1
            }
        )
    }
}

const KEYWORDS: [([u8; 3], KeywordType, i64); 27] = [
    (*b"jan", KeywordType::Month, 1),
    (*b"feb", KeywordType::Month, 2),
    (*b"mar", KeywordType::Month, 3),
    (*b"apr", KeywordType::Month, 4),
    (*b"may", KeywordType::Month, 5),
    (*b"jun", KeywordType::Month, 6),
    (*b"jul", KeywordType::Month, 7),
    (*b"aug", KeywordType::Month, 8),
    (*b"sep", KeywordType::Month, 9),
    (*b"oct", KeywordType::Month, 10),
    (*b"nov", KeywordType::Month, 11),
    (*b"dec", KeywordType::Month, 12),
    (*b"am\0", KeywordType::AmPm, 0),
    (*b"pm\0", KeywordType::AmPm, 12),
    (*b"ut\0", KeywordType::TimeZone, 0),
    (*b"utc", KeywordType::TimeZone, 0),
    (*b"z\0\0", KeywordType::TimeZone, 0),
    (*b"gmt", KeywordType::TimeZone, 0),
    (*b"cdt", KeywordType::TimeZone, -5),
    (*b"cst", KeywordType::TimeZone, -6),
    (*b"edt", KeywordType::TimeZone, -4),
    (*b"est", KeywordType::TimeZone, -5),
    (*b"mdt", KeywordType::TimeZone, -6),
    (*b"mst", KeywordType::TimeZone, -7),
    (*b"pdt", KeywordType::TimeZone, -7),
    (*b"pst", KeywordType::TimeZone, -8),
    (*b"t\0\0", KeywordType::TimeSeparator, 0),
];

fn lookup_keyword(prefix: [u32; 3], length: usize) -> (KeywordType, i64) {
    for (word, kind, value) in KEYWORDS.iter() {
        let matches = (0..3).all(|j| prefix[j] == u32::from(word[j]));
        if matches && (length <= 3 || *kind == KeywordType::Month) {
            return (*kind, *value);
        }
    }
    (KeywordType::Invalid, 0)
}

fn is_end(chars: &[char], at: usize) -> bool {
    at >= chars.len() || chars[at] == '\0'
}

fn scan(chars: &[char], at: &mut usize) -> Token {
    if is_end(chars, *at) {
        return Token::End;
    }
    let c = chars[*at];
    if c.is_ascii_digit() {
        let start = *at;
        while chars.get(*at) == Some(&'0') {
            *at += 1;
        }
        let mut value: i64 = 0;
        let mut significant = 0;
        while let Some(digit) = chars.get(*at).filter(|d| d.is_ascii_digit()) {
            if significant < MAX_SIGNIFICANT_DIGITS {
                value = value * 10 + i64::from(*digit as u8 - b'0');
            }
            significant += 1;
            *at += 1;
        }
        return Token::Number {
            value,
            length: *at - start,
        };
    }
    if matches!(c, ':' | '-' | '+' | '.' | ')') {
        *at += 1;
        return Token::Symbol(c);
    }
    if c >= 'A' && !is_js_space(c) {
        let start = *at;
        while chars.get(*at).is_some_and(|&w| w >= 'A' && !is_js_space(w)) {
            *at += 1;
        }
        let word = &chars[start..*at];
        let length: usize = word.iter().map(|w| w.len_utf16()).sum();
        let mut prefix = [0u32; 3];
        for (slot, w) in prefix.iter_mut().zip(word.iter()) {
            *slot = (*w as u32) | 0x20;
        }
        let (kind, value) = lookup_keyword(prefix, length);
        return Token::Keyword {
            kind,
            value,
            length,
        };
    }
    if is_js_space(c) && c != '\u{b}' {
        *at += 1;
        return Token::WhiteSpace;
    }
    if c == '(' {
        let mut balance = 0;
        loop {
            let ch = chars[*at];
            if ch == ')' {
                balance -= 1;
            } else if ch == '(' {
                balance += 1;
            }
            *at += 1;
            if balance <= 0 || is_end(chars, *at) {
                break;
            }
        }
        return Token::Unknown;
    }
    *at += 1;
    Token::Unknown
}

struct Scanner {
    tokens: Vec<Token>,
    at: usize,
}

impl Scanner {
    fn new(text: &str) -> Self {
        let chars: Vec<char> = text.chars().collect();
        let mut at = 0;
        let mut tokens = Vec::new();
        loop {
            let token = scan(&chars, &mut at);
            tokens.push(token);
            if token == Token::End {
                break;
            }
        }
        Self { tokens, at: 0 }
    }

    fn peek(&self) -> Token {
        self.tokens[self.at.min(self.tokens.len() - 1)]
    }

    fn next(&mut self) -> Token {
        let token = self.peek();
        if token != Token::End {
            self.at += 1;
        }
        token
    }

    fn skip_symbol(&mut self, c: char) -> bool {
        if self.peek().is_symbol(c) {
            self.at += 1;
            true
        } else {
            false
        }
    }
}

fn is_month(x: i64) -> bool {
    (1..=12).contains(&x)
}

fn is_day(x: i64) -> bool {
    (1..=31).contains(&x)
}

fn is_hour(x: i64) -> bool {
    (0..=23).contains(&x)
}

fn is_hour12(x: i64) -> bool {
    (0..=12).contains(&x)
}

fn is_minute(x: i64) -> bool {
    (0..=59).contains(&x)
}

fn is_millisecond(x: i64) -> bool {
    (0..=999).contains(&x)
}

struct Day {
    comp: [i64; 3],
    index: usize,
    named_month: i64,
    iso: bool,
}

impl Day {
    fn new() -> Self {
        Self {
            comp: [0; 3],
            index: 0,
            named_month: NONE,
            iso: false,
        }
    }

    fn add(&mut self, n: i64) -> bool {
        if self.index < 3 {
            self.comp[self.index] = n;
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn is_empty(&self) -> bool {
        self.index == 0
    }

    fn write(&mut self) -> Option<(i64, i64, i64)> {
        if self.index < 1 {
            return None;
        }
        while self.index < 3 {
            self.comp[self.index] = 1;
            self.index += 1;
        }
        let c = self.comp;
        let mut year;
        let month;
        let day;
        if self.named_month == NONE {
            if self.iso || !is_day(c[0]) {
                year = c[0];
                month = c[1];
                day = c[2];
            } else {
                month = c[0];
                day = c[1];
                year = c[2];
            }
        } else {
            month = self.named_month;
            if !is_day(c[0]) {
                year = c[0];
                day = c[1];
            } else {
                day = c[0];
                year = c[1];
            }
        }
        if !self.iso {
            if (0..=49).contains(&year) {
                year += 2000;
            } else if (50..=99).contains(&year) {
                year += 1900;
            }
        }
        if !is_month(month) || !is_day(day) {
            return None;
        }
        Some((year, month - 1, day))
    }
}

struct Time {
    comp: [i64; 4],
    index: usize,
    hour_offset: i64,
}

impl Time {
    fn new() -> Self {
        Self {
            comp: [0; 4],
            index: 0,
            hour_offset: NONE,
        }
    }

    fn add(&mut self, n: i64) -> bool {
        if self.index < 4 {
            self.comp[self.index] = n;
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn add_final(&mut self, n: i64) -> bool {
        if !self.add(n) {
            return false;
        }
        while self.index < 4 {
            self.comp[self.index] = 0;
            self.index += 1;
        }
        true
    }

    fn is_expecting(&self, n: i64) -> bool {
        (self.index == 1 && is_minute(n))
            || (self.index == 2 && is_minute(n))
            || (self.index == 3 && is_millisecond(n))
    }

    fn is_empty(&self) -> bool {
        self.index == 0
    }

    fn write(&mut self) -> Option<[i64; 4]> {
        while self.index < 4 {
            self.comp[self.index] = 0;
            self.index += 1;
        }
        let mut c = self.comp;
        if self.hour_offset != NONE {
            if !is_hour12(c[0]) {
                return None;
            }
            c[0] %= 12;
            c[0] += self.hour_offset;
        }
        let valid = is_hour(c[0]) && is_minute(c[1]) && is_minute(c[2]) && is_millisecond(c[3]);
        if !valid && !(c[0] == 24 && c[1] == 0 && c[2] == 0 && c[3] == 0) {
            return None;
        }
        Some(c)
    }
}

struct Zone {
    sign: i64,
    hour: i64,
    minute: i64,
}

impl Zone {
    fn new() -> Self {
        Self {
            sign: NONE,
            hour: NONE,
            minute: NONE,
        }
    }

    fn set(&mut self, offset_hours: i64) {
        self.sign = if offset_hours < 0 { -1 } else { 1 };
        self.hour = offset_hours * self.sign;
        self.minute = 0;
    }

    fn set_sign(&mut self, sign: i64) {
        self.sign = if sign < 0 { -1 } else { 1 };
    }

    fn is_expecting(&self, n: i64) -> bool {
        self.hour != NONE && self.minute == NONE && is_minute(n)
    }

    fn is_utc(&self) -> bool {
        self.hour == 0 && self.minute == 0
    }

    fn is_empty(&self) -> bool {
        self.sign == NONE
    }

    fn write(&self) -> Option<Option<i64>> {
        if self.sign == NONE {
            return Some(None);
        }
        let hour = if self.hour == NONE { 0 } else { self.hour };
        let minute = if self.minute == NONE { 0 } else { self.minute };
        let total = hour * 3600 + minute * 60;
        if total > SMI_MAX {
            return None;
        }
        Some(Some(self.sign * total))
    }
}

fn read_milliseconds(value: i64, length: usize) -> i64 {
    let mut number = value;
    let mut length = length;
    if length < 3 {
        if length == 1 {
            number *= 100;
        } else if length == 2 {
            number *= 10;
        }
    } else if length > 3 {
        if length > MAX_SIGNIFICANT_DIGITS {
            length = MAX_SIGNIFICANT_DIGITS;
        }
        let mut factor = 1;
        loop {
            factor *= 10;
            length -= 1;
            if length <= 3 {
                break;
            }
        }
        number /= factor;
    }
    number
}

fn parse_es5(
    scanner: &mut Scanner,
    day: &mut Day,
    time: &mut Time,
    zone: &mut Zone,
) -> Option<Token> {
    if scanner.peek().is_sign() {
        let sign_token = scanner.next();
        if !scanner.peek().is_fixed(6) {
            return Some(sign_token);
        }
        let sign = sign_token.sign();
        let year = scanner.next().number();
        if sign < 0 && year == 0 {
            return Some(sign_token);
        }
        day.add(sign * year);
    } else if scanner.peek().is_fixed(4) {
        let year = scanner.next().number();
        day.add(year);
    } else {
        return Some(scanner.next());
    }
    if scanner.skip_symbol('-') {
        if !scanner.peek().is_fixed(2) || !is_month(scanner.peek().number()) {
            return Some(scanner.next());
        }
        let month = scanner.next().number();
        day.add(month);
        if scanner.skip_symbol('-') {
            if !scanner.peek().is_fixed(2) || !is_day(scanner.peek().number()) {
                return Some(scanner.next());
            }
            let date = scanner.next().number();
            day.add(date);
        }
    }
    if !scanner.peek().is_keyword_type(KeywordType::TimeSeparator) {
        if scanner.peek() != Token::End {
            return Some(scanner.next());
        }
    } else {
        scanner.next();
        if !scanner.peek().is_fixed(2) || !(0..=24).contains(&scanner.peek().number()) {
            return None;
        }
        let hour_is_24 = scanner.peek().number() == 24;
        let hour = scanner.next().number();
        time.add(hour);
        if !scanner.skip_symbol(':') {
            return None;
        }
        if !scanner.peek().is_fixed(2)
            || !is_minute(scanner.peek().number())
            || (hour_is_24 && scanner.peek().number() > 0)
        {
            return None;
        }
        let minute = scanner.next().number();
        time.add(minute);
        if scanner.skip_symbol(':') {
            if !scanner.peek().is_fixed(2)
                || !is_minute(scanner.peek().number())
                || (hour_is_24 && scanner.peek().number() > 0)
            {
                return None;
            }
            let second = scanner.next().number();
            time.add(second);
            if scanner.skip_symbol('.') {
                if !scanner.peek().is_number() || (hour_is_24 && scanner.peek().number() > 0) {
                    return None;
                }
                let token = scanner.next();
                time.add(read_milliseconds(token.number(), token.length()));
            }
        }
        if scanner.peek().is_keyword_z() {
            scanner.next();
            zone.set(0);
        } else if scanner.peek().is_sign() {
            zone.set_sign(scanner.next().sign());
            if scanner.peek().is_fixed(4) {
                let hourmin = scanner.next().number();
                let hour = hourmin / 100;
                let minute = hourmin % 100;
                if !is_hour(hour) || !is_minute(minute) {
                    return None;
                }
                zone.hour = hour;
                zone.minute = minute;
            } else {
                if !scanner.peek().is_fixed(2) || !is_hour(scanner.peek().number()) {
                    return None;
                }
                zone.hour = scanner.next().number();
                if !scanner.skip_symbol(':') {
                    return None;
                }
                if !scanner.peek().is_fixed(2) || !is_minute(scanner.peek().number()) {
                    return None;
                }
                zone.minute = scanner.next().number();
            }
        }
        if scanner.peek() != Token::End {
            return None;
        }
    }
    if zone.is_empty() && time.is_empty() {
        zone.set(0);
    }
    day.iso = true;
    Some(Token::End)
}

type Components = (f64, f64, f64, f64, f64, f64, f64, Option<i64>);

fn parse_components(text: &str) -> Option<Components> {
    let mut scanner = Scanner::new(text);
    let mut day = Day::new();
    let mut time = Time::new();
    let mut zone = Zone::new();
    let mut token = parse_es5(&mut scanner, &mut day, &mut time, &mut zone)?;
    let mut has_read_number = !day.is_empty();
    while token != Token::End {
        match token {
            Token::Number { value: n, .. } => {
                has_read_number = true;
                if scanner.skip_symbol(':') {
                    if scanner.skip_symbol(':') {
                        if !time.is_empty() {
                            return None;
                        }
                        time.add(n);
                        time.add(0);
                    } else {
                        if !time.add(n) {
                            return None;
                        }
                        if scanner.peek().is_symbol('.') {
                            scanner.next();
                        }
                    }
                } else if scanner.skip_symbol('.') && time.is_expecting(n) {
                    time.add(n);
                    let ms = scanner.peek();
                    if !ms.is_number() {
                        return None;
                    }
                    scanner.next();
                    time.add_final(read_milliseconds(ms.number(), ms.length()));
                } else if zone.is_expecting(n) {
                    zone.minute = n;
                } else if time.is_expecting(n) {
                    time.add_final(n);
                    let peek = scanner.peek();
                    let ok = matches!(peek, Token::End | Token::WhiteSpace)
                        || peek.is_keyword_z()
                        || peek.is_sign();
                    if !ok {
                        return None;
                    }
                } else {
                    if !day.add(n) {
                        return None;
                    }
                    scanner.skip_symbol('-');
                }
            }
            Token::Keyword { kind, value, .. } => {
                if kind == KeywordType::AmPm && !time.is_empty() {
                    time.hour_offset = value;
                } else if kind == KeywordType::Month {
                    day.named_month = value;
                    scanner.skip_symbol('-');
                } else if kind == KeywordType::TimeZone && has_read_number {
                    zone.set(value);
                } else {
                    if has_read_number {
                        return None;
                    }
                    if scanner.peek().is_number() {
                        return None;
                    }
                }
            }
            sign if sign.is_sign() && (zone.is_utc() || !time.is_empty()) => {
                zone.set_sign(sign.sign());
                let (n, length) = if scanner.peek().is_number() {
                    let number = scanner.next();
                    (number.number(), number.length())
                } else {
                    (0, 0)
                };
                has_read_number = true;
                if scanner.peek().is_symbol(':') {
                    zone.hour = n;
                    zone.minute = NONE;
                } else if length == 1 || length == 2 {
                    zone.hour = n;
                    zone.minute = 0;
                } else if length == 3 || length == 4 {
                    zone.hour = n / 100;
                    zone.minute = n % 100;
                } else {
                    return None;
                }
            }
            garbage if (garbage.is_sign() || garbage.is_symbol(')')) && has_read_number => {
                return None;
            }
            _ => {}
        }
        token = scanner.next();
    }

    let (year, month, date) = day.write()?;
    let [hour, minute, second, millisecond] = time.write()?;
    let offset = zone.write()?;
    Some((
        year as f64,
        month as f64,
        date as f64,
        hour as f64,
        minute as f64,
        second as f64,
        millisecond as f64,
        offset,
    ))
}

fn time_clip(time: f64) -> f64 {
    if !time.is_finite() || time.abs() > MAX_TIME {
        return f64::NAN;
    }
    time.trunc() + 0.0
}

pub fn parse(text: &str) -> f64 {
    let Some((year, month, day, hour, minute, second, millisecond, offset)) =
        parse_components(text)
    else {
        return f64::NAN;
    };
    let date = JsDate::utc_exact(year, month, day, hour, minute, second, millisecond);
    let time = match offset {
        Some(seconds) => date - seconds as f64 * 1000.0,
        None if (-MAX_TIME_BEFORE_UTC..=MAX_TIME_BEFORE_UTC).contains(&date) => {
            JsDate::local_exact(year, month, day, hour, minute, second, millisecond).time()
        }
        None => f64::NAN,
    };
    time_clip(time)
}
