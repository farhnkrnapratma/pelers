use std::collections::HashMap;

use super::db::{PointStyle, QuadrantDb, QuadrantText};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::cap_width;
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ROUNDED, class_names, resolve_style, series_color};
use crate::text::label::{Label, layout_label};
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::{js_max, js_min, js_number, js_number_to_string, js_round};
use crate::util::value::Value;

const GAP: f64 = 4.0;
const CAREFUL: usize = 300;

struct Dot {
    cx: f64,
    cy: f64,
    reach: f64,
    text: Label,
}

fn setting<'a>(config: &'a Config, key: &str) -> Option<&'a Value> {
    config
        .get("quadrantChart")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
}

fn length(config: &Config, key: &str, fallback: f64, max: f64) -> f64 {
    match setting(config, key) {
        Some(Value::Number(value)) if *value >= 0.0 => js_min(*value, max),
        _ => fallback,
    }
}

fn text_size(config: &Config, key: &str, fallback: f64) -> f64 {
    match setting(config, key) {
        Some(Value::Number(value)) if value.is_finite() => js_min(72.0, js_max(4.0, *value)),
        _ => fallback,
    }
}

fn hex(color: &str) -> String {
    if color.starts_with('#') {
        color.to_owned()
    } else {
        format!("#{color}")
    }
}

fn size_attr(own: f64, inherited: f64) -> String {
    if own == inherited {
        String::new()
    } else {
        format!(" font-size=\"{}\"", js_number_to_string(own))
    }
}

fn label(text: Option<&QuadrantText>, font: f64, max: f64) -> Label {
    layout_label(
        text.map(|t| t.text.as_str()),
        text.is_some_and(|t| t.markdown),
        font,
        js_max(max, 40.0),
        0,
    )
}

