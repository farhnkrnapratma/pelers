use std::collections::HashMap;

use super::db::{ArchitectureModel, NodeKind};
use super::layout::{place_on_grid, side_index};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{tighten, title_room};
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, label_svg_with, num};
use crate::svg::edges::{edge_label_svg, marker, marker_trim};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::ROUNDED;
use crate::text::label::{Label, Span, layout_label};
use crate::text::measurer::{BOLD_STYLE, width as measure};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::{Object, Value};

const STUB: f64 = 12.0;
const CORNER: f64 = 6.0;
const LABEL_GAP: f64 = 4.0;
const DOT: f64 = 3.0;
const DX: [f64; 4] = [-1.0, 1.0, 0.0, 0.0];
const DY: [f64; 4] = [0.0, 0.0, -1.0, 1.0];

fn section(config: &Config) -> Option<&Object> {
    config.get("architecture").and_then(Value::as_object)
}

fn setting(config: &Config, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match section(config).and_then(|s| s.get(key)) {
        Some(Value::Number(value)) if value.is_finite() => js_min(max, js_max(min, *value)),
        _ => fallback,
    }
}

fn rank(values: &[i32]) -> HashMap<i32, usize> {
    let mut sorted = values.to_vec();
    sorted.sort();
    let mut out: HashMap<i32, usize> = HashMap::new();
    for value in sorted {
        if !out.contains_key(&value) {
            let next = out.len();
            out.insert(value, next);
        }
    }
    out
}

const GLYPHS: [(&str, &str); 5] = [
    (
        "database",
        "M4 6a8 3 0 1 0 16 0a8 3 0 1 0-16 0M4 6v12a8 3 0 0 0 16 0V6M4 12a8 3 0 0 0 16 0",
    ),
    (
        "server",
        "M3 4h18v7H3zM3 13h18v7H3zM7 7.5h.01M7 16.5h.01M11 7.5h6M11 16.5h6",
    ),
    (
        "disk",
        "M5 3h14v18H5zM12 6.5a4 4 0 1 0 0 8a4 4 0 1 0 0-8M12 10.5h.01M8 18h3",
    ),
    (
        "internet",
        "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M3 12h18M12 3a13 13 0 0 0 0 18M12 3a13 13 0 0 1 0 18",
    ),
    (
        "cloud",
        "M7 18a4 4 0 0 1-.5-7.97A6 6 0 0 1 18 9.5a4.25 4.25 0 0 1-.5 8.5z",
    ),
];

fn builtin_icon(name: &str) -> String {
    match GLYPHS.iter().find(|(n, _)| *n == name) {
        Some((_, d)) => format!("<path d=\"{d}\" stroke=\"var(--_fg)\" stroke-width=\"1.5\"/>"),
        None => String::new(),
    }
}

fn icon_slot(name: &str, x: f64, y: f64, side: f64, options: &RenderOptions) -> String {
    let drawn = match options.icons.as_ref().and_then(|hook| (hook.0)(name)) {
        Some(markup) => markup,
        None => builtin_icon(name),
    };
    format!(
        "<svg class=\"pelers-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{drawn}</svg>",
        esc_text(name),
        num(x),
        num(y),
        num(side),
        num(side)
    )
}

