use std::collections::HashMap;

use super::layered::{chain, ease, layered, orient, rank};
use super::{
    CEdge, CNode, CompoundResult, Dir, Kind, LEdge, LNode, LayeredOptions, LayeredResult, ledge,
    lnode,
};
use crate::util::jsnum::{js_hypot, js_max, js_min, js_round};

const MAX_PORTS: usize = 40000;
const MAX_ORDERED_PORTS: usize = 400;
const MAX_ORDERED_EDGES: usize = 500;
const FLAT_SLACK: f64 = 1.5;
const FLAT_SPARE: f64 = 4.0;
const FAN: f64 = 24.0;
const IN_LINE: f64 = 0.5;
const CURVE_PIECES: usize = 3;
const CORNER_PIECES: usize = 2;
const STEP_PIECES: usize = 3;
const HANDLE: f64 = 0.62;
const MAX_INSERT: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    Curved,
    Stepped,
    Linear,
}

pub(crate) fn style_of(opt: &LayeredOptions) -> Style {
    if opt.tracks {
        Style::Stepped
    } else if opt.straight {
        Style::Linear
    } else {
        Style::Curved
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

fn side_from(value: u8) -> Side {
    match value & 3 {
        0 => Side::Top,
        1 => Side::Right,
        2 => Side::Bottom,
        _ => Side::Left,
    }
}

fn opposite(side: Side) -> Side {
    side_from((side as u8).wrapping_add(2))
}

fn start_side(dir: Dir) -> Side {
    match dir {
        Dir::TB => Side::Top,
        Dir::BT => Side::Bottom,
        Dir::LR => Side::Left,
        Dir::RL => Side::Right,
    }
}

fn js_eq(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[derive(Clone, Debug)]
struct PortEdge {
    e: usize,
    inner: usize,
    out: bool,
    side: Side,
    hint: f64,
    px: f64,
    py: f64,
    route: Vec<f64>,
}

#[derive(Clone, Copy, Debug)]
struct LevelEdge {
    e: usize,
    a: usize,
    b: usize,
}

#[derive(Clone, Debug)]
struct Level {
    id: i32,
    dir: Dir,
    items: Vec<usize>,
    edges: Vec<LevelEdge>,
    ports: Vec<usize>,
    port_of: HashMap<usize, usize>,
    routes: HashMap<usize, Vec<f64>>,
    x: f64,
    y: f64,
}

#[derive(Clone)]
struct Snapshot {
    nodes: Vec<f64>,
    routes: Vec<Vec<f64>>,
    labels: Vec<f64>,
    levels: Vec<f64>,
    total: CompoundResult,
}

struct Laid {
    given: Vec<f64>,
    lnodes: Vec<LNode>,
    ledges: Vec<LEdge>,
    size: LayeredResult,
}

struct Built {
    lnodes: Vec<LNode>,
    ledges: Vec<LEdge>,
    dummies: Vec<i64>,
}

struct FarHint {
    ledge_of: Vec<i64>,
    ledges: Vec<LEdge>,
    last: Vec<i32>,
}

struct Compound<'a> {
    nodes: &'a mut [CNode],
    edges: &'a mut [CEdge],
    opt: &'a LayeredOptions,
    root_dir: Dir,
    n: usize,
    levels: Vec<Level>,
    level_of: HashMap<i32, usize>,
    order: Vec<usize>,
    arena: Vec<PortEdge>,
    depth: Vec<usize>,
    children: Vec<Vec<usize>>,
    ordered: bool,
    keyed: bool,
    key: Vec<f64>,
    far: Option<FarHint>,
    flat_cross: f64,
    laid: HashMap<i32, Vec<Laid>>,
    total: CompoundResult,
}

fn snapshot(c: &Compound) -> Snapshot {
    let mut at = Vec::new();
    for node in c.nodes.iter() {
        at.push(node.x);
        at.push(node.y);
        at.push(node.w);
        at.push(node.h);
    }
    let labels: Vec<f64> = c
        .edges
        .iter()
        .flat_map(|e| [e.label_x, e.label_y])
        .collect();
    let mut places = Vec::new();
    for &li in c.order.iter() {
        let level = &c.levels[li];
        places.push(level.x);
        places.push(level.y);
        for &pi in level.ports.iter() {
            places.push(c.arena[pi].px);
            places.push(c.arena[pi].py);
        }
    }
    Snapshot {
        nodes: at,
        routes: c.edges.iter().map(|e| e.route.clone()).collect(),
        labels,
        levels: places,
        total: c.total.clone(),
    }
}

fn restore(c: &mut Compound, s: &Snapshot) -> CompoundResult {
    for (i, node) in c.nodes.iter_mut().enumerate() {
        node.x = s.nodes[4 * i];
        node.y = s.nodes[4 * i + 1];
        node.w = s.nodes[4 * i + 2];
        node.h = s.nodes[4 * i + 3];
    }
    for (i, edge) in c.edges.iter_mut().enumerate() {
        edge.route = s.routes[i].clone();
        edge.label_x = s.labels[2 * i];
        edge.label_y = s.labels[2 * i + 1];
    }
    let mut k = 0usize;
    let order = c.order.clone();
    for li in order {
        let (ports, _) = {
            let level = &mut c.levels[li];
            level.x = s.levels[k];
            level.y = s.levels[k + 1];
            k += 2;
            (level.ports.clone(), ())
        };
        for pi in ports {
            c.arena[pi].px = s.levels[k];
            c.arena[pi].py = s.levels[k + 1];
            k += 2;
        }
    }
    s.total.clone()
}

pub(crate) fn route_crossings(nodes: &[CNode], edges: &[CEdge], style: Style) -> i32 {
    let mut size = 0usize;
    for e in edges {
        if e.src != e.dst && e.route.len() > 3 {
            size += (e.route.len() / 3 - 1) * STEP_PIECES;
        }
    }
    let mut ax: Vec<f64> = vec![0.0; size];
    let mut ay: Vec<f64> = vec![0.0; size];
    let mut bx: Vec<f64> = vec![0.0; size];
    let mut by: Vec<f64> = vec![0.0; size];
    let mut low: Vec<f64> = vec![0.0; size];
    let mut high: Vec<f64> = vec![0.0; size];
    let mut owner: Vec<i32> = vec![0; size];
    let mut ends: Vec<u8> = vec![0; size];
    let mut count = 0usize;
    let mut top = f64::INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    let mut from_x = f64::NAN;
    let mut from_y = f64::NAN;
    let mut to_x = f64::NAN;
    let mut to_y = f64::NAN;

    let mut add = |ei: usize, part: u8, x0: f64, y0: f64, x1: f64, y1: f64| {
        let on =
            x0 == to_x && y0 == to_y && ((y1 == y0 && from_y == y0) || (x1 == x0 && from_x == x0));
        let mut x0 = x0;
        let mut y0 = y0;
        if on
            && count > 0
            && owner[count - 1] == ei as i32
            && ends[count - 1] == part
            && (x0 - from_x) * (x1 - x0) + (y0 - from_y) * (y1 - y0) > 0.0
        {
            count -= 1;
            x0 = from_x;
            y0 = from_y;
        }
        from_x = x0;
        from_y = y0;
        to_x = x1;
        to_y = y1;
        if count >= ax.len() {
            ax.push(0.0);
            ay.push(0.0);
            bx.push(0.0);
            by.push(0.0);
            low.push(0.0);
            high.push(0.0);
            owner.push(0);
            ends.push(0);
        }
        ends[count] = part;
        ax[count] = js_min(x0, x1);
        bx[count] = js_max(x0, x1);
        ay[count] = if x0 <= x1 { y0 } else { y1 };
        by[count] = if x0 <= x1 { y1 } else { y0 };
        low[count] = js_min(y0, y1);
        high[count] = js_max(y0, y1);
        if low[count] < top {
            top = low[count];
        }
        if high[count] > bottom {
            bottom = high[count];
        }
        owner[count] = ei as i32;
        count += 1;
    };

    for (ei, e) in edges.iter().enumerate() {
        if e.src == e.dst {
            continue;
        }
        let r = &e.route;
        let aligned =
            |k: usize| (r[k] - r[k - 3]).abs() < 0.01 || (r[k + 1] - r[k - 2]).abs() < 0.01;
        let length = |k: usize| (r[k] - r[k - 3]).abs() + (r[k + 1] - r[k - 2]).abs();
        let turns = |k: usize| {
            style == Style::Curved
                && k > 0
                && k < r.len() - 3
                && aligned(k)
                && aligned(k + 3)
                && length(k) >= 0.01
                && length(k + 3) >= 0.01
                && ((r[k] - r[k - 3]).abs() < 0.01) != ((r[k + 3] - r[k]).abs() < 0.01)
        };
        let mut at_x = r.first().copied().unwrap_or(f64::NAN);
        let mut at_y = r.get(1).copied().unwrap_or(f64::NAN);
        let mut k = 3usize;
        while k < r.len() {
            let part = (if k == 3 { 1u8 } else { 0 }) | (if k == r.len() - 3 { 2u8 } else { 0 });
            let x0 = at_x;
            let y0 = at_y;
            let x1 = r[k];
            let y1 = r[k + 1];
            at_x = x1;
            at_y = y1;
            if aligned(k) || style == Style::Linear {
                if !turns(k) {
                    add(ei, part, x0, y0, x1, y1);
                    k += 3;
                    continue;
                }
                let before = if turns(k - 3) {
                    length(k) / 2.0
                } else {
                    length(k)
                };
                let after = if turns(k + 3) {
                    length(k + 3) / 2.0
                } else {
                    length(k + 3)
                };
                let ux = (x1 - r[k - 3]) / length(k);
                let uy = (y1 - r[k - 2]) / length(k);
                let vx = (r[k + 3] - x1) / length(k + 3);
                let vy = (r[k + 4] - y1) / length(k + 3);
                let sx = x1 - ux * before;
                let sy = y1 - uy * before;
                add(ei, part, x0, y0, sx, sy);
                let c1x = x1 - ux * before * (1.0 - HANDLE);
                let c1y = y1 - uy * before * (1.0 - HANDLE);
                let c2x = x1 + vx * after * (1.0 - HANDLE);
                let c2y = y1 + vy * after * (1.0 - HANDLE);
                at_x = x1 + vx * after;
                at_y = y1 + vy * after;
                let turn = part | (if k + 3 == r.len() - 3 { 2u8 } else { 0 });
                let mut px = sx;
                let mut py = sy;
                for i in 1..=CORNER_PIECES {
                    let t = i as f64 / CORNER_PIECES as f64;
                    let u = 1.0 - t;
                    let x = u * u * u * sx
                        + 3.0 * u * u * t * c1x
                        + 3.0 * u * t * t * c2x
                        + t * t * t * at_x;
                    let y = u * u * u * sy
                        + 3.0 * u * u * t * c1y
                        + 3.0 * u * t * t * c2y
                        + t * t * t * at_y;
                    add(ei, turn, px, py, x, y);
                    px = x;
                    py = y;
                }
                k += 3;
                continue;
            }
            if style == Style::Stepped {
                let down = r[k + 2] == 0.0;
                let mx = if down { x0 } else { (x0 + x1) / 2.0 };
                let my = if down { (y0 + y1) / 2.0 } else { y0 };
                add(ei, part, x0, y0, mx, my);
                add(
                    ei,
                    part,
                    mx,
                    my,
                    if down { x1 } else { mx },
                    if down { my } else { y1 },
                );
                add(
                    ei,
                    part,
                    if down { x1 } else { mx },
                    if down { my } else { y1 },
                    x1,
                    y1,
                );
                k += 3;
                continue;
            }
            let down = r[k + 2] == 0.0;
            let c = if down {
                ease(x1 - x0, y1 - y0)
            } else {
                ease(y1 - y0, x1 - x0)
            };
            let mut px = x0;
            let mut py = y0;
            for i in 1..=CURVE_PIECES {
                let t = i as f64 / CURVE_PIECES as f64;
                let u = 1.0 - t;
                let across = t * t * (3.0 - 2.0 * t);
                let along = 3.0 * c * t * u * u + 3.0 * (1.0 - c) * t * t * u + t * t * t;
                let x = x0 + (x1 - x0) * (if down { across } else { along });
                let y = y0 + (y1 - y0) * (if down { along } else { across });
                add(ei, part, px, py, x, y);
                px = x;
                py = y;
            }
            k += 3;
        }
    }

    let bands = if bottom > top {
        js_round((count as f64).sqrt()) as usize
    } else {
        1
    };
    let scale = if bottom > top {
        bands as f64 / (bottom - top)
    } else {
        0.0
    };
    let band_of = |y: f64| -> usize {
        let v = js_min(bands as f64 - 1.0, ((y - top) * scale).floor());
        v as usize
    };
    let mut first = vec![0usize; count];
    let mut band_start = vec![0usize; bands + 1];
    for p in 0..count {
        first[p] = band_of(low[p]);
        let last = band_of(high[p]);
        for b in first[p]..=last {
            band_start[b + 1] += 1;
        }
    }
    for b in 0..bands {
        band_start[b + 1] += band_start[b];
    }
    let mut fill = band_start[..bands].to_vec();
    let mut in_band = vec![0usize; band_start[bands]];
    for p in 0..count {
        let last = band_of(high[p]);
        for b in first[p]..=last {
            in_band[fill[b]] = p;
            fill[b] += 1;
        }
    }

    let mut runs = vec![0.0f64; edges.len() * 8];
    let in_line_at = |r: &[f64], a: usize, b: usize, c: usize| -> bool {
        ((r[a] - r[b]).abs() < IN_LINE && (r[a] - r[c]).abs() < IN_LINE)
            || ((r[a + 1] - r[b + 1]).abs() < IN_LINE && (r[a + 1] - r[c + 1]).abs() < IN_LINE)
    };
    for (ei, e) in edges.iter().enumerate() {
        let r = &e.route;
        if r.len() < 6 {
            continue;
        }
        let mut k = 3usize;
        while k + 3 < r.len() && in_line_at(r, 0, k, k + 3) {
            k += 3;
        }
        let last = r.len() - 3;
        let mut j = last - 3;
        while j >= 3 && in_line_at(r, last, j, j - 3) {
            j -= 3;
        }
        let values = [
            r[0],
            r[1],
            r[k],
            r[k + 1],
            r[j],
            r[j + 1],
            r[last],
            r[last + 1],
        ];
        runs[ei * 8..ei * 8 + 8].copy_from_slice(&values);
    }
    let on_run = |ei: usize, end: u8, x: f64, y: f64| -> bool {
        let o = ei * 8 + if end == 1 { 0 } else { 4 };
        let x0 = runs[o];
        let y0 = runs[o + 1];
        let dx = runs[o + 2] - x0;
        let dy = runs[o + 3] - y0;
        let length = dx * dx + dy * dy;
        let t = if length > 0.0 {
            js_max(0.0, js_min(1.0, ((x - x0) * dx + (y - y0) * dy) / length))
        } else {
            0.0
        };
        js_hypot(x - x0 - t * dx, y - y0 - t * dy) < IN_LINE
    };

    let mut total = 0i32;
    for band in 0..bands {
        let band_end = band_start[band + 1];
        let band_begin = band_start[band];
        if band_end - band_begin > MAX_INSERT {
            in_band[band_begin..band_end].sort_by(|&p, &q| chain(ax[p] - ax[q], p.cmp(&q)));
        } else {
            for i in band_begin + 1..band_end {
                let p = in_band[i];
                let mut j = i;
                while j > band_begin && ax[in_band[j - 1]] > ax[p] {
                    in_band[j] = in_band[j - 1];
                    j -= 1;
                }
                in_band[j] = p;
            }
        }
        for s in band_begin..band_end {
            let p = in_band[s];
            let dx = bx[p] - ax[p];
            let dy = by[p] - ay[p];
            let here = first[p] == band;
            for &q in &in_band[s + 1..band_end] {
                if ax[q] > bx[p] {
                    break;
                }
                if high[q] < low[p]
                    || low[q] > high[p]
                    || owner[p] == owner[q]
                    || (!here && first[q] != band)
                {
                    continue;
                }
                let d1 = dx * (ay[q] - ay[p]) - dy * (ax[q] - ax[p]);
                let d2 = dx * (by[q] - ay[p]) - dy * (bx[q] - ax[p]);
                if (d1 > 0.0) == (d2 > 0.0) {
                    continue;
                }
                let ex = bx[q] - ax[q];
                let ey = by[q] - ay[q];
                let d3 = ex * (ay[p] - ay[q]) - ey * (ax[p] - ax[q]);
                let d4 = ex * (by[p] - ay[q]) - ey * (bx[p] - ax[q]);
                if (d3 > 0.0) == (d4 > 0.0) {
                    continue;
                }
                let ep = &edges[owner[p] as usize];
                let eq = &edges[owner[q] as usize];
                let at = d3 / (d3 - d4);
                let x = ax[p] + at * dx;
                let y = ay[p] + at * dy;
                let fan = |v: usize, pe: u8, qe: u8| -> bool {
                    ((ends[p] & pe) != 0 && (ends[q] & qe) != 0)
                        || ((x - nodes[v].x).abs() < nodes[v].w / 2.0 + FAN
                            && (y - nodes[v].y).abs() < nodes[v].h / 2.0 + FAN)
                        || (on_run(owner[p] as usize, pe, x, y)
                            && on_run(owner[q] as usize, qe, x, y))
                };
                if ep.src == eq.src && fan(ep.src, 1, 1) {
                    continue;
                }
                if ep.src == eq.dst && fan(ep.src, 1, 2) {
                    continue;
                }
                if ep.dst == eq.src && fan(ep.dst, 2, 1) {
                    continue;
                }
                if ep.dst == eq.dst && fan(ep.dst, 2, 2) {
                    continue;
                }
                total += 1;
            }
        }
    }
    total
}

pub(crate) fn inherit_crossed_directions(nodes: &mut [CNode], ends: &[(i32, i32)]) {
    let ancestors = |nodes: &[CNode], i: usize| -> Vec<i32> {
        let mut set: Vec<i32> = Vec::new();
        let mut p = nodes[i].parent;
        while p >= 0 && !set.contains(&p) {
            set.push(p);
            p = nodes[p as usize].parent;
        }
        set
    };
    for &(src, dst) in ends {
        if src < 0 || dst < 0 || src == dst {
            continue;
        }
        let a = ancestors(nodes, src as usize);
        let b = ancestors(nodes, dst as usize);
        for &g in &a {
            if !b.contains(&g) {
                nodes[g as usize].dir = None;
            }
        }
        for &g in &b {
            if !a.contains(&g) {
                nodes[g as usize].dir = None;
            }
        }
    }
}

pub(crate) fn shift_layout(nodes: &mut [CNode], edges: &mut [CEdge], ox: f64, oy: f64) {
    for c in nodes.iter_mut() {
        c.x += ox;
        c.y += oy;
    }
    for e in edges.iter_mut() {
        for k in (0..e.route.len()).step_by(3) {
            e.route[k] += ox;
            e.route[k + 1] += oy;
        }
        e.label_x += ox;
        e.label_y += oy;
    }
}

pub(crate) fn compound_layout(
    nodes: &mut [CNode],
    edges: &mut [CEdge],
    root_dir: Dir,
    opt: &LayeredOptions,
) -> CompoundResult {
    let n = nodes.len();
    let mut c = Compound {
        nodes,
        edges,
        opt,
        root_dir,
        n,
        levels: Vec::new(),
        level_of: HashMap::new(),
        order: Vec::new(),
        arena: Vec::new(),
        depth: vec![0; n],
        children: vec![Vec::new(); n],
        ordered: false,
        keyed: false,
        key: vec![f64::NAN; n],
        far: None,
        flat_cross: 0.0,
        laid: HashMap::new(),
        total: CompoundResult::default(),
    };
    c.run()
}

impl Compound<'_> {
    fn descend(&mut self, from: usize, placed: &mut [bool]) {
        let mut q = from;
        while q < self.order.len() {
            let li = self.order[q];
            let items = self.levels[li].items.clone();
            let level_id = self.levels[li].id;
            let level_dir = self.levels[li].dir;
            for item in items {
                placed[item] = true;
                self.depth[item] = if level_id < 0 {
                    0
                } else {
                    self.depth[level_id as usize] + 1
                };
                if self.nodes[item].is_group {
                    let dir = self.nodes[item].dir.unwrap_or(level_dir);
                    let child = self.make_level(item as i32, dir);
                    self.levels[child].items = self.children[item].clone();
                    self.order.push(child);
                }
            }
            q += 1;
        }
    }

    fn make_level(&mut self, id: i32, dir: Dir) -> usize {
        self.levels.push(Level {
            id,
            dir,
            items: Vec::new(),
            edges: Vec::new(),
            ports: Vec::new(),
            port_of: HashMap::new(),
            routes: HashMap::new(),
            x: 0.0,
            y: 0.0,
        });
        let idx = self.levels.len() - 1;
        self.level_of.insert(id, idx);
        idx
    }

    fn level(&self, id: i32) -> usize {
        self.level_of[&id]
    }

    fn ends_below(&self, item: usize, ei: usize, out: bool) -> bool {
        let e = &self.edges[ei];
        (if out { e.src } else { e.dst }) != item
    }

    fn port_lookup(&self, level_id: i32, ei: usize, out: bool) -> usize {
        let li = self.level(level_id);
        self.levels[li].port_of[&(ei * 2 + if out { 0 } else { 1 })]
    }

    fn set_port_side(&mut self, level_id: i32, ei: usize, out: bool, side: Side) {
        let pi = self.port_lookup(level_id, ei, out);
        self.arena[pi].side = side;
    }

    fn run(&mut self) -> CompoundResult {
        let n = self.n;
        let root_idx = self.make_level(-1, self.root_dir);
        let mut root_items = Vec::new();
        for i in 0..n {
            let p = self.nodes[i].parent;
            if p >= 0 && (p as usize) < n && p as usize != i && self.nodes[p as usize].is_group {
                self.children[p as usize].push(i);
            } else {
                self.nodes[i].parent = -1;
                root_items.push(i);
            }
        }
        self.levels[root_idx].items = root_items;
        self.order.push(root_idx);
        let mut placed = vec![false; n];
        self.descend(0, &mut placed);
        for i in 0..n {
            if placed[i] {
                continue;
            }
            let former = self.nodes[i].parent as usize;
            self.children[former].retain(|&c| c != i);
            self.nodes[i].parent = -1;
            self.levels[root_idx].items.push(i);
            placed[i] = true;
            self.depth[i] = 0;
            if self.nodes[i].is_group {
                let from = self.order.len();
                let dir = self.nodes[i].dir.unwrap_or(self.root_dir);
                let child = self.make_level(i as i32, dir);
                self.levels[child].items = self.children[i].clone();
                self.order.push(child);
                self.descend(from, &mut placed);
            }
        }

        for q in (1..self.order.len()).rev() {
            let li = self.order[q];
            let mut seq = f64::INFINITY;
            for &item in &self.levels[li].items {
                if self.nodes[item].seq < seq {
                    seq = self.nodes[item].seq;
                }
            }
            if seq != f64::INFINITY {
                let id = self.levels[li].id as usize;
                self.nodes[id].seq = seq;
            }
        }
        let order_levels = self.order.clone();
        for li in order_levels {
            let mut items = std::mem::take(&mut self.levels[li].items);
            let nodes = &*self.nodes;
            items.sort_by(|&a, &b| chain(nodes[a].seq - nodes[b].seq, a.cmp(&b)));
            self.levels[li].items = items;
        }

        let mut ports = 0usize;
        for ei in 0..self.edges.len() {
            let mut a = self.edges[ei].src;
            let mut b = self.edges[ei].dst;
            let mut crossed = 0usize;
            while self.depth[a] > self.depth[b] {
                a = self.nodes[a].parent as usize;
                crossed += 1;
            }
            while self.depth[b] > self.depth[a] {
                b = self.nodes[b].parent as usize;
                crossed += 1;
            }
            while a != b && self.nodes[a].parent != self.nodes[b].parent {
                a = self.nodes[a].parent as usize;
                b = self.nodes[b].parent as usize;
                crossed += 2;
            }
            if a == b {
                continue;
            }
            if ports + crossed > MAX_PORTS {
                self.edges[ei].src = a;
                self.edges[ei].dst = b;
            } else {
                ports += crossed;
            }
        }

        let mut loops: Vec<usize> = Vec::new();
        let mut stray: Vec<usize> = Vec::new();
        for ei in 0..self.edges.len() {
            self.edges[ei].route = Vec::new();
            if self.edges[ei].src == self.edges[ei].dst {
                loops.push(ei);
                continue;
            }
            let mut a = self.edges[ei].src;
            let mut b = self.edges[ei].dst;
            let mut a_path: Vec<usize> = Vec::new();
            let mut b_path: Vec<usize> = Vec::new();
            while self.depth[a] > self.depth[b] {
                a_path.push(a);
                a = self.nodes[a].parent as usize;
            }
            while self.depth[b] > self.depth[a] {
                b_path.push(b);
                b = self.nodes[b].parent as usize;
            }
            if a == b {
                stray.push(ei);
                continue;
            }
            while self.nodes[a].parent != self.nodes[b].parent {
                a_path.push(a);
                b_path.push(b);
                a = self.nodes[a].parent as usize;
                b = self.nodes[b].parent as usize;
            }
            let owner_level = self.level(self.nodes[a].parent);
            self.levels[owner_level]
                .edges
                .push(LevelEdge { e: ei, a, b });
            for (path, out) in [(a_path, true), (b_path, false)] {
                for inner in path {
                    let lv = self.level(self.nodes[inner].parent);
                    let pi = self.arena.len();
                    self.arena.push(PortEdge {
                        e: ei,
                        inner,
                        out,
                        side: Side::Top,
                        hint: f64::NAN,
                        px: 0.0,
                        py: 0.0,
                        route: Vec::new(),
                    });
                    self.levels[lv].ports.push(pi);
                    self.levels[lv]
                        .port_of
                        .insert(ei * 2 + if out { 0 } else { 1 }, pi);
                }
            }
        }

        self.ordered =
            ports > 1 && ports <= MAX_ORDERED_PORTS && self.edges.len() <= MAX_ORDERED_EDGES;
        let mut flat = self.order.len() > 1 && self.ordered;
        for &li in &self.order {
            if self.levels[li].dir != self.root_dir {
                flat = false;
            }
        }
        if flat {
            self.flat_layout();
        }

        let style = style_of(self.opt);
        self.keyed = flat;
        self.place();
        let mut crossings: i32 = -1;
        if flat {
            crossings = route_crossings(self.nodes, self.edges, style);
            let flat_crossings = self.flat_cross;
            if (crossings as f64) > flat_crossings * FLAT_SLACK + FLAT_SPARE {
                let first = snapshot(self);
                self.keyed = false;
                self.place();
                let own = route_crossings(self.nodes, self.edges, style);
                if own >= crossings {
                    self.keyed = true;
                    self.total = restore(self, &first);
                } else {
                    crossings = own;
                }
            }
        }

        if self.keyed == flat && self.ordered {
            let mut again = false;
            let order_levels = self.order.clone();
            for li in order_levels {
                let id = self.levels[li].id;
                if id < 0 {
                    continue;
                }
                let box_idx = id as usize;
                let vertical = matches!(self.levels[li].dir, Dir::TB | Dir::BT);
                let end = opposite(start_side(self.levels[li].dir));
                let mut seen: Vec<[f64; 2]> = Vec::new();
                let ports = self.levels[li].ports.clone();
                for pi in ports {
                    if self.arena[pi].side != end {
                        continue;
                    }
                    let e = self.arena[pi].e;
                    let out = self.arena[pi].out;
                    let route = self.edges[e].route.clone();
                    let bx = self.levels[li].x + self.arena[pi].px;
                    let by = self.levels[li].y + self.arena[pi].py;
                    let mut at: isize = -1;
                    let mut k = 0usize;
                    while k < route.len() && (at < 0 || out) {
                        if (route[k] - bx).abs() < 0.01 && (route[k + 1] - by).abs() < 0.01 {
                            at = k as isize;
                        }
                        k += 3;
                    }
                    if at < 0 {
                        continue;
                    }
                    let at = at as usize;
                    let a = if vertical { 0 } else { 1 };
                    let mut next: isize = if out {
                        at as isize + 3
                    } else {
                        at as isize - 3
                    };
                    while next >= 0
                        && (next as usize) < route.len()
                        && (route[next as usize + a] - route[at + a]).abs() < IN_LINE
                    {
                        next += if out { 3 } else { -3 };
                    }
                    if next < 0 || next as usize >= route.len() {
                        continue;
                    }
                    let next = next as usize;
                    let box_origin = if vertical {
                        self.nodes[box_idx].x - self.nodes[box_idx].w / 2.0
                    } else {
                        self.nodes[box_idx].y - self.nodes[box_idx].h / 2.0
                    };
                    let hint = route[next + a] - box_origin;
                    self.arena[pi].hint = hint;
                    let along = if vertical {
                        self.arena[pi].px
                    } else {
                        self.arena[pi].py
                    };
                    seen.push([along, hint]);
                }
                seen.sort_by(|p, q| chain(p[0] - q[0], std::cmp::Ordering::Equal));
                for i in 1..seen.len() {
                    if seen[i - 1][1] > seen[i][1] {
                        again = true;
                    }
                }
            }
            if again {
                let before = if crossings >= 0 {
                    crossings
                } else {
                    route_crossings(self.nodes, self.edges, style)
                };
                if before > 0 {
                    let first = snapshot(self);
                    self.place();
                    if route_crossings(self.nodes, self.edges, style) >= before {
                        self.total = restore(self, &first);
                    }
                }
            }
        }

        for &ei in &stray {
            let (ax, ay) = (
                self.nodes[self.edges[ei].src].x,
                self.nodes[self.edges[ei].src].y,
            );
            let (bx, by) = (
                self.nodes[self.edges[ei].dst].x,
                self.nodes[self.edges[ei].dst].y,
            );
            self.edges[ei].route = vec![ax, ay, 0.0, bx, by, 0.0];
            self.edges[ei].label_x = (ax + bx) / 2.0;
            self.edges[ei].label_y = (ay + by) / 2.0;
        }
        for &ei in &loops {
            let node = &self.nodes[self.edges[ei].src];
            let (nx, ny, nw) = (node.x, node.y, node.w);
            self.edges[ei].label_x = nx + nw / 2.0 + 28.0 + self.edges[ei].label_w / 2.0;
            self.edges[ei].label_y = ny;
        }

        self.total.clone()
    }
}

fn kind_index(kind: Kind) -> f64 {
    match kind {
        Kind::Node => 0.0,
        Kind::Dummy => 1.0,
        Kind::Label => 2.0,
        Kind::StartPort => 3.0,
        Kind::EndPort => 4.0,
    }
}

fn append_route(route: &mut Vec<f64>, part: &[f64], ox: f64, oy: f64) {
    for i in (0..part.len()).step_by(3) {
        let x = part[i] + ox;
        let y = part[i + 1] + oy;
        let len = route.len();
        if len >= 3 && (route[len - 3] - x).abs() < 0.01 && (route[len - 2] - y).abs() < 0.01 {
            continue;
        }
        route.push(x);
        route.push(y);
        route.push(part[i + 2]);
    }
}

impl Compound<'_> {
    fn flat_layout(&mut self) {
        let vertical = matches!(self.root_dir, Dir::TB | Dir::BT);
        let n = self.n;
        let mut leaf = vec![-1i64; n];
        let mut group = vec![-1i64; n];
        let mut lnodes: Vec<LNode> = Vec::new();
        let mut of: Vec<i32> = Vec::new();
        let mut parent: Vec<i32> = Vec::new();
        for i in 0..n {
            if self.nodes[i].is_group {
                group[i] = parent.len() as i64;
                parent.push(-1);
            }
        }
        for i in 0..n {
            let p = self.nodes[i].parent;
            if self.nodes[i].is_group {
                parent[group[i] as usize] = if p >= 0 { group[p as usize] as i32 } else { -1 };
            } else {
                leaf[i] = lnodes.len() as i64;
                let (w, h) = (self.nodes[i].w, self.nodes[i].h);
                lnodes.push(lnode(
                    if vertical { w } else { h },
                    if vertical { h } else { w },
                    Kind::Node,
                ));
                of.push(if p >= 0 { group[p as usize] as i32 } else { -1 });
            }
        }
        let mut ledges: Vec<LEdge> = Vec::new();
        let mut ledge_of = vec![-1i64; self.edges.len()];
        for ei in 0..self.edges.len() {
            let e = &self.edges[ei];
            if e.src == e.dst || leaf[e.src] < 0 || leaf[e.dst] < 0 {
                continue;
            }
            ledge_of[ei] = ledges.len() as i64;
            ledges.push(ledge(
                leaf[e.src] as usize,
                leaf[e.dst] as usize,
                e.minlen,
                if vertical { e.label_w } else { e.label_h },
                if vertical { e.label_h } else { e.label_w },
            ));
        }
        let mut flat_opts = self.opt.clone();
        flat_opts.groups = Some(super::Groups { of, parent });
        flat_opts.order_only = true;
        let flat = layered(&mut lnodes, &mut ledges, &flat_opts);
        self.flat_cross = js_min(
            flat.crossings.unwrap_or(0) as f64,
            flat.declared.unwrap_or(0) as f64,
        );
        let mut sum = vec![0.0f64; n];
        let mut count = vec![0i64; n];
        let mut last = vec![-1i32; n];
        for i in 0..n {
            if leaf[i] < 0 {
                continue;
            }
            let li = leaf[i] as usize;
            self.key[i] = lnodes[li].x;
            let mut p = self.nodes[i].parent;
            while p >= 0 {
                let pu = p as usize;
                sum[pu] += self.key[i];
                count[pu] += 1;
                last[pu] = last[pu].max(lnodes[li].rank as i32);
                p = self.nodes[pu].parent;
            }
        }
        for i in 0..n {
            if count[i] > 0 {
                self.key[i] = sum[i] / count[i] as f64;
            }
        }
        self.far = Some(FarHint {
            ledge_of,
            ledges,
            last,
        });
    }

    fn far_hint(&self, li: usize, pi: usize) -> f64 {
        let far = self.far.as_ref().expect("far hint needs the flat layout");
        let port = &self.arena[pi];
        let level = &self.levels[li];
        let le = far.ledge_of[port.e];
        if le < 0 || level.id < 0 || !port.out || port.side != opposite(start_side(level.dir)) {
            return f64::NAN;
        }
        let points = &far.ledges[le as usize].points;
        let last = far.last[level.id as usize] as f64;
        let mut inside = self.key[self.edges[port.e].src];
        let mut outside = 0.0f64;
        for k in (0..points.len()).step_by(2) {
            if points[k] == last {
                inside = points[k + 1];
            }
            if points[k] == last + 1.0 {
                outside = points[k + 1];
            }
        }
        inside + outside * 1e-6
    }

    fn build(&self, li: usize, sized: bool) -> Built {
        let level = &self.levels[li];
        let vertical = matches!(level.dir, Dir::TB | Dir::BT);
        let start = start_side(level.dir);
        let mut local = vec![0usize; self.n];
        let mut lnodes: Vec<LNode> = Vec::new();
        for (i, &item) in level.items.iter().enumerate() {
            local[item] = i;
            let node = &self.nodes[item];
            let mut ln = if sized {
                lnode(
                    if vertical { node.w } else { node.h },
                    if vertical { node.h } else { node.w },
                    Kind::Node,
                )
            } else {
                lnode(0.0, 0.0, Kind::Node)
            };
            ln.span = if node.is_group {
                js_min(node.span, ln.w - 2.0 * node.pad_x)
            } else {
                node.span
            };
            ln.key = if self.keyed { self.key[item] } else { f64::NAN };
            lnodes.push(ln);
        }
        let held = |item: usize, ei: usize, out: bool| {
            self.nodes[item].is_group && self.ends_below(item, ei, out)
        };
        let offset = |item: usize, ei: usize, out: bool| -> f64 {
            if !sized || !self.nodes[item].is_group || !self.ends_below(item, ei, out) {
                return 0.0;
            }
            let pi = self.port_lookup(item as i32, ei, out);
            let port = &self.arena[pi];
            if vertical {
                port.px - self.nodes[item].w / 2.0
            } else {
                port.py - self.nodes[item].h / 2.0
            }
        };
        let mut ledges: Vec<LEdge> = Vec::new();
        for le in level.edges.iter() {
            let e = &self.edges[le.e];
            let mut edge = ledge(local[le.a], local[le.b], e.minlen, 0.0, 0.0);
            if sized {
                edge.label_w = if vertical { e.label_w } else { e.label_h };
                edge.label_h = if vertical { e.label_h } else { e.label_w };
                edge.tail_dx = offset(le.a, le.e, true);
                edge.head_dx = offset(le.b, le.e, false);
                edge.tail_fixed = held(le.a, le.e, true);
                edge.head_fixed = held(le.b, le.e, false);
            }
            ledges.push(edge);
        }
        let mut dummies = vec![-1i64; level.ports.len()];
        for (i, &pi) in level.ports.iter().enumerate() {
            let port = &self.arena[pi];
            if port.side == start || port.side == opposite(start) {
                let d = lnodes.len();
                let kind = if port.side == start {
                    Kind::StartPort
                } else {
                    Kind::EndPort
                };
                let mut ln = lnode(0.0, 0.0, kind);
                ln.key = if self.keyed {
                    let endpoint = if port.out {
                        self.edges[port.e].dst
                    } else {
                        self.edges[port.e].src
                    };
                    self.key[endpoint]
                } else {
                    f64::NAN
                };
                ln.hint = if port.hint.is_nan() && self.keyed && self.far.is_some() {
                    self.far_hint(li, pi)
                } else {
                    port.hint
                };
                lnodes.push(ln);
                dummies[i] = d as i64;
                let mut edge = if port.out {
                    ledge(local[port.inner], d, 1.0, 0.0, 0.0)
                } else {
                    ledge(d, local[port.inner], 1.0, 0.0, 0.0)
                };
                if port.out {
                    edge.tail_dx = offset(port.inner, port.e, true);
                    edge.tail_fixed = held(port.inner, port.e, true);
                } else {
                    edge.head_dx = offset(port.inner, port.e, false);
                    edge.head_fixed = held(port.inner, port.e, false);
                }
                ledges.push(edge);
            } else {
                lnodes[local[port.inner]].pin = if port.side == Side::Top || port.side == Side::Left
                {
                    -1
                } else {
                    1
                };
            }
        }
        Built {
            lnodes,
            ledges,
            dummies,
        }
    }

    fn lay_out(
        &mut self,
        li: usize,
        lnodes: Vec<LNode>,
        ledges: Vec<LEdge>,
    ) -> (Vec<LNode>, Vec<LEdge>, LayeredResult) {
        let mut lnodes = lnodes;
        let mut ledges = ledges;
        if !self.ordered {
            let size = layered(&mut lnodes, &mut ledges, self.opt);
            return (lnodes, ledges, size);
        }
        let id = self.levels[li].id;
        let mut given: Vec<f64> = Vec::new();
        for ln in &lnodes {
            given.extend([
                ln.w,
                ln.h,
                kind_index(ln.kind),
                ln.pin as f64,
                ln.hint,
                ln.key,
                ln.span,
            ]);
        }
        for le in &ledges {
            given.extend([
                le.tail as f64,
                le.head as f64,
                le.minlen,
                le.label_w,
                le.label_h,
                le.tail_dx,
                le.head_dx,
                if le.tail_fixed { 1.0 } else { 0.0 },
                if le.head_fixed { 1.0 } else { 0.0 },
            ]);
        }
        if let Some(past) = self.laid.get(&id) {
            for p in past {
                let same = p.given.len() == given.len()
                    && p.given.iter().zip(given.iter()).all(|(a, b)| js_eq(*a, *b));
                if same {
                    return (p.lnodes.clone(), p.ledges.clone(), p.size.clone());
                }
            }
        }
        let size = layered(&mut lnodes, &mut ledges, self.opt);
        self.laid.entry(id).or_default().push(Laid {
            given,
            lnodes: lnodes.clone(),
            ledges: ledges.clone(),
            size: size.clone(),
        });
        (lnodes, ledges, size)
    }

    fn place(&mut self) {
        let order = self.order.clone();
        for &li in &order {
            let ports = self.levels[li].ports.clone();
            for pi in ports {
                let (inner, e, out, side) = {
                    let p = &self.arena[pi];
                    (p.inner, p.e, p.out, p.side)
                };
                if self.nodes[inner].is_group && self.ends_below(inner, e, out) {
                    self.set_port_side(inner as i32, e, out, side);
                }
            }
            if self.levels[li].edges.is_empty() {
                continue;
            }
            let needed = self.levels[li].edges.iter().any(|le| {
                let e = &self.edges[le.e];
                (self.nodes[le.a].is_group && e.src != le.a)
                    || (self.nodes[le.b].is_group && e.dst != le.b)
            });
            if !needed {
                continue;
            }
            let dir = self.levels[li].dir;
            let built = self.build(li, false);
            let mut lnodes = built.lnodes;
            let mut ledges = built.ledges;
            orient(&lnodes, &mut ledges);
            rank(&mut lnodes, &ledges, 1);
            let start = start_side(dir);
            let level_edges = self.levels[li].edges.clone();
            for (i, le) in level_edges.iter().enumerate() {
                let (src, dst) = (self.edges[le.e].src, self.edges[le.e].dst);
                let a_above = !ledges[i].reversed;
                if self.nodes[le.a].is_group && src != le.a {
                    let side = if a_above { opposite(start) } else { start };
                    self.set_port_side(le.a as i32, le.e, true, side);
                }
                if self.nodes[le.b].is_group && dst != le.b {
                    let side = if a_above { start } else { opposite(start) };
                    self.set_port_side(le.b as i32, le.e, false, side);
                }
            }
        }

        for q in (0..order.len()).rev() {
            let li = order[q];
            let group = self.levels[li].id;
            let dir = self.levels[li].dir;
            let vertical = matches!(dir, Dir::TB | Dir::BT);
            let flip = matches!(dir, Dir::BT | Dir::RL);
            let built = self.build(li, true);
            let Built {
                lnodes: built_nodes,
                ledges: built_edges,
                dummies,
            } = built;
            let (lnodes, ledges, size) = self.lay_out(li, built_nodes, built_edges);
            let content_w = if vertical { size.width } else { size.height };
            let content_h = if vertical { size.height } else { size.width };
            let (pad_x, pad_top, pad_bottom, min_w, group_idx) = if group >= 0 {
                let g = &self.nodes[group as usize];
                (
                    g.pad_x,
                    g.pad_top,
                    g.pad_bottom,
                    g.min_w,
                    Some(group as usize),
                )
            } else {
                (0.0, 0.0, 0.0, 0.0, None)
            };
            let box_w = js_max(content_w + 2.0 * pad_x, min_w);
            let box_h = content_h + pad_top + pad_bottom;
            let off_x = (box_w - content_w) / 2.0;
            let off_y = pad_top;
            let tx = |lx: f64, ly: f64| {
                off_x
                    + if vertical {
                        lx
                    } else if flip {
                        size.height - ly
                    } else {
                        ly
                    }
            };
            let ty = |lx: f64, ly: f64| {
                off_y
                    + if vertical {
                        if flip { size.height - ly } else { ly }
                    } else {
                        lx
                    }
            };
            let axis = if vertical { 0.0 } else { 1.0 };
            let convert = |points: &[f64]| -> Vec<f64> {
                let mut out = Vec::with_capacity(points.len() / 2 * 3);
                for i in (0..points.len()).step_by(2) {
                    out.push(tx(points[i], points[i + 1]));
                    out.push(ty(points[i], points[i + 1]));
                    out.push(axis);
                }
                out
            };

            let items = self.levels[li].items.clone();
            for (i, &item) in items.iter().enumerate() {
                self.nodes[item].x = tx(lnodes[i].x, lnodes[i].y);
                self.nodes[item].y = ty(lnodes[i].x, lnodes[i].y);
            }
            let level_edges = self.levels[li].edges.clone();
            for (i, le) in level_edges.iter().enumerate() {
                let edge = &ledges[i];
                let route = convert(&edge.points);
                self.levels[li].routes.insert(le.e, route);
                if self.edges[le.e].label_w > 0.0 {
                    self.edges[le.e].label_x = tx(edge.label_x, edge.label_y);
                    self.edges[le.e].label_y = ty(edge.label_x, edge.label_y);
                }
            }
            let mut k = level_edges.len();
            let ports = self.levels[li].ports.clone();
            for (i, &pi) in ports.iter().enumerate() {
                let (inner, e, out, side) = {
                    let p = &self.arena[pi];
                    (p.inner, p.e, p.out, p.side)
                };
                if dummies[i] >= 0 {
                    let mut route = convert(&ledges[k].points);
                    k += 1;
                    let at = if out { route.len() - 3 } else { 0 };
                    let mut bx = route[at];
                    let mut by = route[at + 1];
                    match side {
                        Side::Top => by = 0.0,
                        Side::Bottom => by = box_h,
                        Side::Left => bx = 0.0,
                        Side::Right => bx = box_w,
                    }
                    if out {
                        route.extend([bx, by, axis]);
                    } else {
                        let mut front = vec![bx, by, axis];
                        front.append(&mut route);
                        route = front;
                    }
                    self.arena[pi].px = bx;
                    self.arena[pi].py = by;
                    self.arena[pi].route = route;
                    continue;
                }
                let (node_x, node_y, node_w, node_h, node_group) = {
                    let node = &self.nodes[inner];
                    (node.x, node.y, node.w, node.h, node.is_group)
                };
                let mut ix = node_x;
                let mut iy = node_y;
                if node_group && self.ends_below(inner, e, out) {
                    let inner_port = self.port_lookup(inner as i32, e, out);
                    ix += self.arena[inner_port].px - node_w / 2.0;
                    iy += self.arena[inner_port].py - node_h / 2.0;
                } else if side == Side::Top {
                    iy -= node_h / 2.0;
                } else if side == Side::Bottom {
                    iy += node_h / 2.0;
                } else if side == Side::Left {
                    ix -= node_w / 2.0;
                } else {
                    ix += node_w / 2.0;
                }
                let mut bx = ix;
                let mut by = iy;
                match side {
                    Side::Top => by = 0.0,
                    Side::Bottom => by = box_h,
                    Side::Left => bx = 0.0,
                    Side::Right => bx = box_w,
                }
                let a = if side == Side::Top || side == Side::Bottom {
                    0.0
                } else {
                    1.0
                };
                self.arena[pi].px = bx;
                self.arena[pi].py = by;
                self.arena[pi].route = if out {
                    vec![ix, iy, a, bx, by, a]
                } else {
                    vec![bx, by, a, ix, iy, a]
                };
            }

            match group_idx {
                Some(g) => {
                    self.nodes[g].w = box_w;
                    self.nodes[g].h = box_h;
                }
                None => {
                    self.total = CompoundResult {
                        width: box_w,
                        height: box_h,
                    };
                }
            }
        }

        for &li in &order {
            let (ox, oy) = (self.levels[li].x, self.levels[li].y);
            let items = self.levels[li].items.clone();
            for item in items {
                self.nodes[item].x += ox;
                self.nodes[item].y += oy;
                if self.nodes[item].is_group {
                    let child = self.level(item as i32);
                    let (cx, cy) = (
                        self.nodes[item].x - self.nodes[item].w / 2.0,
                        self.nodes[item].y - self.nodes[item].h / 2.0,
                    );
                    self.levels[child].x = cx;
                    self.levels[child].y = cy;
                }
            }
        }

        for &li in &order {
            let level_id = self.levels[li].id;
            let level_edges = self.levels[li].edges.clone();
            for le in level_edges {
                let (src, dst) = (self.edges[le.e].src, self.edges[le.e].dst);
                let mut route: Vec<f64> = Vec::new();
                let mut outward: Vec<usize> = Vec::new();
                let mut item = src;
                while self.nodes[item].parent != level_id {
                    outward.push(self.level(self.nodes[item].parent));
                    item = self.nodes[item].parent as usize;
                }
                for lv in outward {
                    let pi = self.levels[lv].port_of[&(le.e * 2)];
                    let part = self.arena[pi].route.clone();
                    let (ox, oy) = (self.levels[lv].x, self.levels[lv].y);
                    append_route(&mut route, &part, ox, oy);
                }
                let part = self.levels[li].routes[&le.e].clone();
                let (ox, oy) = (self.levels[li].x, self.levels[li].y);
                append_route(&mut route, &part, ox, oy);
                let mut inward: Vec<usize> = Vec::new();
                let mut item = dst;
                while self.nodes[item].parent != level_id {
                    inward.push(self.level(self.nodes[item].parent));
                    item = self.nodes[item].parent as usize;
                }
                for &lv in inward.iter().rev() {
                    let pi = self.levels[lv].port_of[&(le.e * 2 + 1)];
                    let part = self.arena[pi].route.clone();
                    let (ox, oy) = (self.levels[lv].x, self.levels[lv].y);
                    append_route(&mut route, &part, ox, oy);
                }
                self.edges[le.e].route = route;
                if self.edges[le.e].label_w > 0.0 {
                    self.edges[le.e].label_x += self.levels[li].x;
                    self.edges[le.e].label_y += self.levels[li].y;
                }
            }
        }
    }
}
