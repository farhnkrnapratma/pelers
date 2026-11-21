use crate::util::jsnum::{js_max, js_pow, js_round, js_to_precision_number};

pub fn tick_step(span: f64, count: f64) -> f64 {
    let raw = span / js_max(count, 1.0);
    let power = raw.log10().floor();
    let unit = raw / js_pow(10.0, power);
    let factor = if unit >= 7.07 {
        10.0
    } else if unit >= 3.16 {
        5.0
    } else if unit >= 1.41 {
        2.0
    } else {
        1.0
    };
    if power < 0.0 {
        -(js_pow(10.0, -power)) / factor
    } else {
        factor * js_pow(10.0, power)
    }
}

pub fn tick_index(step: f64, value: f64) -> f64 {
    if step > 0.0 {
        value / step
    } else {
        value * -step
    }
}

pub fn tick_value(step: f64, index: f64) -> f64 {
    js_to_precision_number(
        if step > 0.0 {
            index * step
        } else {
            index / -step
        },
        15,
    )
}

pub fn ticks_between(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let mut first = js_round(tick_index(step, lo));
    if tick_value(step, first) < lo {
        first += 1.0;
    }
    let mut last = js_round(tick_index(step, hi));
    if tick_value(step, last) > hi {
        last -= 1.0;
    }
    let mut out = Vec::new();
    let mut i = first;
    while i <= last && out.len() < 100 {
        out.push(tick_value(step, i));
        i += 1.0;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_values_drop_float_noise() {
        assert_ne!(3.0 * 1e23, 3e23);
        assert_eq!(tick_value(1e23, 3.0), 3e23);
        assert_eq!(tick_value(-10.0, 3.0), 0.3);
        assert!(tick_value(1e308, 10.0).is_infinite());
    }
}
