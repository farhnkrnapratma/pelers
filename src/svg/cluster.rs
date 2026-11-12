use std::collections::HashMap;

use crate::layout::CNode;
use crate::svg::builder::{label_svg, num};
use crate::text::label::Label;
use crate::util::jsnum::{js_min, js_round};

pub type Crossings = HashMap<u64, Vec<f64>>;

fn key(value: f64) -> u64 {
    let rounded = js_round(value);
    if rounded.is_nan() {
        f64::NAN.to_bits()
    } else if rounded == 0.0 {
        0.0f64.to_bits()
    } else {
        rounded.to_bits()
    }
}

pub fn mark_crossings(crossings: &mut Crossings, route: &[f64]) {
    for k in (0..route.len()).step_by(3) {
        crossings
            .entry(key(route[k + 1]))
            .or_default()
            .push(route[k]);
    }
}

pub fn cluster_title_x(c: &CNode, width: f64, crossings: &Crossings, pad: f64) -> f64 {
    if width == 0.0 {
        return c.x;
    }
    let top = c.y - c.h / 2.0;
    let left = c.x - c.w / 2.0;
    let half = width / 2.0;
    let candidates = [left + pad - 6.0 + half, c.x, left + c.w - pad + 6.0 - half];
    let Some(xs) = crossings.get(&key(top)) else {
        return candidates[0];
    };
    let mut best = candidates[0];
    let mut best_gap = -1.0f64;
    for x in candidates {
        let mut gap = f64::INFINITY;
        for &cx in xs {
            if cx > left && cx < left + c.w {
                gap = js_min(gap, (cx - x).abs() - half);
            }
        }
        if gap >= 6.0 {
            return x;
        }
        if gap > best_gap + 0.5 {
            best_gap = gap;
            best = x;
        }
    }
    best
}

pub fn title_crossed(c: &CNode, width: f64, x: f64, crossings: &Crossings) -> bool {
    if width <= 0.0 {
        return false;
    }
    let Some(xs) = crossings.get(&key(c.y - c.h / 2.0)) else {
        return false;
    };
    let left = c.x - c.w / 2.0;
    xs.iter()
        .any(|&cx| cx > left && cx < left + c.w && (cx - x).abs() < width / 2.0 + 4.0)
}

pub fn struck_title(
    label: &Label,
    x: f64,
    y: f64,
    attrs: &str,
    id: &str,
    nodes: &[CNode],
    index: usize,
) -> String {
    let mut depth = 1i32;
    let mut p = nodes[index].parent;
    while p >= 0 && depth < 8 {
        depth += 1;
        p = nodes[p as usize].parent;
    }
    let fill = format!(
        "color-mix(in srgb,var(--_a) {}%,var(--_bg))",
        num(100.0 * (1.0 - 0.5f64.powi(depth)))
    );
    format!(
        "<g class=\"pelers-cluster-title\" data-id=\"{id}\">{}{}</g>",
        label_svg(
            label,
            x,
            y,
            &format!(
                " fill=\"none\" stroke=\"{fill}\" stroke-width=\"5\" stroke-linejoin=\"round\""
            )
        ),
        label_svg(label, x, y, attrs)
    )
}
