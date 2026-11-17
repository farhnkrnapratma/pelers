use crate::svg::builder::{esc_text, num};
use crate::text::icons::resolve_icon;

pub fn icon_svg(name: &str, x: f64, y: f64, side: f64) -> String {
    format!(
        "<svg class=\"pelers-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg>",
        esc_text(name),
        num(x),
        num(y),
        num(side),
        num(side),
        resolve_icon(name)
    )
}
