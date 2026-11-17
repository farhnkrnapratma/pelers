use std::sync::OnceLock;

use super::metrics::{BOLD, REGULAR};

pub const BOLD_STYLE: u32 = 1;
pub const ITALIC: u32 = 2;
pub const MONO: u32 = 4;
pub const SUB: u32 = 8;
pub const SUP: u32 = 16;
pub const UNDERLINE: u32 = 32;
pub const MEASURE_MONO: u32 = 64;

const SMALLER: f64 = 1.2;

fn unpack(packed: &str) -> [u16; 95] {
    let mut out = [0; 95];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u16::from_str_radix(&packed[i * 2..i * 2 + 2], 36).expect("base-36 metrics table");
    }
    out
}

fn regular() -> &'static [u16; 95] {
    static TABLE: OnceLock<[u16; 95]> = OnceLock::new();
    TABLE.get_or_init(|| unpack(REGULAR))
}

fn bold() -> &'static [u16; 95] {
    static TABLE: OnceLock<[u16; 95]> = OnceLock::new();
    TABLE.get_or_init(|| unpack(BOLD))
}

fn is_wide(c: u16) -> bool {
    matches!(c,
        0x1100..=0x115f | 0x2e80..=0xa4cf | 0xac00..=0xd7a3 | 0xf900..=0xfaff | 0xfe30..=0xfe4f
        | 0xff00..=0xff60 | 0xffe0..=0xffe6 | 0xd800..=0xdbff)
}

fn is_zero_width(c: u16) -> bool {
    matches!(c, 0xad | 0x300..=0x36f | 0x200b..=0x200f | 0xdc00..=0xdfff | 0xfe00..=0xfe0f | 0xfeff)
}

pub type MeasureFn = dyn Fn(&str, f64, u32) -> f64;

thread_local! {
    static MEASURER: std::cell::RefCell<Option<std::rc::Rc<MeasureFn>>> = const { std::cell::RefCell::new(None) };
}

struct MeasurerRestore(Option<std::rc::Rc<MeasureFn>>);

impl Drop for MeasurerRestore {
    fn drop(&mut self) {
        MEASURER.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}

pub fn with_measurer<T>(
    measurer: Option<crate::util::hook::Hook<MeasureFn>>,
    run: impl FnOnce() -> T,
) -> T {
    let outer = MEASURER.with(|slot| slot.replace(measurer.map(|hook| hook.0)));
    let _restore = MeasurerRestore(outer);
    run()
}

pub fn width(text: &str, size: f64, style: u32) -> f64 {
    if let Some(measure) = MEASURER.with(|slot| slot.borrow().clone()) {
        return measure(text, size, style);
    }
    let mono = style & (MONO | MEASURE_MONO) != 0;
    let eff = if style & (SUB | SUP) != 0 {
        size / SMALLER
    } else {
        size
    };
    let table = if style & BOLD_STYLE != 0 {
        bold()
    } else {
        regular()
    };
    let mut units: u32 = 0;
    for c in text.encode_utf16() {
        if c < 32 {
            continue;
        }
        if c < 127 {
            units += if mono {
                600
            } else {
                u32::from(table[usize::from(c) - 32])
            };
        } else if is_zero_width(c) {
            continue;
        } else if is_wide(c) {
            units += if mono { 1200 } else { 1000 };
        } else {
            units += if mono { 600 } else { 580 };
        }
    }
    (f64::from(units) * eff) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_ascii_with_the_regular_and_bold_tables() {
        assert_eq!(
            width("Pel", 16.0, 0),
            (f64::from(667 + 556 + 222) * 16.0) / 1000.0
        );
        assert!(width("Pel", 16.0, BOLD_STYLE) > width("Pel", 16.0, 0));
    }

    #[test]
    fn counts_utf16_units() {
        assert_eq!(width("\u{1F600}", 10.0, 0), 10.0);
        assert_eq!(width("é", 10.0, 0), 5.8);
        assert_eq!(width("a\u{301}", 10.0, 0), width("a", 10.0, 0));
        assert_eq!(width("中", 10.0, MONO), 12.0);
    }

    #[test]
    fn shrinks_sub_and_sup() {
        assert_eq!(width("a", 12.0, SUB), width("a", 10.0, 0));
    }
}
