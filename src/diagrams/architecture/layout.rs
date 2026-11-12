use std::collections::HashMap;

use super::db::{ArchEdge, ArchGroup, ArchLayoutHint, ArchNode, Direction};
use super::parser::Side;

pub struct Grid {
    pub col: Vec<i32>,
    pub row: Vec<i32>,
    pub group_col: Vec<i32>,
    pub group_row: Vec<i32>,
    pub group_cols: Vec<i32>,
    pub group_rows: Vec<i32>,
    pub group_parent: Vec<i32>,
    pub group_empty: Vec<bool>,
    pub node_parent: Vec<i32>,
}

const DX: [i32; 4] = [-1, 1, 0, 0];
const DY: [i32; 4] = [0, 0, -1, 1];
const SMALL: i32 = 64;
const OFFSET: i64 = 1 << 25;
const SPAN: i64 = 1 << 26;

pub fn side_index(side: Side) -> usize {
    match side {
        Side::L => 0,
        Side::R => 1,
        Side::T => 2,
        Side::B => 3,
    }
}

fn cell_key(cx: i32, cy: i32) -> i64 {
    (i64::from(cx) + OFFSET) * SPAN + i64::from(cy) + OFFSET
}

struct Layouter<'a> {
    n: usize,
    g: usize,
    parent: Vec<usize>,
    kids: Vec<Vec<usize>>,
    link_a: Vec<usize>,
    link_b: Vec<usize>,
    side_a: Vec<usize>,
    side_b: Vec<usize>,
    item_a: Vec<usize>,
    item_b: Vec<usize>,
    hint_links: Vec<Option<Vec<usize>>>,
    edge_links: Vec<Option<Vec<usize>>>,
    depth: Vec<i32>,
    x: Vec<i32>,
    y: Vec<i32>,
    w: Vec<i32>,
    h: Vec<i32>,
    placed: Vec<bool>,
    order: Vec<usize>,
    cells: HashMap<i64, usize>,
    large: Vec<usize>,
    reach: HashMap<(i32, i32, i32, i32, i32, i32), i32>,
    first: usize,
    ax: i32,
    ay: i32,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl Layouter<'_> {
    fn add_link(&mut self, a: usize, b: usize, sa: usize, sb: usize, hint: bool) {
        if a == b {
            return;
        }
        let mut ia = a;
        let mut ib = b;
        let mut ca = self.parent[a];
        let mut cb = self.parent[b];
        while ca != cb {
            if self.depth[ca] >= self.depth[cb] {
                ia = self.n + ca;
                ca = self.parent[ia];
            } else {
                ib = self.n + cb;
                cb = self.parent[ib];
            }
        }
        let k = self.link_a.len();
        self.link_a.push(a);
        self.link_b.push(b);
        self.side_a.push(sa);
        self.side_b.push(sb);
        self.item_a.push(ia);
        self.item_b.push(ib);
        let lists = if hint {
            &mut self.hint_links
        } else {
            &mut self.edge_links
        };
        lists[ia].get_or_insert_with(Vec::new).push(k);
        lists[ib].get_or_insert_with(Vec::new).push(k);
    }

    fn anchor(&mut self, node: usize, item: usize) {
        self.ax = 0;
        self.ay = 0;
        let mut cur = node;
        while cur != item {
            self.ax += self.x[cur];
            self.ay += self.y[cur];
            cur = self.n + self.parent[cur];
        }
    }

    fn put(&mut self, item: usize, px: i32, py: i32) {
        self.x[item] = px;
        self.y[item] = py;
        self.placed[item] = true;
        self.order.push(item);
        if self.w[item] * self.h[item] > SMALL {
            self.large.push(item);
            return;
        }
        for cy in py..py + self.h[item] {
            for cx in px..px + self.w[item] {
                self.cells.insert(cell_key(cx, cy), item);
            }
        }
    }

    fn overlaps(&self, item: usize, rx: i32, ry: i32, rw: i32, rh: i32) -> bool {
        self.x[item] < rx + rw
            && rx < self.x[item] + self.w[item]
            && self.y[item] < ry + rh
            && ry < self.y[item] + self.h[item]
    }

    fn collide(&self, rx: i32, ry: i32, rw: i32, rh: i32) -> Option<usize> {
        if rw * rh > SMALL {
            for &item in &self.order[self.first..] {
                if self.overlaps(item, rx, ry, rw, rh) {
                    return Some(item);
                }
            }
            return None;
        }
        for cy in ry..ry + rh {
            for cx in rx..rx + rw {
                if let Some(&hit) = self.cells.get(&cell_key(cx, cy)) {
                    return Some(hit);
                }
            }
        }
        self.large
            .iter()
            .find(|&&item| self.overlaps(item, rx, ry, rw, rh))
            .copied()
    }

    fn slide(&mut self, rx: i32, ry: i32, rw: i32, rh: i32, sx: i32, sy: i32) -> i32 {
        let key = (rx, ry, rw, rh, sx, sy);
        let mut k = self.reach.get(&key).copied().unwrap_or(0);
        while let Some(hit) = self.collide(rx + k * sx, ry + k * sy, rw, rh) {
            k = if sx > 0 {
                self.x[hit] + self.w[hit] - rx
            } else if sx < 0 {
                rx + rw - self.x[hit]
            } else if sy > 0 {
                self.y[hit] + self.h[hit] - ry
            } else {
                ry + rh - self.y[hit]
            };
        }
        self.reach.insert(key, k);
        k
    }

    fn follow(&mut self, from: usize, k: usize) {
        let forward = self.item_a[k] == from;
        let to = if forward {
            self.item_b[k]
        } else {
            self.item_a[k]
        };
        if self.placed[to] {
            return;
        }
        let sp = if forward {
            self.side_a[k]
        } else {
            self.side_b[k]
        };
        let sq = if forward {
            self.side_b[k]
        } else {
            self.side_a[k]
        };
        let (first_node, second_node) = if forward {
            (self.link_a[k], self.link_b[k])
        } else {
            (self.link_b[k], self.link_a[k])
        };
        self.anchor(first_node, from);
        let pax = self.ax;
        let pay = self.ay;
        self.anchor(second_node, to);
        let mut dx = DX[sp];
        let mut dy = DY[sp];
        let mut shift_x = 0;
        let mut shift_y = 0;
        let facing = DX[sp] + DX[sq] == 0 && DY[sp] + DY[sq] == 0;
        if sp == sq {
            dx = if DX[sp] == 0 { 1 } else { 0 };
            dy = 1 - dx;
        } else if !facing {
            shift_x = -DX[sq];
            shift_y = -DY[sq];
        }
        let (mut bx, mut by) = if dx != 0 {
            (
                if dx > 0 {
                    self.x[from] + self.w[from]
                } else {
                    self.x[from] - self.w[to]
                },
                self.y[from] + pay + shift_y - self.ay,
            )
        } else {
            (
                self.x[from] + pax + shift_x - self.ax,
                if dy > 0 {
                    self.y[from] + self.h[from]
                } else {
                    self.y[from] - self.h[to]
                },
            )
        };
        if facing {
            let list = self.edge_links[to].clone();
            if let Some(list) = list.filter(|l| l.len() > 1) {
                let mut wanted: Vec<i32> = Vec::new();
                for j in list {
                    let to_is_b = self.item_b[j] == to;
                    let other = if to_is_b {
                        self.item_a[j]
                    } else {
                        self.item_b[j]
                    };
                    let (s_a, s_b) = (self.side_a[j], self.side_b[j]);
                    let (side_here, side_there) = if to_is_b { (s_a, s_b) } else { (s_b, s_a) };
                    if !self.placed[other] || side_here != sp || side_there != sq {
                        continue;
                    }
                    let (n1, n2) = if to_is_b {
                        (self.link_a[j], self.link_b[j])
                    } else {
                        (self.link_b[j], self.link_a[j])
                    };
                    self.anchor(n1, other);
                    let at = if dx != 0 {
                        self.y[other] + self.ay
                    } else {
                        self.x[other] + self.ax
                    };
                    self.anchor(n2, to);
                    wanted.push(at - if dx != 0 { self.ay } else { self.ax });
                }
                if wanted.len() > 1 {
                    wanted.sort();
                    let middle = wanted[(wanted.len() - 1) >> 1];
                    if dx != 0 {
                        by = middle;
                    } else {
                        bx = middle;
                    }
                }
            }
            let px = if dx == 0 { 1 } else { 0 };
            let py = 1 - px;
            let (tw, th) = (self.w[to], self.h[to]);
            let forth = self.slide(bx, by, tw, th, px, py);
            let back = if forth == 0 {
                0
            } else {
                self.slide(bx, by, tw, th, -px, -py)
            };
            let near = if forth <= back { forth } else { -back };
            bx += px * near;
            by += py * near;
        } else {
            let (tw, th) = (self.w[to], self.h[to]);
            let far = self.slide(bx, by, tw, th, dx, dy);
            bx += dx * far;
            by += dy * far;
        }
        self.put(to, bx, by);
    }

    fn layout(&mut self, c: usize) {
        let items = self.kids[c].clone();
        if items.is_empty() {
            return;
        }
        self.order.clear();
        let mut starts: Vec<usize> = Vec::new();
        for start in items {
            if self.placed[start] {
                continue;
            }
            self.first = self.order.len();
            starts.push(self.first);
            self.cells.clear();
            self.large.clear();
            self.reach.clear();
            self.put(start, 0, 0);
            let mut hint_at = self.first;
            let mut edge_at = self.first;
            let mut link_at = 0usize;
            loop {
                if hint_at < self.order.len() {
                    let from = self.order[hint_at];
                    hint_at += 1;
                    let list = self.hint_links[from].clone();
                    if let Some(list) = list {
                        for k in list {
                            self.follow(from, k);
                        }
                    }
                    continue;
                }
                if edge_at >= self.order.len() {
                    break;
                }
                let node = self.order[edge_at];
                let list = self.edge_links[node].clone();
                match list {
                    Some(list) if link_at < list.len() => {
                        let k = list[link_at];
                        link_at += 1;
                        self.follow(node, k);
                    }
                    _ => {
                        edge_at += 1;
                        link_at = 0;
                    }
                }
            }
        }
        starts.push(self.order.len());

        let count = starts.len() - 1;
        let mut min_x = vec![0i32; count];
        let mut min_y = vec![0i32; count];
        let mut part_w = vec![0i32; count];
        let mut part_h = vec![0i32; count];
        let mut area: f64 = 0.0;
        let mut widest: i32 = 0;
        for p in 0..count {
            let mut x0 = i32::MAX;
            let mut y0 = i32::MAX;
            let mut x1 = i32::MIN;
            let mut y1 = i32::MIN;
            for &item in &self.order[starts[p]..starts[p + 1]] {
                x0 = x0.min(self.x[item]);
                y0 = y0.min(self.y[item]);
                x1 = x1.max(self.x[item] + self.w[item]);
                y1 = y1.max(self.y[item] + self.h[item]);
            }
            min_x[p] = x0;
            min_y[p] = y0;
            part_w[p] = x1 - x0;
            part_h[p] = y1 - y0;
            area += f64::from(part_w[p]) * f64::from(part_h[p]);
            widest = widest.max(part_w[p]);
        }
        let limit = widest.max((area * 2.0).sqrt().ceil() as i32);
        let mut cx = 0i32;
        let mut cy = 0i32;
        let mut shelf = 0i32;
        let mut width = 0i32;
        for p in 0..count {
            if cx > 0 && cx + part_w[p] > limit {
                cy += shelf;
                cx = 0;
                shelf = 0;
            }
            for &item in &self.order[starts[p]..starts[p + 1]] {
                self.x[item] += cx - min_x[p];
                self.y[item] += cy - min_y[p];
            }
            cx += part_w[p];
            shelf = shelf.max(part_h[p]);
            width = width.max(cx);
        }
        if c < self.g {
            self.w[self.n + c] = width;
            self.h[self.n + c] = cy + shelf;
        }
    }
}

