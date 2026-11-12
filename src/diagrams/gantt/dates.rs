use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;

use regex::Regex;

use super::jsdate::{Fields, JsDate, MS_PER_DAY};

pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
pub const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

const WEEK: f64 = 7.0 * MS_PER_DAY;

const WS: &str = "\\t\\n\\x0b\\x0c\\r \\x{a0}\\x{1680}\\x{2000}-\\x{200a}\\x{2028}\\x{2029}\\x{202f}\\x{205f}\\x{3000}\\x{feff}";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Ms,
    S,
    M,
    H,
    D,
    W,
    Month,
    Y,
}

pub fn is_valid(date: &JsDate) -> bool {
    date.is_valid()
}

fn pad(value: i64, length: usize) -> String {
    let s = value.to_string();
    if s.len() >= length {
        s
    } else {
        format!("{}{s}", "0".repeat(length - s.len()))
    }
}

fn ordinal(n: i64) -> String {
    let suffixes = ["th", "st", "nd", "rd"];
    let v = n % 100;
    let first = usize::try_from((v - 20) % 10)
        .ok()
        .and_then(|i| suffixes.get(i).copied());
    let second = usize::try_from(v)
        .ok()
        .and_then(|i| suffixes.get(i).copied());
    let text = first
        .filter(|s| !s.is_empty())
        .or(second)
        .unwrap_or(suffixes[0]);
    format!("{n}{text}")
}

fn utc_offset(date: &JsDate) -> i64 {
    let offset = -(js_round(date.timezone_offset() / 15.0) * 15.0) as i64;
    if offset == 0 { 0 } else { offset }
}

fn zone(date: &JsDate) -> String {
    let offset = utc_offset(date);
    let minutes = offset.abs();
    format!(
        "{}{}:{}",
        if offset >= 0 { '+' } else { '-' },
        pad(minutes / 60, 2),
        pad(minutes % 60, 2)
    )
}

fn fields_of(date: &JsDate) -> Fields {
    date.fields().unwrap_or(Fields {
        year: 0,
        month: 0,
        day: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        milliseconds: 0,
        weekday: 0,
    })
}

pub fn iso_weekday(date: &JsDate) -> i64 {
    match fields_of(date).weekday {
        0 => 7,
        day => day,
    }
}

pub fn add_days(date: &JsDate, days: f64) -> JsDate {
    let mut next = *date;
    let day = fields_of(&next).day as f64;
    next.set_date(day + days);
    next
}

fn days_in_month(date: &JsDate) -> i64 {
    let f = fields_of(date);
    let next_month = JsDate::local(
        f.year as f64,
        (f.month + 1) as f64,
        0.0,
        23.0,
        59.0,
        59.0,
        999.0,
    );
    fields_of(&next_month).day
}

fn set_calendar(date: &JsDate, apply: impl Fn(&mut JsDate)) -> JsDate {
    let mut next = *date;
    next.set_date(1.0);
    apply(&mut next);
    let day = fields_of(date).day;
    next.set_date(f64::min(day as f64, days_in_month(&next) as f64));
    next
}

pub fn add(date: &JsDate, amount: f64, unit: Unit) -> JsDate {
    let f = fields_of(date);
    match unit {
        Unit::Month => set_calendar(date, |d| d.set_month((f.month as f64) + amount)),
        Unit::Y => set_calendar(date, |d| d.set_full_year((f.year as f64) + amount)),
        Unit::D => add_days(date, js_round(amount)),
        Unit::W => add_days(date, js_round(7.0 * amount)),
        Unit::H => JsDate::from_ms(date.time() + amount * 3_600_000.0),
        Unit::M => JsDate::from_ms(date.time() + amount * 60_000.0),
        Unit::S => JsDate::from_ms(date.time() + amount * 1000.0),
        Unit::Ms => JsDate::from_ms(date.time() + amount),
    }
}

fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

fn week_thursday(date: &JsDate) -> JsDate {
    add_days(date, 4.0 - iso_weekday(date) as f64)
}

