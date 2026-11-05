//! JavaScript number semantics the SVG output depends on: `Math.round` and `Number#toString`.

/// `Math.round`: ties go toward +infinity (`f64::round` rounds ties away from zero).
pub fn js_round(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let floor = value.floor();
    if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// `String(value)` for a JavaScript number (ECMAScript Number::toString, radix 10).
pub fn js_number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-Infinity" } else { "Infinity" }.to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("`{:e}` always has an exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent
        .parse::<i32>()
        .expect("`{:e}` exponent is an integer")
        + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let power = n - 1;
        let sign_power = if power < 0 { "-" } else { "+" };
        let tail = if k == 1 {
            String::new()
        } else {
            format!(".{}", &digits[1..])
        };
        format!("{}{tail}e{sign_power}{}", &digits[..1], power.abs())
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests {
    use super::{js_number_to_string, js_round};

    #[test]
    fn rounds_ties_toward_positive_infinity() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.5), 1.0);
        assert_eq!(js_round(-0.5), 0.0);
        assert_eq!(js_round(-1.5), -1.0);
        assert_eq!(js_round(1.4999999999999998), 1.0);
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(js_round(-0.2), 0.0);
        assert_eq!(js_round(7.0), 7.0);
        assert!(js_round(f64::NAN).is_nan());
        assert_eq!(js_round(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn formats_numbers_like_javascript() {
        let cases: [(f64, &str); 18] = [
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-1.5, "-1.5"),
            (100.0, "100"),
            (0.1 + 0.2, "0.30000000000000004"),
            (123.456, "123.456"),
            (0.000001, "0.000001"),
            (0.0000001, "1e-7"),
            (1.5e-7, "1.5e-7"),
            (1e21, "1e+21"),
            (1.5e21, "1.5e+21"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e20, "100000000000000000000"),
            (4294967296.0, "4294967296"),
            (-0.000123, "-0.000123"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
        ];
        for (value, expected) in cases {
            assert_eq!(js_number_to_string(value), expected, "{value:?}");
        }
        assert_eq!(js_number_to_string(f64::NAN), "NaN");
    }
}
