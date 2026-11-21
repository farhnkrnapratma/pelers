use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::util::hook::Hook;

use jiff::Timestamp;
use jiff::tz::TimeZone;

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

const SLOTS: usize = 256;

pub type OffsetFn = dyn Fn(f64) -> f64;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

thread_local! {
    static CACHE: RefCell<Vec<(i64, f64, u64)>> = RefCell::new(vec![(i64::MIN, 0.0, u64::MAX); SLOTS]);
    static GENERATION: Cell<u64> = const { Cell::new(1) };
    static VERSION: Cell<u64> = const { Cell::new(0) };
    static CHECKED: Cell<bool> = const { Cell::new(false) };
    static ZONE: TimeZone = TimeZone::system();
    static OFFSETS: RefCell<Option<Rc<OffsetFn>>> = const { RefCell::new(None) };
    static LAST: RefCell<Option<Rc<OffsetFn>>> = const { RefCell::new(None) };
}

fn forget_offsets() {
    GENERATION.with(|generation| generation.set(generation.get() + 1));
}

fn install(offsets: Option<Rc<OffsetFn>>) -> Option<Rc<OffsetFn>> {
    if let Some(host) = &offsets {
        let known = LAST.with(|last| last.borrow().as_ref().is_some_and(|l| Rc::ptr_eq(l, host)));
        if !known {
            LAST.with(|last| *last.borrow_mut() = Some(Rc::clone(host)));
            forget_offsets();
        }
    }
    CHECKED.with(|checked| checked.set(false));
    OFFSETS.with(|slot| slot.replace(offsets))
}

struct OffsetsRestore(Option<Rc<OffsetFn>>);

impl Drop for OffsetsRestore {
    fn drop(&mut self) {
        install(self.0.take());
    }
}

pub fn with_offsets<T>(offsets: Option<Hook<OffsetFn>>, run: impl FnOnce() -> T) -> T {
    let _restore = OffsetsRestore(install(offsets.map(|hook| hook.0)));
    run()
}

fn check_zone(offsets: &OffsetFn) {
    if !CHECKED.with(|checked| checked.replace(true)) {
        let version = offsets(f64::NAN).to_bits();
        if VERSION.with(|known| known.replace(version)) != version {
            forget_offsets();
        }
    }
}

fn offset_at(utc_ms: f64) -> f64 {
    if !utc_ms.is_finite() {
        return 0.0;
    }
    let seconds = (utc_ms / 1000.0).floor() as i64;
    let slot = ((seconds as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize;
    let host = OFFSETS.with(|slot| slot.borrow().clone());
    if let Some(offsets) = &host {
        check_zone(offsets.as_ref());
    }
    let stamp = if host.is_some() {
        GENERATION.with(Cell::get)
    } else {
        0
    };
    CACHE.with(|cache| {
        if let Some(&(known, offset, cached)) = cache.borrow().get(slot)
            && known == seconds
            && cached == stamp
        {
            return offset;
        }
        let instant = seconds.clamp(Timestamp::MIN.as_second(), Timestamp::MAX.as_second());
        let offset = match &host {
            Some(offsets) => offsets(instant as f64 * 1000.0),
            None => ZONE.with(|zone| {
                let at = Timestamp::from_second(instant).unwrap_or_default();
                f64::from(zone.to_offset(at).seconds()) * 1000.0
            }),
        };
        cache.borrow_mut()[slot] = (seconds, offset, stamp);
        offset
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_host_offsets_until_the_zone_changes() {
        let calls = Rc::new(Cell::new(0));
        let version = Rc::new(Cell::new(1.0));
        let (counted, zone) = (Rc::clone(&calls), Rc::clone(&version));
        let host: Rc<OffsetFn> = Rc::new(move |ms: f64| {
            if ms.is_nan() {
                zone.get()
            } else {
                counted.set(counted.get() + 1);
                3_600_000.0
            }
        });
        let date = JsDate::from_ms(0.0);
        let run = || with_offsets(Some(Hook(Rc::clone(&host))), || date.timezone_offset());
        assert_eq!((run(), run(), calls.get()), (-60.0, -60.0, 1));
        version.set(2.0);
        assert_eq!((run(), calls.get()), (-60.0, 2));
    }

    #[test]
    fn offsets_come_from_the_host_inside_the_scope() {
        let east: Hook<OffsetFn> = Hook(Rc::new(|_| 7.0 * 3_600_000.0));
        let midnight = JsDate::from_ms(JsDate::utc(2017.0, 6.0, 20.0, 0.0, 0.0, 0.0, 0.0));
        let (offset, hours) = with_offsets(Some(east), || {
            (
                midnight.timezone_offset(),
                JsDate::local(2017.0, 6.0, 20.0, 7.0, 0.0, 0.0, 0.0).time(),
            )
        });
        assert_eq!(offset, -420.0);
        assert_eq!(hours, midnight.time());
        assert_eq!(
            midnight.timezone_offset(),
            -offset_at(midnight.time()) / 60_000.0
        );
        assert_eq!(offset_at(f64::NAN), 0.0);
    }
}
