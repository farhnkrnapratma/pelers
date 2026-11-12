pub fn js_parse_int(text: &str) -> f64 {
    let trimmed = text.trim_start_matches(crate::util::chars::is_js_space);
    let (sign, digits) = match trimmed.as_bytes().first() {
        Some(b'-') => (-1.0, &trimmed[1..]),
        Some(b'+') => (1.0, &trimmed[1..]),
        _ => (1.0, trimmed),
    };
    let run: String = digits.chars().take_while(|c| c.is_ascii_digit()).collect();
    if run.is_empty() {
        return f64::NAN;
    }
    sign * run.parse::<f64>().unwrap_or(f64::NAN)
}

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

pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b {
        a
    } else {
        b
    }
}

pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a < b {
        a
    } else {
        b
    }
}

pub fn js_hypot(a: f64, b: f64) -> f64 {
    a.hypot(b)
}

pub fn to_fixed_0(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.abs() >= 1e21 {
        return js_number_to_string(value);
    }
    if value < 0.0 {
        return format!("-{}", to_fixed_0(-value));
    }
    format!("{:.0}", js_round(value) + 0.0)
}

pub fn js_number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == 0.0 {
        return "0".to_owned();
    }
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return (value as i64).to_string();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-Infinity" } else { "Infinity" }.to_owned();
    }
    if (1e-6..1e21).contains(&value.abs()) {
        return format!("{value}");
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

pub fn js_number(text: &str) -> f64 {
    let text = crate::util::chars::trim(text);
    if text.is_empty() {
        return 0.0;
    }
    let radix = match text.get(..2) {
        Some("0x" | "0X") => 16,
        Some("0o" | "0O") => 8,
        Some("0b" | "0B") => 2,
        _ => 0,
    };
    if radix != 0 {
        let digits = &text[2..];
        if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
            return f64::NAN;
        }
        return match u128::from_str_radix(digits, radix) {
            Ok(value) => value as f64,
            Err(_) => digits.chars().fold(0.0, |acc, c| {
                acc * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
            }),
        };
    }
    let body = text.strip_prefix(['+', '-']).unwrap_or(text);
    let sign = if text.starts_with('-') { -1.0 } else { 1.0 };
    if body == "Infinity" {
        return sign * f64::INFINITY;
    }
    let bytes = body.as_bytes();
    let digits = |from: usize| {
        from + bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let int_end = digits(0);
    let has_dot = bytes.get(int_end) == Some(&b'.');
    let frac_end = if has_dot {
        digits(int_end + 1)
    } else {
        int_end
    };
    let mantissa_digits = int_end + if has_dot { frac_end - int_end - 1 } else { 0 };
    let exp_end = match bytes.get(frac_end) {
        Some(b'e' | b'E') => {
            let start = match bytes.get(frac_end + 1) {
                Some(b'+' | b'-') => frac_end + 2,
                _ => frac_end + 1,
            };
            let end = digits(start);
            if end == start {
                return f64::NAN;
            }
            end
        }
        _ => frac_end,
    };
    if mantissa_digits == 0 || exp_end != bytes.len() {
        return f64::NAN;
    }
    text.parse::<f64>().unwrap_or(f64::NAN)
}

pub fn js_parse_float(text: &str) -> f64 {
    let text = crate::util::chars::trim_start(text);
    let (sign, body) = match text.as_bytes().first() {
        Some(b'-') => (-1.0, &text[1..]),
        Some(b'+') => (1.0, &text[1..]),
        _ => (1.0, text),
    };
    if body.starts_with("Infinity") {
        return sign * f64::INFINITY;
    }
    let bytes = body.as_bytes();
    let digits = |from: usize| {
        from + bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let int_end = digits(0);
    let (mantissa_end, mantissa_digits) = if bytes.get(int_end) == Some(&b'.') {
        let frac_end = digits(int_end + 1);
        (frac_end, int_end + frac_end - int_end - 1)
    } else {
        (int_end, int_end)
    };
    if mantissa_digits == 0 {
        return f64::NAN;
    }
    let mut end = mantissa_end;
    if let Some(b'e' | b'E') = bytes.get(end) {
        let start = match bytes.get(end + 1) {
            Some(b'+' | b'-') => end + 2,
            _ => end + 1,
        };
        let exp_end = digits(start);
        if exp_end > start {
            end = exp_end;
        }
    }
    sign * body[..end].parse::<f64>().unwrap_or(f64::NAN)
}

fn exact_parts(x: f64) -> (String, String) {
    let text = format!("{:.1100}", x.abs());
    match text.split_once('.') {
        Some((int, frac)) => (int.to_owned(), frac.to_owned()),
        None => (text, String::new()),
    }
}

fn increment(digits: &mut Vec<u8>) -> bool {
    for d in digits.iter_mut().rev() {
        if *d == b'9' {
            *d = b'0';
        } else {
            *d += 1;
            return false;
        }
    }
    digits.insert(0, b'1');
    true
}

fn non_finite(x: f64) -> Option<String> {
    if x.is_nan() {
        Some("NaN".to_owned())
    } else if x.is_infinite() {
        Some(if x < 0.0 { "-Infinity" } else { "Infinity" }.to_owned())
    } else {
        None
    }
}

pub fn js_to_fixed(x: f64, digits: usize) -> String {
    if let Some(text) = non_finite(x) {
        return text;
    }
    if x.abs() >= 1e21 {
        return js_number_to_string(x);
    }
    let (int, frac) = exact_parts(x);
    let int_len = int.len();
    let cut = int_len + digits;
    let mut all: Vec<u8> = int
        .bytes()
        .chain(frac.bytes().chain(std::iter::repeat(b'0')))
        .take(cut + 1)
        .collect();
    let round_up = all[cut] >= b'5';
    all.truncate(cut);
    let mut int_count = int_len;
    if round_up && increment(&mut all) {
        int_count += 1;
    }
    let (whole, fraction) = all.split_at(int_count);
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    let trimmed = whole
        .iter()
        .position(|&d| d != b'0')
        .map_or(&whole[whole.len() - 1..], |at| &whole[at..]);
    out.push_str(std::str::from_utf8(trimmed).unwrap_or("0"));
    if digits > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(fraction).unwrap_or(""));
    }
    out
}

