use std::sync::OnceLock;

use regex::Regex;

use super::dates::{MONTHS, WEEKDAYS};
use super::jsdate::JsDate;

const SECOND: f64 = 1000.0;
const MINUTE: f64 = 60.0 * SECOND;
const HOUR: f64 = 60.0 * MINUTE;
const DAY: f64 = 24.0 * HOUR;
const WEEK: f64 = 7.0 * DAY;
const MONTH: f64 = 30.0 * DAY;
const YEAR: f64 = 365.0 * DAY;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickUnit {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calendar {
    unit: TickUnit,
    step: f64,
    week_start: f64,
}

fn start_of_day(d: &mut JsDate) {
    d.set_hours(0.0, 0.0, 0.0, 0.0);
}

fn base_floor(unit: TickUnit, week_start: f64, d: &mut JsDate) {
    let t = d.time();
    match unit {
        TickUnit::Millisecond => {}
        TickUnit::Second => *d = JsDate::from_ms(t - ms_of(d)),
        TickUnit::Minute => *d = JsDate::from_ms(t - ms_of(d) - seconds_of(d) * SECOND),
        TickUnit::Hour => {
            *d = JsDate::from_ms(t - ms_of(d) - seconds_of(d) * SECOND - minutes_of(d) * MINUTE)
        }
        TickUnit::Day => start_of_day(d),
        TickUnit::Week => {
            let weekday = field_or_nan(d, |f| f.weekday as f64);
            let day = field_or_nan(d, |f| f.day as f64);
            d.set_date(day - ((weekday + 7.0 - week_start) % 7.0));
            start_of_day(d);
        }
        TickUnit::Month => {
            d.set_date(1.0);
            start_of_day(d);
        }
        TickUnit::Year => {
            d.set_month_and_date(0.0, 1.0);
            start_of_day(d);
        }
    }
}

fn base_offset(unit: TickUnit, d: &mut JsDate, step: f64) {
    let t = d.time();
    match unit {
        TickUnit::Millisecond => *d = JsDate::from_ms(t + step),
        TickUnit::Second => *d = JsDate::from_ms(t + step * SECOND),
        TickUnit::Minute => *d = JsDate::from_ms(t + step * MINUTE),
        TickUnit::Hour => *d = JsDate::from_ms(t + step * HOUR),
        TickUnit::Day => {
            let day = field_or_nan(d, |f| f.day as f64);
            d.set_date(day + step);
        }
        TickUnit::Week => {
            let day = field_or_nan(d, |f| f.day as f64);
            d.set_date(day + step * 7.0);
        }
        TickUnit::Month => {
            let month = field_or_nan(d, |f| f.month as f64);
            d.set_month(month + step);
        }
        TickUnit::Year => {
            let year = field_or_nan(d, |f| f.year as f64);
            d.set_full_year(year + step);
        }
    }
}

fn ms_of(d: &JsDate) -> f64 {
    field_or_nan(d, |f| f.milliseconds as f64)
}

fn seconds_of(d: &JsDate) -> f64 {
    field_or_nan(d, |f| f.seconds as f64)
}

fn minutes_of(d: &JsDate) -> f64 {
    field_or_nan(d, |f| f.minutes as f64)
}

fn field_or_nan(d: &JsDate, pick: impl Fn(&super::jsdate::Fields) -> f64) -> f64 {
    d.fields().map_or(f64::NAN, |f| pick(&f))
}

fn base_field(unit: TickUnit, d: &JsDate) -> Option<f64> {
    match unit {
        TickUnit::Second => Some(if d.is_valid() {
            (d.time() / SECOND).floor().rem_euclid(60.0)
        } else {
            f64::NAN
        }),
        TickUnit::Minute => Some(field_or_nan(d, |f| f.minutes as f64)),
        TickUnit::Hour => Some(field_or_nan(d, |f| f.hours as f64)),
        TickUnit::Day => Some(field_or_nan(d, |f| (f.day - 1) as f64)),
        TickUnit::Month => Some(field_or_nan(d, |f| f.month as f64)),
        _ => None,
    }
}

fn cycle_of(unit: TickUnit) -> f64 {
    match unit {
        TickUnit::Millisecond | TickUnit::Year | TickUnit::Week => f64::INFINITY,
        TickUnit::Second | TickUnit::Minute => 60.0,
        TickUnit::Hour => 24.0,
        TickUnit::Day => 31.0,
        TickUnit::Month => 12.0,
    }
}

fn ms_of_unit(unit: TickUnit) -> f64 {
    match unit {
        TickUnit::Millisecond => 1.0,
        TickUnit::Second => SECOND,
        TickUnit::Minute => MINUTE,
        TickUnit::Hour => HOUR,
        TickUnit::Day => DAY,
        TickUnit::Week => WEEK,
        TickUnit::Month => MONTH,
        TickUnit::Year => YEAR,
    }
}

fn js_mod(a: f64, b: f64) -> f64 {
    ((a % b) + b) % b
}

fn span(start: &JsDate, end: &JsDate, unit: f64) -> f64 {
    let tz = end.timezone_offset() - start.timezone_offset();
    ((end.time() - start.time() - tz * MINUTE) / unit).floor()
}

fn week_calendar(week_start: f64) -> Calendar {
    Calendar {
        unit: TickUnit::Week,
        step: 1.0,
        week_start,
    }
}

impl Calendar {
    fn floor(&self, d: &mut JsDate) {
        if self.step <= 1.0 {
            base_floor(self.unit, self.week_start, d);
            return;
        }
        match self.unit {
            TickUnit::Millisecond => {
                *d = JsDate::from_ms((d.time() / self.step).floor() * self.step);
            }
            TickUnit::Year => {
                let year = field_or_nan(d, |f| f.year as f64);
                d.set_full_year((year / self.step).floor() * self.step);
                base_floor(TickUnit::Year, self.week_start, d);
            }
            TickUnit::Week => {
                base_floor(TickUnit::Week, self.week_start, d);
                let epoch = week_epoch(self.week_start);
                let back = -js_mod(span(&epoch, d, WEEK), self.step);
                base_offset_week(self.week_start, d, back);
            }
            _ => {
                let base_unit = self.unit;
                let step = self.step;
                base_floor(base_unit, self.week_start, d);
                while d.is_valid() && !test(base_unit, step, d) {
                    *d = JsDate::from_ms(d.time() - 1.0);
                    base_floor(base_unit, self.week_start, d);
                }
            }
        }
    }

    fn offset(&self, d: &mut JsDate, n: f64) {
        if self.step <= 1.0 {
            base_offset(self.unit, d, n);
            return;
        }
        match self.unit {
            TickUnit::Millisecond => *d = JsDate::from_ms(d.time() + n * self.step),
            TickUnit::Year => {
                let year = field_or_nan(d, |f| f.year as f64);
                d.set_full_year(year + n * self.step);
            }
            TickUnit::Week => base_offset_week(self.week_start, d, n * self.step),
            _ => {
                let mut i = 0.0;
                while i < n && d.is_valid() {
                    loop {
                        base_offset(self.unit, d, 1.0);
                        if !(d.is_valid() && !test(self.unit, self.step, d)) {
                            break;
                        }
                    }
                    i += 1.0;
                }
            }
        }
    }
}

fn test(unit: TickUnit, step: f64, d: &JsDate) -> bool {
    match base_field(unit, d) {
        Some(value) => js_mod(value, step) == 0.0,
        None => false,
    }
}

fn week_epoch(week_start: f64) -> JsDate {
    let mut epoch = JsDate::from_ms(0.0);
    base_floor(TickUnit::Week, week_start, &mut epoch);
    epoch
}

fn base_offset_week(week_start: f64, d: &mut JsDate, n: f64) {
    let _ = week_start;
    base_offset(TickUnit::Week, d, n);
}

fn every(unit: TickUnit, step: f64, week_start: f64) -> Calendar {
    let base = if unit == TickUnit::Week {
        week_calendar(week_start)
    } else {
        Calendar {
            unit,
            step: 1.0,
            week_start,
        }
    };
    if step <= 1.0 {
        return base;
    }
    Calendar {
        unit,
        step,
        week_start,
    }
}

fn range(calendar: &Calendar, start: f64, stop: f64, limit: usize) -> Option<Vec<f64>> {
    let mut d = JsDate::from_ms(start - 1.0);
    calendar.floor(&mut d);
    calendar.offset(&mut d, 1.0);
    calendar.floor(&mut d);
    let mut out: Vec<f64> = Vec::new();
    let mut previous = f64::NEG_INFINITY;
    while d.time() <= stop && d.time() > previous {
        if out.len() >= limit {
            return None;
        }
        previous = d.time();
        out.push(previous);
        calendar.offset(&mut d, 1.0);
        calendar.floor(&mut d);
    }
    Some(out)
}

fn tick_step(start: f64, stop: f64, count: f64) -> f64 {
    let step = (stop - start) / count.max(0.0);
    let power = step.log10().floor();
    let error = step / 10f64.powf(power);
    let factor = if error >= 50f64.sqrt() {
        10.0
    } else if error >= 10f64.sqrt() {
        5.0
    } else if error >= std::f64::consts::SQRT_2 {
        2.0
    } else {
        1.0
    };
    10f64.powf(power) * factor
}

const AUTO: [(TickUnit, f64, f64); 18] = [
    (TickUnit::Second, 1.0, SECOND),
    (TickUnit::Second, 5.0, 5.0 * SECOND),
    (TickUnit::Second, 15.0, 15.0 * SECOND),
    (TickUnit::Second, 30.0, 30.0 * SECOND),
    (TickUnit::Minute, 1.0, MINUTE),
    (TickUnit::Minute, 5.0, 5.0 * MINUTE),
    (TickUnit::Minute, 15.0, 15.0 * MINUTE),
    (TickUnit::Minute, 30.0, 30.0 * MINUTE),
    (TickUnit::Hour, 1.0, HOUR),
    (TickUnit::Hour, 3.0, 3.0 * HOUR),
    (TickUnit::Hour, 6.0, 6.0 * HOUR),
    (TickUnit::Hour, 12.0, 12.0 * HOUR),
    (TickUnit::Day, 1.0, DAY),
    (TickUnit::Day, 2.0, 2.0 * DAY),
    (TickUnit::Week, 1.0, WEEK),
    (TickUnit::Month, 1.0, MONTH),
    (TickUnit::Month, 3.0, 3.0 * MONTH),
    (TickUnit::Year, 1.0, YEAR),
];

pub fn auto_ticks(start: f64, stop: f64, count: f64) -> Vec<f64> {
    let target = (stop - start) / count;
    let mut i = 0;
    while i < AUTO.len() && AUTO[i].2 <= target {
        i += 1;
    }
    let calendar = if i == AUTO.len() {
        every(
            TickUnit::Year,
            js_max1(tick_step(start / YEAR, stop / YEAR, count)),
            0.0,
        )
    } else if i == 0 {
        every(
            TickUnit::Millisecond,
            js_max1_below(tick_step(start, stop, count)),
            0.0,
        )
    } else {
        let chosen = if target / AUTO[i - 1].2 < AUTO[i].2 / target {
            AUTO[i - 1]
        } else {
            AUTO[i]
        };
        every(chosen.0, chosen.1, 0.0)
    };
    range(&calendar, start, stop, (4.0 * count + 8.0) as usize).unwrap_or_default()
}

fn js_max1(value: f64) -> f64 {
    if value.is_nan() {
        f64::NAN
    } else {
        value.max(1.0)
    }
}

fn js_max1_below(value: f64) -> f64 {
    if value.is_nan() {
        f64::NAN
    } else {
        value.max(1.0)
    }
}

fn interval_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^([1-9][0-9]*)(millisecond|second|minute|hour|day|week|month)$")
            .expect("pattern")
    })
}

