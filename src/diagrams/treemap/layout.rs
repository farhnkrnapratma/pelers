use super::model::TreeNode;
use crate::util::jsnum::{js_max, js_min, js_round};

#[derive(Clone, Debug)]
pub struct Box_ {
    pub node: usize,
    pub parent: Option<usize>,
    pub series: usize,
    pub value: f64,
    pub children: Vec<usize>,
    pub headed: bool,
    pub header: f64,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Insets {
    pub gap: f64,
    pub header: f64,
    pub short: f64,
    pub side: f64,
}

fn clamp(v: f64, low: f64, high: f64) -> f64 {
    if v >= low { js_min(v, high) } else { low }
}

#[allow(clippy::too_many_arguments)]
fn squarify(
    boxes: &mut [Box_],
    ids: &[usize],
    total: f64,
    mut x: f64,
    mut y: f64,
    mut w: f64,
    mut h: f64,
    gap: f64,
) {
    let mut rest = total;
    let mut from = 0;
    while from < ids.len() {
        let scale = (w * h) / rest;
        let side = js_min(w, h);
        let mut sum = boxes[ids[from]].value * scale;
        let largest = sum;
        let mut worst = js_max(
            (side * side * largest) / (sum * sum),
            (sum * sum) / (side * side * largest),
        );
        let mut to = from + 1;
        while to < ids.len() {
            let smallest = boxes[ids[to]].value * scale;
            let next = sum + smallest;
            let ratio = js_max(
                (side * side * largest) / (next * next),
                (next * next) / (side * side * smallest),
            );
            if ratio > worst {
                break;
            }
            worst = ratio;
            sum = next;
            to += 1;
        }

        let thickness = sum / side;
        let mut offset = 0.0;
        for (offset_index, &id) in ids[from..to].iter().enumerate() {
            let i = from + offset_index;
            let value = boxes[id].value;
            let length = if i == to - 1 {
                side - offset
            } else {
                (value * scale) / thickness
            };
            let bx = if w >= h { x } else { x + offset };
            let by = if w >= h { y + offset } else { y };
            let bw = if w >= h { thickness } else { length };
            let bh = if w >= h { length } else { thickness };
            let gx = if js_round(bx + bw - gap / 2.0) > js_round(bx + gap / 2.0) {
                gap / 2.0
            } else {
                0.0
            };
            let gy = if js_round(by + bh - gap / 2.0) > js_round(by + gap / 2.0) {
                gap / 2.0
            } else {
                0.0
            };
            let b = &mut boxes[id];
            b.x = js_round(bx + gx);
            b.y = js_round(by + gy);
            b.w = js_round(bx + bw - gx) - b.x;
            b.h = js_round(by + bh - gy) - b.y;
            offset += length;
            rest -= value;
        }
        if w >= h {
            x += thickness;
            w -= thickness;
        } else {
            y += thickness;
            h -= thickness;
        }
        from = to;
    }
}

fn new_box(
    boxes: &mut Vec<Box_>,
    nodes: &[TreeNode],
    node: usize,
    parent: Option<usize>,
    series: usize,
) -> usize {
    let value = match nodes[node].value {
        Some(v) if nodes[node].children.is_none() && v.is_finite() && v > 0.0 => v,
        _ => 0.0,
    };
    boxes.push(Box_ {
        node,
        parent,
        series,
        value,
        children: Vec::new(),
        headed: false,
        header: 0.0,
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    });
    boxes.len() - 1
}

pub fn layout_treemap(
    nodes: &[TreeNode],
    roots: &[usize],
    width: f64,
    height: f64,
    insets: Insets,
) -> (Vec<Box_>, Vec<usize>) {
    let mut boxes: Vec<Box_> = Vec::new();
    let mut all: Vec<usize> = roots
        .iter()
        .enumerate()
        .map(|(index, &node)| new_box(&mut boxes, nodes, node, None, index))
        .collect();
    let mut i = 0;
    while i < all.len() {
        let parent = all[i];
        let parent_node = boxes[parent].node;
        let series = boxes[parent].series;
        for &node in nodes[parent_node].children.as_deref().unwrap_or(&[]) {
            let child = new_box(&mut boxes, nodes, node, Some(parent), series);
            boxes[parent].children.push(child);
            all.push(child);
        }
        i += 1;
    }
    for &b in all.iter().rev() {
        if let Some(parent) = boxes[b].parent {
            let value = boxes[b].value;
            boxes[parent].value += value;
        }
    }

    let Insets {
        gap,
        header,
        short,
        side,
    } = insets;
    let mut placed: Vec<usize> = Vec::new();
    let place = |boxes: &mut Vec<Box_>,
                 placed: &mut Vec<usize>,
                 children: &[usize],
                 x: f64,
                 y: f64,
                 w: f64,
                 h: f64| {
        let mut shown: Vec<usize> = children
            .iter()
            .copied()
            .filter(|&b| boxes[b].value > 0.0)
            .collect();
        shown.sort_by(|&a, &b| {
            (boxes[b].value - boxes[a].value)
                .partial_cmp(&0.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut total = 0.0;
        for &b in &shown {
            total += boxes[b].value;
        }
        if shown.is_empty() || !total.is_finite() {
            return;
        }
        let w = js_max(0.0, w);
        let h = js_max(0.0, h);
        if w > 0.0 && h > 0.0 {
            squarify(
                boxes,
                &shown,
                total,
                x - gap / 2.0,
                y - gap / 2.0,
                w + gap,
                h + gap,
                gap,
            );
        }
        for &b in &shown {
            let (bx, by, bw, bh) = (boxes[b].x, boxes[b].y, boxes[b].w, boxes[b].h);
            let x0 = clamp(bx, x, x + w);
            let y0 = clamp(by, y, y + h);
            let entry = &mut boxes[b];
            entry.w = clamp(bx + bw, x0, x + w) - x0;
            entry.h = clamp(by + bh, y0, y + h) - y0;
            entry.x = x0;
            entry.y = y0;
            placed.push(b);
        }
    };

    place(
        &mut boxes,
        &mut placed,
        &all[..roots.len()],
        0.0,
        0.0,
        width,
        height,
    );
    let mut k = 0;
    while k < placed.len() {
        let b = placed[k];
        k += 1;
        if boxes[b].children.is_empty() {
            continue;
        }
        let (bx, by, bw, bh) = (boxes[b].x, boxes[b].y, boxes[b].w, boxes[b].h);
        let top = if bh - header - side >= 12.0 {
            header
        } else {
            short
        };
        let headed = bw > js_max(24.0, 2.0 * side) && bh - top - side >= 8.0;
        boxes[b].headed = headed;
        boxes[b].header = if headed { top } else { 0.0 };
        let children = boxes[b].children.clone();
        if headed {
            place(
                &mut boxes,
                &mut placed,
                &children,
                bx + side,
                by + top,
                bw - 2.0 * side,
                bh - top - side,
            );
        } else {
            place(&mut boxes, &mut placed, &children, bx, by, bw, bh);
        }
    }
    (boxes, placed)
}