fn iso_week(date: &JsDate) -> i64 {
    let thursday = week_thursday(date);
    let jan1 = JsDate::local(
        fields_of(&thursday).year as f64,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
    );
    let weekday = iso_weekday(&jan1);
    let first = add_days(
        &jan1,
        4.0 - weekday as f64 + if weekday > 4 { 7.0 } else { 0.0 },
    );
    let zone_delta = (utc_offset(&first) - utc_offset(&thursday)) as f64 * 60_000.0;
    let weeks = (thursday.time() - first.time() - zone_delta) / WEEK;
    let whole = if weeks < 0.0 {
        let c = weeks.ceil();
        if c == 0.0 { 0.0 } else { c }
    } else {
        weeks.floor()
    };
    (whole + 1.0) as i64
}

fn advanced_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\[([^\]]+)\]|Q|wo|ww|w|WW|W|zzz|z|gggg|GGGG|Do|X|x|k{1,2}|S").expect("pattern")
    })
}

fn format_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\[([^\]]+)\]|YYYY|YY|M{1,4}|D{1,2}|d{1,4}|H{1,2}|h{1,2}|a|A|m{1,2}|s{1,2}|Z{1,2}|SSS",
        )
        .expect("pattern")
    })
}

const DEFAULT_FORMAT: &str = "YYYY-MM-DDTHH:mm:ssZ";

pub fn format(date: &JsDate, template: &str) -> Option<String> {
    if !is_valid(date) {
        return Some("Invalid Date".to_owned());
    }
    let str_template = if template.is_empty() {
        DEFAULT_FORMAT
    } else {
        template
    };
    if str_template == "YYYY-MM-DD" {
        return Some(iso_date(date));
    }
    let f = fields_of(date);
    let mut unsupported = false;
    let advanced = advanced_re().replace_all(str_template, |caps: &regex::Captures| {
        let token = caps.get(0).map_or("", |m| m.as_str());
        match token {
            "Q" => (f.month / 3 + 1).to_string(),
            "Do" => format!("[{}]", ordinal(f.day)),
            "GGGG" => fields_of(&week_thursday(date)).year.to_string(),
            "W" => iso_week(date).to_string(),
            "WW" => pad(iso_week(date), 2),
            "k" => (if f.hours == 0 { 24 } else { f.hours }).to_string(),
            "kk" => pad(if f.hours == 0 { 24 } else { f.hours }, 2),
            "X" => ((date.time() / 1000.0).floor() as i64).to_string(),
            "x" => (date.time() as i64).to_string(),
            "gggg" | "wo" | "w" | "ww" | "z" | "zzz" => {
                unsupported = true;
                token.to_owned()
            }
            _ => token.to_owned(),
        }
    });
    if unsupported {
        return None;
    }
    let hours = f.hours;
    Some(
        format_re()
            .replace_all(&advanced, |caps: &regex::Captures| {
                let token = caps.get(0).map_or("", |m| m.as_str());
                if let Some(literal) = caps.get(1) {
                    return literal.as_str().to_owned();
                }
                match token {
                    "YY" => {
                        let s = f.year.to_string();
                        let n = s.chars().count();
                        s.chars().skip(n.saturating_sub(2)).collect()
                    }
                    "YYYY" => pad(f.year, 4),
                    "M" => (f.month + 1).to_string(),
                    "MM" => pad(f.month + 1, 2),
                    "MMM" => MONTHS[f.month as usize][..3].to_owned(),
                    "MMMM" => MONTHS[f.month as usize].to_owned(),
                    "D" => f.day.to_string(),
                    "DD" => pad(f.day, 2),
                    "d" => f.weekday.to_string(),
                    "dd" => WEEKDAYS[f.weekday as usize][..2].to_owned(),
                    "ddd" => WEEKDAYS[f.weekday as usize][..3].to_owned(),
                    "dddd" => WEEKDAYS[f.weekday as usize].to_owned(),
                    "H" => hours.to_string(),
                    "HH" => pad(hours, 2),
                    "h" => (if hours % 12 == 0 { 12 } else { hours % 12 }).to_string(),
                    "hh" => pad(if hours % 12 == 0 { 12 } else { hours % 12 }, 2),
                    "a" => (if hours < 12 { "am" } else { "pm" }).to_owned(),
                    "A" => (if hours < 12 { "AM" } else { "PM" }).to_owned(),
                    "m" => f.minutes.to_string(),
                    "mm" => pad(f.minutes, 2),
                    "s" => f.seconds.to_string(),
                    "ss" => pad(f.seconds, 2),
                    "SSS" => pad(f.milliseconds, 3),
                    "Z" => zone(date),
                    _ => zone(date).replacen(':', "", 1),
                }
            })
            .into_owned(),
    )
}

