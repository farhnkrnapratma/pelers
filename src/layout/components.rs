use crate::layout::{CEdge, CNode, CompoundResult};
use crate::util::jsnum::{js_max, js_min};

#[derive(Default)]
struct Part {
    nodes: Vec<usize>,
    edges: Vec<usize>,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn include(p: &mut Part, x: f64, y: f64, w: f64, h: f64) {
    p.left = js_min(p.left, x - w / 2.0);
    p.top = js_min(p.top, y - h / 2.0);
    p.right = js_max(p.right, x + w / 2.0);
    p.bottom = js_max(p.bottom, y + h / 2.0);
}

pub(crate) fn wrap_components(
    nodes: &mut [CNode],
    edges: &mut [CEdge],
    width: f64,
    gap: f64,
) -> Option<CompoundResult> {
    let n = nodes.len();
    let mut parent: Vec<usize> = (0..n).collect();
    for (i, node) in nodes.iter().enumerate().take(n) {
        if node.parent >= 0 {
            let a = root(&mut parent, i);
            let b = root(&mut parent, node.parent as usize);
            parent[a] = b;
        }
    }
    for e in edges.iter() {
        let a = root(&mut parent, e.src);
        let b = root(&mut parent, e.dst);
        parent[a] = b;
    }

    let mut order: Vec<usize> = Vec::new();
    let mut parts: Vec<Part> = Vec::new();
    let mut index_of: Vec<Option<usize>> = vec![None; n];
    let new_part = || Part {
        nodes: Vec::new(),
        edges: Vec::new(),
        left: f64::INFINITY,
        top: f64::INFINITY,
        right: f64::NEG_INFINITY,
        bottom: f64::NEG_INFINITY,
    };
    for (i, c) in nodes.iter().enumerate().take(n) {
        let id = root(&mut parent, i);
        let slot = match index_of[id] {
            Some(slot) => slot,
            None => {
                parts.push(new_part());
                index_of[id] = Some(parts.len() - 1);
                order.push(id);
                parts.len() - 1
            }
        };
        parts[slot].nodes.push(i);
        include(&mut parts[slot], c.x, c.y, c.w, c.h);
    }
    if parts.len() < 2 {
        return None;
    }
    for (ei, e) in edges.iter().enumerate() {
        let r = root(&mut parent, e.src);
        let slot = index_of[r].expect("every edge end belongs to a part");
        parts[slot].edges.push(ei);
        for k in (0..e.route.len()).step_by(3) {
            include(&mut parts[slot], e.route[k], e.route[k + 1], 0.0, 0.0);
        }
        if e.label_w > 0.0 {
            include(&mut parts[slot], e.label_x, e.label_y, e.label_w, e.label_h);
        }
    }

    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut row_height = 0.0f64;
    let mut right = 0.0f64;
    let mut from = f64::INFINITY;
    let mut to = f64::NEG_INFINITY;
    let mut shifts: Vec<(f64, f64)> = Vec::new();
    for part in &parts {
        let w = part.right - part.left;
        let h = part.bottom - part.top;
        if x > 0.0 && x + w > width {
            y += row_height + gap;
            x = 0.0;
            row_height = 0.0;
        }
        shifts.push((x - part.left, y - part.top));
        from = js_min(from, part.left);
        to = js_max(to, part.right);
        right = js_max(right, x + w);
        row_height = js_max(row_height, h);
        x += w + gap;
    }
    if right >= to - from {
        return None;
    }
    for (part, (ox, oy)) in parts.iter().zip(shifts) {
        for &i in &part.nodes {
            nodes[i].x += ox;
            nodes[i].y += oy;
        }
        for &ei in &part.edges {
            let e = &mut edges[ei];
            for k in (0..e.route.len()).step_by(3) {
                e.route[k] += ox;
                e.route[k + 1] += oy;
            }
            e.label_x += ox;
            e.label_y += oy;
        }
    }
    Some(CompoundResult {
        width: right,
        height: y + row_height,
    })
}