const WEEK_STARTS: [&str; 7] = [
    "sunday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
];

fn unit_of(name: &str) -> TickUnit {
    match name {
        "millisecond" => TickUnit::Millisecond,
        "second" => TickUnit::Second,
        "minute" => TickUnit::Minute,
        "hour" => TickUnit::Hour,
        "day" => TickUnit::Day,
        "week" => TickUnit::Week,
        _ => TickUnit::Month,
    }
}

pub fn interval_ticks(
    interval: &str,
    weekday: &str,
    start: f64,
    stop: f64,
    limit: usize,
) -> Option<Vec<f64>> {
    let caps = interval_re().captures(interval)?;
    let step: f64 = caps[1].parse().ok()?;
    let unit = unit_of(&caps[2]);
    let week_index = WEEK_STARTS
        .iter()
        .position(|d| *d == weekday)
        .map_or(0.0, |i| i as f64);
    let calendar = every(unit, step, week_index);
    let cycle = cycle_of(unit);
    if (stop - start) / (step.min(cycle) * ms_of_unit(unit)) > limit as f64 {
        return None;
    }
    range(&calendar, start, stop, limit)
}

fn pad_value(value: f64, fill: &str, width: usize) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let digits = if sign.is_empty() {
        format_integer(value)
    } else {
        format_integer(-value)
    };
    let body = if digits.len() < width {
        format!("{}{digits}", fill.repeat(width - digits.len()))
    } else {
        digits
    };
    format!("{sign}{body}")
}