pub fn iso_date(date: &JsDate) -> String {
    let f = fields_of(date);
    let year = if (1000..=9999).contains(&f.year) {
        f.year.to_string()
    } else {
        pad(f.year, 4)
    };
    let month = f.month + 1;
    let day = f.day;
    format!(
        "{year}{}{month}{}{day}",
        if month < 10 { "-0" } else { "-" },
        if day < 10 { "-0" } else { "-" }
    )
}

#[derive(Clone, Debug, Default, PartialEq)]
struct ParsedFields {
    year: Option<f64>,
    month: Option<f64>,
    day: Option<f64>,
    day_given: bool,
    hours: Option<f64>,
    minutes: Option<f64>,
    seconds: Option<f64>,
    milliseconds: Option<f64>,
    week: Option<f64>,
    offset: Option<f64>,
    afternoon: Option<bool>,
}

fn truthy(value: Option<f64>) -> bool {
    value.is_some_and(|v| v != 0.0 && !v.is_nan())
}

fn number(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(f64::NAN)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Reader {
    Afternoon(bool),
    Quarter,
    Milliseconds(f64),
    Seconds,
    Minutes,
    Hours,
    Day,
    Ordinal,
    Week,
    Month,
    MonthName(bool),
    Year,
    Year2,
    Offset,
}

impl Reader {
    fn read(self, f: &mut ParsedFields, input: &str) -> bool {
        match self {
            Reader::Afternoon(am_pm_upper) => {
                f.afternoon = Some(if am_pm_upper {
                    input == "PM"
                } else {
                    input == "pm"
                });
                true
            }
            Reader::Quarter => {
                f.month = Some((number(input) - 1.0) * 3.0 + 1.0);
                true
            }
            Reader::Milliseconds(factor) => {
                f.milliseconds = Some(number(input) * factor);
                true
            }
            Reader::Seconds => {
                f.seconds = Some(number(input));
                true
            }
            Reader::Minutes => {
                f.minutes = Some(number(input));
                true
            }
            Reader::Hours => {
                f.hours = Some(number(input));
                true
            }
            Reader::Day => {
                f.day = Some(number(input));
                true
            }
            Reader::Ordinal => {
                let Some(digits) = token_re(r"[0-9]+")
                    .find(input)
                    .map(|m| m.as_str().to_owned())
                else {
                    return false;
                };
                f.day = Some(number(&digits));
                f.day_given = true;
                for i in 1..=31 {
                    if ordinal(i) == input {
                        f.day = Some(i as f64);
                    }
                }
                true
            }
            Reader::Week => {
                f.week = Some(number(input));
                true
            }
            Reader::Month => {
                f.month = Some(number(input));
                true
            }
            Reader::MonthName(long) => {
                let index = MONTHS
                    .iter()
                    .position(|m| if long { *m == input } else { &m[..3] == input })
                    .map_or(0, |p| p + 1);
                if index < 1 {
                    return false;
                }
                f.month = Some(index as f64);
                true
            }
            Reader::Year => {
                f.year = Some(number(input));
                true
            }
            Reader::Year2 => {
                let value = number(input);
                f.year = Some(value + if value > 68.0 { 1900.0 } else { 2000.0 });
                true
            }
            Reader::Offset => read_offset(f, input),
        }
    }
}

fn read_offset(f: &mut ParsedFields, input: &str) -> bool {
    if input == "Z" {
        f.offset = Some(0.0);
        return true;
    }
    let parts: Vec<String> = offset_parts(input);
    let sign = parts.first().cloned().unwrap_or_default();
    let hours = parts.get(1).map_or(0.0, |p| number(p));
    let minutes_part = parts.get(2).map_or(0.0, |p| number(p));
    let minutes = hours * 60.0
        + if minutes_part.is_nan() || minutes_part == 0.0 {
            0.0
        } else {
            minutes_part
        };
    f.offset = Some(if minutes == 0.0 {
        0.0
    } else if sign == "+" {
        -minutes
    } else {
        minutes
    });
    true
}

fn offset_parts(input: &str) -> Vec<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte == b'+' || byte == b'-' {
            out.push(input[at..at + 1].to_owned());
            at += 1;
        } else if byte.is_ascii_digit() && bytes.get(at + 1).is_some_and(u8::is_ascii_digit) {
            out.push(input[at..at + 2].to_owned());
            at += 2;
        } else {
            at += 1;
        }
    }
    out
}

