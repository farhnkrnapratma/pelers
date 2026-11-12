use std::sync::OnceLock;

use regex::Regex;

use crate::text::label::{Label, layout_label};
use crate::text::measurer::width;
use crate::util::chars::{is_js_space, trim, utf16_len};

const JS_WS: &str = "\\t\\n\\x{b}\\x{c}\\r \\x{a0}\\x{1680}\\x{2000}-\\x{200a}\\x{2028}\\x{2029}\\x{202f}\\x{205f}\\x{3000}\\x{feff}";

fn unit_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            "<[bB][rR][{JS_WS}]*/?>|ﬂ°°?[A-Za-z0-9_]+¶ß|&#?[A-Za-z0-9_]+;|(?s:.)"
        ))
        .expect("unit pattern")
    })
}

fn escape_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (i, c) in text.char_indices() {
        if c == '<' && !is_br_at(&text[i + 1..]) {
            out.push_str("&lt;");
        } else {
            out.push(c);
        }
    }
    out
}

fn is_br_at(rest: &str) -> bool {
    if !rest.get(..2).is_some_and(|s| s.eq_ignore_ascii_case("br")) {
        return false;
    }
    let body = rest[2..].trim_start_matches(is_js_space);
    body.strip_prefix('/').unwrap_or(body).starts_with('>')
}

pub fn plain_label(raw: Option<&str>, size: f64, max: f64, base: u32) -> Label {
    let text = raw.map(|text| {
        let text = if text.contains('<') {
            escape_tags(text)
        } else {
            text.to_owned()
        };
        text.replace(":fa-", "&colon;fa-")
    });
    layout_label(text.as_deref(), false, size, max, base)
}

pub fn fit_label(raw: Option<&str>, size: f64, max: f64, base: u32) -> Label {
    let label = plain_label(raw, size, max, base);
    let Some(raw) = raw else {
        return label;
    };
    if label.width <= max {
        return label;
    }
    let mut out = String::with_capacity(raw.len());
    let mut at = 0.0;
    for unit in unit_re().find_iter(raw).map(|m| m.as_str()) {
        if trim(unit).is_empty() || (utf16_len(unit) > 2 && unit.starts_with('<')) {
            at = 0.0;
        } else {
            let measured = if utf16_len(unit) > 2 { "x" } else { unit };
            let w = width(measured, size, base);
            if at > 0.0 && at + w > max {
                out.push('\n');
                at = 0.0;
            }
            at += w;
        }
        out.push_str(unit);
    }
    plain_label(Some(&out), size, max, base)
}
