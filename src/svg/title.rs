use crate::svg::builder::{label_svg, num};
use crate::text::label::{Label, layout_label};
use crate::text::math::without_math;
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::js_max;

pub fn title_label(title: Option<&str>, size: f64, max_width: f64) -> Label {
    without_math(|| layout_label(title, false, size, max_width, BOLD_STYLE))
}

pub struct Titled {
    pub width: f64,
    pub height: f64,
    pub content: String,
}

pub fn with_title(
    title: Option<&str>,
    size: f64,
    pad: f64,
    width: f64,
    height: f64,
    content: &str,
) -> Titled {
    let label = title_label(title, size, 4000.0);
    if label.height == 0.0 {
        return Titled {
            width,
            height,
            content: content.to_owned(),
        };
    }
    let shift = label.height + 12.0;
    let full = js_max(width, (label.width + 2.0 * pad).ceil());
    Titled {
        width: full,
        height: (height + shift).ceil(),
        content: label_svg(
            &label,
            full / 2.0,
            pad + label.height / 2.0,
            " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
        ) + &format!(
            "<g transform=\"translate({},{})\">{content}</g>",
            num((full - width) / 2.0),
            num(shift)
        ),
    }
}
