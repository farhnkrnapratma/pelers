use super::PieModel;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{fit_width, title_room};
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, num};
use crate::svg::palette::series_ink_fill;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::series_color;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, width};
use crate::util::jsnum::{js_max, js_number_to_string, js_round, to_fixed_0};
use crate::util::value::Value;

const RADIUS: f64 = 120.0;
const SWATCH: f64 = 12.0;
const ROW: f64 = 26.0;
const GAP: f64 = 32.0;
const OUTSIDE: f64 = 28.0;

fn option<'a>(config: &'a Config, key: &str) -> Option<&'a Value> {
    match config.get("pie") {
        Some(Value::Object(section)) => section.get(key),
        _ => None,
    }
}

fn point(angle: f64, radius: f64) -> String {
    format!(
        "{},{}",
        num(angle.sin() * radius),
        num(-angle.cos() * radius)
    )
}

fn ring(r: f64, sweep: u8) -> String {
    format!(
        "M0,{}A{},{} 0 1 {sweep} 0,{}A{},{} 0 1 {sweep} 0,{}Z",
        num(-r),
        num(r),
        num(r),
        num(r),
        num(r),
        num(r),
        num(-r)
    )
}

fn slice(start: f64, end: f64, outer: f64, inner: f64) -> String {
    if end - start >= std::f64::consts::PI * 2.0 - 1e-9 {
        return ring(outer, 1)
            + &if inner > 0.0 {
                ring(inner, 0)
            } else {
                String::new()
            };
    }
    let large = u8::from(end - start > std::f64::consts::PI);
    if inner <= 0.0 {
        return format!(
            "M0,0L{}A{},{} 0 {large} 1 {}Z",
            point(start, outer),
            num(outer),
            num(outer),
            point(end, outer)
        );
    }
    format!(
        "M{}A{},{} 0 {large} 1 {}L{}A{},{} 0 {large} 0 {}Z",
        point(start, outer),
        num(outer),
        num(outer),
        point(end, outer),
        point(end, inner),
        num(inner),
        num(inner),
        point(start, inner)
    )
}

pub fn render_pie(model: &PieModel, config: &Config, options: &RenderOptions) -> Drawn {
    fit_width(options, RADIUS * 2.0, 140.0, |diameter, tight| {
        draw(model, config, options, diameter / 2.0, tight)
    })
}

struct Outside {
    x: f64,
    y: f64,
    width: f64,
}

