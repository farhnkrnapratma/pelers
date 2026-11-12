use crate::svg::builder::{esc, esc_text};
use crate::svg::options::RenderOptions;
use crate::svg::palette::palette_style;
use crate::svg::theme::font_style;
use crate::util::jsnum::js_number_to_string;

const RADIUS: &str = "var(--pelers-radius,4px)";
const ROOT_STYLE: &str = "--_bg:var(--pelers-bg,#fff);--_fg:var(--pelers-fg,#1f1f1f);--_m:var(--pelers-muted,#6e6e6e);--_l:var(--pelers-line,#1f1f1f);--_s:var(--pelers-surface,#f3f3f3);--_a:var(--pelers-surface-alt,#e6e6e6);--_b:var(--pelers-border,#8a8a8a);--_c:var(--pelers-accent,#5b7bd5);color:var(--_fg)";

const WEIGHTS: &[(&str, &str)] = &[("tw", "title"), ("hw", "heading"), ("w", "bold")];

#[derive(Clone, Copy, Debug, Default)]
pub struct Accessible<'a> {
    pub acc_title: Option<&'a str>,
    pub acc_descr: Option<&'a str>,
}

pub fn svg_document(
    kind: &str,
    width: f64,
    height: f64,
    font_size: f64,
    options: &RenderOptions,
    acc: Accessible,
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
    if let Some(descr) = acc.acc_descr.filter(|t| !t.is_empty()) {
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
    let used = |alias: &str| content.contains(&format!("var(--_{alias})"));
    let mut weights = if used("r") {
        format!(";--_r:{RADIUS}")
    } else {
        String::new()
    };
    for (alias, name) in WEIGHTS {
        if used(alias) {
            weights.push_str(&format!(";--_{alias}:var(--pelers-{name}-weight,bold)"));
        }
    }
    let width = js_number_to_string(width);
    let height = js_number_to_string(height);
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" class=\"pelers pelers-{kind}\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" style=\"{fit}{ROOT_STYLE}{weights}{}{}\" fill=\"var(--_fg)\" role=\"graphics-document document\" aria-roledescription=\"{kind}\"{aria}>{head}{content}</svg>",
        esc(&font_style(
            font_size,
            options.font_family.as_deref(),
            options.font_family_mono.as_deref()
        )),
        palette_style()
    )
}
