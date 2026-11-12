use std::collections::HashMap;

use super::model::{C4Db, Element};
use super::route::{Box, HEAD, LABEL_PAD, LANE, NARROW};
use crate::svg::edges::ARROW;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::js_max;

const SHAPE_W: f64 = 216.0;
const SHAPE_MIN_H: f64 = 64.0;
const PAD_X: f64 = 14.0;
const PAD_Y: f64 = 12.0;
const GAP_X: f64 = NARROW + LABEL_PAD + 3.0 * ARROW;
const GAP_Y: f64 = 88.0;
pub const GROUP_PAD: f64 = LANE + 8.0;
const GROUP_GAP: f64 = 48.0;
pub const HEAD_LEFT: f64 = 16.0;
pub const HEAD_TOP: f64 = 10.0;

fn form_named(word: &str) -> Option<&'static str> {
    Some(match word {
        "person" => "person",
        "box" | "rounded" | "component" => "rect",
        "cylinder" | "database" | "db" => "cyl",
        "queue" | "pipe" => "h-cyl",
        _ => return None,
    })
}

pub struct Node {
    pub boxed: Box,
    pub shape: usize,
    pub labels: Vec<Label>,
    pub text_h: f64,
}

pub struct Group {
    pub boxed: Box,
    pub boundary: usize,
    pub kids: Vec<usize>,
    pub nodes: Vec<usize>,
    pub labels: Vec<Label>,
    pub head: f64,
    pub head_w: f64,
    pub head_x: f64,
}

pub struct Layout {
    pub order: Vec<usize>,
    pub groups: Vec<Group>,
    pub nodes: Vec<Node>,
    pub shape_boxes: Vec<Option<Box>>,
    pub boundary_boxes: Vec<Option<Box>>,
}

fn form_of(shape: &Element) -> &'static str {
    let by_shape = form_named(&shape.shape.clone().unwrap_or_default().to_lowercase());
    let by_sprite = form_named(&shape.sprite.clone().unwrap_or_default().to_lowercase());
    if let Some(form) = by_shape.or(by_sprite) {
        return form;
    }
    if let Some(tags) = &shape.tags {
        for tag in tags.split(',') {
            if let Some(form) = form_named(&tag.trim().to_lowercase()) {
                return form;
            }
        }
    }
    let kind = shape.kind.as_str();
    if kind.ends_with("person") {
        "person"
    } else if kind.ends_with("_db") {
        "cyl"
    } else if kind.ends_with("_queue") {
        "h-cyl"
    } else {
        "rect"
    }
}

fn measure(
    id: usize,
    shape: &Element,
    shape_index: usize,
    size: f64,
    small: f64,
    width: f64,
) -> Node {
    let form = form_of(shape);
    let cap_w = if form == "h-cyl" { 36.0 } else { 0.0 };
    let cap_h = match form {
        "person" => HEAD,
        "cyl" => 24.0,
        _ => 0.0,
    };
    let inner = width - 2.0 * PAD_X - cap_w;
    let kind_label = format!("«{}»", shape.kind.replace('_', " "));
    let techn = shape
        .techn
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(|t| format!("[{t}]"));
    let labels = vec![
        layout_label(Some(&kind_label), false, small - 1.0, inner, 0),
        layout_label(Some(&shape.label), false, size, inner, BOLD_STYLE),
        layout_label(techn.as_deref(), false, small, inner, 0),
        layout_label(shape.descr.as_deref(), false, small, inner, 0),
    ];
    let mut tw: f64 = 0.0;
    let mut text_h = if labels[3].height > 0.0 { 4.0 } else { 0.0 };
    for label in &labels {
        tw = js_max(tw, label.width);
        text_h += label.height;
    }
    Node {
        boxed: Box {
            id,
            x: 0.0,
            y: 0.0,
            w: js_max(width, tw + 2.0 * PAD_X + cap_w),
            h: js_max(SHAPE_MIN_H, text_h + 2.0 * PAD_Y + cap_h),
            form,
        },
        shape: shape_index,
        labels,
        text_h,
    }
}