fn draw(
    model: &PieModel,
    config: &Config,
    options: &RenderOptions,
    radius: f64,
    tight: bool,
) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);

    let text_position = match option(config, "textPosition") {
        Some(Value::Number(n)) if *n >= 0.0 && *n <= 1.0 => *n,
        _ => 0.75,
    };
    let inner = match option(config, "donutHole") {
        Some(Value::Number(n)) if *n > 0.0 && *n <= 0.9 => n * radius,
        _ => 0.0,
    };
    let placed = match option(config, "legendPosition") {
        None | Some(Value::Null) => "right".to_owned(),
        Some(value) => value.to_js_string(),
    };
    let legend_position = if tight && (placed == "left" || placed == "right") {
        "bottom".to_owned()
    } else {
        placed
    };

    let entries = &model.sections;
    let mut sum = 0.0;
    for (_, value) in entries {
        sum += value;
    }

    let legend: Vec<Label> = entries
        .iter()
        .map(|(label, value)| {
            let text = if model.show_data {
                format!("{label} [{}]", js_number_to_string(*value))
            } else {
                label.clone()
            };
            layout_label(Some(&text), false, small, 4000.0, 0)
        })
        .collect();
    let mut legend_width = 0.0;
    for label in &legend {
        legend_width = js_max(legend_width, label.width);
    }
    legend_width += if legend.is_empty() { 0.0 } else { SWATCH + 8.0 };
    let legend_height = legend.len() as f64 * ROW;

    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        title_room(options),
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 16.0
    } else {
        0.0
    };

    let shares: Vec<f64> = entries
        .iter()
        .map(|(_, value)| {
            let share = if sum > 0.0 { value / sum } else { 0.0 };
            if share * 100.0 >= 1.0 && to_fixed_0(share * 100.0) != "0" {
                share
            } else {
                0.0
            }
        })
        .collect();
    let narrow: Vec<bool> = shares
        .iter()
        .map(|&share| {
            if share == 0.0 || share >= 0.5 {
                return false;
            }
            let chord = 2.0 * radius * text_position * (share * std::f64::consts::PI).sin();
            chord < width(&format!("{}%", to_fixed_0(share * 100.0)), small, 0) + 6.0
        })
        .collect();
    let margin = if narrow.contains(&true) { OUTSIDE } else { 0.0 };

    let side = match legend_position.as_str() {
        "left" | "right" | "center" => legend_position.as_str(),
        _ => "",
    };
    let stacked = legend_position == "top" || legend_position == "bottom";
    let diameter = (radius + margin) * 2.0;
    let mut total = diameter;
    let mut height = diameter;
    if side == "left" || side == "right" {
        total += if legend_width > 0.0 {
            GAP + legend_width
        } else {
            0.0
        };
        height = js_max(height, legend_height);
    } else if stacked {
        total = js_max(total, legend_width);
        height += if legend_height > 0.0 {
            20.0 + legend_height
        } else {
            0.0
        };
    }
    total = js_max(total, title.width);

    let cx = pad
        + match side {
            "left" => legend_width + GAP + diameter / 2.0,
            "right" => diameter / 2.0,
            _ => total / 2.0,
        };
    let body_top = pad + title_height;
    let cy = body_top
        + if legend_position == "top" {
            legend_height + 20.0 + diameter / 2.0
        } else if stacked {
            diameter / 2.0
        } else {
            height / 2.0
        };

    let mut slices = String::new();
    let mut labels = String::new();
    let mut outside = String::new();
    let mut angle = 0.0;
    let rim = radius + OUTSIDE / 2.0;
    let mut placed_outside: Vec<Outside> = Vec::new();
    for (index, (label, _)) in entries.iter().enumerate() {
        let share = shares[index];
        if share == 0.0 {
            continue;
        }
        let end = angle + share * std::f64::consts::PI * 2.0;
        slices.push_str(&format!(
            "<path class=\"pelers-slice\" data-id=\"{}\" d=\"{}\" fill=\"{}\"/>",
            esc_text(label),
            slice(angle, end, radius, inner),
            series_color(index)
        ));
        let mid = (angle + end) / 2.0;
        let text = format!("{}%", to_fixed_0(share * 100.0));
        if narrow[index] {
            let text_width = width(&text, small, 0);
            let clear = |x: f64, y: f64, w: f64, placed: &[Outside]| {
                placed.iter().all(|other| {
                    (x - other.x).abs() >= (w + other.width) / 2.0 + 2.0
                        || (y - other.y).abs() >= small + 2.0
                })
            };
            let mut at = mid;
            let mut step = 0;
            while step < 360 && !clear(at.sin() * rim, -at.cos() * rim, text_width, &placed_outside)
            {
                at += 2.0 / rim;
                step += 1;
            }
            let x = at.sin() * rim;
            let y = -at.cos() * rim;
            placed_outside.push(Outside {
                x,
                y,
                width: text_width,
            });
            outside.push_str(&format!(
                "<text x=\"{}\" y=\"{}\">{text}</text>",
                num(x),
                num(y + small * 0.35)
            ));
        } else {
            let r = radius * text_position;
            labels.push_str(&format!(
                "<text x=\"{}\" y=\"{}\"{}>{text}</text>",
                num(mid.sin() * r),
                num(-mid.cos() * r + small * 0.35),
                series_ink_fill(index)
            ));
        }
        angle = end;
    }
    if slices.is_empty() {
        slices = format!(
            "<circle class=\"pelers-slice\" r=\"{}\" fill=\"none\" stroke=\"var(--_b)\"/>",
            js_number_to_string(radius)
        );
    }

    let (lx, ly) = match side {
        "right" => (
            cx + diameter / 2.0 + GAP,
            body_top + (height - legend_height) / 2.0,
        ),
        "left" => (pad, body_top + (height - legend_height) / 2.0),
        "center" => (cx - legend_width / 2.0, cy - legend_height / 2.0),
        _ => (
            pad + (total - legend_width) / 2.0,
            if legend_position == "top" {
                body_top
            } else {
                body_top + diameter + 20.0
            },
        ),
    };
    let mut legend_out = String::new();
    for (index, label) in legend.iter().enumerate() {
        let y = ly + index as f64 * ROW + ROW / 2.0;
        legend_out.push_str(&format!(
            "<g class=\"pelers-legend-item\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{}\"/>{}</g>",
            esc_text(&entries[index].0),
            num(lx),
            num(y - SWATCH / 2.0),
            js_number_to_string(SWATCH),
            js_number_to_string(SWATCH),
            series_color(index),
            label_svg(label, lx + SWATCH + 8.0 + label.width / 2.0, y, "")
        ));
    }

    let total_width = (total + 2.0 * pad).ceil();
    let total_height = (height + title_height + 2.0 * pad).ceil();
    let small_text = js_number_to_string(small);
    let translate = format!("translate({},{})", num(cx), num(cy));
    let mut content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    content.push_str(&format!(
        "<g class=\"pelers-slices\" transform=\"{translate}\" stroke=\"var(--_bg)\" stroke-width=\"1\" stroke-linejoin=\"round\">{slices}</g>"
    ));
    if !labels.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-slice-labels\" transform=\"{translate}\" font-size=\"{small_text}\" text-anchor=\"middle\" fill=\"var(--_bg)\">{labels}</g>"
        ));
    }
    if !outside.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-slice-labels\" transform=\"{translate}\" font-size=\"{small_text}\" text-anchor=\"middle\">{outside}</g>"
        ));
    }
    if !legend_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-legend\" font-size=\"{small_text}\">{legend_out}</g>"
        ));
    }

    let acc = Accessible {
        acc_title: model.acc_title.as_deref(),
        acc_descr: model.acc_descr.as_deref(),
    };
    Drawn {
        links: Vec::new(),
        svg: svg_document(
            "pie",
            total_width,
            total_height,
            size,
            options,
            acc,
            &content,
        ),
        width: total_width,
        height: total_height,
    }
}
