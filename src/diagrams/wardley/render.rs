use std::collections::{HashMap, HashSet};

use super::model::{Flow, WardleyModel, WardleyNode};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::fit_width;
use crate::preprocess::Config;
use crate::svg::builder::{
    esc, esc_text, label_svg_with, num, shared_style, span_style, text_style,
};

use crate::svg::edges::{marker, marker_trim};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::ROUNDED;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::{Object, Value};

const DEFAULT_STAGES: [&str; 4] = ["Genesis", "Custom Built", "Product", "Commodity"];
const CANVAS_MARGIN: f64 = 48.0;
const R: f64 = 6.0;
const SQUARE: f64 = 10.0;
const LABEL_OFFSET: f64 = 8.0;
const PIPELINE_PAD: f64 = 15.0;
const FORCE_WIDTH: f64 = 60.0;
const FORCE_HEIGHT: f64 = 30.0;
const FORCE_HEAD: f64 = 20.0;
const ACCENT: &str = "var(--_c)";
const LINE: &str = "var(--_l)";

fn section(config: &Object) -> Option<&Object> {
    config.get("wardley-beta").and_then(Value::as_object)
}

fn setting(config: &Object, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match section(config).and_then(|s| s.get(key)) {
        Some(Value::Number(value)) if *value >= min && *value <= max => *value,
        _ => fallback,
    }
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn start_label(label: &Label, x: f64, y: f64, attrs: &str) -> String {
    let shared = shared_style(label);
    let mut body = String::new();
    let mut baseline = y;
    for line in &label.lines {
        let mut first = true;
        for span in line {
            if span.icon.is_some() {
                continue;
            }
            let style = span_style(span.style & !shared);
            let position = if first {
                format!(" x=\"{}\" y=\"{}\"", num(x), num(baseline))
            } else {
                String::new()
            };
            body.push_str(&format!(
                "<tspan{position}{style}>{}</tspan>",
                esc(&span.text)
            ));
            first = false;
        }
        baseline += label.line_height;
    }
    if body.is_empty() {
        String::new()
    } else {
        format!(
            "<text{} xml:space=\"preserve\">{body}</text>",
            text_style(attrs, shared)
        )
    }
}

fn dimension(config: &Object, given: Option<f64>, key: &str, fallback: f64) -> f64 {
    let value = match given {
        Some(value) if value.is_finite() => value,
        _ => setting(config, key, fallback, 1.0, 1e6),
    };
    clamp(value, 200.0, 10000.0) - 2.0 * CANVAS_MARGIN
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    fn grow(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        if x0 < self.min_x {
            self.min_x = x0;
        }
        if y0 < self.min_y {
            self.min_y = y0;
        }
        if x1 > self.max_x {
            self.max_x = x1;
        }
        if y1 > self.max_y {
            self.max_y = y1;
        }
    }
}

struct Placed {
    x: f64,
    y: f64,
    node: usize,
}

struct Canvas {
    plot_w: f64,
    plot_h: f64,
    tight: bool,
    small: f64,
    icons: bool,
    bounds: Bounds,
}

impl Canvas {
    fn text(&self, raw: &str, font_size: f64, max_width: f64, style: u32) -> Label {
        layout_label(Some(raw), false, font_size, max_width, style)
    }

    fn text_default(&self, raw: &str, font_size: f64, style: u32) -> Label {
        let max = if self.tight {
            js_max(64.0, js_min(120.0, self.plot_w / 2.0))
        } else {
            4000.0
        };
        self.text(raw, font_size, max, style)
    }

    fn anchor_text(&self, raw: &str) -> Label {
        let max = if self.tight {
            js_max(64.0, self.plot_w / 2.0)
        } else {
            4000.0
        };
        self.text(raw, self.small, max, BOLD_STYLE)
    }

    fn label_svg(&self, label: &Label, cx: f64, cy: f64, attrs: &str) -> String {
        label_svg_with(label, cx, cy, attrs, self.icons)
    }

    fn centered(&mut self, label: &Label, cx: f64, cy: f64, attrs: &str) -> String {
        self.bounds.grow(
            cx - label.width / 2.0,
            cy - label.height / 2.0,
            cx + label.width / 2.0,
            cy + label.height / 2.0,
        );
        self.label_svg(label, cx, cy, attrs)
    }

    fn starting(&mut self, label: &Label, x: f64, y: f64, attrs: &str) -> String {
        self.bounds.grow(
            x,
            y - label.size,
            x + label.width,
            y - label.size + label.height + 4.0,
        );
        start_label(label, x, y, attrs)
    }

    fn project_x(&self, value: f64) -> f64 {
        (value / 100.0) * self.plot_w
    }

    fn project_y(&self, value: f64) -> f64 {
        self.plot_h - (value / 100.0) * self.plot_h
    }

    fn reach(&self, node: &WardleyNode, ux: f64, uy: f64) -> f64 {
        if node.class_name.as_deref() == Some("anchor") {
            let label = self.anchor_text(&node.label);
            return 1.0 / (ux / (label.width / 2.0 + 5.0)).hypot(uy / (self.small * 0.8));
        }
        if node.is_pipeline_parent {
            return SQUARE / std::f64::consts::SQRT_2;
        }
        if node.source_strategy.is_some() {
            2.0 * R
        } else {
            R
        }
    }
}

pub fn render_wardley(model: &WardleyModel, config: &Config, options: &RenderOptions) -> Drawn {
    let natural = dimension(config, model.size.map(|s| s.0), "width", 900.0);
    fit_width(options, natural, 160.0, |plot_w, tight| {
        draw(model, config, options, plot_w, natural, tight)
    })
}

fn draw(
    model: &WardleyModel,
    config: &Config,
    options: &RenderOptions,
    plot_w: f64,
    natural_w: f64,
    tight: bool,
) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.8125);
    let tiny = js_round(size * 0.6875);
    let pad = options.padding.unwrap_or(8.0);

    let plot_h = dimension(config, model.size.map(|s| s.1), "height", 600.0)
        * js_max(0.75, plot_w / natural_w);
    let show_grid = section(config)
        .and_then(|s| s.get("showGrid"))
        .is_some_and(|v| matches!(v, Value::Bool(true)));

    let mut canvas = Canvas {
        plot_w,
        plot_h,
        tight,
        small,
        icons: options.icons.is_some(),
        bounds: Bounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: plot_w,
            max_y: plot_h,
        },
    };

    let stages: Vec<String> = match &model.axes.stages {
        Some(stages) if !stages.is_empty() => stages.clone(),
        _ => DEFAULT_STAGES.iter().map(|s| (*s).to_owned()).collect(),
    };
    let boundaries = model.axes.stage_boundaries.clone();
    let custom = boundaries.as_ref().is_some_and(|b| b.len() == stages.len());
    let mut frame = String::new();
    let mut stage_labels = String::new();
    let mut stage_bottom = plot_h;
    let mut previous = 0.0f64;
    let count = stages.len() as f64;
    for (index, stage) in stages.iter().enumerate() {
        let start = if custom {
            previous
        } else {
            index as f64 / count
        };
        let end = if custom {
            clamp(boundaries.as_ref().map_or(0.0, |b| b[index]), 0.0, 1.0)
        } else {
            (index as f64 + 1.0) / count
        };
        previous = end;
        if index > 0 {
            frame.push_str(&format!(
                "<path class=\"pelers-wardley-stage\" d=\"M{},0V{}\" stroke=\"var(--_b)\" stroke-dasharray=\"4 4\"/>",
                num(start * plot_w),
                num(plot_h)
            ));
        }
        let label = canvas.text(
            stage,
            small,
            js_max(48.0, (end - start).abs() * plot_w - 12.0),
            0,
        );
        let markup = canvas.centered(
            &label,
            ((start + end) / 2.0) * plot_w,
            plot_h + 8.0 + label.height / 2.0,
            &format!(
                " class=\"pelers-wardley-stage-label\" font-size=\"{}\" fill=\"var(--_m)\"",
                js_number_to_string(small)
            ),
        );
        stage_labels.push_str(&markup);
        stage_bottom = js_max(stage_bottom, plot_h + 8.0 + label.height);
    }
    if show_grid {
        for i in 1..4 {
            let i = i as f64;
            frame.push_str(&format!(
                "<path class=\"pelers-wardley-grid\" d=\"M{},0V{}M0,{}H{}\" stroke=\"var(--_b)\" stroke-dasharray=\"2 6\"/>",
                num(plot_w * i / 4.0),
                num(plot_h),
                num(plot_h * i / 4.0),
                num(plot_w)
            ));
        }
    }
    frame.push_str(&format!(
        "<path class=\"pelers-wardley-axis\" d=\"M0,0V{}H{}\" stroke=\"{LINE}\"/>",
        num(plot_h),
        num(plot_w)
    ));
    let x_title = canvas.text_default("Evolution", small, 0);
    let y_title = canvas.text_default("Visibility", small, 0);
    let axis_attrs = format!(
        " class=\"pelers-wardley-axis-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
        js_number_to_string(small)
    );
    let axis_labels = canvas.centered(
        &x_title,
        plot_w / 2.0,
        stage_bottom + 6.0 + x_title.height / 2.0,
        &axis_attrs,
    ) + &canvas.label_svg(
        &y_title,
        -16.0,
        plot_h / 2.0,
        &format!(
            "{axis_attrs} transform=\"rotate(-90 -16 {})\"",
            num(plot_h / 2.0)
        ),
    );
    canvas
        .bounds
        .grow(-16.0 - y_title.height / 2.0, 0.0, 0.0, 0.0);
    let legend_top = stage_bottom + 6.0 + x_title.height + 10.0;

    let mut placed: Vec<Placed> = Vec::with_capacity(model.nodes.len());
    let mut index_of: HashMap<String, usize> = HashMap::new();
    for (node_index, node) in model.nodes.iter().enumerate() {
        index_of.insert(node.id.clone(), placed.len());
        placed.push(Placed {
            x: canvas.project_x(node.x.unwrap_or(0.0)),
            y: canvas.project_y(node.y.unwrap_or(0.0)),
            node: node_index,
        });
    }

    let mut pipelines = String::new();
    let mut members: HashMap<&str, HashSet<&str>> = HashMap::new();
    for pipeline in &model.pipelines {
        members.insert(
            pipeline.node_id.as_str(),
            pipeline.component_ids.iter().map(String::as_str).collect(),
        );
        let mut parts: Vec<usize> = pipeline
            .component_ids
            .iter()
            .filter_map(|id| index_of.get(id).copied())
            .collect();
        if parts.is_empty() {
            continue;
        }
        parts.sort_by(|&a, &b| {
            placed[a]
                .x
                .partial_cmp(&placed[b].x)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let left = placed[parts[0]].x;
        let right = placed[parts[parts.len() - 1]].x;
        let y = placed[parts[0]].y;
        let top = y - 2.0 * R;
        if let Some(&parent) = index_of.get(&pipeline.node_id) {
            placed[parent].x = (left + right) / 2.0;
            placed[parent].y = top - SQUARE / 6.0;
        }
        canvas.bounds.grow(
            left - PIPELINE_PAD,
            top,
            right + PIPELINE_PAD,
            top + 4.0 * R,
        );
        let edge = if parts.len() > 1 {
            format!(
                "<path d=\"M{},{}H{}\" stroke=\"{LINE}\" stroke-dasharray=\"4 4\"/>",
                num(left),
                num(y),
                num(right)
            )
        } else {
            String::new()
        };
        pipelines.push_str(&format!(
            "<g class=\"pelers-wardley-pipeline\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{ROUNDED}/>{edge}</g>",
            esc_text(&pipeline.node_id),
            num(left - PIPELINE_PAD),
            num(top),
            num(right - left + 2.0 * PIPELINE_PAD),
            num(4.0 * R)
        ));
    }

    let mut taken = vec![0u8; placed.len()];
    let occupy = |taken: &mut [u8], at: usize, ux: f64, uy: f64| {
        if ux.abs() < 0.25 || uy.abs() < 0.25 {
            return;
        }
        let bits = if ux > 0.0 {
            if uy < 0.0 { 1 } else { 2 }
        } else if uy < 0.0 {
            4
        } else {
            8
        };
        taken[at] |= bits;
    };

    let mut links = String::new();
    for link in &model.links {
        let (Some(&from), Some(&to)) = (index_of.get(&link.source), index_of.get(&link.target))
        else {
            continue;
        };
        if from == to {
            continue;
        }
        if members
            .get(link.target.as_str())
            .is_some_and(|set| set.contains(link.source.as_str()))
        {
            continue;
        }
        let (fx, fy) = (placed[from].x, placed[from].y);
        let (tx, ty) = (placed[to].x, placed[to].y);
        let dx = tx - fx;
        let dy = ty - fy;
        let distance = dx.hypot(dy);
        if distance < 0.01 {
            continue;
        }
        let ux = dx / distance;
        let uy = dy / distance;
        let start = canvas.reach(&model.nodes[placed[from].node], ux, uy);
        let end = canvas.reach(&model.nodes[placed[to].node], ux, uy);
        if start + end >= distance {
            continue;
        }
        occupy(&mut taken, from, ux, uy);
        occupy(&mut taken, to, -ux, -uy);
        let mut x1 = fx + ux * start;
        let mut y1 = fy + uy * start;
        let mut x2 = tx - ux * end;
        let mut y2 = ty - uy * end;
        let mut heads = String::new();
        let trim = marker_trim("arrow_point");
        if matches!(link.flow, Some(Flow::Forward) | Some(Flow::Bidirectional)) {
            heads.push_str(&marker("arrow_point", x2, y2, ux, uy, LINE, "none"));
            x2 -= ux * trim;
            y2 -= uy * trim;
        }
        if matches!(link.flow, Some(Flow::Backward) | Some(Flow::Bidirectional)) {
            heads.push_str(&marker("arrow_point", x1, y1, -ux, -uy, LINE, "none"));
            x1 += ux * trim;
            y1 += uy * trim;
        }
        let mut caption = String::new();
        if let Some(raw) = link.label.as_deref().filter(|l| !l.is_empty()) {
            let label = canvas.text(raw, tiny + 1.0, 200.0, 0);
            let side = if ux >= 0.0 { 1.0 } else { -1.0 };
            let px = uy * side;
            let py = -ux * side;
            let away = 5.0 + px.abs() * label.width / 2.0 + py.abs() * label.height / 2.0;
            caption = canvas.centered(
                &label,
                (fx + tx) / 2.0 + px * away,
                (fy + ty) / 2.0 + py * away,
                &format!(
                    " class=\"pelers-edge-label\" font-size=\"{}\" fill=\"var(--_m)\"",
                    js_number_to_string(tiny + 1.0)
                ),
            );
        }
        links.push_str(&format!(
            "<g class=\"pelers-edge{}\" data-id=\"{}-{}\"><path d=\"M{},{}L{},{}\" stroke=\"{LINE}\"{}/>{heads}{caption}</g>",
            if link.dashed { " pelers-wardley-dashed" } else { "" },
            esc_text(&link.source),
            esc_text(&link.target),
            num(x1),
            num(y1),
            num(x2),
            num(y2),
            if link.dashed { " stroke-dasharray=\"6 6\"" } else { "" }
        ));
    }

    let mut trends = String::new();
    for trend in &model.trends {
        let Some(&origin) = index_of.get(&trend.node_id) else {
            continue;
        };
        let tx = canvas.project_x(trend.target_x);
        let ty = canvas.project_y(trend.target_y);
        let (ox, oy) = (placed[origin].x, placed[origin].y);
        let dx = tx - ox;
        let dy = ty - oy;
        let distance = dx.hypot(dy);
        let mut arrow = String::new();
        let start = if distance > 0.0 {
            canvas.reach(
                &model.nodes[placed[origin].node],
                dx / distance,
                dy / distance,
            )
        } else {
            0.0
        };
        if distance > start + R + 4.0 {
            let ux = dx / distance;
            let uy = dy / distance;
            let ex = tx - ux * (R + 2.0);
            let ey = ty - uy * (R + 2.0);
            let trim = marker_trim("arrow_point");
            arrow = format!(
                "<path d=\"M{},{}L{},{}\" stroke=\"{ACCENT}\" stroke-dasharray=\"4 4\"/>{}",
                num(ox + ux * start),
                num(oy + uy * start),
                num(ex - ux * trim),
                num(ey - uy * trim),
                marker("arrow_point", ex, ey, ux, uy, ACCENT, "none")
            );
        }
        canvas.bounds.grow(tx - R, ty - R, tx + R, ty + R);
        trends.push_str(&format!(
            "<g class=\"pelers-wardley-trend\" data-id=\"{}\">{arrow}<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"var(--_bg)\" stroke=\"{ACCENT}\"/></g>",
            esc_text(&trend.node_id),
            num(tx),
            num(ty)
        ));
    }

    let mut nodes = String::new();
    let tri = 1.2 * R;
    let dot = 0.7 * R;
    for (at_index, at) in placed.iter().enumerate() {
        let node = &model.nodes[at.node];
        let (x, y) = (at.x, at.y);
        let anchor = node.class_name.as_deref() == Some("anchor");
        let strategy = node.source_strategy.as_deref();
        let mut shape = String::new();
        if let Some(strategy) = strategy {
            let fill = match strategy {
                "outsource" => "var(--_m)",
                "buy" => "var(--_a)",
                "build" => "var(--_s)",
                _ => "var(--_bg)",
            };
            shape.push_str(&format!(
                "<circle class=\"pelers-wardley-{strategy}\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{fill}\" stroke=\"{LINE}\"/>",
                num(x),
                num(y),
                num(2.0 * R)
            ));
        }
        if strategy == Some("market") {
            let bx = tri * (std::f64::consts::PI / 6.0).cos();
            let by = y + tri * (std::f64::consts::PI / 6.0).sin();
            shape.push_str(&format!(
                "<path d=\"M{},{}L{},{}H{}Z\" fill=\"none\" stroke=\"{LINE}\"/>",
                num(x),
                num(y - tri),
                num(x - bx),
                num(by),
                num(x + bx)
            ));
            for (cx, cy) in [(x, y - tri), (x - bx, by), (x + bx, by)] {
                shape.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>",
                    num(cx),
                    num(cy),
                    num(dot)
                ));
            }
        } else if node.is_pipeline_parent {
            shape.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>",
                num(x - SQUARE / 2.0),
                num(y - SQUARE / 2.0),
                js_number_to_string(SQUARE),
                js_number_to_string(SQUARE)
            ));
        } else if !anchor {
            shape.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>",
                num(x),
                num(y)
            ));
        }
        if node.inertia {
            let offset = (if node.is_pipeline_parent {
                SQUARE / 2.0
            } else {
                R
            }) + 10.0
                + if strategy.is_some() { R + 4.0 } else { 0.0 };
            shape.push_str(&format!(
                "<rect class=\"pelers-wardley-inertia\" x=\"{}\" y=\"{}\" width=\"4\" height=\"12\" fill=\"{LINE}\"/>",
                num(x + offset),
                num(y - R)
            ));
        }
        let ox = node.label_offset_x.map(|v| clamp(v, -5000.0, 5000.0));
        let oy = node.label_offset_y.map(|v| clamp(v, -5000.0, 5000.0));
        let caption = if anchor {
            let label = canvas.anchor_text(&node.label);
            canvas.centered(
                &label,
                x + ox.unwrap_or(0.0),
                y + oy.unwrap_or(-3.0),
                &format!(
                    " class=\"pelers-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
                    js_number_to_string(small)
                ),
            )
        } else {
            let label = canvas.text_default(&node.label, small, 0);
            let attrs = format!(
                " class=\"pelers-label\" font-size=\"{}\"",
                js_number_to_string(small)
            );
            let lift = LABEL_OFFSET + if strategy.is_some() { 10.0 } else { 0.0 };
            if ox.is_some() || oy.is_some() {
                canvas.starting(
                    &label,
                    x + ox.unwrap_or(lift),
                    y + oy.unwrap_or(-lift),
                    &attrs,
                )
            } else if node.in_pipeline {
                canvas.centered(&label, x, y + 2.0 * R + 4.0 + label.height / 2.0, &attrs)
            } else {
                let busy = taken[at_index];
                let corner = if busy & 1 == 0 {
                    1
                } else if busy & 2 == 0 {
                    2
                } else if busy & 4 == 0 {
                    4
                } else if busy & 8 == 0 {
                    8
                } else {
                    1
                };
                let right = corner == 1 || corner == 2;
                let up = corner == 1 || corner == 4;
                canvas.starting(
                    &label,
                    if right {
                        x + lift
                    } else {
                        x - lift - label.width
                    },
                    if up {
                        y - lift
                    } else {
                        y + lift + small * 0.75
                    },
                    &attrs,
                )
            }
        };
        canvas
            .bounds
            .grow(x - 2.0 * R, y - 2.0 * R, x + 2.0 * R + 20.0, y + 2.0 * R);
        let class = if anchor {
            "anchor"
        } else if node.is_pipeline_parent {
            "pipeline-parent"
        } else if node.in_pipeline {
            "pipeline-component"
        } else {
            "component"
        };
        nodes.push_str(&format!(
            "<g class=\"pelers-node pelers-wardley-{class}\" data-id=\"{}\">{shape}{caption}</g>",
            esc_text(&node.id)
        ));
    }

    let mut annotations = String::new();
    for annotation in &model.annotations {
        let number = js_number_to_string(annotation.number);
        let label = canvas.text(&number, tiny, 4000.0, BOLD_STYLE);
        let mut marks = String::new();
        for &(cx, cy) in &annotation.coordinates {
            let x = canvas.project_x(cx);
            let y = canvas.project_y(cy);
            let r = js_max(10.0, label.width / 2.0 + 4.0);
            canvas.bounds.grow(x - r, y - r, x + r, y + r);
            marks.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_bg)\" stroke=\"{LINE}\"/>{}",
                num(x),
                num(y),
                num(r),
                canvas.label_svg(
                    &label,
                    x,
                    y,
                    &format!(
                        " font-size=\"{}\" font-weight=\"var(--_hw)\"",
                        js_number_to_string(tiny)
                    )
                )
            ));
        }
        annotations.push_str(&format!(
            "<g class=\"pelers-wardley-annotation\" data-id=\"{}\">{marks}</g>",
            esc_text(&number)
        ));
    }
    let mut listed: Vec<&super::model::WardleyAnnotation> = model
        .annotations
        .iter()
        .filter(|a| !a.text.is_empty())
        .collect();
    listed.sort_by(|a, b| {
        a.number
            .partial_cmp(&b.number)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if !listed.is_empty() {
        let lines: Vec<Label> = listed
            .iter()
            .map(|a| {
                canvas.text(
                    &format!("{}. {}", js_number_to_string(a.number), a.text),
                    small,
                    if tight {
                        js_max(80.0, plot_w - 20.0)
                    } else {
                        320.0
                    },
                    0,
                )
            })
            .collect();
        let mut width = 0.0f64;
        let mut height = 0.0f64;
        for line in &lines {
            width = js_max(width, line.width);
            height += line.height;
        }
        width += 20.0;
        height += 14.0;
        let (bx, by) = match model.annotations_box {
            Some((box_x, box_y)) => (
                clamp(canvas.project_x(box_x), 0.0, js_max(0.0, plot_w - width)),
                clamp(canvas.project_y(box_y), 0.0, js_max(0.0, plot_h - height)),
            ),
            None => (0.0, legend_top),
        };
        canvas.bounds.grow(bx, by, bx + width, by + height);
        let mut rows = String::new();
        let mut y = by + 7.0;
        for line in &lines {
            rows.push_str(&start_label(
                line,
                bx + 10.0,
                y + line.line_height / 2.0 + small * 0.35,
                &format!(" font-size=\"{}\"", js_number_to_string(small)),
            ));
            y += line.height;
        }
        annotations.push_str(&format!(
            "<g class=\"pelers-wardley-annotations\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"{ROUNDED}/>{rows}</g>",
            num(bx),
            num(by),
            num(width),
            num(height)
        ));
    }

    let mut notes = String::new();
    for note in &model.notes {
        let label = canvas.text(&note.text, small, js_min(360.0, plot_w * 0.6), BOLD_STYLE);
        let x = canvas.project_x(note.x);
        let y = canvas.project_y(note.y);
        notes.push_str(&canvas.starting(
            &label,
            x,
            y,
            &format!(
                " class=\"pelers-wardley-note\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
                js_number_to_string(small)
            ),
        ));
    }

    let mut forces = String::new();
    let force_list: Vec<(&str, f64, f64, bool)> = model
        .accelerators
        .iter()
        .map(|a| (a.name.as_str(), a.x, a.y, true))
        .chain(
            model
                .deaccelerators
                .iter()
                .map(|d| (d.name.as_str(), d.x, d.y, false)),
        )
        .collect();
    for (name, px, py, forward) in force_list {
        let x = canvas.project_x(px);
        let y = canvas.project_y(py);
        let half = FORCE_HEIGHT / 2.0;
        let tip = if forward { x + FORCE_WIDTH } else { x };
        let tail = if forward { x } else { x + FORCE_WIDTH };
        let neck = if forward {
            x + FORCE_WIDTH - FORCE_HEAD
        } else {
            x + FORCE_HEAD
        };
        let d = format!(
            "M{},{}H{}V{}L{},{}L{},{}V{}H{}Z",
            num(tail),
            num(y - half),
            num(neck),
            num(y - half - 8.0),
            num(tip),
            num(y),
            num(neck),
            num(y + half + 8.0),
            num(y + half),
            num(tail)
        );
        canvas
            .bounds
            .grow(x, y - half - 8.0, x + FORCE_WIDTH, y + half + 8.0);
        let label = canvas.text(name, tiny + 1.0, 160.0, BOLD_STYLE);
        let caption = canvas.centered(
            &label,
            x + FORCE_WIDTH / 2.0,
            y + half + 8.0 + 12.0,
            &format!(
                " font-size=\"{}\" font-weight=\"var(--_hw)\"",
                js_number_to_string(tiny + 1.0)
            ),
        );
        forces.push_str(&format!(
            "<g class=\"pelers-wardley-{}\" data-id=\"{}\"><path d=\"{d}\" fill=\"var(--_s)\" stroke=\"{LINE}\"/>{caption}</g>",
            if forward { "accelerator" } else { "deaccelerator" },
            esc_text(name)
        ));
    }

    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        if tight { js_max(80.0, plot_w) } else { 4000.0 },
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let bounds = canvas.bounds;
    let left = js_min(bounds.min_x, plot_w / 2.0 - title.width / 2.0);
    let right = js_max(bounds.max_x, plot_w / 2.0 + title.width / 2.0);
    let tx = pad - left;
    let ty = pad + title_height - bounds.min_y;
    let total_width = (right - left + 2.0 * pad).ceil();
    let total_height = (bounds.max_y - bounds.min_y + title_height + 2.0 * pad).ceil();
    let content = label_svg_with(
        &title,
        tx + plot_w / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
        canvas.icons,
    ) + &format!(
        "<g transform=\"translate({},{})\"><g class=\"pelers-wardley-frame\" fill=\"none\">{frame}</g>{stage_labels}{axis_labels}{pipelines}{}{}{nodes}{annotations}{notes}{forces}</g>",
        num(tx),
        num(ty),
        if links.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pelers-edges\" fill=\"none\">{links}</g>")
        },
        if trends.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pelers-wardley-trends\" fill=\"none\">{trends}</g>")
        }
    );

    Drawn {
        svg: svg_document(
            "wardley",
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
