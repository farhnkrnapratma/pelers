use std::cell::RefCell;

use super::builder::esc;
use super::ink::{ink, plain_color};
use crate::util::chars::trim;
use crate::util::value::{Object, Value};

const SLOTS: usize = 8;

const FUNCTIONS: [&str; 13] = [
    "rgb",
    "rgba",
    "hsl",
    "hsla",
    "hwb",
    "lab",
    "lch",
    "oklab",
    "oklch",
    "color",
    "color-mix",
    "light-dark",
    "var",
];

struct Scale {
    prefix: &'static str,
    first: usize,
    repeat: bool,
}

fn scale(kind: &str) -> Option<Scale> {
    let (prefix, first, repeat) = match kind {
        "pie" => ("pie", 1, false),
        "gitGraph" => ("git", 0, false),
        "timeline" | "radar" | "treemap" => ("cScale", 0, false),
        "mindmap" => ("cScale", 1, false),
        "venn" => ("venn", 1, true),
        _ => return None,
    };
    Some(Scale {
        prefix,
        first,
        repeat,
    })
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn is_color_char(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || c == '_'
        || crate::util::chars::is_js_space(c)
        || "#%.,+-/()".contains(c)
}

fn function_names_allowed(color: &str) -> bool {
    let bytes = color.as_bytes();
    for (open, _) in color.match_indices('(') {
        let mut end = open;
        while end > 0 && crate::util::chars::is_js_space(color[..end].chars().next_back().unwrap())
        {
            end -= color[..end].chars().next_back().unwrap().len_utf8();
        }
        let mut start = end;
        while start > 0 && (is_word_byte(bytes[start - 1]) || bytes[start - 1] == b'-') {
            start -= 1;
        }
        if !FUNCTIONS.contains(&color[start..end].to_ascii_lowercase().as_str()) {
            return false;
        }
    }
    true
}

fn variables_allowed(color: &str) -> bool {
    let lower = color.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find("var(") {
        let mut at = from + found + 4;
        while let Some(c) = color[at..].chars().next() {
            if !crate::util::chars::is_js_space(c) {
                break;
            }
            at += c.len_utf8();
        }
        let bytes = color.as_bytes();
        let mut end = at;
        while end < bytes.len() && (is_word_byte(bytes[end]) || bytes[end] == b'-') {
            end += 1;
        }
        if !color[at..end].starts_with("--pele-") {
            return false;
        }
        from = end;
    }
    true
}

fn safe_color(value: &Value) -> Option<String> {
    let Value::String(raw) = value else {
        return None;
    };
    let color = trim(raw);
    let units = color.encode_utf16().count();
    if units == 0 || units > 200 || !color.chars().all(is_color_char) {
        return None;
    }
    if !color.contains('(') {
        return plain_color(color).then(|| color.to_owned());
    }
    let mut depth = 0i64;
    for c in color.chars() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth < 0 {
                return None;
            }
        }
    }
    if depth != 0 || !function_names_allowed(color) || !variables_allowed(color) {
        return None;
    }
    Some(color.to_owned())
}

fn split_list(value: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i64;
    let mut start = 0;
    for (i, c) in value.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(trim(&value[start..i]).to_owned());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(trim(&value[start..]).to_owned());
    parts
}

fn record(value: Option<&Value>) -> Option<&Object> {
    match value {
        Some(Value::Object(object)) => Some(object),
        _ => None,
    }
}

fn entries(kind: &str, vars: Option<&Object>) -> Option<Vec<Value>> {
    if kind == "xychart" {
        let list =
            record(vars.and_then(|v| v.get("xyChart"))).and_then(|x| x.get("plotColorPalette"));
        return match list {
            Some(Value::String(text)) => {
                Some(split_list(text).into_iter().map(Value::String).collect())
            }
            _ => None,
        };
    }
    let scale = scale(kind)?;
    let mut list: Vec<Value> = (0..SLOTS)
        .map(|i| {
            vars.and_then(|v| v.get(&format!("{}{}", scale.prefix, scale.first + i)))
                .cloned()
                .unwrap_or(Value::Null)
        })
        .collect();
    if scale.repeat {
        list.retain(|value| {
            !matches!(value, Value::Null) && *value != Value::String(String::new())
        });
    }
    Some(list)
}

