use super::db::{Plot, XAxis, XyChartDb};
use super::ticks::{tick_index, tick_step, tick_value, ticks_between};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::cap_width;
use crate::errors::{PelersError, PelersErrorCode};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::series_color;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, width as measure};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::{Object, Value};

const TICK: f64 = 4.0;
const SWATCH: f64 = 12.0;
const ROW: f64 = 26.0;
const LEGEND_GAP: f64 = 24.0;
const LIMIT: f64 = 1e300;

struct Mark {
    at: f64,
    label: Label,
}

fn record(value: Option<&Value>) -> Object {
    match value {
        Some(Value::Object(object)) => object.clone(),
        _ => Object::default(),
    }
}

fn on(config: &Object, key: &str, fallback: bool) -> bool {
    match config.get(key) {
        Some(Value::Bool(flag)) => *flag,
        _ => fallback,
    }
}

fn dimension(value: Option<&Value>, fallback: f64) -> f64 {
    match value {
        Some(Value::Number(n)) if *n > 0.0 => js_round(js_min(js_max(*n, 100.0), 4000.0)),
        _ => fallback,
    }
}

fn finite(value: Option<f64>) -> Option<f64> {
    value
        .filter(|v| v.is_finite())
        .map(|v| js_min(js_max(v, -LIMIT), LIMIT))
}

fn widest(marks: &[Mark]) -> f64 {
    let mut width = 0.0f64;
    for mark in marks {
        width = js_max(width, mark.label.width);
    }
    width
}

fn aligned(label: &Label, x: f64, y: f64, anchor: &str) -> String {
    if label.lines.len() == 1 {
        let line = &label.lines[0];
        if line.len() == 1 && line[0].icon.is_none() && line[0].style == 0 {
            return format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\">{}</text>",
                num(x),
                num(y + label.size * 0.35),
                esc(&line[0].text)
            );
        }
    }
    let offset = match anchor {
        "start" => label.width / 2.0,
        "end" => -label.width / 2.0,
        _ => 0.0,
    };
    label_svg(label, x + offset, y, "")
}

fn value_marks(
    text: &dyn Fn(Option<&str>, f64, f64) -> Label,
    small: f64,
    lo: &mut f64,
    hi: &mut f64,
    exact: bool,
    length: f64,
    spacing: f64,
) -> Vec<Mark> {
    let step = tick_step(
        (*hi - *lo).abs(),
        js_max(2.0, js_min(10.0, (length / spacing).floor())),
    );
    if !exact {
        *lo = tick_value(step, (tick_index(step, *lo) + 1e-9).floor());
        *hi = tick_value(step, (tick_index(step, *hi) - 1e-9).ceil());
    }
    let (low, high) = (js_min(*lo, *hi), js_max(*lo, *hi));
    ticks_between(low, high, step)
        .into_iter()
        .map(|value| Mark {
            at: (value - *lo) / (*hi - *lo),
            label: text(Some(&js_number_to_string(value)), small, 4000.0),
        })
        .collect()
}

