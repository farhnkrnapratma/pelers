use super::db::{MindmapDb, MindmapNode};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{title_room, turn_to_fit};
use crate::diagrams::common::icon::icon_svg;
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg_with, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::shapes::{draw_shape, shape_inset, shape_size};
use crate::svg::theme::{class_names, series_color};
use crate::svg::title::title_label;
use crate::text::icons::icon_markup;
use crate::text::label::{Label, label_line, layout_label, line_middle};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::Value;

const SHAPES: [&str; 7] = ["", "rounded", "rect", "circle", "cloud", "bang", "hex"];
const ROOT_SHAPE: &str = "stadium";

const GAP_X: f64 = 36.0;
const GAP_Y: f64 = 8.0;
const GROUP_GAP: f64 = 18.0;
const BRANCH_GAP_X: f64 = 52.0;
const BRANCH_GAP_Y: f64 = 28.0;
const TEXT_PAD: f64 = 6.0;
const PILL_PAD: f64 = 12.0;
const TINT: f64 = 0.22;
const ICON_GAP: f64 = 6.0;
const INDENT: f64 = 28.0;
const STEM: f64 = 12.0;
const BEND: f64 = 8.0;
const ROW_GAP: f64 = 10.0;

fn left_label(label: &Label, left: f64, cy: f64, icons: bool) -> String {
    if label.lines.len() < 2 {
        return label_svg_with(
            label,
            left + label.width / 2.0,
            cy,
            " class=\"pelers-label\"",
            icons,
        );
    }
    let mut out = String::new();
    let top = cy - label.height / 2.0;
    for k in 0..label.lines.len() {
        out.push_str(&label_svg_with(
            &label_line(label, k),
            left + label.widths[k] / 2.0,
            top + line_middle(label, k),
            " class=\"pelers-label\"",
            icons,
        ));
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn stack(
    nodes: &[MindmapNode],
    list: &[usize],
    from: usize,
    to: usize,
    gap: f64,
    extent: &[f64],
    anchor: &[f64],
    own: &[f64],
    rel: &mut [f64],
) -> (f64, f64) {
    let mut y = 0.0;
    let mut last = 0.0;
    for k in from..to {
        let c = list[k];
        rel[c] = y;
        let grouped = !nodes[list[k]].children.is_empty()
            || (k + 1 < to && !nodes[list[k + 1]].children.is_empty());
        last = if grouped { js_max(gap, GROUP_GAP) } else { gap };
        y += extent[c] + last;
    }
    let stacked = y - last;
    let a = list[from];
    let b = list[to - 1];
    let middle = (rel[a] + own[a] + anchor[a] + rel[b] + own[b] + anchor[b]) / 2.0;
    (middle, stacked)
}

fn flush(
    out: &mut String,
    edges: &mut String,
    body: &mut String,
    color: &str,
    section: Option<usize>,
) {
    if body.is_empty() {
        return;
    }
    out.push_str(&format!(
        "<g class=\"pelers-branch pelers-section-{}\"><g class=\"pelers-edges\" fill=\"none\" stroke=\"{color}\" stroke-width=\"1.5\">{edges}</g>{body}</g>",
        section.unwrap_or(0)
    ));
    edges.clear();
    body.clear();
}

pub fn render_mindmap(model: &MindmapDb, config: &Config, options: &RenderOptions) -> Drawn {
    turn_to_fit(options, |down| draw(model, config, options, down))
}

fn draw(model: &MindmapDb, config: &Config, options: &RenderOptions, down: bool) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();
    let max_node_width = match config
        .get("mindmap")
        .and_then(Value::as_object)
        .and_then(|m| m.get("maxNodeWidth"))
    {
        Some(Value::Number(n)) => Some(*n),
        _ => None,
    };
    let wrap = match max_node_width {
        Some(w) if w > 0.0 => js_min(w, 2000.0),
        _ => 200.0,
    };
    let glyph = size * 1.25;

    let nodes = &model.nodes;
    let n = nodes.len();
    let mut parent = vec![0usize; n];
    let mut depth = vec![0f64; n];
    for i in 0..n {
        for &child in &nodes[i].children {
            parent[child] = i;
            depth[child] = depth[i] + 1.0;
        }
    }
    let room = match options.max_width {
        Some(max) if down && max > 0.0 => max - 2.0 * pad,
        _ => f64::INFINITY,
    };
    let mut labels: Vec<Label> = Vec::with_capacity(n);
    let mut shapes: Vec<&'static str> = Vec::with_capacity(n);
    let mut w = vec![0f64; n];
    let mut h = vec![0f64; n];
    let mut dy = vec![0f64; n];
    let mut anchor = vec![0f64; n];
    let mut drawn = vec![false; n];

    for i in 0..n {
        let node = &nodes[i];
        drawn[i] = node
            .icon
            .as_deref()
            .is_some_and(|icon| !icon_markup(icon).is_empty());
        let fit = js_min(
            wrap,
            js_max(
                72.0,
                room - depth[i] * INDENT
                    - 2.0 * TEXT_PAD
                    - if drawn[i] { glyph + ICON_GAP } else { 0.0 },
            ),
        );
        let label = layout_label(Some(&node.descr), true, size, fit, 0);
        let label_width = label.width;
        let label_height = label.height;
        labels.push(label);
        let tw = label_width
            + if drawn[i] {
                glyph + if label_width > 0.0 { ICON_GAP } else { 0.0 }
            } else {
                0.0
            };
        let th = js_max(label_height, js_round(size * 1.5));
        let mut shape = match SHAPES.get(node.kind as usize).copied().unwrap_or("") {
            "" if i == 0 => ROOT_SHAPE,
            other => other,
        };
        if down && i == 0 && shape != "rect" && shape != "rounded" {
            shape = ROOT_SHAPE;
        }
        shapes.push(shape);
        if shape.is_empty() {
            let pill = !node.children.is_empty();
            w[i] = tw + 2.0 * if pill { PILL_PAD } else { TEXT_PAD };
            h[i] = th + if pill { 6.0 } else { 0.0 };
            anchor[i] = h[i] / 2.0;
        } else {
            let s = shape_size(shape, tw, th);
            w[i] = s.w;
            h[i] = s.h;
            dy[i] = s.dy;
            anchor[i] = s.h / 2.0;
        }
    }

    let mut extent = vec![0f64; n];
    let mut rel = vec![0f64; n];
    let mut own = vec![0f64; n];
    let mut kids = vec![0f64; n];

    for i in (1..n).rev() {
        let children = nodes[i].children.clone();
        if children.is_empty() {
            extent[i] = h[i];
            continue;
        }
        let (middle, stacked) = stack(
            nodes,
            &children,
            0,
            children.len(),
            GAP_Y,
            &extent,
            &anchor,
            &own,
            &mut rel,
        );
        let top = middle - anchor[i];
        let low = js_min(0.0, top);
        extent[i] = js_max(stacked, top + h[i]) - low;
        own[i] = top - low;
        kids[i] = -low;
    }

    let mut x = vec![0f64; n];
    let mut y = vec![0f64; n];
    let mut band = vec![0f64; n];
    let mut side = vec![0i8; n];
    let mut trunk = vec![0f64; n];

    if down {
        let mut row = 0.0;
        for i in 0..n {
            if i > 0 && parent[i] == 0 {
                row += ROW_GAP;
            }
            x[i] = depth[i] * INDENT + w[i] / 2.0;
            y[i] = row;
            row += h[i] + ROW_GAP;
        }
    } else if n > 0 {
        let branches = nodes[0].children.clone();
        let count = branches.len();
        let total: f64 = branches.iter().map(|&b| extent[b]).sum();
        let mut split = count;
        let mut best = f64::INFINITY;
        let mut right = 0.0;
        for k in 1..count {
            right += extent[branches[k - 1]];
            let diff = (2.0 * right - total + (2.0 * k as f64 - count as f64) * BRANCH_GAP_Y).abs();
            if diff < best {
                best = diff;
                split = k;
            }
        }
        let mut from = 0usize;
        let mut to = split;
        let mut dir: i8 = 1;
        while from < count {
            let (middle, _) = stack(
                nodes,
                &branches,
                from,
                to,
                BRANCH_GAP_Y,
                &extent,
                &anchor,
                &own,
                &mut rel,
            );
            for &c in &branches[from..to] {
                band[c] = rel[c] - middle;
                side[c] = dir;
            }
            from = to;
            to = count;
            dir = -1;
        }
        y[0] = -h[0] / 2.0;
        for i in 0..n {
            if i > 0 {
                y[i] = band[i] + own[i];
            }
            for &c in &nodes[i].children.clone() {
                if i > 0 {
                    side[c] = side[i];
                    band[c] = band[i] + kids[i] + rel[c];
                }
                x[c] = x[i]
                    + side[c] as f64
                        * (w[i] / 2.0 + if i > 0 { GAP_X } else { BRANCH_GAP_X } + w[c] / 2.0);
            }
        }
    }

    let mut min_x = 0.0f64;
    let mut max_x = 0.0f64;
    let mut min_y = 0.0f64;
    let mut max_y = 0.0f64;
    let mut out = String::new();
    let mut edges = String::new();
    let mut body = String::new();
    let mut color = String::new();

    for k in 1..=n {
        let i = k % n;
        let node = &nodes[i];
        let shape = shapes[i];
        let label = &labels[i];
        let cx = x[i];
        let top = y[i];
        min_x = js_min(min_x, cx - w[i] / 2.0);
        max_x = js_max(max_x, cx + w[i] / 2.0);
        min_y = js_min(min_y, top);
        max_y = js_max(max_y, top + h[i]);

        if i == 0 || parent[i] == 0 {
            flush(
                &mut out,
                &mut edges,
                &mut body,
                &color,
                nodes[k - 1].section,
            );
        }
        if i > 0 && down {
            if parent[i] == 0 {
                color = series_color(node.section.unwrap_or(0));
            }
            let p = parent[i];
            let stem = if p == 0 {
                js_min(js_max(STEM, h[0] / 2.0), INDENT - BEND)
            } else {
                STEM
            };
            let x1 = x[p] - w[p] / 2.0 + js_min(stem, w[p] / 2.0);
            let y1 = if trunk[p] != 0.0 && !trunk[p].is_nan() {
                trunk[p]
            } else {
                y[p] + h[p]
            };
            let left = cx - w[i] / 2.0;
            let x2 = left
                + if shape.is_empty() {
                    0.0
                } else {
                    shape_inset(shape, w[i], h[i], 3, 0.0)
                };
            let y2 = top + anchor[i];
            let bend = js_max(0.0, js_min(js_min(BEND, x2 - x1), y2 - y1));
            trunk[p] = y2 - bend;
            edges.push_str(&format!(
                "<path class=\"pelers-edge\" d=\"M{},{}V{}Q{},{} {},{}H{}\"/>",
                num(x1),
                num(y1),
                num(y2 - bend),
                num(x1),
                num(y2),
                num(x1 + bend),
                num(y2),
                num(x2)
            ));
        } else if i > 0 {
            if parent[i] == 0 {
                color = series_color(node.section.unwrap_or(0));
            }
            let p = parent[i];
            let dir = side[i] as f64;
            let from: i32 = if side[i] > 0 { 1 } else { 3 };
            let x1 = x[p]
                + dir
                    * (w[p] / 2.0
                        - if shapes[p].is_empty() {
                            0.0
                        } else {
                            shape_inset(shapes[p], w[p], h[p], from, 0.0)
                        });
            let y1 = y[p] + anchor[p];
            let near = cx - (dir * w[i]) / 2.0;
            let x2 = near
                + if shape.is_empty() {
                    0.0
                } else {
                    dir * shape_inset(shape, w[i], h[i], 4 - from, 0.0)
                };
            let y2 = top + anchor[i];
            let mid = num((x1 + x2) / 2.0);
            edges.push_str(&format!(
                "<path class=\"pelers-edge\" d=\"M{},{}C{mid},{} {mid},{} {},{}\"/>",
                num(x1),
                num(y1),
                num(y1),
                num(y2),
                num(x2),
                num(y2)
            ));
        }

        let pill = shape.is_empty() && !node.children.is_empty();
        let tint = format!(
            " fill=\"{color}\" fill-opacity=\"{}\" stroke=\"{color}\"",
            js_number_to_string(TINT)
        );
        let mut inner = String::new();
        if !shape.is_empty() {
            inner = if i > 0 {
                draw_shape(shape, w[i], h[i], &tint, &format!(" stroke=\"{color}\""))
            } else {
                draw_shape(
                    shape,
                    w[i],
                    h[i],
                    " fill=\"var(--_s)\" stroke=\"var(--_b)\"",
                    " stroke=\"var(--_b)\"",
                )
            };
        } else if pill {
            inner = format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\"{tint}/>",
                num(-w[i] / 2.0),
                num(-h[i] / 2.0),
                num(w[i]),
                num(h[i]),
                num(h[i] / 2.0)
            );
        }
        let mut shift = 0.0;
        if let Some(icon) = node.icon.as_deref().filter(|_| drawn[i]) {
            shift = (glyph + if label.width > 0.0 { ICON_GAP } else { 0.0 }) / 2.0;
            inner.push_str(&icon_svg(
                icon,
                -shift - label.width / 2.0,
                dy[i] - glyph / 2.0,
                glyph,
            ));
        }
        let kind_class = if !shape.is_empty() {
            shape
        } else if pill {
            "branch"
        } else {
            "text"
        };
        let root_class = if i > 0 { "" } else { " pelers-root" };
        let group = format!(
            "<g class=\"pelers-node pelers-shape-{kind_class}{root_class}{}\" data-id=\"{}\" transform=\"translate({},{})\">{inner}{}</g>",
            class_names(node.class.as_deref().unwrap_or("")),
            esc_text(&node.node_id),
            num(cx),
            num(top + h[i] / 2.0),
            if down && shape.is_empty() {
                left_label(label, shift - label.width / 2.0, dy[i], icons)
            } else {
                label_svg_with(label, shift, dy[i], " class=\"pelers-label\"", icons)
            }
        );
        if i > 0 {
            body.push_str(&group);
        } else {
            out.push_str(&group);
        }
    }

    let title = title_label(model.title.as_deref(), size, title_room(options));
    let title_height = if title.height > 0.0 {
        title.height + 16.0
    } else {
        0.0
    };
    let width = js_max(max_x - min_x, title.width);
    let total_width = (width + 2.0 * pad).ceil();
    let total_height = (max_y - min_y + title_height + 2.0 * pad).ceil();
    let content = label_svg_with(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
        icons,
    ) + &format!(
        "<g transform=\"translate({},{})\">{out}</g>",
        num(pad - min_x + (width - max_x + min_x) / 2.0),
        num(pad + title_height - min_y)
    );

    Drawn {
        svg: svg_document(
            "mindmap",
            total_width,
            total_height,
            size,
            options,
            Accessible::default(),
            &content,
        ),
        width: total_width,
        height: total_height,
        links: Vec::new(),
    }
}