#[derive(Clone, Copy)]
struct Dim {
    w: f64,
    h: f64,
    x: f64,
    y: f64,
}

fn rows(dims: &mut [Dim], per_row: usize, gap_x: f64, gap_y: f64, stretch: bool) -> (f64, f64) {
    let mut width: f64 = 0.0;
    let mut y: f64 = 0.0;
    let mut from = 0;
    while from < dims.len() {
        let to = dims.len().min(from + per_row);
        let mut x: f64 = 0.0;
        let mut row_h: f64 = 0.0;
        for dim in &dims[from..to] {
            row_h = js_max(row_h, dim.h);
        }
        for dim in &mut dims[from..to] {
            dim.x = x;
            dim.y = y;
            if stretch {
                dim.h = row_h;
            }
            x += dim.w + gap_x;
        }
        width = js_max(width, x - gap_x);
        y += row_h + gap_y;
        from += per_row;
    }
    (width, if dims.is_empty() { 0.0 } else { y - gap_y })
}

pub fn layout_c4(db: &C4Db, size: f64, small: f64, room: Option<f64>, gap: f64) -> Layout {
    let group_pad = if room.is_none() { GROUP_PAD } else { 12.0 };
    let shape_count = db.shapes.len();
    let mut groups: Vec<Group> = Vec::new();
    for (j, boundary) in db.boundaries.iter().enumerate() {
        let label_room = match room {
            None => 4000.0,
            Some(r) => js_max(80.0, r - 2.0 * HEAD_LEFT - 2.0 * GROUP_PAD * 2.0),
        };
        let labels = if boundary.parent.is_some() {
            let type_text = boundary
                .type_
                .as_deref()
                .filter(|t| !t.is_empty())
                .map(|t| format!("[{t}]"));
            vec![
                layout_label(Some(&boundary.label), false, size, label_room, BOLD_STYLE),
                layout_label(type_text.as_deref(), false, small, label_room, 0),
                layout_label(boundary.descr.as_deref(), false, small, label_room, 0),
            ]
        } else {
            Vec::new()
        };
        let mut head = 0.0;
        let mut head_w: f64 = 0.0;
        for label in &labels {
            head += label.height;
            head_w = js_max(head_w, label.width);
        }
        groups.push(Group {
            boxed: Box {
                id: shape_count + j,
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                form: "rect",
            },
            boundary: j,
            kids: Vec::new(),
            nodes: Vec::new(),
            labels,
            head: if head > 0.0 {
                head + HEAD_TOP + 6.0 + LANE
            } else {
                group_pad
            },
            head_w,
            head_x: HEAD_LEFT,
        });
    }
    for (j, boundary) in db.boundaries.iter().enumerate() {
        if let Some(parent) = boundary.parent {
            groups[parent].kids.push(j);
        }
    }

    let mut order: Vec<usize> = vec![0];
    let mut depths: HashMap<usize, usize> = HashMap::from([(0, 0)]);
    let mut i = 0;
    while i < order.len() {
        let current = order[i];
        let depth = depths[&current];
        for kid in groups[current].kids.clone() {
            depths.insert(kid, depth + 1);
            order.push(kid);
        }
        i += 1;
    }

    let mut nodes: Vec<Node> = Vec::new();
    for (k, shape) in db.shapes.iter().enumerate() {
        let Some(parent) = shape.parent else {
            continue;
        };
        let depth = depths.get(&parent).copied().unwrap_or(0) as f64;
        let node_width = match room {
            None => SHAPE_W,
            Some(r) => js_max(100.0, f64::min(SHAPE_W, r - 2.0 * depth * group_pad - 32.0)),
        };
        let node = measure(k, shape, k, size, small, node_width);
        let index = nodes.len();
        nodes.push(node);
        groups[parent].nodes.push(index);
    }

    let per_row = js_max(1.0, db.shape_in_row).floor() as usize;
    let groups_per_row = js_max(1.0, db.boundary_in_row).floor() as usize;
    let mut kids_top: HashMap<usize, f64> = HashMap::new();
    for &g in order.iter().rev() {
        let gap_x = if room.is_none() { GAP_X } else { gap };
        let count = |dims: &[Dim], limit: usize, gap: f64| -> usize {
            match room {
                None => limit,
                Some(r) => {
                    let mut widest: f64 = 1.0;
                    for dim in dims {
                        widest = js_max(widest, dim.w);
                    }
                    let fit = ((r + gap - 2.0 * group_pad) / (widest + gap)).floor();
                    js_max(1.0, f64::min(limit as f64, fit)) as usize
                }
            }
        };
        let mut node_dims: Vec<Dim> = groups[g]
            .nodes
            .iter()
            .map(|&n| Dim {
                w: nodes[n].boxed.w,
                h: nodes[n].boxed.h,
                x: 0.0,
                y: 0.0,
            })
            .collect();
        let node_per_row = count(&node_dims, per_row, gap_x);
        let (grid_w, grid_h) = rows(&mut node_dims, node_per_row, gap_x, GAP_Y, true);
        for (dim, &n) in node_dims.iter().zip(&groups[g].nodes) {
            nodes[n].boxed.x = dim.x;
            nodes[n].boxed.y = dim.y;
            nodes[n].boxed.h = dim.h;
        }
        let mut kid_dims: Vec<Dim> = groups[g]
            .kids
            .iter()
            .map(|&k| Dim {
                w: groups[k].boxed.w,
                h: groups[k].boxed.h,
                x: 0.0,
                y: 0.0,
            })
            .collect();
        let kid_per_row = count(&kid_dims, groups_per_row, GROUP_GAP);
        let (kids_w, kids_h) = rows(&mut kid_dims, kid_per_row, GROUP_GAP, GROUP_GAP, false);
        let kid_list = groups[g].kids.clone();
        for (dim, &k) in kid_dims.iter().zip(&kid_list) {
            groups[k].boxed.x = dim.x;
            groups[k].boxed.y = dim.y;
        }
        let between = if grid_h > 0.0 && kids_h > 0.0 {
            GROUP_GAP
        } else {
            0.0
        };
        let frame = if g == 0 { 0.0 } else { group_pad };
        kids_top.insert(g, grid_h + between);
        let group = &mut groups[g];
        group.boxed.w = js_max(
            js_max(grid_w, kids_w),
            group.head_w + 2.0 * HEAD_LEFT - 2.0 * frame,
        ) + 2.0 * frame;
        let head_part = if frame != 0.0 { group.head } else { 0.0 };
        group.boxed.h = head_part + grid_h + between + kids_h + frame;
    }

    for &g in &order {
        let root = g == 0;
        let left = groups[g].boxed.x + if root { 0.0 } else { group_pad };
        let top = groups[g].boxed.y + if root { 0.0 } else { groups[g].head };
        for &n in &groups[g].nodes.clone() {
            nodes[n].boxed.x += left;
            nodes[n].boxed.y += top;
        }
        let kids_offset = kids_top.get(&g).copied().unwrap_or(0.0);
        for &k in &groups[g].kids.clone() {
            groups[k].boxed.x += left;
            groups[k].boxed.y += top + kids_offset;
        }
    }

    let mut shape_boxes: Vec<Option<Box>> = vec![None; db.shapes.len()];
    let mut boundary_boxes: Vec<Option<Box>> = vec![None; db.boundaries.len()];
    for &g in &order {
        for &n in &groups[g].nodes {
            shape_boxes[nodes[n].shape] = Some(nodes[n].boxed.clone());
        }
        if g != 0 {
            boundary_boxes[groups[g].boundary] = Some(groups[g].boxed.clone());
        }
    }

    Layout {
        order,
        groups,
        nodes,
        shape_boxes,
        boundary_boxes,
    }
}
