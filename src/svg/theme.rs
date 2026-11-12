pub const ROOT_STYLE: &str = "--_bg:var(--pelers-bg,#fff);--_fg:var(--pelers-fg,#1f1f1f);--_m:var(--pelers-muted,#6e6e6e);\
--_l:var(--pelers-line,#1f1f1f);--_s:var(--pelers-surface,#f3f3f3);--_a:var(--pelers-surface-alt,#e6e6e6);\
--_b:var(--pelers-border,#8a8a8a);--_c:var(--pelers-accent,#5b7bd5);color:var(--_fg)";

pub const ROUNDED: &str = " style=\"rx:var(--_r)\"";
pub const CRITICAL: &str = "var(--pelers-critical,#e45756)";
pub const RADIUS: &str = "var(--pelers-radius,4px)";

fn quoted_body(c: char) -> bool {
    !matches!(
        c,
        '"' | '\'' | '\\' | '\n' | '<' | '>' | '{' | '}' | '(' | ')' | ';' | ':' | '@'
    )
}

fn unquoted_start(c: char) -> bool {
    c.is_alphabetic() || c.is_numeric() || c == '_' || c == '-'
}

fn unquoted_rest(c: char) -> bool {
    c.is_alphabetic() || c.is_numeric() || matches!(c, '_' | ' ' | '.' | '-')
}

fn is_family_list(list: &str) -> bool {
    use crate::util::chars::{is_js_space, trim_start};
    let mut rest = trim_start(list);
    loop {
        let mut chars = rest.chars();
        let used = match chars.next() {
            Some(quote @ ('"' | '\'')) => {
                let body = rest[1..]
                    .find(|c| !quoted_body(c))
                    .unwrap_or(rest.len() - 1);
                if !rest[1 + body..].starts_with(quote) {
                    return false;
                }
                1 + body + 1
            }
            Some(first) if unquoted_start(first) => {
                let tail = rest[first.len_utf8()..]
                    .find(|c| !unquoted_rest(c))
                    .unwrap_or(rest.len() - first.len_utf8());
                first.len_utf8() + tail
            }
            _ => return false,
        };
        rest = &rest[used..];
        let after = rest.trim_start_matches(is_js_space);
        if after.is_empty() {
            return true;
        }
        match after.strip_prefix(',') {
            Some(next) => rest = trim_start(next),
            None => return false,
        }
    }
}

fn families<'a>(list: Option<&'a str>, generic: &'a str) -> &'a str {
    match list {
        Some(list) if is_family_list(list) => list,
        _ => generic,
    }
}

type FontEntry = (u64, Option<String>, Option<String>, String);

thread_local! {
    static FONT_CACHE: std::cell::RefCell<Option<FontEntry>> =
        const { std::cell::RefCell::new(None) };
}

pub fn font_style(size: f64, family: Option<&str>, mono: Option<&str>) -> String {
    let key = size.to_bits();
    FONT_CACHE.with(|cache| {
        if let Some((bits, cached_family, cached_mono, css)) = cache.borrow().as_ref()
            && *bits == key
            && cached_family.as_deref() == family
            && cached_mono.as_deref() == mono
        {
            return css.clone();
        }
        let css = format!(
            ";--_fm:var(--pelers-font-mono,{});font:{}px sans-serif;letter-spacing:normal;word-spacing:normal;text-transform:none;direction:ltr;font-family:var(--pelers-font,{})",
            families(mono, "monospace"),
            crate::util::jsnum::js_number_to_string(size),
            families(family, "sans-serif")
        );
        *cache.borrow_mut() = Some((key, family.map(str::to_owned), mono.map(str::to_owned), css.clone()));
        css
    })
}

const SERIES: [&str; 8] = [
    "#4c78a8", "#f58518", "#54a24b", "#e45756", "#72b7b2", "#eeca3b", "#b279a2", "#9d755d",
];

pub fn series_color(index: usize) -> String {
    let k = index % 8;
    format!("var(--pelers-series-{},{})", k + 1, SERIES[k])
}

