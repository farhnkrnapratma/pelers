pub mod fit_width;
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