#[derive(Clone, Debug)]
enum Step {
    Literal(usize),
    Token(Reader, &'static Regex),
}

fn token_re(pattern: &str) -> &'static Regex {
    thread_local! {
        static CACHE: RefCell<HashMap<String, &'static Regex>> = RefCell::new(HashMap::new());
    }
    CACHE.with(|cache| {
        if let Some(re) = cache.borrow().get(pattern) {
            return *re;
        }
        let re: &'static Regex = Box::leak(Box::new(Regex::new(pattern).expect("pattern")));
        cache.borrow_mut().insert(pattern.to_owned(), re);
        re
    })
}

fn reader_for(token: &str) -> Option<(Reader, &'static Regex)> {
    let digits1 = "[0-9]";
    let digits2 = "[0-9][0-9]";
    let digits3 = "[0-9]{3}";
    let digits4 = "[0-9]{4}";
    let one_to_two = "[0-9][0-9]?";
    let word = "[0-9]*[^-_:/,()\\s0-9]+";
    let signed = "[+-]?[0-9]+";
    let offset = "[+-][0-9][0-9]:?([0-9][0-9])?|Z";
    let ws_word = word.replace("\\s", WS);
    let (reader, pattern) = match token {
        "A" => (Reader::Afternoon(true), ws_word.as_str()),
        "a" => (Reader::Afternoon(false), ws_word.as_str()),
        "Q" => (Reader::Quarter, digits1),
        "S" => (Reader::Milliseconds(100.0), digits1),
        "SS" => (Reader::Milliseconds(10.0), digits2),
        "SSS" => (Reader::Milliseconds(1.0), digits3),
        "s" | "ss" => (Reader::Seconds, one_to_two),
        "m" | "mm" => (Reader::Minutes, one_to_two),
        "H" | "h" | "HH" | "hh" => (Reader::Hours, one_to_two),
        "D" => (Reader::Day, one_to_two),
        "DD" => (Reader::Day, digits2),
        "Do" => (Reader::Ordinal, ws_word.as_str()),
        "w" => (Reader::Week, one_to_two),
        "ww" => (Reader::Week, digits2),
        "M" => (Reader::Month, one_to_two),
        "MM" => (Reader::Month, digits2),
        "MMM" => (Reader::MonthName(false), ws_word.as_str()),
        "MMMM" => (Reader::MonthName(true), ws_word.as_str()),
        "Y" => (Reader::Year, signed),
        "YY" => (Reader::Year2, digits2),
        "YYYY" => (Reader::Year, digits4),
        "Z" | "ZZ" => (Reader::Offset, offset),
        _ => return None,
    };
    let owned = pattern.to_owned();
    Some((reader, token_re(&owned)))
}

fn tokens_of(template: &str) -> Vec<String> {
    let pattern = r"(\[[^\[]*\])|([-_:/.,()\s]+)|(A|a|Q|YYYY|YY?|ww?|MM?M?M?|Do|DD?|hh?|HH?|mm?|ss?|S{1,3}|z|ZZ?)";
    let re = token_re(&pattern.replace("\\s", WS));
    re.find_iter(template)
        .map(|m| m.as_str().to_owned())
        .collect()
}

