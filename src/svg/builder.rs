use std::sync::OnceLock;

use crate::text::entities::decode_entities;
use crate::text::label::Label;
use crate::text::measurer::{BOLD_STYLE, ITALIC, MONO, SUB, SUP, UNDERLINE};
use crate::util::jsnum::{js_number_to_string, js_round};

fn invalid_xml(c: char) -> bool {
    matches!(c, '\0'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}' | '\u{fffe}' | '\u{ffff}')
}

pub fn has_invalid_xml(text: &str) -> bool {
    text.chars().any(invalid_xml)
}

pub fn esc(text: &str) -> String {
    if !text.contains(|c| matches!(c, '&' | '<' | '>' | '"') || invalid_xml(c)) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c if invalid_xml(c) => {}
            c => out.push(c),
        }
    }
    out
}

pub fn esc_text(text: &str) -> String {
    esc(&decode_entities(text))
}

fn fraction(i: usize) -> &'static str {
    static TABLE: OnceLock<Vec<String>> = OnceLock::new();
    &TABLE.get_or_init(|| {
        (0..100)
            .map(|i| match i {
                0 => String::new(),
                i if i % 10 == 0 => format!(".{}", i / 10),
                i if i < 10 => format!(".0{i}"),
                i => format!(".{i}"),
            })
            .collect()
    })[i]
}

pub fn num(value: f64) -> String {
    let n = js_round(value * 100.0);
    if n == 0.0 {
        return "0".to_owned();
    }
    if n > 0.0 {
        if n < 1e11 {
            let q = (n / 100.0).floor();
            return format!("{}{}", q as u64, fraction((n - q * 100.0) as usize));
        }
    } else if n > -1e11 {
        let m = -n;
        let q = (m / 100.0).floor();
        return format!("-{}{}", q as u64, fraction((m - q * 100.0) as usize));
    }
    js_number_to_string(n / 100.0)
}

const BOLD: &str = " font-weight=\"var(--_w)\"";
const ITALIC_ATTR: &str = " font-style=\"italic\"";
const UNDERLINE_ATTR: &str = " text-decoration=\"underline\"";
const MONO_ATTR: &str = " font-family=\"var(--_fm)\"";
const SUB_ATTR: &str = " baseline-shift=\"sub\" font-size=\"smaller\"";
const SUP_ATTR: &str = " baseline-shift=\"super\" font-size=\"smaller\"";

pub fn span_style(style: u32) -> String {
    let mut out = String::new();
    if style & BOLD_STYLE != 0 {
        out.push_str(BOLD);
    }
    if style & ITALIC != 0 {
        out.push_str(ITALIC_ATTR);
    }
    if style & MONO != 0 {
        out.push_str(MONO_ATTR);
    }
    if style & UNDERLINE != 0 {
        out.push_str(UNDERLINE_ATTR);
    }
    if style & SUB != 0 {
        out.push_str(SUB_ATTR);
    } else if style & SUP != 0 {
        out.push_str(SUP_ATTR);
    }
    out
}

pub fn shared_style(label: &Label) -> u32 {
    let mut shared = BOLD_STYLE | ITALIC | MONO;
    let mut any = false;
    for span in label.lines.iter().flatten() {
        if span.text.is_empty() {
            continue;
        }
        shared &= span.style;
        any = true;
    }
    if any { shared } else { 0 }
}

pub fn text_style(attrs: &str, shared: u32) -> String {
    let mut out = attrs.to_owned();
    if shared & BOLD_STYLE != 0 && !attrs.contains(" font-weight=\"") {
        out.push_str(BOLD);
    }
    if shared & ITALIC != 0 && !attrs.contains(" font-style=\"") {
        out.push_str(ITALIC_ATTR);
    }
    if shared & MONO != 0 && !attrs.contains(" font-family=\"") {
        out.push_str(MONO_ATTR);
    }
    out
}

pub fn label_svg(label: &Label, cx: f64, cy: f64, attrs: &str) -> String {
    let lines = &label.lines;
    if lines.is_empty() {
        return String::new();
    }
    let lh = label.line_height;
    let shift = label.size * 0.35;
    let mut top = cy - label.height / 2.0;

    if let [line] = lines.as_slice()
        && let [only] = line.as_slice()
        && only.style == 0
    {
        return format!(
            "<text{attrs} x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
            num(cx),
            num(top + lh / 2.0 + shift),
            esc(&only.text)
        );
    }

    let shared = shared_style(label);
    let attrs = text_style(attrs, shared);
    let mut body = String::new();
    for line in lines {
        let mid = top + lh / 2.0;
        let y = mid + shift;
        for (j, span) in line.iter().enumerate() {
            let pos = if j == 0 {
                format!(" x=\"{}\" y=\"{}\"", num(cx), num(y))
            } else {
                String::new()
            };
            body.push_str(&format!(
                "<tspan{pos}{}>{}</tspan>",
                span_style(span.style & !shared),
                esc(&span.text)
            ));
        }
        if line.is_empty() {
            body.push_str(&format!(
                "<tspan x=\"{}\" y=\"{}\"> </tspan>",
                num(cx),
                num(y)
            ));
        }
        top += lh;
    }
    format!("<text{attrs} text-anchor=\"middle\" xml:space=\"preserve\">{body}</text>")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::label::layout_label;

    #[test]
    fn formats_numbers_like_the_oracle() {
        assert_eq!(num(0.0), "0");
        assert_eq!(num(-0.004), "0");
        assert_eq!(num(25.6), "25.6");
        assert_eq!(num(75.0), "75");
        assert_eq!(num(1.005), "1");
        assert_eq!(num(1.006), "1.01");
        assert_eq!(num(-2.5), "-2.5");
        assert_eq!(num(-0.125), "-0.12");
        assert_eq!(num(1e12), "1000000000000");
        assert_eq!(num(f64::NAN), "NaN");
    }

    #[test]
    fn escapes_markup_and_drops_invalid_characters() {
        assert_eq!(esc("a<b>&\"c\u{1}\u{7f}d"), "a&lt;b&gt;&amp;&quot;cd");
        assert_eq!(esc_text("&amp;#35;"), "&amp;#35;");
    }

    #[test]
    fn writes_a_plain_label_as_one_text_element() {
        let label = layout_label(Some("Pele"), false, 16.0, 4000.0, 0);
        assert_eq!(
            label_svg(&label, 75.0, 20.0, " class=\"x\""),
            "<text class=\"x\" x=\"75\" y=\"25.6\" text-anchor=\"middle\">Pele</text>"
        );
    }

    #[test]
    fn detects_characters_xml_cannot_hold() {
        assert!(has_invalid_xml("a\u{1}b"));
        assert!(!has_invalid_xml("plain\ttext\n"));
    }
}