pub fn rounded(attrs: &str) -> String {
    match attrs.find(" style=\"") {
        None => {
            let mut out = String::with_capacity(attrs.len() + ROUNDED.len());
            out.push_str(attrs);
            out.push_str(ROUNDED);
            out
        }
        Some(at) => {
            let split = at + 8;
            let mut out = String::with_capacity(attrs.len() + 16);
            out.push_str(&attrs[..split]);
            out.push_str("rx:var(--_r);");
            out.push_str(&attrs[split..]);
            out
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct ResolvedStyle {
    pub shape: String,
    pub line: String,
    pub text: String,
    pub stroke: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub font_size: Option<f64>,
}

const SHAPE_PROPS: &[&str] = &[
    "fill",
    "fill-opacity",
    "stroke",
    "stroke-width",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-opacity",
    "opacity",
    "rx",
    "ry",
];

const TEXT_PROPS: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
    "text-decoration",
    "letter-spacing",
    "word-spacing",
    "opacity",
];

fn set_entry(map: &mut Vec<(String, String)>, key: &str, value: String) {
    match map.iter_mut().find(|(k, _)| k == key) {
        Some(slot) => slot.1 = value,
        None => map.push((key.to_owned(), value)),
    }
}

fn get_entry<'a>(map: &'a [(String, String)], key: &str) -> Option<&'a str> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

fn unsafe_value(value: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)url\s*\(|expression\s*\(|javascript:|[<>{};\\]|@import|image-set|/\*",
        )
        .expect("unsafe style pattern")
    })
    .is_match(value)
}

fn important_pattern() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?i)!important").expect("important pattern"))
}

fn js_number(text: &str) -> f64 {
    let t = text.trim();
    if t.is_empty() {
        0.0
    } else {
        t.parse::<f64>().unwrap_or(f64::NAN)
    }
}

pub fn resolve_style(declarations: &[String]) -> ResolvedStyle {
    if declarations.is_empty() {
        return ResolvedStyle::default();
    }
    let mut shape: Vec<(String, String)> = Vec::new();
    let mut text: Vec<(String, String)> = Vec::new();
    for decl in declarations {
        let Some(at) = decl.find(':').filter(|&at| at > 0) else {
            continue;
        };
        let prop = decl[..at].trim().to_lowercase();
        let value = important_pattern()
            .replace_all(&decl[at + 1..], "")
            .trim()
            .to_owned();
        if value.is_empty() || unsafe_value(&value) || crate::svg::builder::has_invalid_xml(&value)
        {
            continue;
        }
        if SHAPE_PROPS.contains(&prop.as_str()) {
            set_entry(&mut shape, &prop, value.clone());
        }
        if TEXT_PROPS.contains(&prop.as_str()) {
            let key = if prop == "color" {
                "fill".to_owned()
            } else {
                prop.clone()
            };
            set_entry(&mut text, &key, value);
        }
    }
    if shape.is_empty() && text.is_empty() {
        return ResolvedStyle::default();
    }
    let mut shape_css = String::new();
    let mut line_css = String::new();
    for (prop, value) in &shape {
        shape_css.push_str(&format!("{prop}:{value};"));
        if prop != "fill" && prop != "fill-opacity" {
            line_css.push_str(&format!("{prop}:{value};"));
        }
    }
    let mut text_css = String::new();
    for (prop, value) in &text {
        text_css.push_str(&format!("{prop}:{value};"));
    }
    let weight = get_entry(&text, "font-weight");
    let size = get_entry(&text, "font-size");
    let px = size.and_then(|s| {
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        let re = RE.get_or_init(|| regex::Regex::new(r"^[\d.]+(px)?$").expect("size pattern"));
        if re.is_match(s) {
            Some(crate::util::jsnum::js_parse_float(s))
        } else {
            None
        }
    });
    let bold = matches!(weight, Some("bold") | Some("bolder"))
        || weight.is_some_and(|w| js_number(w) >= 600.0);
    ResolvedStyle {
        shape: if shape_css.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", crate::svg::builder::esc(&shape_css))
        },
        line: if line_css.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", crate::svg::builder::esc(&line_css))
        },
        text: if text_css.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", crate::svg::builder::esc(&text_css))
        },
        stroke: get_entry(&shape, "stroke").map(str::to_owned),
        bold,
        italic: get_entry(&text, "font-style") == Some("italic"),
        font_size: px.filter(|&p| p > 0.0 && p <= 200.0),
    }
}

pub fn class_names(raw: &str) -> String {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"[^A-Za-z0-9_-]+").expect("class pattern"));
    let cleaned = re.replace_all(raw, " ");
    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        String::new()
    } else {
        format!(" {cleaned}")
    }
}

pub fn sets_fill(shape: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"[";]fill:"#).expect("fill pattern"))
        .is_match(shape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_font_lists_and_rejects_declaration_breakers() {
        assert!(is_family_list("Inter, \"Helvetica Neue\", sans-serif"));
        assert!(is_family_list(" 'Fira Code' "));
        assert!(!is_family_list("Inter; color: red"));
        assert!(!is_family_list("\"unclosed"));
        assert!(!is_family_list("a,"));
        assert!(!is_family_list(""));
    }

    #[test]
    fn falls_back_to_generic_families() {
        let css = font_style(16.0, Some("x{}"), None);
        assert!(css.ends_with("font-family:var(--pelers-font,sans-serif)"));
        assert!(css.starts_with(";--_fm:var(--pelers-font-mono,monospace);font:16px sans-serif;"));
    }
}
