pub const ROOT_STYLE: &str = "--_bg:var(--pele-bg,#fff);--_fg:var(--pele-fg,#1f1f1f);--_m:var(--pele-muted,#6e6e6e);\
--_l:var(--pele-line,#1f1f1f);--_s:var(--pele-surface,#f3f3f3);--_a:var(--pele-surface-alt,#e6e6e6);\
--_b:var(--pele-border,#8a8a8a);--_c:var(--pele-accent,#5b7bd5);color:var(--_fg)";

pub const RADIUS: &str = "var(--pele-radius,4px)";

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

pub fn font_style(size: f64, family: Option<&str>, mono: Option<&str>) -> String {
    format!(
        ";--_fm:var(--pele-font-mono,{});font:{}px sans-serif;letter-spacing:normal;word-spacing:normal;text-transform:none;direction:ltr;font-family:var(--pele-font,{})",
        families(mono, "monospace"),
        crate::util::jsnum::js_number_to_string(size),
        families(family, "sans-serif")
    )
}

const SERIES: [&str; 8] = [
    "#4c78a8", "#f58518", "#54a24b", "#e45756", "#72b7b2", "#eeca3b", "#b279a2", "#9d755d",
];

pub fn series_color(index: usize) -> String {
    let k = index % 8;
    format!("var(--pele-series-{},{})", k + 1, SERIES[k])
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
        assert!(css.ends_with("font-family:var(--pele-font,sans-serif)"));
        assert!(css.starts_with(";--_fm:var(--pele-font-mono,monospace);font:16px sans-serif;"));
    }
}
