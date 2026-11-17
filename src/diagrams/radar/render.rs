use super::model::RadarModel;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{fit_width, title_room};
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::series_color;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::Value;

const SWATCH: f64 = 12.0;
const ROW: f64 = 26.0;
const GAP: f64 = 24.0;

#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

fn option(config: &Config, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match config
        .get("radar")
        .and_then(Value::as_object)
        .and_then(|r| r.get(key))
    {
        Some(Value::Number(value)) if *value >= min => js_min(*value, max),
        _ => fallback,
    }
}

pub fn relative_radius(value: f64, min: f64, max: f64, radius: f64) -> f64 {
    let clipped = js_min(js_max(value, min), max);
    let scaled = radius * (clipped - min);
    if scaled.is_finite() {
        scaled / (max - min)
    } else {
        radius * ((clipped - min) / (max - min))
    }
}

fn closed_round_curve(points: &[Point], tension: f64) -> String {
    let n = points.len();
    let mut d = format!("M{},{}", num(points[0].x), num(points[0].y));
    for i in 0..n {
        let p0 = points[(i + n - 1) % n];
        let p1 = points[i];
        let p2 = points[(i + 1) % n];
        let p3 = points[(i + 2) % n];
        d.push_str(&format!(
            " C{},{} {},{} {},{}",
            num(p1.x + (p2.x - p0.x) * tension),
            num(p1.y + (p2.y - p0.y) * tension),
            num(p2.x - (p3.x - p1.x) * tension),
            num(p2.y - (p3.y - p1.y) * tension),
            num(p2.x),
            num(p2.y)
        ));
    }
    d.push_str(" Z");
    d
}

fn polygon(points: &[Point]) -> String {
    let mut out = String::new();
    for p in points {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(&format!("{},{}", num(p.x), num(p.y)));
    }
    out
}

pub fn render_radar(model: &RadarModel, config: &Config, options: &RenderOptions) -> Drawn {
    let natural = option(config, "width", 320.0, 1.0, 4000.0);
    fit_width(options, natural, 160.0, |width, tight| {
        let height = option(config, "height", 320.0, 1.0, 4000.0) * width / natural;
        draw(model, config, options, width, height, tight)
    })
}