fn format_integer(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else {
        format!("{}", value as i64)
    }
}

fn pad_fill(c: char) -> Option<&'static str> {
    match c {
        '-' => Some(""),
        '_' => Some(" "),
        '0' => Some("0"),
        _ => None,
    }
}

pub fn time_format(specifier: &str, date: &JsDate) -> String {
    let fields = date.fields().unwrap_or(super::jsdate::Fields {
        year: 0,
        month: 0,
        day: 0,
        hours: 0,
        minutes: 0,
        seconds: 0,
        milliseconds: 0,
        weekday: 0,
    });
    let mut out = String::new();
    let chars: Vec<char> = specifier.chars().collect();
    let mut from = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        out.extend(chars[from..i].iter());
        i += 1;
        let mut c = chars.get(i).copied().unwrap_or('\0');
        let fill = match pad_fill(c) {
            Some(fill) => {
                i += 1;
                c = chars.get(i).copied().unwrap_or('\0');
                fill
            }
            None => {
                if c == 'e' {
                    " "
                } else {
                    "0"
                }
            }
        };
        out.push_str(&directive(c, date, fields, fill));
        i += 1;
        from = i;
    }
    out.extend(chars[from.min(chars.len())..].iter());
    out
}

fn iso_thursday(date: &JsDate) -> JsDate {
    let thursday = week_calendar(4.0);
    let day = date.fields().map_or(0, |f| f.weekday);
    let mut d = if day >= 4 || day == 0 {
        *date
    } else {
        JsDate::from_ms(date.time() - 1.0)
    };
    thursday.floor(&mut d);
    if !(day >= 4 || day == 0) {
        thursday.offset(&mut d, 1.0);
        thursday.floor(&mut d);
    }
    d
}