fn fit(label: &Label, max_width: f64, max_lines: usize) -> Label {
    let cut = label.lines.len() > max_lines;
    if !cut && label.width <= max_width {
        return label.clone();
    }
    let dots = measure("…", label.size, 0);
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut widths: Vec<f64> = Vec::new();
    let mut widest = 0.0f64;
    for i in 0..js_min_usize(max_lines, label.lines.len()) {
        let mut line: Vec<Span> = label.lines[i].clone();
        let mut width = label.widths[i];
        if width > max_width || (cut && i == max_lines - 1) {
            loop {
                let Some(span) = line.last_mut() else {
                    line.push(Span {
                        text: "…".to_owned(),
                        style: 0,
                        icon: None,
                        math: None,
                        width: dots,
                    });
                    width = dots;
                    break;
                };
                let rest = width - span.width;
                if span.icon.is_none() && rest + dots < max_width {
                    let units: Vec<u16> = span.text.encode_utf16().collect();
                    let mut lo = 0usize;
                    let mut hi = units.len();
                    while lo < hi {
                        let mid = (lo + hi).div_ceil(2);
                        let piece = String::from_utf16_lossy(&units[..mid]);
                        if rest + measure(&piece, label.size, span.style) + dots <= max_width {
                            lo = mid;
                        } else {
                            hi = mid - 1;
                        }
                    }
                    let code = if lo > 0 { units[lo - 1] } else { 0 };
                    if (0xd800..=0xdbff).contains(&code) {
                        lo -= 1;
                    }
                    let kept = String::from_utf16_lossy(&units[..lo]);
                    span.text = format!(
                        "{}…",
                        kept.trim_end_matches(crate::util::chars::is_js_space)
                    );
                    span.width = measure(&span.text, label.size, span.style);
                    width = rest + span.width;
                    break;
                }
                line.pop();
                width = rest;
            }
        }
        lines.push(line);
        widths.push(width);
        widest = js_max(widest, width);
    }
    Label {
        lines: lines.clone(),
        widths,
        width: widest,
        height: lines.len() as f64 * label.line_height,
        ..label.clone()
    }
}

fn js_min_usize(a: usize, b: usize) -> usize {
    a.min(b)
}

#[derive(Clone, Copy)]
struct Port {
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    gap: f64,
    wire: bool,
}

fn route(s: Port, e: Port) -> Vec<f64> {
    let (sx, sy, sdx, sdy) = (s.x, s.y, s.dx, s.dy);
    let (ex, ey, edx, edy) = (e.x, e.y, e.dx, e.dy);
    let s1x = sx + sdx * STUB;
    let s1y = sy + sdy * STUB;
    let e1x = ex + edx * STUB;
    let e1y = ey + edy * STUB;
    if sdx * edx + sdy * edy == 0.0 {
        let cx = if sdx != 0.0 { ex } else { sx };
        let cy = if sdx != 0.0 { sy } else { ey };
        if (cx - sx) * sdx + (cy - sy) * sdy > 0.0 && (cx - ex) * edx + (cy - ey) * edy > 0.0 {
            return vec![sx, sy, cx, cy, ex, ey];
        }
        return if sdx != 0.0 {
            vec![sx, sy, s1x, s1y, s1x, e1y, e1x, e1y, ex, ey]
        } else {
            vec![sx, sy, s1x, s1y, e1x, s1y, e1x, e1y, ex, ey]
        };
    }
    let sideways = sdx != 0.0;
    let offset = if sideways { ey - sy } else { ex - sx };
    if sdx == -edx && sdy == -edy {
        let ahead = (ex - sx) * sdx + (ey - sy) * sdy;
        if ahead > 0.0 {
            if offset.abs() < 0.5 {
                return vec![sx, sy, ex, ey];
            }
            if sideways {
                let lo = if sdx > 0.0 { s.x1 } else { e.x1 };
                let hi = if sdx > 0.0 { e.x0 } else { s.x0 };
                let mx = if lo < hi {
                    (lo + hi) / 2.0
                } else {
                    (sx + ex) / 2.0
                };
                return vec![sx, sy, mx, sy, mx, ey, ex, ey];
            }
            let lo = if sdy > 0.0 { s.y1 } else { e.y1 };
            let hi = if sdy > 0.0 { e.y0 } else { s.y0 };
            let my = if lo < hi {
                (lo + hi) / 2.0
            } else {
                (sy + ey) / 2.0
            };
            return vec![sx, sy, sx, my, ex, my, ex, ey];
        }
    } else if offset.abs() >= 1.0 {
        if sideways {
            let x = if sdx > 0.0 {
                js_max(sx, ex) + STUB
            } else {
                js_min(sx, ex) - STUB
            };
            return vec![sx, sy, x, sy, x, ey, ex, ey];
        }
        let y = if sdy > 0.0 {
            js_max(sy, ey) + STUB
        } else {
            js_min(sy, ey) - STUB
        };
        return vec![sx, sy, sx, y, ex, y, ex, ey];
    }
    if sideways {
        let lane = if s.y1 + STUB <= e.y0 {
            (s.y1 + e.y0) / 2.0
        } else if e.y1 + STUB <= s.y0 {
            (e.y1 + s.y0) / 2.0
        } else {
            js_max(s.y1, e.y1) + STUB
        };
        return vec![sx, sy, s1x, sy, s1x, lane, e1x, lane, e1x, ey, ex, ey];
    }
    let lane = if s.x1 + STUB <= e.x0 {
        (s.x1 + e.x0) / 2.0
    } else if e.x1 + STUB <= s.x0 {
        (e.x1 + s.x0) / 2.0
    } else {
        js_max(s.x1, e.x1) + STUB
    };
    vec![sx, sy, sx, s1y, lane, s1y, lane, e1y, ex, e1y, ex, ey]
}

