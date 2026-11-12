use super::builder::{esc, esc_text};
use super::palette::{palette_style_len, push_palette_style};
use super::theme::{RADIUS, ROOT_STYLE, font_style};
use crate::util::jsnum::js_number_to_string;

const WEIGHTS: [(&str, &str); 3] = [("tw", "title"), ("hw", "heading"), ("w", "bold")];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderOptions {
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
    pub font_family_mono: Option<String>,
    pub padding: Option<f64>,
    pub max_width: Option<f64>,
    pub auto_direction: Option<bool>,
    pub direction_breakpoint: Option<f64>,
    pub responsive: Option<bool>,
    pub id_prefix: Option<String>,
    pub monospace: Option<Vec<String>>,
    pub now: Option<f64>,
    pub output_limit: Option<usize>,
    pub link_schemes: Option<Vec<String>>,
    pub image_schemes: Option<Vec<String>>,
    pub links: Option<bool>,
    pub images: Option<bool>,
    pub link_rel: Option<String>,
    pub icons: Option<crate::util::hook::Hook<crate::text::icons::IconFn>>,
    pub math: Option<crate::util::hook::Hook<crate::text::math::MathFn>>,
    pub measurer: Option<crate::util::hook::Hook<crate::text::measurer::MeasureFn>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Accessible<'a> {
    pub acc_title: Option<&'a str>,
    pub acc_descr: Option<&'a str>,
}

fn used_aliases(content: &str) -> [bool; 4] {
    let mut used = [false; 4];
    let mut rest = content;
    while let Some(at) = rest.find("var(--_") {
        rest = &rest[at + 7..];
        for (k, alias) in ["r", "tw", "hw", "w"].iter().enumerate() {
            if rest
                .strip_prefix(alias)
                .is_some_and(|tail| tail.starts_with(')'))
            {
                used[k] = true;
                break;
            }
        }
        if used.iter().all(|&u| u) {
            break;
        }
    }
    used
}

pub fn svg_document(
    kind: &str,
    width: f64,
    height: f64,
    font_size: f64,
    options: &RenderOptions,
    acc: Accessible<'_>,
    content: &str,
) -> String {
    let prefix = esc(options.id_prefix.as_deref().unwrap_or("pelers"));
    let mut head = String::new();
    let mut aria = String::new();
    if let Some(title) = acc.acc_title.filter(|t| !t.is_empty()) {
        head.push_str(&format!(
            "<title id=\"{prefix}-title\">{}</title>",
            esc_text(title)
        ));
        aria.push_str(&format!(" aria-labelledby=\"{prefix}-title\""));
    }
    if let Some(descr) = acc.acc_descr.filter(|d| !d.is_empty()) {
        head.push_str(&format!(
            "<desc id=\"{prefix}-desc\">{}</desc>",
            esc_text(descr)
        ));
        aria.push_str(&format!(" aria-describedby=\"{prefix}-desc\""));
    }
    let fit = if options.responsive == Some(false) {
        ""
    } else {
        "max-width:100%;height:auto;"
    };
    let used = used_aliases(content);
    let mut weights = if used[0] {
        format!(";--_r:{RADIUS}")
    } else {
        String::new()
    };
    for (k, (alias, name)) in WEIGHTS.iter().enumerate() {
        if used[k + 1] {
            weights.push_str(&format!(";--_{alias}:var(--pelers-{name}-weight,bold)"));
        }
    }
    let fonts = font_style(
        font_size,
        options.font_family.as_deref(),
        options.font_family_mono.as_deref(),
    );
    let (w, h) = (js_number_to_string(width), js_number_to_string(height));
    let palette_len = palette_style_len();
    let mut out = String::with_capacity(
        head.len()
            + aria.len()
            + content.len()
            + fonts.len()
            + palette_len
            + ROOT_STYLE.len()
            + weights.len()
            + 320,
    );
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"pelers pelers-");
    out.push_str(kind);
    out.push_str("\" viewBox=\"0 0 ");
    out.push_str(&w);
    out.push(' ');
    out.push_str(&h);
    out.push_str("\" width=\"");
    out.push_str(&w);
    out.push_str("\" height=\"");
    out.push_str(&h);
    out.push_str("\" style=\"");
    out.push_str(fit);
    out.push_str(ROOT_STYLE);
    out.push_str(&weights);
    out.push_str(&esc(&fonts));
    push_palette_style(&mut out);
    out.push_str(
        "\" fill=\"var(--_fg)\" role=\"graphics-document document\" aria-roledescription=\"",
    );
    out.push_str(kind);
    out.push('"');
    out.push_str(&aria);
    out.push('>');
    out.push_str(&head);
    out.push_str(content);
    out.push_str("</svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_aliases_a_diagram_uses() {
        assert_eq!(
            used_aliases("<g style=\"rx:var(--_r)\" font-weight=\"var(--_tw)\"/>"),
            [true, true, false, false]
        );
        assert_eq!(
            used_aliases("var(--_wx) var(--_w)"),
            [false, false, false, true]
        );
        assert_eq!(used_aliases("none"), [false; 4]);
    }

    #[test]
    fn omits_the_fit_style_when_not_responsive() {
        let options = RenderOptions {
            responsive: Some(false),
            ..RenderOptions::default()
        };
        let svg = svg_document("info", 10.0, 5.0, 16.0, &options, Accessible::default(), "");
        assert!(svg.contains("style=\"--_bg:"));
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"pelers pelers-info\" viewBox=\"0 0 10 5\""));
    }

    #[test]
    fn stops_scanning_once_every_alias_is_used() {
        let content = "var(--_r) var(--_tw) var(--_hw) var(--_w) var(--_r)";
        assert_eq!(used_aliases(content), [true; 4]);
        assert_eq!(used_aliases("var(--_x) var(--_rr)"), [false; 4]);
    }
}
