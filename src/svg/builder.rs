use std::borrow::Cow;
use std::sync::OnceLock;

use crate::text::entities::decode_entities;
use crate::text::label::Label;
use crate::text::math::MathMarkup;
use crate::text::measurer::{BOLD_STYLE, ITALIC, MEASURE_MONO, MONO, SUB, SUP, UNDERLINE};
use crate::util::chars::trim;
use crate::util::jsnum::{js_number_to_string, js_round};

pub fn invalid_xml(c: char) -> bool {
    matches!(c, '\0'..='\u{8}' | '\u{b}' | '\u{c}' | '\u{e}'..='\u{1f}' | '\u{7f}' | '\u{fffe}' | '\u{ffff}')
}

pub fn has_invalid_xml(text: &str) -> bool {
    text.chars().any(invalid_xml)
}

fn needs_escape(text: &str) -> bool {
    text.bytes().any(|byte| match byte {
        b'&' | b'<' | b'>' | b'"' => true,
        0..=0x1f | 0x7f => invalid_xml(char::from(byte)),
        _ => false,
    }) || (!text.is_ascii() && text.chars().any(invalid_xml))
}

pub fn esc(text: &str) -> Cow<'_, str> {
    if !needs_escape(text) {
        return Cow::Borrowed(text);
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
    Cow::Owned(out)
}

