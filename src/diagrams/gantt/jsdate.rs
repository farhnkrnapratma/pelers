use std::cell::Cell;

use chrono::{Local, TimeZone};

pub const MS_PER_DAY: f64 = 86_400_000.0;
const MAX_TIME: f64 = 8.64e15;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JsDate(pub f64);

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn make_day(year: f64, month: f64, day: f64) -> f64 {
    if !(year.is_finite() && month.is_finite() && day.is_finite()) {
        return f64::NAN;
    }
    let (y, m, d) = (year.trunc(), month.trunc(), day.trunc());
    let ym = y + (m / 12.0).floor();
    let mn = m.rem_euclid(12.0);
    if ym.abs() > 400_000.0 {
        return f64::NAN;
    }
    days_from_civil(ym as i64, mn as i64 + 1, 1) as f64 + d - 1.0
}

fn make_date(day: f64, time: f64) -> f64 {
    let t = day * MS_PER_DAY + time;
    if t.is_finite() && t.abs() <= MAX_TIME {
        t.trunc() + 0.0
    } else {
        f64::NAN
    }
}

fn full_year(year: f64) -> f64 {
    if year.is_finite() && (0.0..=99.0).contains(&year.trunc()) {
        1900.0 + year.trunc()
    } else {
        year
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fields {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub milliseconds: i64,
    pub weekday: i64,
}

fn offset_at(utc_ms: f64) -> f64 {
    thread_local! {
        static LAST: Cell<Option<(i64, f64)>> = const { Cell::new(None) };
    }
    if !utc_ms.is_finite() {
        return 0.0;
    }
    let seconds = (utc_ms / 1000.0).floor() as i64;
    if let Some((last, offset)) = LAST.get()
        && last == seconds
    {
        return offset;
    }
    let offset = match Local.timestamp_opt(seconds, 0).single() {
        Some(local) => {
            let offset = chrono::Offset::fix(local.offset());
            f64::from(offset.local_minus_utc()) * 1000.0
        }
        None => 0.0,
    };
    LAST.set(Some((seconds, offset)));
    offset
}

impl JsDate {
    pub fn invalid() -> Self {
        JsDate(f64::NAN)
    }

    pub fn from_ms(ms: f64) -> Self {
        JsDate(if ms.is_finite() && ms.abs() <= MAX_TIME {
            ms.trunc() + 0.0
        } else {
            f64::NAN
        })
    }

    pub fn local(
        year: f64,
        month: f64,
        day: f64,
        hours: f64,
        minutes: f64,
        seconds: f64,
        ms: f64,
    ) -> Self {
        Self::local_exact(full_year(year), month, day, hours, minutes, seconds, ms)
    }

    pub fn local_exact(
        year: f64,
        month: f64,
        day: f64,
        hours: f64,
        minutes: f64,
        seconds: f64,
        ms: f64,
    ) -> Self {
        let utc_fields = make_date(
            make_day(year, month, day),
            hours * 3_600_000.0 + minutes * 60_000.0 + seconds * 1000.0 + ms,
        );
        if !utc_fields.is_finite() {
            return JsDate::invalid();
        }
        let guess = utc_fields - offset_at(utc_fields);
        JsDate::from_ms(utc_fields - offset_at(guess))
    }

    pub fn utc(
        year: f64,
        month: f64,
        day: f64,
        hours: f64,
        minutes: f64,
        seconds: f64,
        ms: f64,
    ) -> f64 {
        Self::utc_exact(full_year(year), month, day, hours, minutes, seconds, ms)
    }

    pub fn utc_exact(
        year: f64,
        month: f64,
        day: f64,
        hours: f64,
        minutes: f64,
        seconds: f64,
        ms: f64,
    ) -> f64 {
        make_date(
            make_day(year, month, day),
            hours * 3_600_000.0 + minutes * 60_000.0 + seconds * 1000.0 + ms,
        )
    }

    pub fn time(&self) -> f64 {
        self.0
    }

    pub fn is_valid(&self) -> bool {
        !self.0.is_nan()
    }

    pub fn fields(&self) -> Option<Fields> {
        if !self.is_valid() {
            return None;
        }
        let local = self.0 + offset_at(self.0);
        let days = (local / MS_PER_DAY).floor();
        let ms_of_day = (local - days * MS_PER_DAY) as i64;
        let (year, month, day) = civil_from_days(days as i64);
        Some(Fields {
            year,
            month: month - 1,
            day,
            hours: ms_of_day / 3_600_000,
            minutes: (ms_of_day / 60_000) % 60,
            seconds: (ms_of_day / 1000) % 60,
            milliseconds: ms_of_day % 1000,
            weekday: (days as i64 + 4).rem_euclid(7),
        })
    }

    pub fn timezone_offset(&self) -> f64 {
        -offset_at(self.0) / 60_000.0
    }

    fn update(&mut self, change: impl FnOnce(&mut [f64; 7])) {
        if let Some(f) = self.fields() {
            let mut parts = [
                f.year as f64,
                f.month as f64,
                f.day as f64,
                f.hours as f64,
                f.minutes as f64,
                f.seconds as f64,
                f.milliseconds as f64,
            ];
            change(&mut parts);
            let [year, month, day, hours, minutes, seconds, ms] = parts;
            *self = JsDate::local_exact(year, month, day, hours, minutes, seconds, ms);
        }
    }

    pub fn set_date(&mut self, day: f64) {
        self.update(|p| p[2] = day);
    }

    pub fn set_month(&mut self, month: f64) {
        self.update(|p| p[1] = month);
    }

    pub fn set_full_year(&mut self, year: f64) {
        self.update(|p| p[0] = year);
    }

    pub fn set_month_and_date(&mut self, month: f64, day: f64) {
        self.update(|p| {
            p[1] = month;
            p[2] = day;
        });
    }

    pub fn set_hours(&mut self, hours: f64, minutes: f64, seconds: f64, ms: f64) {
        self.update(|p| p[3..7].copy_from_slice(&[hours, minutes, seconds, ms]));
    }
}