fn simplify(p: &[f64]) -> Vec<f64> {
    let mut out = vec![p[0], p[1]];
    let mut i = 2;
    while i < p.len() {
        let x = p[i];
        let y = p[i + 1];
        let n = out.len();
        let same_x = (x - out[n - 2]).abs() < 0.01;
        let same_y = (y - out[n - 1]).abs() < 0.01;
        i += 2;
        if same_x && same_y {
            continue;
        }
        if n >= 4
            && ((same_x && (out[n - 4] - out[n - 2]).abs() < 0.01)
                || (same_y && (out[n - 3] - out[n - 1]).abs() < 0.01))
        {
            out[n - 2] = x;
            out[n - 1] = y;
        } else {
            out.push(x);
            out.push(y);
        }
    }
    out
}

fn path_data(p: &[f64], start_trim: f64, end_trim: f64) -> String {
    let n = p.len() / 2;
    let mut xs: Vec<f64> = (0..n).map(|i| p[i * 2]).collect();
    let mut ys: Vec<f64> = (0..n).map(|i| p[i * 2 + 1]).collect();
    let trim = |xs: &mut [f64], ys: &mut [f64], at: usize, toward: usize, amount: f64| {
        let len = js_hypot_pair(xs[toward] - xs[at], ys[toward] - ys[at]);
        let t = js_min(amount, js_max(0.0, len - 1.0));
        if len > 0.0 && t > 0.0 {
            xs[at] += ((xs[toward] - xs[at]) / len) * t;
            ys[at] += ((ys[toward] - ys[at]) / len) * t;
        }
    };
    trim(&mut xs, &mut ys, 0, 1, start_trim);
    trim(&mut xs, &mut ys, n - 1, n - 2, end_trim);
    let mut d = format!("M{},{}", num(xs[0]), num(ys[0]));
    for i in 1..n - 1 {
        let before = js_hypot_pair(xs[i] - xs[i - 1], ys[i] - ys[i - 1]);
        let after = js_hypot_pair(xs[i + 1] - xs[i], ys[i + 1] - ys[i]);
        let r = js_min(CORNER, js_min(before / 2.0, after / 2.0));
        if r < 0.5 {
            d.push_str(&format!("L{},{}", num(xs[i]), num(ys[i])));
            continue;
        }
        let in_x = xs[i] - ((xs[i] - xs[i - 1]) / before) * r;
        let in_y = ys[i] - ((ys[i] - ys[i - 1]) / before) * r;
        let out_x = xs[i] + ((xs[i + 1] - xs[i]) / after) * r;
        let out_y = ys[i] + ((ys[i + 1] - ys[i]) / after) * r;
        d.push_str(&format!(
            "L{},{}Q{},{} {},{}",
            num(in_x),
            num(in_y),
            num(xs[i]),
            num(ys[i]),
            num(out_x),
            num(out_y)
        ));
    }
    d.push_str(&format!("L{},{}", num(xs[n - 1]), num(ys[n - 1])));
    d
}

fn js_hypot_pair(a: f64, b: f64) -> f64 {
    crate::util::jsnum::js_hypot(a, b)
}

pub fn render_architecture(
    model: &ArchitectureModel,
    config: &Config,
    options: &RenderOptions,
) -> Drawn {
    tighten(options, |tight| draw(model, config, options, tight))
}