pub fn render_quadrant(model: &QuadrantDb, config: &Config, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let title_size = text_size(config, "titleFontSize", size);
    let quadrant_size = text_size(config, "quadrantLabelFontSize", size);
    let x_axis_size = text_size(config, "xAxisLabelFontSize", js_round(size * 0.875));
    let y_axis_size = text_size(config, "yAxisLabelFontSize", js_round(size * 0.875));
    let point_size = text_size(config, "pointLabelFontSize", js_round(size * 0.75));
    let natural = js_round(js_max(100.0, length(config, "chartWidth", 500.0, 4000.0)));
    let width = js_round(cap_width(options, natural, 280.0, 0.0));
    let height =
        js_round(js_max(100.0, length(config, "chartHeight", 500.0, 4000.0)) * width / natural);
    let outer = length(
        config,
        "quadrantPadding",
        options.padding.unwrap_or(8.0),
        200.0,
    );
    let x_pad = length(config, "xAxisLabelPadding", 5.0, 200.0);
    let y_pad = length(config, "yAxisLabelPadding", 5.0, 200.0);
    let has_points = !model.points.is_empty();

    let title = layout_label(
        model.title.as_deref(),
        false,
        title_size,
        js_max(width - 2.0 * outer, 40.0),
        BOLD_STYLE,
    );
    let title_pad = length(config, "titlePadding", 10.0, 200.0);
    let title_space = if title.height > 0.0 {
        title.height + title_pad
    } else {
        0.0
    };

    let has_x = model
        .x_axis_left
        .as_ref()
        .is_some_and(|t| !t.text.is_empty())
        || model
            .x_axis_right
            .as_ref()
            .is_some_and(|t| !t.text.is_empty());
    let has_y = model
        .y_axis_bottom
        .as_ref()
        .is_some_and(|t| !t.text.is_empty())
        || model
            .y_axis_top
            .as_ref()
            .is_some_and(|t| !t.text.is_empty());
    let x_below = has_points
        || matches!(setting(config, "xAxisPosition"), Some(Value::String(p)) if p == "bottom");
    let y_right =
        matches!(setting(config, "yAxisPosition"), Some(Value::String(p)) if p == "right");

    let side = width - 2.0 * outer;
    let x_left = label(
        model.x_axis_left.as_ref(),
        x_axis_size,
        if model.x_axis_right.is_some() {
            side / 2.0 - 16.0
        } else {
            side
        },
    );
    let x_right = label(model.x_axis_right.as_ref(), x_axis_size, side / 2.0 - 16.0);
    let x_height = js_max(x_left.height, x_right.height);
    let x_space = if has_x { x_height + x_pad } else { 0.0 };
    let tall = height - 2.0 * outer - title_space - x_space;
    let y_bottom = label(
        model.y_axis_bottom.as_ref(),
        y_axis_size,
        if model.y_axis_top.is_some() {
            tall / 2.0 - 16.0
        } else {
            tall
        },
    );
    let y_top = label(model.y_axis_top.as_ref(), y_axis_size, tall / 2.0 - 16.0);
    let y_height = js_max(y_bottom.height, y_top.height);
    let y_space = if has_y { y_height + y_pad } else { 0.0 };

    let left = outer + if y_right { 0.0 } else { y_space };
    let top = outer + title_space + if x_below { 0.0 } else { x_space };
    let w = js_max(0.0, side - y_space);
    let h = js_max(0.0, tall);
    let tile_w = js_max(0.0, (w - GAP) / 2.0);
    let tile_h = js_max(0.0, (h - GAP) / 2.0);

    let mut taken: Vec<f64> = vec![
        left + tile_w,
        top,
        left + w - tile_w,
        top + h,
        left,
        top + tile_h,
        left + w,
        top + h - tile_h,
    ];
    let is_clear = |taken: &[f64], from: usize, x0: f64, y0: f64, x1: f64, y1: f64| -> bool {
        let mut k = from;
        while k < taken.len() {
            let empty = taken[k] >= taken[k + 2] || taken[k + 1] >= taken[k + 3];
            if !empty
                && x0 < taken[k + 2]
                && x1 > taken[k]
                && y0 < taken[k + 3]
                && y1 > taken[k + 1]
            {
                return false;
            }
            k += 4;
        }
        true
    };

    let text_top = length(config, "quadrantTextTopPadding", 5.0, 200.0);
    let mut quadrants = String::new();
    for (k, quadrant) in model.quadrants.iter().enumerate().take(4) {
        let x = if k == 0 || k == 3 {
            left + w - tile_w
        } else {
            left
        };
        let y = if k < 2 { top } else { top + h - tile_h };
        let text = label(quadrant.as_ref(), quadrant_size, tile_w - 16.0);
        let cx = x + tile_w / 2.0;
        let cy = if has_points {
            y + text_top + text.height / 2.0
        } else {
            y + tile_h / 2.0
        };
        taken.push(cx - text.width / 2.0);
        taken.push(cy - text.height / 2.0);
        taken.push(cx + text.width / 2.0);
        taken.push(cy + text.height / 2.0);
        quadrants.push_str(&format!(
            "<g class=\"pelers-quadrant\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{ROUNDED}/>{}</g>",
            k + 1,
            num(x),
            num(y),
            num(tile_w),
            num(tile_h),
            label_svg(
                &text,
                cx,
                cy,
                &format!(
                    " class=\"pelers-label\"{} fill=\"var(--_fg)\" stroke=\"none\"",
                    size_attr(quadrant_size, size)
                )
            )
        ));
    }

    let mut axes = String::new();
    if has_x {
        let y = if x_below {
            top + h + x_pad + x_height / 2.0
        } else {
            top - x_pad - x_height / 2.0
        };
        let left_x = if x_right.height > 0.0 {
            left + w / 4.0
        } else {
            left + x_left.width / 2.0
        };
        axes.push_str(&label_svg(&x_left, left_x, y, ""));
        axes.push_str(&label_svg(&x_right, left + w * 3.0 / 4.0, y, ""));
    }
    if has_y {
        let x = if y_right {
            left + w + y_pad + y_height / 2.0
        } else {
            left - y_pad - y_height / 2.0
        };
        let turned = |text: &Label, y: f64| -> String {
            if text.height > 0.0 {
                format!(
                    "<g transform=\"translate({},{}) rotate(-90)\"{}>{}</g>",
                    num(x),
                    num(y),
                    size_attr(y_axis_size, x_axis_size),
                    label_svg(text, 0.0, 0.0, "")
                )
            } else {
                String::new()
            }
        };
        let bottom_y = if y_top.height > 0.0 {
            top + h * 3.0 / 4.0
        } else {
            top + h - y_bottom.width / 2.0
        };
        axes.push_str(&turned(&y_bottom, bottom_y));
        axes.push_str(&turned(&y_top, top + h / 4.0));
    }

    let radius = length(config, "pointRadius", 5.0, 200.0);
    let text_pad = length(config, "pointTextPadding", 5.0, 200.0);
    let fill = series_color(0);
    let mut dots: Vec<Dot> = Vec::new();
    let mut circles: Vec<String> = Vec::new();
    let classes: &HashMap<String, PointStyle> = &model.classes;
    for point in &model.points {
        if !(point.x >= 0.0 && point.x <= 1.0 && point.y >= 0.0 && point.y <= 1.0) {
            continue;
        }
        let shared = classes.get(&point.class_name);
        let style = &point.style;
        let r = js_min(
            style
                .radius
                .or_else(|| shared.and_then(|s| s.radius))
                .unwrap_or(radius),
            2000.0,
        );
        let color = style
            .color
            .clone()
            .or_else(|| shared.and_then(|s| s.color.clone()));
        let stroke_color = style
            .stroke_color
            .clone()
            .or_else(|| shared.and_then(|s| s.stroke_color.clone()));
        let stroke_width = style
            .stroke_width
            .clone()
            .or_else(|| shared.and_then(|s| s.stroke_width.clone()));
        let mut styles: Vec<String> = Vec::new();
        if let Some(color) = &color {
            styles.push(format!("fill:{}", hex(color)));
        }
        if let Some(width) = &stroke_width {
            if let Some(stroke) = &stroke_color {
                styles.push(format!("stroke:{}", hex(stroke)));
            }
            styles.push(format!("stroke-width:{width}"));
        }
        let cx = left + point.x * w;
        let cy = top + (1.0 - point.y) * h;
        let reach = r + match &stroke_width {
            Some(width) => js_min(leading_int(width), 400.0) / 2.0,
            None => 0.0,
        };
        dots.push(Dot {
            cx,
            cy,
            reach,
            text: label(Some(&point.text), point_size, w),
        });
        circles.push(format!(
            "<circle class=\"pelers-point{}\" data-id=\"{}\" cx=\"{}\" cy=\"{}\" r=\"{}\"{}{}/>",
            class_names(&point.class_name),
            esc_text(&point.text.text),
            num(cx),
            num(cy),
            num(r),
            match (&stroke_width, &stroke_color) {
                (Some(_), None) => format!(" stroke=\"{fill}\""),
                _ => String::new(),
            },
            resolve_style(&styles).shape
        ));
    }

    let careful = dots.len() <= CAREFUL;
    if careful {
        for dot in &dots {
            taken.push(dot.cx - dot.reach);
            taken.push(dot.cy - dot.reach);
            taken.push(dot.cx + dot.reach);
            taken.push(dot.cy + dot.reach);
        }
    }
    let mut labels = String::new();
    for dot in &dots {
        let (cx, cy, reach, text) = (dot.cx, dot.cy, dot.reach, &dot.text);
        let half_w = text.width / 2.0;
        let half_h = text.size * 0.6 + (text.height - text.line_height) / 2.0;
        let away = reach + text_pad;
        let inset = js_min(away, js_max(0.0, (tile_w - text.width) / 2.0));
        let heading = 8 + 4 * if cx < left + w / 2.0 {
            if cy < top + h / 2.0 { 1 } else { 2 }
        } else if cy < top + h / 2.0 {
            0
        } else {
            3
        };
        let below = js_max(cy + away, taken[heading + 3] + text_pad) + half_h;
        let spots = [
            cx,
            cy + away + half_h,
            cx,
            cy - away - half_h,
            cx + away + half_w,
            cy,
            cx - away - half_w,
            cy,
            cx,
            below,
        ];
        let mut tx = cx;
        let mut ty = spots[1];
        let mut found = false;
        let mut k = 0;
        while k < 20 {
            let spot = k % 10;
            let margin = if spot == 8 { inset } else { 0.0 };
            let x = if (spot < 4 || spot == 8) && text.width < w {
                js_min(
                    js_max(spots[spot], left + half_w + margin),
                    left + w - half_w - margin,
                )
            } else {
                spots[spot]
            };
            let y = spots[spot + 1];
            if x - half_w < left
                || x + half_w > left + w
                || y - half_h < top
                || y + half_h > top + h
            {
                k += 2;
                continue;
            }
            let free = !careful
                || is_clear(
                    &taken,
                    if k < 10 { 0 } else { 8 },
                    x - half_w,
                    y - half_h,
                    x + half_w,
                    y + half_h,
                );
            if !found || free {
                tx = x;
                ty = y;
                found = true;
            }
            if free {
                break;
            }
            k += 2;
        }
        if careful {
            taken.push(tx - half_w);
            taken.push(ty - half_h);
            taken.push(tx + half_w);
            taken.push(ty + half_h);
        }
        labels.push_str(&label_svg(text, tx, ty, " class=\"pelers-label\""));
    }

    circles.reverse();
    let content = label_svg(
        &title,
        width / 2.0,
        title_pad / 2.0 + title.height / 2.0,
        &format!(
            " class=\"pelers-title\"{} font-weight=\"var(--_tw)\"",
            size_attr(title_size, size)
        ),
    ) + &format!(
        "<g class=\"pelers-quadrants\" fill=\"var(--_s)\" stroke=\"var(--_b)\">{quadrants}</g>"
    ) + &if axes.is_empty() {
        String::new()
    } else {
        format!(
            "<g class=\"pelers-axis-labels\" font-size=\"{}\" fill=\"var(--_m)\">{axes}</g>",
            js_number_to_string(x_axis_size)
        )
    } + &if dots.is_empty() {
        String::new()
    } else {
        format!(
            "<g class=\"pelers-points\" fill=\"{fill}\">{}</g><g class=\"pelers-point-labels\" font-size=\"{}\">{labels}</g>",
            circles.concat(),
            js_number_to_string(point_size)
        )
    };

    Drawn {
        svg: svg_document(
            "quadrantChart",
            width,
            height,
            size,
            options,
            Accessible {
                acc_title: model.acc_title.as_deref(),
                acc_descr: model.acc_descr.as_deref(),
            },
            &content,
        ),
        width,
        height,
        links: Vec::new(),
    }
}

fn leading_int(text: &str) -> f64 {
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        f64::NAN
    } else {
        js_number(&digits)
    }
}