pub fn esc_text(text: &str) -> String {
    esc(&decode_entities(text)).into_owned()
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

#[derive(Clone, Copy, Debug)]
pub struct Num {
    len: usize,
    bytes: [u8; NUM_CAP],
}

const NUM_CAP: usize = 32;

impl Num {
    fn of(text: &str) -> Self {
        let mut out = Self {
            len: 0,
            bytes: [0; NUM_CAP],
        };
        out.push_str(text);
        out
    }

    fn push(&mut self, byte: u8) {
        self.bytes[self.len] = byte;
        self.len += 1;
    }

    fn push_str(&mut self, text: &str) {
        for &byte in text.as_bytes() {
            self.push(byte);
        }
    }
}

impl std::ops::Deref for Num {
    type Target = str;

    fn deref(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("ascii number")
    }
}

impl std::fmt::Display for Num {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}

impl PartialEq<&str> for Num {
    fn eq(&self, other: &&str) -> bool {
        &**self == *other
    }
}

impl From<Num> for String {
    fn from(value: Num) -> Self {
        value.to_string()
    }
}

fn push_integer(out: &mut Num, mut value: u64) {
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    loop {
        at -= 1;
        digits[at] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    for &digit in &digits[at..] {
        out.push(digit);
    }
}

pub fn num(value: f64) -> Num {
    let n = js_round(value * 100.0);
    if n == 0.0 {
        return Num::of("0");
    }
    if (n > 0.0 && n < 1e11) || (n < 0.0 && n > -1e11) {
        let m = n.abs();
        let q = (m / 100.0) as u64;
        let mut out = Num::of("");
        if n < 0.0 {
            out.push(b'-');
        }
        push_integer(&mut out, q);
        out.push_str(fraction((m - q as f64 * 100.0) as usize));
        return out;
    }
    Num::of(&js_number_to_string(n / 100.0))
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
        if span.icon.is_some() || span.text.is_empty() {
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
    label_svg_with(label, cx, cy, attrs, false)
}

pub fn label_svg_with(label: &Label, cx: f64, cy: f64, attrs: &str, icons: bool) -> String {
    let lines = &label.lines;
    if lines.is_empty() {
        return String::new();
    }
    let lh = label.line_height;
    let shift = label.size * 0.35;
    let mut top = cy - label.height / 2.0;

    if let [line] = lines.as_slice()
        && let [only] = line.as_slice()
        && only.style & !MEASURE_MONO == 0
        && only.icon.is_none()
        && only.math.is_none()
    {
        let text = esc(&only.text);
        let mut out = String::with_capacity(text.len() + attrs.len() + 64);
        out.push_str("<text");
        out.push_str(attrs);
        out.push_str(" x=\"");
        out.push_str(&num(cx));
        out.push_str("\" y=\"");
        out.push_str(&num(top + lh / 2.0 + shift));
        out.push_str("\" text-anchor=\"middle\">");
        out.push_str(&text);
        out.push_str("</text>");
        return out;
    }

    let shared = shared_style(label);
    let given = attrs;
    let mut math: Option<MathLook> = None;
    let attrs = text_style(attrs, shared);
    let mut body = String::new();
    let mut extra = String::new();
    for (i, line) in lines.iter().enumerate() {
        let h = label.heights.as_ref().map_or(lh, |heights| heights[i]);
        let mid = top + h / 2.0;
        let y = mid + shift;
        let placed = line
            .iter()
            .any(|span| span.icon.is_some() || span.math.is_some());
        if placed {
            let mut x = cx - label.widths[i] / 2.0;
            for span in line {
                if let Some(markup) = &span.math {
                    let box_ = format!(
                        "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
                        num(x),
                        num(mid - markup.height() / 2.0),
                        num(markup.width()),
                        num(markup.height())
                    );
                    let look = math.get_or_insert_with(|| math_look(given));
                    extra.push_str(&match &look.halo {
                        None => math_svg(markup, &box_, label.size, &look.attrs),
                        Some(halo) => format!("<rect {box_} rx=\"2\" fill=\"{halo}\"/>"),
                    });
                } else if let Some(icon) = &span.icon {
                    let side = label.size * 1.1;
                    let inner = if icons {
                        crate::text::icons::resolve_icon(icon)
                    } else {
                        String::new()
                    };
                    extra.push_str(&format!(
                        "<svg class=\"pelers-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{inner}</svg>",
                        esc_text(icon),
                        num(x),
                        num(mid - side / 2.0),
                        num(side),
                        num(side)
                    ));
                } else if !span.text.is_empty() {
                    body.push_str(&format!(
                        "<tspan x=\"{}\" y=\"{}\" text-anchor=\"start\"{}>{}</tspan>",
                        num(x),
                        num(y),
                        span_style(span.style & !shared),
                        esc(&span.text)
                    ));
                }
                x += span.width;
            }
        } else {
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
        }
        top += h;
    }
    format!("<text{attrs} text-anchor=\"middle\" xml:space=\"preserve\">{body}</text>{extra}")
}

struct MathLook {
    attrs: String,
    halo: Option<String>,
}

fn math_look(attrs: &str) -> MathLook {
    let style = capture_attr(attrs, " style=\"").unwrap_or_default();
    let fill = fill_from_style(&style)
        .or_else(|| capture_attr(attrs, " fill=\""))
        .unwrap_or_default();
    let fill = trim(&fill).to_owned();
    if fill == "none" {
        return MathLook {
            attrs: String::new(),
            halo: Some(capture_attr(attrs, " stroke=\"").unwrap_or_else(|| "none".to_owned())),
        };
    }
    let opacity = opacity_from_style(&style).map(|o| trim(&o).to_owned());
    let mut out = if fill.is_empty() {
        String::new()
    } else {
        format!(" color=\"{fill}\"")
    };
    if let Some(opacity) = opacity.filter(|o| !o.is_empty()) {
        out.push_str(&format!(" opacity=\"{opacity}\""));
    }
    MathLook {
        attrs: out,
        halo: None,
    }
}

fn capture_attr(attrs: &str, prefix: &str) -> Option<String> {
    let start = attrs.find(prefix)? + prefix.len();
    let end = attrs[start..].find('"')?;
    Some(attrs[start..start + end].to_owned())
}

fn declaration(style: &str, name: &str) -> Option<String> {
    let mut rest = style;
    loop {
        let trimmed =
            rest.trim_start_matches(|c: char| c == ';' || crate::util::chars::is_js_space(c));
        if let Some(value) = trimmed.strip_prefix(name)
            && let Some(value) = value.strip_prefix(':')
        {
            let end = value.find(';').unwrap_or(value.len());
            return Some(value[..end].to_owned());
        }
        rest = &rest[rest.find(';')? + 1..];
    }
}

fn fill_from_style(style: &str) -> Option<String> {
    declaration(style, "fill")
}

fn opacity_from_style(style: &str) -> Option<String> {
    declaration(style, "opacity")
}

fn math_svg(math: &MathMarkup, box_: &str, size: f64, attrs: &str) -> String {
    match math {
        MathMarkup::Svg { svg, .. } => {
            format!("<svg class=\"pelers-math\" {box_} overflow=\"visible\"{attrs}>{svg}</svg>")
        }
        MathMarkup::Html { html, .. } => format!(
            "<foreignObject class=\"pelers-math\" {box_} overflow=\"visible\"{attrs}><div xmlns=\"http://www.w3.org/1999/xhtml\" style=\"display:flex;align-items:center;justify-content:center;width:100%;height:100%;font-size:{}px;line-height:normal;white-space:nowrap\">{html}</div></foreignObject>",
            num(size)
        ),
    }
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
        let label = layout_label(Some("Pelers"), false, 16.0, 4000.0, 0);
        assert_eq!(
            label_svg(&label, 75.0, 20.0, " class=\"x\""),
            "<text class=\"x\" x=\"75\" y=\"25.6\" text-anchor=\"middle\">Pelers</text>"
        );
    }

    #[test]
    fn detects_characters_xml_cannot_hold() {
        assert!(has_invalid_xml("a\u{1}b"));
        assert!(!has_invalid_xml("plain\ttext\n"));
    }
}