fn draw(
    model: &RadarModel,
    config: &Config,
    options: &RenderOptions,
    width: f64,
    height: f64,
    tight: bool,
) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);

    let axis_scale = option(config, "axisScaleFactor", 1.0, 0.0, 10.0);
    let label_factor = option(config, "axisLabelFactor", 1.05, 0.0, 10.0);
    let tension = option(config, "curveTension", 0.17, 0.0, 1.0);
    let radius = js_min(width, height) / 2.0;

    let axes = &model.axes;
    let curves = &model.curves;
    let ticks = model.options.ticks;
    let graticule = model.options.graticule.as_str();
    let min = model.options.min;
    let count = axes.len();
    let unit: Vec<Point> = (0..count)
        .map(|i| {
            let angle =
                2.0 * std::f64::consts::PI * i as f64 / count as f64 - std::f64::consts::PI / 2.0;
            Point {
                x: angle.cos(),
                y: angle.sin(),
            }
        })
        .collect();

    let max = match model.options.max {
        Some(max) => max,
        None => {
            let mut max = f64::NEG_INFINITY;
            for curve in curves {
                for &value in &curve.entries {
                    if value > max {
                        max = value;
                    }
                }
            }
            max
        }
    };

    let mut x0 = -width / 2.0 - option(config, "marginLeft", 16.0, 0.0, 2000.0);
    let mut x1 = width / 2.0 + option(config, "marginRight", 16.0, 0.0, 2000.0);
    let mut y0 = -height / 2.0 - option(config, "marginTop", 16.0, 0.0, 2000.0);
    let mut y1 = height / 2.0 + option(config, "marginBottom", 16.0, 0.0, 2000.0);

    let mut grid = String::new();
    let mut i = 0.0;
    while i < ticks {
        let r = js_min(radius, radius * (i + 1.0) / ticks);
        if graticule == "circle" {
            grid.push_str(&format!("<circle r=\"{}\"/>", num(r)));
        } else {
            let scaled: Vec<Point> = unit
                .iter()
                .map(|u| Point {
                    x: u.x * r,
                    y: u.y * r,
                })
                .collect();
            grid.push_str(&format!("<polygon points=\"{}\"/>", polygon(&scaled)));
        }
        i += 1.0;
    }

    let mut spokes = String::new();
    let mut labels = String::new();
    for (i, u) in unit.iter().enumerate() {
        let reach = radius * axis_scale;
        spokes.push_str(&format!("M0,0L{},{}", num(u.x * reach), num(u.y * reach)));
        let label: Label = layout_label(Some(&axes[i].label), false, small, 4000.0, 0);
        let at = radius * label_factor + 6.0;
        let cx = u.x * at
            + if u.x > 0.01 {
                label.width / 2.0
            } else if u.x < -0.01 {
                -label.width / 2.0
            } else {
                0.0
            };
        let cy = u.y * at
            + if u.y > 0.01 {
                label.height / 2.0
            } else if u.y < -0.01 {
                -label.height / 2.0
            } else {
                0.0
            };
        labels.push_str(&format!(
            "<g class=\"pelers-axis-label\" data-id=\"{}\">{}</g>",
            esc_text(&axes[i].name),
            label_svg(&label, cx, cy, "")
        ));
        x0 = js_min(x0, cx - label.width / 2.0);
        x1 = js_max(x1, cx + label.width / 2.0);
        y0 = js_min(y0, cy - label.height / 2.0);
        y1 = js_max(y1, cy + label.height / 2.0);
    }

    let mut shapes = String::new();
    for (index, curve) in curves.iter().enumerate() {
        if curve.entries.len() != count {
            continue;
        }
        let points: Vec<Point> = curve
            .entries
            .iter()
            .enumerate()
            .map(|(i, &value)| {
                let r = relative_radius(value, min, max, radius);
                let safe = if r.is_finite() { r } else { 0.0 };
                Point {
                    x: unit[i].x * safe,
                    y: unit[i].y * safe,
                }
            })
            .collect();
        let color = series_color(index);
        let attrs = format!(
            " class=\"pelers-curve\" data-id=\"{}\" fill=\"{color}\" stroke=\"{color}\"",
            esc_text(&curve.name)
        );
        if graticule == "circle" {
            shapes.push_str(&format!(
                "<path{attrs} d=\"{}\"/>",
                closed_round_curve(&points, tension)
            ));
        } else {
            shapes.push_str(&format!(
                "<polygon{attrs} points=\"{}\"/>",
                polygon(&points)
            ));
        }
    }

    let legend: Vec<Label> = if model.options.show_legend {
        curves
            .iter()
            .map(|curve| layout_label(Some(&curve.label), false, small, 4000.0, 0))
            .collect()
    } else {
        Vec::new()
    };
    let mut legend_width = 0.0f64;
    for label in &legend {
        legend_width = js_max(legend_width, label.width);
    }
    let legend_height = legend.len() as f64 * ROW;
    let legend_x = if tight {
        -(SWATCH + 8.0 + legend_width) / 2.0
    } else {
        x1 + GAP
    };
    let legend_top = if tight {
        y1 + GAP / 2.0
    } else {
        -legend_height / 2.0
    };
    if !legend.is_empty() && tight {
        x0 = js_min(x0, legend_x);
        x1 = js_max(x1, -legend_x);
        y1 = legend_top + legend_height;
    } else if !legend.is_empty() {
        x1 = legend_x + SWATCH + 8.0 + legend_width;
        y0 = js_min(y0, -legend_height / 2.0);
        y1 = js_max(y1, legend_height / 2.0);
    }
    let mut legend_out = String::new();
    for (index, label) in legend.iter().enumerate() {
        let y = legend_top + index as f64 * ROW + ROW / 2.0;
        let color = series_color(index);
        legend_out.push_str(&format!(
            "<g class=\"pelers-legend-item\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{color}\" fill-opacity=\"0.4\" stroke=\"{color}\"/>{}</g>",
            esc_text(&curves[index].name),
            num(legend_x),
            num(y - SWATCH / 2.0),
            js_number_to_string(SWATCH),
            js_number_to_string(SWATCH),
            label_svg(label, legend_x + SWATCH + 8.0 + label.width / 2.0, y, "")
        ));
    }

    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        title_room(options),
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 8.0
    } else {
        0.0
    };
    let body_width = js_max(x1 - x0, title.width);
    let total_width = (body_width + 2.0 * pad).ceil();
    let total_height = (y1 - y0 + title_height + 2.0 * pad).ceil();
    let origin = format!(
        "translate({},{})",
        num(pad + (body_width - (x1 - x0)) / 2.0 - x0),
        num(pad + title_height - y0)
    );

    let content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &format!("<g transform=\"{origin}\">")
        + &if grid.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pelers-graticule\" fill=\"none\" stroke=\"var(--_a)\">{grid}</g>")
        }
        + &if spokes.is_empty() {
            String::new()
        } else {
            format!(
                "<path class=\"pelers-axes\" d=\"{spokes}\" fill=\"none\" stroke=\"var(--_b)\"/>"
            )
        }
        + &if shapes.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-curves\" fill-opacity=\"0.2\" stroke-width=\"1.5\" stroke-linejoin=\"round\">{shapes}</g>"
            )
        }
        + &if labels.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-axis-labels\" font-size=\"{}\">{labels}</g>",
                js_number_to_string(small)
            )
        }
        + &if legend_out.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-legend\" font-size=\"{}\">{legend_out}</g>",
                js_number_to_string(small)
            )
        }
        + "</g>";

    Drawn {
        svg: svg_document(
            "radar",
            total_width,
            total_height,
            size,
            options,
            Accessible {
                acc_title: model.acc_title.as_deref(),
                acc_descr: model.acc_descr.as_deref(),
            },
            &content,
        ),
        width: total_width,
        height: total_height,
        links: Vec::new(),
    }
}