pub fn render_xychart(
    model: &XyChartDb,
    config: &Config,
    options: &RenderOptions,
) -> Result<Drawn, PelersError> {
    if model.plots.is_empty() {
        let mut error = PelersError::new(
            "No Plot to render, please provide a plot with some data",
            PelersErrorCode::Semantic,
        );
        error.diagram_type = Some("xychart".to_owned());
        return Err(error);
    }
    let chart = record(config.get("xyChart"));
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let tiny = js_round(size * 0.75);
    let pad = options.padding.unwrap_or(8.0);
    let natural = dimension(chart.get("width"), 700.0);
    let width = cap_width(options, natural, 240.0, 0.0);
    let height = js_round(dimension(chart.get("height"), 500.0) * js_max(0.6, width / natural));
    let orientation =
        model
            .orientation
            .map(str::to_owned)
            .or_else(|| match chart.get("chartOrientation") {
                Some(Value::String(text)) => Some(text.clone()),
                _ => None,
            });
    let horizontal = orientation.as_deref() == Some("horizontal");
    let show_values = on(&chart, "showDataLabel", false);
    let x = &model.x_axis;
    let plots: &[Plot] = &model.plots;
    let text = |raw: Option<&str>, font: f64, max: f64| -> Label {
        layout_label(raw, false, font, max, 0)
    };
    let line_height = text(Some("0"), small, 4000.0).height;
    let tiny_height = text(Some("0"), tiny, 4000.0).height;

    let title = layout_label(
        if on(&chart, "showTitle", true) {
            model.title.as_deref()
        } else {
            None
        },
        false,
        size,
        width - 2.0 * pad,
        BOLD_STYLE,
    );
    let title_space = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };

    let mut legend: Vec<(usize, Label)> = Vec::new();
    let mut legend_width = 0.0f64;
    if on(&chart, "showLegend", true) {
        for (index, plot) in plots.iter().enumerate() {
            if plot.title.is_empty() {
                continue;
            }
            let label = text(Some(&plot.title), small, width * 0.3);
            legend_width = js_max(legend_width, label.width + SWATCH + 8.0);
            legend.push((index, label));
        }
    }
    let legend_space = if legend.is_empty() {
        0.0
    } else {
        legend_width + LEGEND_GAP
    };

    let bars = plots.iter().filter(|plot| plot.bar).count();
    let mut lo = finite(Some(model.y_axis.min)).unwrap_or(f64::INFINITY);
    let mut hi = finite(Some(model.y_axis.max)).unwrap_or(f64::NEG_INFINITY);
    let exact = model.has_set_y && lo.is_finite() && hi.is_finite() && lo != hi;
    if !exact {
        lo = f64::INFINITY;
        hi = f64::NEG_INFINITY;
        for plot in plots {
            for (_, raw) in &plot.data {
                let Some(value) = finite(*raw) else {
                    continue;
                };
                if value < lo {
                    lo = value;
                }
                if value > hi {
                    hi = value;
                }
            }
        }
        if lo > hi {
            lo = 0.0;
            hi = 0.0;
        }
        if bars > 0 {
            lo = js_min(lo, 0.0);
            hi = js_max(hi, 0.0);
        }
        if lo == hi {
            if lo == 0.0 {
                hi = 1.0;
            } else {
                lo -= lo.abs() / 2.0;
                hi += hi.abs() / 2.0;
            }
        }
    }

    let mut count = 1usize;
    if let XAxis::Band { categories, .. } = x {
        count = js_max(categories.len() as f64, 1.0) as usize;
    } else {
        for plot in plots {
            count = count.max(plot.data.len());
        }
    }
    let slot = 1.0 / count as f64;
    let is_band = matches!(x, XAxis::Band { .. });
    let inset = if is_band || bars > 0 { slot / 2.0 } else { 0.0 };
    let (x0, x1) = match x {
        XAxis::Linear { min, max, .. } => (finite(Some(*min)), finite(Some(*max))),
        XAxis::Band { .. } => (None, None),
    };
    let ranged = matches!((x0, x1), (Some(a), Some(b)) if a != b);
    let place = |i: usize, n: usize| -> f64 {
        if is_band {
            (i as f64 + 0.5) * slot
        } else {
            let along = if n > 1 {
                i as f64 / (n as f64 - 1.0)
            } else if ranged {
                0.0
            } else {
                0.5
            };
            inset + along * (1.0 - 2.0 * inset)
        }
    };

    let category_conf = record(chart.get("xAxis"));
    let value_conf = record(chart.get("yAxis"));
    let (left_conf, bottom_conf) = if horizontal {
        (&category_conf, &value_conf)
    } else {
        (&value_conf, &category_conf)
    };
    let x_title = match x {
        XAxis::Band { title, .. } | XAxis::Linear { title, .. } => title.clone(),
    };
    let left_title = text(
        if on(left_conf, "showTitle", true) {
            Some(if horizontal {
                x_title.as_str()
            } else {
                model.y_axis.title.as_str()
            })
        } else {
            None
        },
        small,
        height * 0.8,
    );
    let bottom_title = text(
        if on(bottom_conf, "showTitle", true) {
            Some(if horizontal {
                model.y_axis.title.as_str()
            } else {
                x_title.as_str()
            })
        } else {
            None
        },
        small,
        width * 0.8,
    );
    let left_tick = if on(left_conf, "showTick", true) {
        TICK
    } else {
        0.0
    };
    let bottom_tick = if on(bottom_conf, "showTick", true) {
        TICK
    } else {
        0.0
    };
    let left_labels = on(left_conf, "showLabel", true);
    let bottom_labels = on(bottom_conf, "showLabel", true);

    let wrap_at = if horizontal {
        width * 0.3
    } else {
        js_max((width - 2.0 * pad - 64.0 - legend_space) * slot - 8.0, 48.0)
    };
    let categories: Vec<Label> = match x {
        XAxis::Band { categories, .. } if on(&category_conf, "showLabel", true) => categories
            .iter()
            .map(|category| text(Some(category), small, wrap_at))
            .collect(),
        _ => Vec::new(),
    };
    let mut category_width = 0.0f64;
    let mut category_height = 0.0f64;
    for label in &categories {
        category_width = js_max(category_width, label.width);
        category_height = js_max(category_height, label.height);
    }

    let category_marks = |length: f64| -> Vec<Mark> {
        let mut marks = Vec::new();
        match x {
            XAxis::Band {
                categories: all, ..
            } => {
                let need = if categories.is_empty() {
                    8.0
                } else if horizontal {
                    category_height
                } else {
                    category_width + 12.0
                };
                let every = js_max(1.0, (need / (length * slot)).ceil()) as usize;
                let mut i = 0usize;
                while i < all.len() {
                    marks.push(Mark {
                        at: (i as f64 + 0.5) * slot,
                        label: match categories.get(i) {
                            Some(label) => label.clone(),
                            None => text(None, small, 0.0),
                        },
                    });
                    i += every;
                }
            }
            XAxis::Linear { .. } if ranged => {
                let (a, b) = (x0.unwrap(), x1.unwrap());
                let step = tick_step(
                    (b - a).abs(),
                    js_max(
                        2.0,
                        js_min(
                            10.0,
                            (length / if horizontal { 32.0 } else { 80.0 }).floor(),
                        ),
                    ),
                );
                for value in ticks_between(js_min(a, b), js_max(a, b), step) {
                    marks.push(Mark {
                        at: inset + ((value - a) / (b - a)) * (1.0 - 2.0 * inset),
                        label: text(Some(&js_number_to_string(value)), small, 4000.0),
                    });
                }
            }
            XAxis::Linear { .. } => {
                if let Some(a) = x0 {
                    marks.push(Mark {
                        at: 0.5,
                        label: text(Some(&js_number_to_string(a)), small, 4000.0),
                    });
                }
            }
        }
        marks
    };

    let mut point_label_width = 0.0f64;
    let mut value_width = 0.0f64;
    for plot in plots {
        for label in plot.point_labels.iter().flatten() {
            point_label_width = js_max(point_label_width, measure(label, tiny, 0));
        }
        if show_values && horizontal && plot.bar {
            for (_, value) in &plot.data {
                let text = match finite(*value) {
                    Some(v) => js_number_to_string(v),
                    None => String::new(),
                };
                value_width = js_max(value_width, measure(&text, tiny, 0));
            }
        }
    }

    let bottom_label_height = if !bottom_labels {
        0.0
    } else if horizontal || !is_band {
        line_height
    } else {
        category_height
    };
    let bottom_space = bottom_tick
        + if bottom_label_height > 0.0 {
            bottom_label_height + 4.0
        } else {
            0.0
        }
        + if bottom_title.height > 0.0 {
            bottom_title.height + 4.0
        } else {
            0.0
        };
    let headroom = if horizontal {
        0.0
    } else if point_label_width > 0.0 || show_values {
        tiny_height + 4.0
    } else if left_labels {
        line_height / 2.0
    } else {
        0.0
    };
    let top = pad + title_space + headroom;
    let plot_height = js_max(1.0, height - pad - bottom_space - top);
    let bottom = top + plot_height;

    let left_marks = if horizontal {
        category_marks(plot_height)
    } else {
        value_marks(&text, small, &mut lo, &mut hi, exact, plot_height, 56.0)
    };
    let left_label_width = if left_labels {
        widest(&left_marks)
    } else {
        0.0
    };
    let left =
        pad + if left_title.height > 0.0 {
            left_title.height + 6.0
        } else {
            0.0
        } + if left_label_width > 0.0 {
            left_label_width + 6.0
        } else {
            0.0
        } + left_tick;
    let beside = js_max(
        if point_label_width > 0.0 {
            point_label_width + 12.0
        } else {
            0.0
        },
        if value_width > 0.0 {
            value_width + 8.0
        } else {
            0.0
        },
    );
    let reach = if horizontal {
        js_min(width * 0.25, beside)
    } else {
        0.0
    };
    let mut plot_width = js_max(1.0, width - pad - left - legend_space - reach);
    let bottom_marks = if horizontal {
        value_marks(&text, small, &mut lo, &mut hi, exact, plot_width, 80.0)
    } else {
        category_marks(plot_width)
    };
    if bottom_labels && legend_space + reach == 0.0 {
        let mut over = 0.0f64;
        for mark in &bottom_marks {
            over = js_max(
                over,
                mark.at * plot_width + mark.label.width / 2.0 - plot_width,
            );
        }
        plot_width = js_max(1.0, plot_width - over);
    }
    let right = left + plot_width;

    let px = |c: f64, v: f64| -> f64 {
        if horizontal {
            left + v * plot_width
        } else {
            left + c * plot_width
        }
    };
    let py = |c: f64, v: f64| -> f64 {
        if horizontal {
            top + c * plot_height
        } else {
            bottom - v * plot_height
        }
    };
    let scale = |value: f64| (value - lo) / (hi - lo);
    let clamp = |v: f64| js_min(js_max(v, 0.0), 1.0);

    let rule = |v: f64| -> String {
        if horizontal {
            format!("M{},{}V{}", num(px(0.0, v)), num(top), num(bottom))
        } else {
            format!("M{},{}H{}", num(left), num(py(0.0, v)), num(right))
        }
    };

    let mut grid = String::new();
    for mark in if horizontal {
        &bottom_marks
    } else {
        &left_marks
    } {
        grid.push_str(&rule(mark.at));
    }

    let mut axis = String::new();
    let mut labels = String::new();
    if on(left_conf, "showAxisLine", true) {
        axis.push_str(&format!("M{},{}V{}", num(left), num(top), num(bottom)));
    }
    if on(bottom_conf, "showAxisLine", true) {
        axis.push_str(&format!("M{},{}H{}", num(left), num(bottom), num(right)));
    }
    for mark in &left_marks {
        let y = if horizontal {
            top + mark.at * plot_height
        } else {
            bottom - mark.at * plot_height
        };
        if left_tick != 0.0 {
            axis.push_str(&format!(
                "M{},{}h{}",
                num(left),
                num(y),
                js_number_to_string(-left_tick)
            ));
        }
        if left_labels {
            labels.push_str(&aligned(&mark.label, left - left_tick - 6.0, y, "end"));
        }
    }
    for mark in &bottom_marks {
        let at = left + mark.at * plot_width;
        if bottom_tick != 0.0 {
            axis.push_str(&format!(
                "M{},{}v{}",
                num(at),
                num(bottom),
                js_number_to_string(bottom_tick)
            ));
        }
        if bottom_labels {
            labels.push_str(&aligned(
                &mark.label,
                at,
                bottom + bottom_tick + 4.0 + mark.label.height / 2.0,
                "middle",
            ));
        }
    }

    let span = (if horizontal { plot_height } else { plot_width }) * slot;
    let group = span * 0.7;
    let thickness = group / js_max(bars as f64, 1.0);
    let gap = if bars > 1 {
        js_min(2.0, thickness * 0.2)
    } else {
        0.0
    };
    let base = clamp(scale(0.0));
    if bars > 0 && base > 0.0 && base < 1.0 {
        axis.push_str(&rule(base));
    }

    let mut series = String::new();
    let mut lines = String::new();
    let mut outside = String::new();
    let mut point_labels = String::new();
    let mut bar = 0usize;
    for (index, plot) in plots.iter().enumerate() {
        let color = series_color(index);
        let id = if plot.title.is_empty() {
            String::new()
        } else {
            format!(" data-id=\"{}\"", esc_text(&plot.title))
        };
        let n = if is_band {
            plot.data.len().min(count)
        } else {
            plot.data.len()
        };
        if plot.bar {
            let offset =
                (bar as f64 - (bars as f64 - 1.0) / 2.0) * thickness - (thickness - gap) / 2.0;
            bar += 1;
            let mut rects = String::new();
            for (i, point) in plot.data.iter().enumerate().take(n) {
                let Some(value) = finite(point.1) else {
                    continue;
                };
                let c = place(i, n);
                let v = clamp(scale(value));
                let from = js_min(base, v);
                let to = js_max(base, v);
                let bx = if horizontal {
                    px(c, from)
                } else {
                    px(c, 0.0) + offset
                };
                let by = if horizontal {
                    py(c, 0.0) + offset
                } else {
                    py(c, to)
                };
                let bw = if horizontal {
                    (to - from) * plot_width
                } else {
                    thickness - gap
                };
                let bh = if horizontal {
                    thickness - gap
                } else {
                    (to - from) * plot_height
                };
                rects.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                    num(bx),
                    num(by),
                    num(bw),
                    num(bh)
                ));
                if !show_values {
                    continue;
                }
                let label = js_number_to_string(value);
                let w = measure(&label, tiny, 0);
                let out = if v >= base { 1.0 } else { -1.0 };
                let mut tx = bx + bw / 2.0;
                let mut ty = by + bh / 2.0;
                let mut anchor = "middle";
                if horizontal {
                    if bh < tiny {
                        continue;
                    }
                    tx = (if out > 0.0 { bx + bw } else { bx }) + out * 6.0;
                    anchor = if out > 0.0 { "start" } else { "end" };
                } else {
                    if w + 2.0 > span / bars as f64 {
                        continue;
                    }
                    ty = (if out > 0.0 { by } else { by + bh }) - out * (tiny_height / 2.0 + 2.0);
                }
                outside.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"{anchor}\">{label}</text>",
                    num(tx),
                    num(ty + tiny * 0.35)
                ));
            }
            series.push_str(&format!(
                "<g class=\"pelers-series pelers-bars\"{id} fill=\"{color}\">{rects}</g>"
            ));
            continue;
        }

        let mut d = String::new();
        let mut dots = String::new();
        let mut pen = false;
        let mut pc = 0.0f64;
        let mut pv = f64::NAN;
        for i in 0..n {
            let Some(value) = finite(plot.data[i].1) else {
                pv = f64::NAN;
                continue;
            };
            let c = place(i, n);
            let v = scale(value);
            let visible = (0.0..=1.0).contains(&v);
            if pv.is_nan() {
                if visible {
                    d.push_str(&format!("M{},{}", num(px(c, v)), num(py(c, v))));
                }
                pen = visible;
            } else {
                let dv = v - pv;
                let mut t0 = 0.0f64;
                let mut t1 = 1.0f64;
                if dv != 0.0 {
                    let a = -pv / dv;
                    let b = (1.0 - pv) / dv;
                    t0 = js_max(t0, js_min(a, b));
                    t1 = js_min(t1, js_max(a, b));
                } else if !visible {
                    t0 = 2.0;
                }
                if t0 <= t1 {
                    let c0 = pc + (c - pc) * t0;
                    let v0 = pv + dv * t0;
                    let c1 = pc + (c - pc) * t1;
                    let v1 = pv + dv * t1;
                    if !pen || t0 > 0.0 {
                        d.push_str(&format!("M{},{}", num(px(c0, v0)), num(py(c0, v0))));
                    }
                    d.push_str(&format!("L{},{}", num(px(c1, v1)), num(py(c1, v1))));
                }
                pen = visible;
            }
            pc = c;
            pv = v;
            if !visible {
                continue;
            }
            let cx = px(c, v);
            let cy = py(c, v);
            if span >= 12.0 {
                dots.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"3\"/>",
                    num(cx),
                    num(cy)
                ));
            }
            let label = text(
                plot.point_labels
                    .as_ref()
                    .and_then(|labels| labels.get(i))
                    .map(String::as_str),
                tiny,
                4000.0,
            );
            if label.height > 0.0 {
                let half = label.width / 2.0;
                point_labels.push_str(&if horizontal {
                    aligned(&label, cx + 10.0, cy, "start")
                } else {
                    aligned(
                        &label,
                        js_max(js_min(cx, right - half), left + half),
                        cy - 6.0 - label.height / 2.0,
                        "middle",
                    )
                });
            }
        }
        lines.push_str(&format!(
            "<g class=\"pelers-series pelers-line\"{id} fill=\"{color}\">{}{dots}</g>",
            if d.is_empty() {
                String::new()
            } else {
                format!(
                    "<path d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"
                )
            }
        ));
    }

    let mut titles = String::new();
    if left_title.height > 0.0 {
        titles.push_str(&format!(
            "<g transform=\"translate({},{}) rotate(-90)\">{}</g>",
            num(pad + left_title.height / 2.0),
            num(top + plot_height / 2.0),
            label_svg(&left_title, 0.0, 0.0, "")
        ));
    }
    titles.push_str(&label_svg(
        &bottom_title,
        left + plot_width / 2.0,
        height - pad - bottom_title.height / 2.0,
        "",
    ));

    let mut legend_height = 0.0f64;
    for (_, label) in &legend {
        legend_height += js_max(ROW, label.height + 6.0);
    }
    let mut ly = top + js_max(0.0, (plot_height - legend_height) / 2.0);
    let lx = right + reach + LEGEND_GAP;
    let mut legend_out = String::new();
    for (index, label) in &legend {
        let row = js_max(ROW, label.height + 6.0);
        if ly + row > height {
            break;
        }
        let y = ly + row / 2.0;
        let color = series_color(*index);
        let plot = &plots[*index];
        let mark = if plot.bar {
            format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{color}\"/>",
                num(lx),
                num(y - SWATCH / 2.0),
                js_number_to_string(SWATCH),
                js_number_to_string(SWATCH)
            )
        } else {
            format!(
                "<path d=\"M{},{}h{}\" stroke=\"{color}\" stroke-width=\"2\" stroke-linecap=\"round\"/>",
                num(lx),
                num(y),
                js_number_to_string(SWATCH)
            )
        };
        legend_out.push_str(&format!(
            "<g class=\"pelers-legend-item\" data-id=\"{}\">{mark}{}</g>",
            esc_text(&plot.title),
            label_svg(label, lx + SWATCH + 8.0 + label.width / 2.0, y, "")
        ));
        ly += row;
    }

    let content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &if grid.is_empty() {
        String::new()
    } else {
        format!("<path class=\"pelers-grid\" d=\"{grid}\" stroke=\"var(--_a)\"/>")
    } + &format!("<g class=\"pelers-plot\">{series}{lines}</g>")
        + &if axis.is_empty() {
            String::new()
        } else {
            format!("<path class=\"pelers-axis\" d=\"{axis}\" fill=\"none\" stroke=\"var(--_b)\"/>")
        }
        + &if labels.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-axis-labels\" font-size=\"{}\" fill=\"var(--_m)\">{labels}</g>",
                js_number_to_string(small)
            )
        }
        + &if titles.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-axis-titles\" font-size=\"{}\">{titles}</g>",
                js_number_to_string(small)
            )
        }
        + &if outside.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-data-labels\" font-size=\"{}\">{outside}</g>",
                js_number_to_string(tiny)
            )
        }
        + &if point_labels.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-point-labels\" font-size=\"{}\">{point_labels}</g>",
                js_number_to_string(tiny)
            )
        }
        + &if legend_out.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-legend\" font-size=\"{}\">{legend_out}</g>",
                js_number_to_string(small)
            )
        };

    Ok(Drawn {
        svg: svg_document(
            "xychart",
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
    })
}