fn draw(model: &ArchitectureModel, config: &Config, options: &RenderOptions, tight: bool) -> Drawn {
    let icon_size = setting(config, "iconSize", 48.0, 8.0, 512.0);
    let size = setting(
        config,
        "fontSize",
        options.font_size.unwrap_or(16.0),
        4.0,
        128.0,
    );
    let group_pad = js_min(
        setting(config, "padding", 20.0, 0.0, 256.0),
        if tight { 12.0 } else { f64::INFINITY },
    );
    let small = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);
    let half = icon_size / 2.0;
    let icons = options.icons.is_some();

    let nodes = &model.nodes;
    let groups = &model.groups;
    let n = nodes.len();
    let g_count = groups.len();
    let grid = place_on_grid(nodes, groups, &model.edges, &model.layout_hints);
    let mut index: HashMap<&str, usize> = HashMap::new();
    for (i, node) in nodes.iter().enumerate() {
        index.insert(node.id.as_str(), i);
    }

    let wrap = if tight {
        js_max(icon_size, 64.0)
    } else {
        js_max(icon_size * 2.25, 96.0)
    };
    let mut labels: Vec<Label> = Vec::with_capacity(n);
    let mut box_w = vec![0.0f64; n];
    let mut box_h = vec![0.0f64; n];
    for (i, node) in nodes.iter().enumerate() {
        let title = if node.kind == NodeKind::Service {
            node.title.as_deref()
        } else {
            None
        };
        let label = layout_label(title, true, size, wrap, 0);
        box_w[i] = js_max(icon_size, 2.0 * (label.width / 2.0).ceil());
        box_h[i] = icon_size
            + if label.height > 0.0 {
                LABEL_GAP + label.height
            } else {
                0.0
            };
        labels.push(label);
    }

    let mut col_values: Vec<i32> = Vec::new();
    let mut row_values: Vec<i32> = Vec::new();
    for i in 0..n {
        col_values.push(grid.col[i]);
        row_values.push(grid.row[i]);
    }
    for g in 0..g_count {
        if !grid.group_empty[g] {
            continue;
        }
        col_values.push(grid.group_col[g]);
        row_values.push(grid.group_row[g]);
    }
    let col_of = rank(&col_values);
    let row_of = rank(&row_values);
    let c_count = col_of.len();
    let r_count = row_of.len();
    let mut col_w = vec![0.0f64; c_count];
    let mut row_h = vec![0.0f64; r_count];
    let gap = if tight {
        js_max(24.0, 2.0 * STUB)
    } else {
        js_max(40.0, js_round(icon_size * 0.75))
    };
    let mut gap_x = vec![gap; c_count];
    let mut gap_y = vec![gap; r_count];
    let mut node_col = vec![0usize; n];
    let mut node_row = vec![0usize; n];
    for i in 0..n {
        let c = col_of[&grid.col[i]];
        let r = row_of[&grid.row[i]];
        node_col[i] = c;
        node_row[i] = r;
        col_w[c] = js_max(col_w[c], box_w[i]);
        row_h[r] = js_max(row_h[r], box_h[i]);
    }
    for g in 0..g_count {
        if !grid.group_empty[g] {
            continue;
        }
        let c = col_of[&grid.group_col[g]];
        let r = row_of[&grid.group_row[g]];
        col_w[c] = js_max(col_w[c], icon_size);
        row_h[r] = js_max(row_h[r], half);
    }

    let e_count = model.edges.len();
    let mut edge_labels: Vec<Label> = Vec::with_capacity(e_count);
    let mut ends: Vec<(i64, i64)> = Vec::with_capacity(e_count);
    for edge in &model.edges {
        let a = index.get(edge.lhs_id.as_str()).map_or(-1, |&v| v as i64);
        let b = index.get(edge.rhs_id.as_str()).map_or(-1, |&v| v as i64);
        ends.push((a, b));
        let label = layout_label(edge.title.as_deref(), true, small, 160.0, 0);
        if label.width != 0.0 && a >= 0 && b >= 0 {
            let (a, b) = (a as usize, b as usize);
            if node_col[a] != node_col[b] {
                let c = js_min(node_col[a] as f64, node_col[b] as f64) as usize;
                gap_x[c] = js_max(gap_x[c], label.width.ceil() + 28.0);
            } else {
                let c = node_col[a];
                col_w[c] = js_max(col_w[c], 2.0 * (label.width / 2.0).ceil() + 8.0);
                if node_row[a] != node_row[b] {
                    let r = js_min(node_row[a] as f64, node_row[b] as f64) as usize;
                    gap_y[r] = js_max(gap_y[r], label.height.ceil() + 24.0);
                }
            }
        }
        edge_labels.push(label);
    }

    let badge = js_round(size * 1.25);
    let mut group_labels: Vec<Label> = Vec::with_capacity(g_count);
    let mut c0 = vec![0usize; g_count];
    let mut c1 = vec![0usize; g_count];
    let mut r0 = vec![0usize; g_count];
    let mut r1 = vec![0usize; g_count];
    let mut ins_l = vec![0.0f64; g_count];
    let mut ins_r = vec![0.0f64; g_count];
    let mut ins_t = vec![0.0f64; g_count];
    let mut ins_b = vec![0.0f64; g_count];
    let mut head_w = vec![0.0f64; g_count];
    let mut head_h = vec![0.0f64; g_count];
    let mut col_l = vec![0.0f64; c_count];
    let mut col_r = vec![0.0f64; c_count];
    let mut row_t = vec![0.0f64; r_count];
    let mut row_b = vec![0.0f64; r_count];
    let lookup_col = |value: i32| col_of.get(&value).copied().unwrap_or(0);
    let lookup_row = |value: i32| row_of.get(&value).copied().unwrap_or(0);
    for g in 0..g_count {
        c0[g] = lookup_col(grid.group_col[g]);
        c1[g] = lookup_col(grid.group_col[g] + grid.group_cols[g] - 1);
        r0[g] = lookup_row(grid.group_row[g]);
        r1[g] = lookup_row(grid.group_row[g] + grid.group_rows[g] - 1);
        let label = layout_label(
            groups[g].title.as_deref(),
            true,
            size,
            if tight {
                js_max(80.0, title_room(options) / 2.0)
            } else {
                320.0
            },
            0,
        );
        let icon = if groups[g].icon.as_deref().is_some_and(|s| !s.is_empty()) {
            badge
        } else {
            0.0
        };
        head_w[g] = label.width
            + icon
            + if label.width > 0.0 && icon > 0.0 {
                6.0
            } else {
                0.0
            };
        head_h[g] = js_max(label.height, icon);
        group_labels.push(label);
    }
    for g in (0..g_count).rev() {
        let left = group_pad + ins_l[g];
        let mut right = group_pad + ins_r[g];
        let top = (if head_h[g] > 0.0 {
            js_max(group_pad, head_h[g] + 12.0)
        } else {
            group_pad
        }) + ins_t[g];
        let bottom = group_pad + ins_b[g];
        if head_w[g] > 0.0 {
            let mut inner = left + right;
            for c in c0[g]..=c1[g] {
                inner += col_w[c]
                    + if c < c1[g] {
                        col_r[c] + gap_x[c] + col_l[c + 1]
                    } else {
                        0.0
                    };
            }
            if head_w[g] + 20.0 > inner {
                right += (head_w[g] + 20.0 - inner).ceil();
            }
        }
        ins_l[g] = left;
        ins_r[g] = right;
        ins_t[g] = top;
        ins_b[g] = bottom;
        col_l[c0[g]] = js_max(col_l[c0[g]], left);
        col_r[c1[g]] = js_max(col_r[c1[g]], right);
        row_t[r0[g]] = js_max(row_t[r0[g]], top);
        row_b[r1[g]] = js_max(row_b[r1[g]], bottom);
        let p_raw = grid.group_parent[g];
        if p_raw < 0 {
            continue;
        }
        let p = p_raw as usize;
        if c0[p] == c0[g] {
            ins_l[p] = js_max(ins_l[p], left);
        }
        if c1[p] == c1[g] {
            ins_r[p] = js_max(ins_r[p], right);
        }
        if r0[p] == r0[g] {
            ins_t[p] = js_max(ins_t[p], top);
        }
        if r1[p] == r1[g] {
            ins_b[p] = js_max(ins_b[p], bottom);
        }
    }

    let mut xs = vec![0.0f64; c_count];
    let mut ys = vec![0.0f64; r_count];
    let mut inner_w = 0.0f64;
    for c in 0..c_count {
        xs[c] = inner_w + col_l[c];
        inner_w = xs[c] + col_w[c] + col_r[c] + if c + 1 < c_count { gap_x[c] } else { 0.0 };
    }
    let mut inner_h = 0.0f64;
    for r in 0..r_count {
        ys[r] = inner_h + row_t[r];
        inner_h = ys[r] + row_h[r] + row_b[r] + if r + 1 < r_count { gap_y[r] } else { 0.0 };
    }
    let mut min_x = 0.0f64;
    let mut min_y = 0.0f64;
    let mut max_x = inner_w;
    let mut max_y = inner_h;

    let left_of = |g: usize| xs[c0[g]] - ins_l[g];
    let right_of = |g: usize| xs[c1[g]] + col_w[c1[g]] + ins_r[g];
    let top_of = |g: usize| ys[r0[g]] - ins_t[g];
    let bottom_of = |g: usize| ys[r1[g]] + row_h[r1[g]] + ins_b[g];
    let center_x = |i: usize| xs[node_col[i]] + col_w[node_col[i]] / 2.0;

    let port = |i: usize, side: usize, via_group: bool| -> Port {
        let dx = DX[side];
        let dy = DY[side];
        let cx = center_x(i);
        let y = ys[node_row[i]];
        let is_junction = nodes[i].kind == NodeKind::Junction;
        let mut p = Port {
            x: cx,
            y: y + half,
            dx,
            dy,
            x0: cx - box_w[i] / 2.0,
            y0: y,
            x1: cx + box_w[i] / 2.0,
            y1: y + box_h[i],
            gap: 0.0,
            wire: false,
        };
        if is_junction {
            p.x0 = cx;
            p.x1 = cx;
            p.y0 = p.y;
            p.y1 = p.y;
            p.gap = DOT;
        } else if dx != 0.0 {
            p.x = cx + dx * half;
        } else {
            p.y = if dy < 0.0 {
                y
            } else {
                y + box_h[i] + if labels[i].height > 0.0 { 2.0 } else { 0.0 }
            };
        }
        let g = if via_group { grid.node_parent[i] } else { -1 };
        if g >= 0 {
            let g = g as usize;
            p.x0 = left_of(g);
            p.x1 = right_of(g);
            p.y0 = top_of(g);
            p.y1 = bottom_of(g);
            p.gap = 0.0;
            p.wire = is_junction;
            if dx != 0.0 {
                p.x = if dx < 0.0 { p.x0 } else { p.x1 };
            } else {
                p.y = if dy < 0.0 { p.y0 } else { p.y1 };
            }
        }
        p
    };

    let mut edges_out = String::new();
    let mut labels_out = String::new();
    let mut runs: Vec<f64> = Vec::new();
    let trim_length = marker_trim("arrow_point");
    for k in 0..e_count {
        let edge = &model.edges[k];
        let (a_raw, b_raw) = ends[k];
        if a_raw < 0 || b_raw < 0 {
            continue;
        }
        let a = a_raw as usize;
        let b = b_raw as usize;
        let mut s = port(a, side_index(edge.lhs_dir), edge.lhs_group);
        let mut e = port(b, side_index(edge.rhs_dir), edge.rhs_group);
        if s.x == e.x && s.y == e.y && s.dx == e.dx {
            s.x -= 8.0 * s.dy.abs();
            s.y -= 8.0 * s.dx.abs();
            e.x += 8.0 * s.dy.abs();
            e.y += 8.0 * s.dx.abs();
        }
        let mut routed = route(s, e);
        if s.wire {
            routed.splice(0..0, [center_x(a), ys[node_row[a]] + half]);
        }
        if e.wire {
            routed.push(center_x(b));
            routed.push(ys[node_row[b]] + half);
        }
        let points = simplify(&routed);
        if points.len() < 4 {
            continue;
        }
        let id = esc_text(&format!("{}-{}", edge.lhs_id, edge.rhs_id));
        let start_trim = if edge.lhs_into && !s.wire {
            trim_length + s.gap
        } else {
            0.0
        };
        let end_trim = if edge.rhs_into && !e.wire {
            trim_length + e.gap
        } else {
            0.0
        };
        edges_out.push_str(&format!(
            "<g class=\"pelers-edge\" data-id=\"{id}\"><path d=\"{}\"/>{}{}</g>",
            path_data(&points, start_trim, end_trim),
            if edge.lhs_into {
                marker(
                    "arrow_point",
                    s.x + s.dx * s.gap,
                    s.y + s.dy * s.gap,
                    -s.dx,
                    -s.dy,
                    "var(--_l)",
                    "none",
                )
            } else {
                String::new()
            },
            if edge.rhs_into {
                marker(
                    "arrow_point",
                    e.x + e.dx * e.gap,
                    e.y + e.dy * e.gap,
                    -e.dx,
                    -e.dy,
                    "var(--_l)",
                    "none",
                )
            } else {
                String::new()
            }
        ));

        let mut best: i64 = -1;
        let mut best_length = -1.0f64;
        let mut flat: i64 = -1;
        let mut flat_length = -1.0f64;
        let label = &edge_labels[k];
        let mut i = 0;
        while i < points.len() {
            min_x = js_min(min_x, points[i] - 4.0);
            max_x = js_max(max_x, points[i] + 4.0);
            min_y = js_min(min_y, points[i + 1] - 4.0);
            max_y = js_max(max_y, points[i + 1] + 4.0);
            i += 2;
        }
        let mut i = 0;
        while i + 3 < points.len() {
            let horizontal = (points[i + 1] - points[i + 3]).abs() < 0.01;
            let length = (points[i] - points[i + 2]).abs() + (points[i + 1] - points[i + 3]).abs();
            if !horizontal {
                runs.push(points[i]);
                runs.push(js_min(points[i + 1], points[i + 3]));
                runs.push(js_max(points[i + 1], points[i + 3]));
            }
            if length > best_length {
                best = i as i64;
                best_length = length;
            }
            if horizontal && length > flat_length {
                flat = i as i64;
                flat_length = length;
            }
            i += 2;
        }
        if label.width > 0.0 {
            let at = if flat_length >= label.width + 8.0 {
                flat
            } else {
                best
            } as usize;
            let mut x = (points[at] + points[at + 2]) / 2.0;
            let mut y = (points[at + 1] + points[at + 3]) / 2.0;
            let w = label.width + 8.0;
            let upright = (points[at] - points[at + 2]).abs() < 0.01;
            for p in [s, e] {
                if x + w / 2.0 <= p.x0
                    || x - w / 2.0 >= p.x1
                    || y + label.height / 2.0 <= p.y0
                    || y - label.height / 2.0 >= p.y1
                {
                    continue;
                }
                if upright {
                    x = if x >= (p.x0 + p.x1) / 2.0 {
                        p.x1 + w / 2.0 + 4.0
                    } else {
                        p.x0 - w / 2.0 - 4.0
                    };
                } else {
                    y = if y >= (p.y0 + p.y1) / 2.0 {
                        p.y1 + label.height / 2.0 + 4.0
                    } else {
                        p.y0 - label.height / 2.0 - 4.0
                    };
                }
            }
            min_x = js_min(min_x, x - w / 2.0);
            max_x = js_max(max_x, x + w / 2.0);
            min_y = js_min(min_y, y - label.height / 2.0);
            max_y = js_max(max_y, y + label.height / 2.0);
            labels_out.push_str(&edge_label_svg(
                &format!("{}-{}", edge.lhs_id, edge.rhs_id),
                label,
                x,
                y,
                "",
                icons,
            ));
        }
    }

    let mut clusters = String::new();
    for g in 0..g_count {
        let group = &groups[g];
        let x = left_of(g);
        let y = top_of(g);
        let w = right_of(g) - x;
        let label = &group_labels[g];
        let mut head = String::new();
        if head_w[g] > 0.0 {
            let band = y + head_h[g] + 12.0;
            let mut crossings: Vec<f64> = Vec::new();
            let mut i = 0;
            while i < runs.len() {
                if runs[i + 1] < band && runs[i + 2] > y && runs[i] > x && runs[i] < x + w {
                    crossings.push(runs[i]);
                }
                i += 3;
            }
            crossings.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            crossings.push(x + w - 4.0);
            let mut start = x + 10.0;
            for crossing in crossings {
                if crossing - 6.0 - start >= head_w[g] {
                    break;
                }
                start = js_max(start, crossing + 6.0);
            }
            if start + head_w[g] > x + w - 4.0 {
                start = x + 10.0;
            }
            let cy = y + 6.0 + head_h[g] / 2.0;
            let mut text_x = start;
            if let Some(icon) = group.icon.as_deref().filter(|s| !s.is_empty()) {
                head.push_str(&icon_slot(icon, start, cy - badge / 2.0, badge, options));
                text_x += badge + 6.0;
            }
            head.push_str(&label_svg_with(
                label,
                text_x + label.width / 2.0,
                cy,
                " class=\"pelers-cluster-label\" fill=\"var(--_m)\"",
                icons,
            ));
        }
        clusters.push_str(&format!(
            "<g class=\"pelers-cluster\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\" fill-opacity=\"0.5\" stroke=\"var(--_b)\"{ROUNDED}/>{head}</g>",
            esc_text(&group.id),
            num(x),
            num(y),
            num(w),
            num(bottom_of(g) - y)
        ));
    }

    let mut nodes_out = String::new();
    let glyph = icon_size * 0.5;
    let text_size = js_max(6.0, js_round(size * 0.75));
    for i in 0..n {
        let node = &nodes[i];
        let at = format!(
            "data-id=\"{}\" transform=\"translate({},{})\"",
            esc_text(&node.id),
            num(center_x(i)),
            num(ys[node_row[i]])
        );
        if node.kind == NodeKind::Junction {
            nodes_out.push_str(&format!(
                "<g class=\"pelers-node pelers-junction\" {at}><circle cy=\"{}\" r=\"{}\" fill=\"var(--_l)\"/></g>",
                num(half),
                js_number_to_string(DOT)
            ));
            continue;
        }
        let mut inner = String::new();
        if let Some(icon) = node.icon.as_deref().filter(|s| !s.is_empty()) {
            inner = icon_slot(
                icon,
                -glyph / 2.0,
                (icon_size - glyph) / 2.0,
                glyph,
                options,
            );
        } else if let Some(icon_text) = node.icon_text.as_deref().filter(|s| !s.is_empty()) {
            let text = layout_label(Some(icon_text), false, text_size, icon_size - 8.0, 0);
            let lines = js_max(
                1.0,
                js_round_floor(
                    (icon_size - 4.0)
                        / if text.line_height != 0.0 {
                            text.line_height
                        } else {
                            1.0
                        },
                ),
            );
            inner = label_svg_with(
                &fit(&text, icon_size - 8.0, lines as usize),
                0.0,
                half,
                &format!(
                    " class=\"pelers-icon-text\" font-size=\"{}\"",
                    js_number_to_string(text_size)
                ),
                icons,
            );
        }
        nodes_out.push_str(&format!(
            "<g class=\"pelers-node pelers-service\" {at}><rect x=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{ROUNDED}/>{inner}{}</g>",
            num(-half),
            num(icon_size),
            num(icon_size),
            label_svg_with(
                &labels[i],
                0.0,
                icon_size + LABEL_GAP + labels[i].height / 2.0,
                " class=\"pelers-label\"",
                icons
            )
        ));
    }

    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        if tight { title_room(options) } else { 4000.0 },
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let width = js_max(max_x - min_x, title.width) + 2.0 * pad;
    let width = width.ceil();
    let height = (max_y - min_y + title_height + 2.0 * pad).ceil();
    let ox = js_round((width - (max_x - min_x)) / 2.0 - min_x);
    let oy = js_round(pad + title_height - min_y);
    let mut body = String::new();
    if !clusters.is_empty() {
        body.push_str(&format!("<g class=\"pelers-clusters\">{clusters}</g>"));
    }
    if !edges_out.is_empty() {
        body.push_str(&format!(
            "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{edges_out}</g>"
        ));
    }
    if !labels_out.is_empty() {
        body.push_str(&format!(
            "<g class=\"pelers-edge-labels\" font-size=\"{}\">{labels_out}</g>",
            js_number_to_string(small)
        ));
    }
    if !nodes_out.is_empty() {
        body.push_str(&format!("<g class=\"pelers-nodes\">{nodes_out}</g>"));
    }
    let mut content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    if !body.is_empty() {
        content.push_str(&format!(
            "<g transform=\"translate({},{})\">{body}</g>",
            js_number_to_string(ox),
            js_number_to_string(oy)
        ));
    }
    Drawn {
        svg: svg_document(
            "architecture",
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

fn js_round_floor(value: f64) -> f64 {
    value.floor()
}