fn compiled(template: &str) -> Option<Rc<Vec<Step>>> {
    thread_local! {
        static CACHE: RefCell<HashMap<String, Option<Rc<Vec<Step>>>>> = RefCell::new(HashMap::new());
    }
    CACHE.with(|cache| {
        if let Some(steps) = cache.borrow().get(template) {
            return steps.clone();
        }
        let steps = compile(template).map(Rc::new);
        cache
            .borrow_mut()
            .insert(template.to_owned(), steps.clone());
        steps
    })
}

fn compile(template: &str) -> Option<Vec<Step>> {
    let literal = token_re(r"\[[^\]]+\]").replace_all(template, "");
    if literal.contains('L') || literal.contains('l') {
        return None;
    }
    let tokens = tokens_of(template);
    if tokens.is_empty() {
        return None;
    }
    let mut steps = Vec::with_capacity(tokens.len());
    for token in tokens {
        match reader_for(&token) {
            Some((reader, re)) => steps.push(Step::Token(reader, re)),
            None => {
                let stripped = token
                    .strip_prefix('[')
                    .unwrap_or(&token)
                    .strip_suffix(']')
                    .unwrap_or(token.strip_prefix('[').unwrap_or(&token));
                steps.push(Step::Literal(stripped.chars().count()));
            }
        }
    }
    Some(steps)
}

fn loose(input: &str, template: &str, now: &JsDate) -> Option<JsDate> {
    if template == "x" || template == "X" {
        let value: f64 = number(input) * if template == "X" { 1000.0 } else { 1.0 };
        return Some(JsDate::from_ms(value));
    }
    let steps = compiled(template)?;
    let mut f = ParsedFields::default();
    let mut rest: String = input.to_owned();
    let mut start = 0usize;
    for step in steps.iter() {
        match step {
            Step::Literal(length) => {
                start += length;
            }
            Step::Token(reader, re) => {
                let tail: String = rest.chars().skip(start).collect();
                let m = re.find(&tail)?;
                if !reader.read(&mut f, m.as_str()) {
                    return None;
                }
                rest = rest.replacen(m.as_str(), "", 1);
            }
        }
    }
    if let Some(afternoon) = f.afternoon
        && let Some(hours) = f.hours
    {
        if afternoon {
            if hours < 12.0 {
                f.hours = Some(hours + 12.0);
            }
        } else if hours == 12.0 {
            f.hours = Some(0.0);
        }
    }
    let now_fields = fields_of(now);
    let day = if f.day_given || truthy(f.day) {
        f.day.unwrap_or(0.0)
    } else if !truthy(f.year) && !truthy(f.month) {
        now_fields.day as f64
    } else {
        1.0
    };
    let year = if truthy(f.year) {
        f.year.unwrap_or(0.0)
    } else {
        now_fields.year as f64
    };
    let mut month = 0.0;
    if !(truthy(f.year) && !truthy(f.month)) {
        month = if f.month.unwrap_or(0.0) > 0.0 {
            f.month.unwrap_or(0.0) - 1.0
        } else {
            now_fields.month as f64
        };
    }
    let hours = if truthy(f.hours) {
        f.hours.unwrap_or(0.0)
    } else {
        0.0
    };
    let minutes = if truthy(f.minutes) {
        f.minutes.unwrap_or(0.0)
    } else {
        0.0
    };
    let seconds = if truthy(f.seconds) {
        f.seconds.unwrap_or(0.0)
    } else {
        0.0
    };
    let ms = if truthy(f.milliseconds) {
        f.milliseconds.unwrap_or(0.0)
    } else {
        0.0
    };
    if let Some(offset) = f.offset {
        return Some(JsDate::from_ms(JsDate::utc(
            year,
            month,
            day,
            hours,
            minutes,
            seconds,
            ms + offset * 60000.0,
        )));
    }
    if truthy(f.week) {
        return None;
    }
    Some(JsDate::local(year, month, day, hours, minutes, seconds, ms))
}

pub fn parse_strict(input: &str, template: &str, now: &JsDate) -> Option<JsDate> {
    let date = loose(input, template, now)?;
    if is_valid(&date) && format(&date, template).as_deref() == Some(input) {
        Some(date)
    } else {
        None
    }
}

#[allow(dead_code)]
fn ws_class() -> &'static str {
    WS
}
