use std::sync::OnceLock;

use regex::Regex;

use crate::util::jsnum::{
    js_number, js_number_to_string, js_round, js_to_exponential, js_to_fixed, js_to_precision,
};

fn spec_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^(?:[^\n\r\x{2028}\x{2029}]?[<>=^])?([-+( ])?(\$)?0?[0-9]*(,)?(?:\.([0-9]+))?(~)?([a-zA-Z%])?$")
            .expect("format specifier pattern")
    })
}

fn group(digits: &str) -> String {
    let mut out = String::new();
    let mut i = digits.len();
    while i > 0 {
        let start = i.saturating_sub(3);
        let piece = &digits[start..i];
        out = if out.is_empty() {
            piece.to_owned()
        } else {
            format!("{piece},{out}")
        };
        i = start;
    }
    out
}

fn trim_zeros(body: &str) -> String {
    let stripped = body.trim_end_matches('0');
    let end = if stripped.len() < body.len() && stripped.ends_with('.') {
        stripped.len() - 1
    } else {
        stripped.len()
    };
    body[..end].to_owned()
}

pub struct Formatter {
    plus: bool,
    comma: bool,
    precision: Option<usize>,
    kind: String,
    currency: String,
    trim: bool,
}

pub fn formatter(spec: &str) -> Formatter {
    let spec = if spec == "$0,0" { "$," } else { spec };
    let re = spec_re();
    let caps = re
        .captures(spec)
        .or_else(|| re.captures(","))
        .expect("the fallback specifier matches");
    Formatter {
        plus: caps.get(1).is_some_and(|m| m.as_str() == "+"),
        comma: caps.get(3).is_some(),
        precision: caps
            .get(4)
            .map(|m| js_number(m.as_str()).min(20.0) as usize),
        kind: caps.get(6).map_or(String::new(), |m| m.as_str().to_owned()),
        currency: caps.get(2).map_or(String::new(), |m| m.as_str().to_owned()),
        trim: caps.get(5).is_some(),
    }
}

impl Formatter {
    pub fn format(&self, value: f64) -> String {
        let abs = value.abs();
        let mut suffix = "";
        let mut body = match self.kind.as_str() {
            "f" => js_to_fixed(abs, self.precision.unwrap_or(6)),
            "%" => {
                suffix = "%";
                js_to_fixed(abs * 100.0, self.precision.unwrap_or(6))
            }
            "d" => js_to_fixed(js_round(abs), 0),
            "e" => js_to_exponential(abs, self.precision.unwrap_or(6)),
            _ => {
                let digits = js_max_digits(self.precision);
                js_number_to_string(js_number(&js_to_precision(abs, digits)))
            }
        };
        if self.trim && body.contains('.') && !body.contains('e') {
            body = trim_zeros(&body);
        }
        if self.comma && !body.contains('e') {
            body = match body.find('.') {
                None => group(&body),
                Some(dot) => format!("{}{}", group(&body[..dot]), &body[dot..]),
            };
        }
        let sign = if value < 0.0 {
            "-"
        } else if self.plus {
            "+"
        } else {
            ""
        };
        format!("{sign}{}{body}{suffix}", self.currency)
    }
}

fn js_max_digits(precision: Option<usize>) -> usize {
    precision.unwrap_or(12).max(1)
}
