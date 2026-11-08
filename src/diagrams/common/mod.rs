pub mod fit_text;
pub mod fit_width;
pub mod literal_label;
pub mod tokens;

use crate::util::chars::{is_js_space, trim_start};

pub struct Drawn {
    pub svg: String,
    pub width: f64,
    pub height: f64,
}

pub fn dedent_descr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for c in text.chars() {
        if skipping && is_js_space(c) {
            continue;
        }
        skipping = c == '\n';
        out.push(c);
    }
    out
}

pub fn acc_title(text: Option<String>) -> Option<String> {
    text.filter(|t| !t.is_empty())
        .map(|t| trim_start(&t).to_owned())
}

pub fn acc_descr(text: Option<String>) -> Option<String> {
    text.filter(|d| !d.is_empty()).map(|d| dedent_descr(&d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedents_description_lines() {
        assert_eq!(dedent_descr("a\n   b\n\t c"), "a\nb\nc");
        assert_eq!(dedent_descr("a  b"), "a  b");
    }

    #[test]
    fn drops_empty_accessible_text() {
        assert_eq!(acc_title(Some(String::new())), None);
        assert_eq!(acc_title(Some("  t".into())).as_deref(), Some("t"));
        assert_eq!(acc_descr(Some(String::new())), None);
        assert_eq!(acc_descr(Some("a\n b".into())).as_deref(), Some("a\nb"));
    }
}