#[derive(Clone, Default)]
pub struct Palette {
    pub style: String,
    pub inks: [Option<&'static str>; SLOTS],
}

pub fn palette(kind: &str, config: &Object) -> Palette {
    let vars = record(config.get("themeVariables"));
    let Some(list) = entries(kind, vars) else {
        return Palette::default();
    };
    if list.is_empty() {
        return Palette::default();
    }
    let mut style = String::new();
    let mut inks = [None; SLOTS];
    for i in 0..SLOTS {
        let Some(color) = safe_color(&list[i % list.len()]) else {
            continue;
        };
        style.push_str(&format!(";--pele-series-{}:{color}", i + 1));
        inks[i] = ink(&format!(";fill:{color}"), "");
    }
    if style.is_empty() {
        return Palette::default();
    }
    Palette {
        style: esc(&style),
        inks,
    }
}

thread_local! {
    static CURRENT: RefCell<Palette> = RefCell::new(Palette::default());
}

pub fn with_palette<T>(next: Palette, run: impl FnOnce() -> T) -> T {
    struct Restore(Palette);
    impl Drop for Restore {
        fn drop(&mut self) {
            let outer = std::mem::take(&mut self.0);
            CURRENT.with(|current| *current.borrow_mut() = outer);
        }
    }
    let outer = CURRENT.with(|current| current.replace(next));
    let _restore = Restore(outer);
    run()
}

pub fn palette_style() -> String {
    CURRENT.with(|current| current.borrow().style.clone())
}

pub fn series_ink_fill(index: usize) -> String {
    CURRENT.with(|current| match current.borrow().inks[index % SLOTS] {
        Some(color) => format!(" fill=\"{color}\""),
        None => String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::json::parse as parse_json;

    fn config(text: &str) -> Object {
        parse_json(text).unwrap().as_object().cloned().unwrap()
    }

    #[test]
    fn builds_series_declarations_for_a_pie() {
        let made = palette(
            "pie",
            &config(
                r##"{"themeVariables":{"pie1":"#000","pie3":"rgb(1 2 3)","pie4":"red;x","pie5":5}}"##,
            ),
        );
        assert_eq!(
            made.style,
            ";--pele-series-1:#000;--pele-series-3:rgb(1 2 3)"
        );
        assert_eq!(made.inks[0], Some(crate::svg::ink::ON_DARK));
        assert_eq!(made.inks[1], None);
    }

    #[test]
    fn rejects_unsafe_colors() {
        for bad in [
            "url(x)",
            "var(--other)",
            "rgb(1 2 3",
            "calc(1)",
            "expression(1)",
        ] {
            assert_eq!(safe_color(&Value::String(bad.to_owned())), None, "{bad}");
        }
        assert!(safe_color(&Value::String("var(--pele-a)".to_owned())).is_some());
        assert!(safe_color(&Value::String("color-mix(in srgb, red, blue)".to_owned())).is_some());
    }

    #[test]
    fn repeats_venn_and_xychart_lists() {
        let venn = palette(
            "venn",
            &config(r#"{"themeVariables":{"venn1":"red","venn2":""}}"#),
        );
        assert_eq!(venn.style.matches("--pele-series-").count(), 8);
        let xy = palette(
            "xychart",
            &config(
                r##"{"themeVariables":{"xyChart":{"plotColorPalette":"#111, rgb(1, 2, 3)"}}}"##,
            ),
        );
        assert!(xy.style.contains("--pele-series-2:rgb(1, 2, 3)"));
    }

    #[test]
    fn restores_the_outer_palette() {
        let inner = palette("pie", &config(r#"{"themeVariables":{"pie1":"red"}}"#));
        with_palette(inner, || assert!(palette_style().contains("red")));
        assert_eq!(palette_style(), "");
    }
}