pub fn place_on_grid(
    nodes: &[ArchNode],
    groups: &[ArchGroup],
    edges: &[ArchEdge],
    hints: &[ArchLayoutHint],
) -> Grid {
    let n = nodes.len();
    let g_count = groups.len();
    let root = g_count;
    let mut node_index: HashMap<&str, usize> = HashMap::new();
    let mut group_index: HashMap<&str, usize> = HashMap::new();
    let mut parent = vec![root; n + g_count];
    let mut depth = vec![0i32; g_count + 1];
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); g_count + 1];
    for (g, group) in groups.iter().enumerate() {
        group_index.insert(group.id.as_str(), g);
        let p = group
            .in_group
            .as_deref()
            .and_then(|name| group_index.get(name).copied());
        match p {
            Some(p) if p != g => {
                parent[n + g] = p;
                depth[g] = depth[p] + 1;
            }
            _ => depth[g] = 1,
        }
    }
    for (i, node) in nodes.iter().enumerate() {
        node_index.insert(node.id.as_str(), i);
        if let Some(p) = node
            .in_group
            .as_deref()
            .and_then(|name| group_index.get(name).copied())
        {
            parent[i] = p;
        }
        kids[parent[i]].push(i);
    }
    for g in 0..g_count {
        let p = parent[n + g];
        kids[p].push(n + g);
    }

    let mut layouter = Layouter {
        n,
        g: g_count,
        parent,
        kids,
        link_a: Vec::new(),
        link_b: Vec::new(),
        side_a: Vec::new(),
        side_b: Vec::new(),
        item_a: Vec::new(),
        item_b: Vec::new(),
        hint_links: vec![None; n + g_count],
        edge_links: vec![None; n + g_count],
        depth,
        x: vec![0; n + g_count],
        y: vec![0; n + g_count],
        w: vec![1; n + g_count],
        h: vec![1; n + g_count],
        placed: vec![false; n + g_count],
        order: Vec::new(),
        cells: HashMap::new(),
        large: Vec::new(),
        reach: HashMap::new(),
        first: 0,
        ax: 0,
        ay: 0,
        _marker: std::marker::PhantomData,
    };

    for hint in hints {
        let row = hint.direction == Direction::Row;
        for i in 1..hint.members.len() {
            let a = node_index.get(hint.members[i - 1].as_str()).copied();
            let b = node_index.get(hint.members[i].as_str()).copied();
            if let (Some(a), Some(b)) = (a, b) {
                let (sa, sb) = if row { (1, 0) } else { (3, 2) };
                layouter.add_link(a, b, sa, sb, true);
            }
        }
    }
    for like_sides in [false, true] {
        for edge in edges {
            if (edge.lhs_dir == edge.rhs_dir) != like_sides {
                continue;
            }
            let a = node_index.get(edge.lhs_id.as_str()).copied();
            let b = node_index.get(edge.rhs_id.as_str()).copied();
            if let (Some(a), Some(b)) = (a, b) {
                layouter.add_link(
                    a,
                    b,
                    side_index(edge.lhs_dir),
                    side_index(edge.rhs_dir),
                    false,
                );
            }
        }
    }

    for c in (0..g_count).rev() {
        layouter.layout(c);
    }
    layouter.layout(root);

    let mut group_col = vec![0i32; g_count];
    let mut group_row = vec![0i32; g_count];
    let mut group_parent = vec![0i32; g_count];
    let mut group_empty = vec![false; g_count];
    for g in 0..g_count {
        let p = layouter.parent[n + g];
        group_parent[g] = if p == root { -1 } else { p as i32 };
        group_col[g] = layouter.x[n + g] + if p == root { 0 } else { group_col[p] };
        group_row[g] = layouter.y[n + g] + if p == root { 0 } else { group_row[p] };
        if layouter.kids[g].is_empty() {
            group_empty[g] = true;
        }
    }
    let mut col = vec![0i32; n];
    let mut row = vec![0i32; n];
    let mut node_parent = vec![0i32; n];
    for i in 0..n {
        let p = layouter.parent[i];
        node_parent[i] = if p == root { -1 } else { p as i32 };
        col[i] = layouter.x[i] + if p == root { 0 } else { group_col[p] };
        row[i] = layouter.y[i] + if p == root { 0 } else { group_row[p] };
    }
    Grid {
        col,
        row,
        group_col,
        group_row,
        group_cols: layouter.w[n..].to_vec(),
        group_rows: layouter.h[n..].to_vec(),
        group_parent,
        group_empty,
        node_parent,
    }
}