fn floored(date: &JsDate, calendar: &Calendar) -> JsDate {
    let mut d = *date;
    calendar.floor(&mut d);
    d
}

fn year_calendar() -> Calendar {
    Calendar {
        unit: TickUnit::Year,
        step: 1.0,
        week_start: 0.0,
    }
}

fn day_calendar() -> Calendar {
    Calendar {
        unit: TickUnit::Day,
        step: 1.0,
        week_start: 0.0,
    }
}

fn week_of_year(date: &JsDate, week_start: f64) -> f64 {
    let week = week_calendar(week_start);
    let year_start = floored(date, &year_calendar());
    let before = JsDate::from_ms(year_start.time() - 1.0);
    span(&floored(&before, &week), &floored(date, &week), WEEK)
}

fn directive(c: char, d: &JsDate, f: super::jsdate::Fields, p: &str) -> String {
    match c {
        'a' => WEEKDAYS[f.weekday as usize][..3].to_owned(),
        'A' => WEEKDAYS[f.weekday as usize].to_owned(),
        'b' => MONTHS[f.month as usize][..3].to_owned(),
        'B' => MONTHS[f.month as usize].to_owned(),
        'c' => time_format("%x, %X", d),
        'd' | 'e' => pad_value(f.day as f64, p, 2),
        'f' => format!("{}000", pad_value(f.milliseconds as f64, p, 3)),
        'g' => pad_value(
            (iso_thursday(d).fields().map_or(0, |x| x.year) % 100) as f64,
            p,
            2,
        ),
        'G' => pad_value(
            (iso_thursday(d).fields().map_or(0, |x| x.year) % 10000) as f64,
            p,
            4,
        ),
        'H' => pad_value(f.hours as f64, p, 2),
        'I' => pad_value(
            (if f.hours % 12 == 0 { 12 } else { f.hours % 12 }) as f64,
            p,
            2,
        ),
        'j' => pad_value(
            1.0 + span(
                &floored(d, &year_calendar()),
                &floored(d, &day_calendar()),
                DAY,
            ),
            p,
            3,
        ),
        'L' => pad_value(f.milliseconds as f64, p, 3),
        'm' => pad_value((f.month + 1) as f64, p, 2),
        'M' => pad_value(f.minutes as f64, p, 2),
        'p' => (if f.hours >= 12 { "PM" } else { "AM" }).to_owned(),
        'q' => (1 + f.month / 3).to_string(),
        'Q' => format_integer(d.time()),
        's' => format_integer((d.time() / 1000.0).floor()),
        'S' => pad_value(f.seconds as f64, p, 2),
        'u' => (if f.weekday == 0 { 7 } else { f.weekday }).to_string(),
        'U' => pad_value(week_of_year(d, 0.0), p, 2),
        'V' => {
            let thursday = iso_thursday(d);
            let year_start = floored(&thursday, &year_calendar());
            let first = floored(&year_start, &week_calendar(4.0));
            let extra = if year_start.fields().map_or(0, |x| x.weekday) == 4 {
                1.0
            } else {
                0.0
            };
            pad_value(span(&first, &thursday, WEEK) + extra, p, 2)
        }
        'w' => f.weekday.to_string(),
        'W' => pad_value(week_of_year(d, 1.0), p, 2),
        'x' => time_format("%-m/%-d/%Y", d),
        'X' => time_format("%-I:%M:%S %p", d),
        'y' => pad_value((f.year % 100) as f64, p, 2),
        'Y' => pad_value((f.year % 10000) as f64, p, 4),
        'Z' => {
            let z = d.timezone_offset();
            let abs = z.abs();
            format!(
                "{}{}{}",
                if z > 0.0 { "-" } else { "+" },
                pad_value((abs / 60.0).floor(), "0", 2),
                pad_value(abs % 60.0, "0", 2)
            )
        }
        '%' => "%".to_owned(),
        other => other.to_string(),
    }
}