fn significant(x: f64, significant: usize) -> (Vec<u8>, i32) {
    let (int, frac) = exact_parts(x);
    let all: Vec<u8> = int.bytes().chain(frac.bytes()).collect();
    let first = all.iter().position(|&d| d != b'0').unwrap_or(0);
    let exponent = int.len() as i32 - 1 - first as i32;
    let mut digits: Vec<u8> = all[first..]
        .iter()
        .copied()
        .chain(std::iter::repeat(b'0'))
        .take(significant + 1)
        .collect();
    let round_up = digits[significant] >= b'5';
    digits.truncate(significant);
    let mut exponent = exponent;
    if round_up && increment(&mut digits) {
        digits.truncate(significant);
        exponent += 1;
    }
    (digits, exponent)
}

fn exponent_text(exponent: i32) -> String {
    format!(
        "e{}{}",
        if exponent < 0 { '-' } else { '+' },
        exponent.abs()
    )
}

pub fn js_to_exponential(x: f64, digits: usize) -> String {
    if let Some(text) = non_finite(x) {
        return text;
    }
    let sign = if x < 0.0 { "-" } else { "" };
    if x == 0.0 {
        let zeros = if digits > 0 {
            format!(".{}", "0".repeat(digits))
        } else {
            String::new()
        };
        return format!("{sign}0{zeros}e+0");
    }
    let (sig, exponent) = significant(x, digits + 1);
    let mut out = format!("{sign}{}", sig[0] as char);
    if digits > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&sig[1..]).unwrap_or(""));
    }
    out.push_str(&exponent_text(exponent));
    out
}

pub fn js_to_precision(x: f64, precision: usize) -> String {
    if let Some(text) = non_finite(x) {
        return text;
    }
    let sign = if x < 0.0 { "-" } else { "" };
    let (sig, exponent) = if x == 0.0 {
        (vec![b'0'; precision], 0)
    } else {
        significant(x, precision)
    };
    let digits = std::str::from_utf8(&sig).unwrap_or("");
    if exponent < -6 || exponent >= precision as i32 {
        let mut out = format!("{sign}{}", &digits[..1]);
        if precision > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push_str(&exponent_text(exponent));
        return out;
    }
    if exponent >= 0 {
        let at = exponent as usize + 1;
        let mut out = format!("{sign}{}", &digits[..at]);
        if precision > at {
            out.push('.');
            out.push_str(&digits[at..]);
        }
        return out;
    }
    format!("{sign}0.{}{digits}", "0".repeat((-exponent - 1) as usize))
}

#[cfg(test)]
mod tests {
    use super::{js_min, js_number_to_string, js_round, to_fixed_0};

    #[test]
    fn fixes_to_zero_places_like_javascript() {
        assert_eq!(to_fixed_0(0.5), "1");
        assert_eq!(to_fixed_0(1.5), "2");
        assert_eq!(to_fixed_0(2.5), "3");
        assert_eq!(to_fixed_0(0.49999999999999994), "0");
        assert_eq!(to_fixed_0(-0.4), "-0");
        assert_eq!(to_fixed_0(-0.0), "0");
        assert_eq!(to_fixed_0(45.96), "46");
        assert_eq!(to_fixed_0(1e21), "1e+21");
    }

    #[test]
    fn min_propagates_nan() {
        assert!(js_min(f64::NAN, 1.0).is_nan());
        assert_eq!(js_min(2.0, 1.0), 1.0);
    }

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

    #[test]
    fn keeps_nan_through_fixed_and_min() {
        assert_eq!(to_fixed_0(f64::NAN), "NaN");
        assert!(js_min(f64::NAN, 1.0).is_nan());
        assert!(js_min(1.0, f64::NAN).is_nan());
        assert_eq!(js_min(2.0, 1.0), 1.0);
        assert_eq!(js_min(1.0, 2.0), 1.0);
    }
}
