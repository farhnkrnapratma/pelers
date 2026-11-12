use super::layered::{chain, orient, rank};
use super::{CEdge, CNode, CompoundResult, Dir, Kind, LEdge, LNode, LayeredOptions, ledge, lnode};
use crate::util::jsnum::{js_max, js_min, js_round};

const LANE_HEAD: f64 = 34.0;
pub(crate) const LANE_PAD: f64 = 24.0;
const SWEEPS: usize = 8;
const MAX_DUMMIES: i64 = 20000;
const MAX_HOPS: usize = 10000;
const ROUTE_BUDGET: f64 = 2000000.0;
const NEAR: f64 = 4.0;
const ROUNDS: usize = 3;
const RELAY: f64 = 8.0;
const TURN: f64 = 20.0;
const MARGIN: f64 = 20.0;
const LEG: f64 = 14.0;
const CLOSE: f64 = 3.0;

fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn settle(row: &mut [usize], lane: &[i32], key: &[f64], pos: &mut [f64]) {
    row.sort_by(|&a, &b| {
        chain(
            (lane[a] - lane[b]) as f64,
            chain(key[a] - key[b], a.cmp(&b)),
        )
    });
    for (k, &item) in row.iter().enumerate() {
        pos[item] = k as f64;
    }
}

fn run_bounds(row: &[usize], lane: &[i32]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut from = 0usize;
    while from < row.len() {
        let mut to = from;
        while to + 1 < row.len() && lane[row[to + 1]] == lane[row[from]] {
            to += 1;
        }
        runs.push((from, to));
        from = to + 1;
    }
    runs
}

fn border(from: &CNode, to: &CNode) -> (f64, f64) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let hw = js_max(from.w / 2.0, 0.5);
    let hh = js_max(from.h / 2.0, 0.5);
    if dx.abs() < hw && dy.abs() < hh {
        let to_side = hw - dx.abs();
        let to_end = hh - dy.abs();
        return if to_side < to_end {
            (from.x + if dx < 0.0 { -hw } else { hw }, to.y)
        } else {
            (to.x, from.y + if dy < 0.0 { -hh } else { hh })
        };
    }
    let scale = js_min(
        if dx != 0.0 {
            hw / dx.abs()
        } else {
            f64::INFINITY
        },
        if dy != 0.0 {
            hh / dy.abs()
        } else {
            f64::INFINITY
        },
    );
    (from.x + dx * scale, from.y + dy * scale)
}

fn grow(boxes: &mut [[f64; 4]], g: usize, node: &CNode) {
    let b = &mut boxes[g];
    if node.x - node.w / 2.0 < b[0] {
        b[0] = node.x - node.w / 2.0;
    }
    if node.y - node.h / 2.0 < b[1] {
        b[1] = node.y - node.h / 2.0;
    }
    if node.x + node.w / 2.0 > b[2] {
        b[2] = node.x + node.w / 2.0;
    }
    if node.y + node.h / 2.0 > b[3] {
        b[3] = node.y + node.h / 2.0;
    }
}

struct Layout<'a> {
    nodes: &'a mut [CNode],
    edges: &'a mut [CEdge],
    opt: &'a LayeredOptions,
    lane_pad: f64,
    sideways: bool,
    flipped: bool,
    lanes: Vec<usize>,
    lane_count: usize,
    lane_pads: Vec<f64>,
    end_pad: f64,
    lane_slot: Vec<i64>,
    item_of: Vec<i64>,
    real: Vec<usize>,
    lnodes: Vec<LNode>,
    ledges: Vec<LEdge>,
    edge_of: Vec<usize>,
    step: i64,
    item_lane: Vec<i32>,
    n: usize,
}

struct PlaceOut {
    width: f64,
    height: f64,
    routed: Vec<bool>,
    margins: Vec<bool>,
    work: f64,
}

struct Geo {
    c: Vec<f64>,
    size: Vec<f64>,
    depth: Vec<f64>,
    rnk: Vec<usize>,
    band_start: Vec<f64>,
    band_end: Vec<f64>,
    real_len: usize,
    edges: Vec<(usize, usize, f64, f64)>,
    ledge_dims: Vec<(f64, f64, usize, usize)>,
    sideways: bool,
    flipped: bool,
    length: f64,
    breadth_total: f64,
    head: f64,
    axis: f64,
}

struct RouteState {
    routes: Vec<Vec<f64>>,
    labels: Vec<(f64, f64)>,
    label_out: Vec<(f64, f64)>,
    routed: Vec<bool>,
    used: Vec<bool>,
}

impl Geo {
    fn amid(&self, i: usize) -> f64 {
        (self.band_start[self.rnk[i]] + self.band_end[self.rnk[i]]) / 2.0
    }

    fn flip(&self, a: f64) -> f64 {
        if self.flipped {
            self.head + self.length - a
        } else {
            a
        }
    }

    fn px(&self, cc: f64, a: f64) -> f64 {
        if self.sideways { self.flip(a) } else { cc }
    }

    fn py(&self, cc: f64, a: f64) -> f64 {
        if self.sideways { cc } else { self.flip(a) }
    }

    fn overlaps(span: (f64, f64, f64, f64), label: (f64, f64), size: (f64, f64)) -> bool {
        let (c0, a0, c1, a1) = span;
        let (lc, la) = label;
        let (cw, ah) = size;
        c1 > lc - cw / 2.0 && c0 < lc + cw / 2.0 && a1 > la - ah / 2.0 && a0 < la + ah / 2.0
    }

    fn clear(
        &self,
        bounds: (f64, f64, f64, f64),
        src: usize,
        dst: usize,
        ei: usize,
        labels: &[(f64, f64)],
    ) -> bool {
        let (c0, a0, c1, a1) = bounds;
        for i in 0..self.real_len {
            if i != src
                && i != dst
                && Self::overlaps(
                    (c0, a0, c1, a1),
                    (self.c[i], self.amid(i)),
                    (self.size[i] + 8.0, self.depth[i] + 8.0),
                )
            {
                return false;
            }
        }
        for i in 0..self.edges.len() {
            let (esrc, edst, lw, lh) = self.edges[i];
            if i != ei && lw > 0.0 && esrc != edst {
                let (cw, ah) = if self.sideways { (lh, lw) } else { (lw, lh) };
                if Self::overlaps((c0, a0, c1, a1), (labels[i].0, labels[i].1), (cw, ah)) {
                    return false;
                }
            }
        }
        true
    }

    fn crossings(&self, bounds: (f64, f64, f64, f64), ei: usize, routes: &[Vec<f64>]) -> i32 {
        let (x0, y0, x1, y1) = bounds;
        let mut count = 0;
        for (i, route) in routes.iter().enumerate() {
            if i == ei {
                continue;
            }
            for j in (3..route.len()).step_by(3) {
                if js_max(route[j - 3], route[j]) > x0
                    && js_min(route[j - 3], route[j]) < x1
                    && js_max(route[j - 2], route[j + 1]) > y0
                    && js_min(route[j - 2], route[j + 1]) < y1
                {
                    count += 1;
                    break;
                }
            }
        }
        count
    }

    fn meets(&self, x: f64, y: f64, ei: usize, routes: &[Vec<f64>]) -> i32 {
        let mut count = 0;
        for (i, route) in routes.iter().enumerate() {
            if i == ei || route.is_empty() {
                continue;
            }
            let last = route.len() - 3;
            if ((route[0] - x).abs() < 1.0 && (route[1] - y).abs() < 1.0)
                || ((route[last] - x).abs() < 1.0 && (route[last + 1] - y).abs() < 1.0)
            {
                count += 1;
            }
        }
        count
    }

    fn retraces(&self, seg: (f64, f64, f64, f64), ei: usize, routes: &[Vec<f64>]) -> bool {
        let (x0, y0, x1, y1) = seg;
        let upright = (x1 - x0).abs() < 0.5;
        let lo = if upright {
            js_min(y0, y1)
        } else {
            js_min(x0, x1)
        };
        let hi = if upright {
            js_max(y0, y1)
        } else {
            js_max(x0, x1)
        };
        let line = if upright { x0 } else { y0 };
        let (esrc, edst) = (self.edges[ei].0, self.edges[ei].1);
        for (i, route) in routes.iter().enumerate() {
            let (osrc, odst, _, _) = self.edges[i];
            if i == ei || osrc == esrc || odst == edst {
                continue;
            }
            for j in (3..route.len()).step_by(3) {
                let u0 = route[j - 3 + if upright { 0 } else { 1 }];
                let u1 = route[j + if upright { 0 } else { 1 }];
                if (u0 - line).abs() >= NEAR || (u1 - line).abs() >= NEAR {
                    continue;
                }
                let v0 = route[j - 3 + if upright { 1 } else { 0 }];
                let v1 = route[j + if upright { 1 } else { 0 }];
                if js_min(hi, js_max(v0, v1)) - js_max(lo, js_min(v0, v1)) > 1.0 {
                    return true;
                }
            }
        }
        false
    }

    fn attempt(
        &self,
        k: usize,
        ei: usize,
        ends: (usize, usize),
        pts: &[f64],
        margin: f64,
        st: &mut RouteState,
    ) -> bool {
        let (a, b) = ends;
        for p in (2..pts.len()).step_by(2) {
            let bounds = (
                js_min(pts[p - 2], pts[p]) - margin,
                js_min(pts[p - 1], pts[p + 1]) - margin,
                js_max(pts[p - 2], pts[p]) + margin,
                js_max(pts[p - 1], pts[p + 1]) + margin,
            );
            if !self.clear(bounds, a, b, ei, &st.labels) {
                return false;
            }
            let seg = (
                self.px(pts[p - 2], pts[p - 1]),
                self.py(pts[p - 2], pts[p - 1]),
                self.px(pts[p], pts[p + 1]),
                self.py(pts[p], pts[p + 1]),
            );
            if self.retraces(seg, ei, &st.routes) {
                return false;
            }
        }
        let mut lc = 0.0f64;
        let mut la = 0.0f64;
        let mut longest = -1.0f64;
        let (out_w, out_h) = (self.edges[ei].2, self.edges[ei].3);
        if out_w > 0.0 {
            let (cw, ah) = (self.ledge_dims[k].0, self.ledge_dims[k].1);
            for p in (2..pts.len()).step_by(2) {
                let cc = (pts[p - 2] + pts[p]) / 2.0;
                let ca = (pts[p - 1] + pts[p + 1]) / 2.0;
                let across = (pts[p] - pts[p - 2]).abs();
                let along = (pts[p + 1] - pts[p - 1]).abs();
                let run = across + along;
                if run <= longest
                    || (if across > 0.0 {
                        across < cw + 8.0
                    } else {
                        along < ah + 8.0
                    })
                {
                    continue;
                }
                if cc - cw / 2.0 < 0.0
                    || cc + cw / 2.0 > self.breadth_total
                    || ca - ah / 2.0 < 0.0
                    || ca + ah / 2.0 > self.length
                {
                    continue;
                }
                if !self.clear(
                    (cc - cw / 2.0, ca - ah / 2.0, cc + cw / 2.0, ca + ah / 2.0),
                    usize::MAX,
                    usize::MAX,
                    ei,
                    &st.labels,
                ) {
                    continue;
                }
                let lx = self.px(cc, ca);
                let ly = self.py(cc, ca);
                if self.crossings(
                    (
                        lx - out_w / 2.0,
                        ly - out_h / 2.0,
                        lx + out_w / 2.0,
                        ly + out_h / 2.0,
                    ),
                    ei,
                    &st.routes,
                ) > 0
                {
                    continue;
                }
                lc = cc;
                la = ca;
                longest = run;
            }
            if longest < 0.0 {
                return false;
            }
        }
        let mut way = vec![pts[0], pts[1]];
        for p in (2..pts.len()).step_by(2) {
            let dx = pts[p] - pts[p - 2];
            let dy = pts[p + 1] - pts[p - 1];
            let run = dx.abs() + dy.abs();
            let first = if p > 2 { TURN } else { 0.0 };
            let last = if p + 2 < pts.len() { TURN } else { 0.0 };
            if run > first + last + 1.0 {
                if first != 0.0 {
                    way.push(pts[p - 2] + (dx / run) * first);
                    way.push(pts[p - 1] + (dy / run) * first);
                }
                if last != 0.0 {
                    way.push(pts[p] - (dx / run) * last);
                    way.push(pts[p + 1] - (dy / run) * last);
                }
            }
            way.push(pts[p]);
            way.push(pts[p + 1]);
        }
        let mut route: Vec<f64> = Vec::new();
        for p in (0..way.len()).step_by(2) {
            let crosswise = if p == 0 {
                way[0] != way[2]
            } else {
                way[p] != way[p - 2]
            };
            route.push(self.px(way[p], way[p + 1]));
            route.push(self.py(way[p], way[p + 1]));
            route.push(if crosswise {
                1.0 - self.axis
            } else {
                self.axis
            });
        }
        st.routes[ei] = route;
        if out_w > 0.0 {
            st.label_out[ei] = (self.px(lc, la), self.py(lc, la));
            st.labels[ei] = (lc, la);
        }
        st.routed[k] = true;
        true
    }
}

impl Layout<'_> {
    fn lane_of(&mut self, i: usize) -> usize {
        let mut t = i;
        let mut room = self.lane_pad;
        let mut hops = 0usize;
        while self.nodes[t].parent >= 0 && hops < MAX_HOPS {
            t = self.nodes[t].parent as usize;
            if self.lane_slot[t] < 0 {
                let node = &self.nodes[t];
                room += js_max(js_max(node.pad_x, node.pad_top), node.pad_bottom);
            }
            hops += 1;
        }
        let lane = self.lane_slot[t].max(0) as usize;
        if room > self.lane_pads[lane] {
            self.lane_pads[lane] = room;
        }
        if room > self.end_pad {
            self.end_pad = room;
        }
        lane
    }

    fn place(&mut self, plain: &[bool], margins: &[bool]) -> PlaceOut {
        let m = self.ledges.len();
        let lane_count = self.lane_count;
        let pad: Vec<f64> = (0..lane_count)
            .map(|lane| {
                if margins[lane] {
                    js_max(self.lane_pads[lane], MARGIN)
                } else {
                    self.lane_pads[lane]
                }
            })
            .collect();
        let used = vec![false; lane_count];
        let span: Vec<usize> = (0..m)
            .map(|k| {
                self.lnodes[self.ledges[k].head]
                    .rank
                    .abs_diff(self.lnodes[self.ledges[k].tail].rank)
            })
            .collect();
        let mut dummies: i64 = 0;
        for k in 0..m {
            if !plain[k] {
                dummies += span[k] as i64 - 1;
            }
        }
        let direct = dummies > MAX_DUMMIES;
        let real_len = self.real.len();
        let total = real_len + if direct { 0 } else { dummies as usize };
        let mut rnk = vec![0usize; total];
        let mut lane_of_item = vec![0i32; total];
        let mut size = vec![0.0f64; total];
        let mut depth = vec![0.0f64; total];
        let mut dummy = vec![false; total];
        let mut max_rank = 0usize;
        for k in 0..real_len {
            rnk[k] = self.lnodes[k].rank;
            lane_of_item[k] = self.item_lane[k];
            size[k] = self.lnodes[k].w;
            depth[k] = self.lnodes[k].h;
            if rnk[k] > max_rank {
                max_rank = rnk[k];
            }
        }
        let mut first_dummy = vec![-1i64; m];
        let mut label_item = vec![-1i64; m];
        let mut links: Vec<usize> = Vec::new();
        {
            let mut next = real_len;
            for k in 0..m {
                let e = self.ledges[k].clone();
                let a = if e.reversed { e.head } else { e.tail };
                let b = if e.reversed { e.tail } else { e.head };
                if direct || span[k] < 2 || plain[k] {
                    links.push(a);
                    links.push(b);
                    continue;
                }
                let ra = rnk[a] as i64;
                let rb = rnk[b] as i64;
                let mut label_rank: i64 = -1;
                if e.label_w > 0.0 {
                    label_rank = ra + ((rb - ra) >> 1);
                    if self.step == 2 && (label_rank & 1) == 0 {
                        label_rank -= 1;
                    }
                }
                first_dummy[k] = next as i64;
                let mut prev = a;
                for r in ra + 1..rb {
                    let d = next;
                    next += 1;
                    rnk[d] = r as usize;
                    lane_of_item[d] = lane_of_item[a]
                        + js_round(
                            ((lane_of_item[b] - lane_of_item[a]) as f64 * (r - ra) as f64)
                                / (rb - ra) as f64,
                        ) as i32;
                    dummy[d] = true;
                    if r == label_rank {
                        size[d] = e.label_w;
                        depth[d] = e.label_h;
                        label_item[k] = d as i64;
                    }
                    links.push(prev);
                    links.push(d);
                    prev = d;
                }
                links.push(prev);
                links.push(b);
            }
        }

        let mut up_start = vec![0usize; total + 1];
        let mut down_start = vec![0usize; total + 1];
        for k in (0..links.len()).step_by(2) {
            down_start[links[k] + 1] += 1;
            up_start[links[k + 1] + 1] += 1;
        }
        for i in 0..total {
            up_start[i + 1] += up_start[i];
            down_start[i + 1] += down_start[i];
        }
        let mut up = vec![0usize; links.len() / 2];
        let mut down = vec![0usize; links.len() / 2];
        {
            let mut uf = up_start[..total].to_vec();
            let mut df = down_start[..total].to_vec();
            for k in (0..links.len()).step_by(2) {
                down[df[links[k]]] = links[k + 1];
                df[links[k]] += 1;
                up[uf[links[k + 1]]] = links[k];
                uf[links[k + 1]] += 1;
            }
        }

        let mut rows: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
        for i in 0..total {
            rows[rnk[i]].push(i);
        }
        let mut pos = vec![0.0f64; total];
        let mut key = vec![0.0f64; total];
        for row in rows.iter_mut() {
            for k in 0..row.len() {
                key[row[k]] = k as f64;
            }
            settle(row, &lane_of_item, &key, &mut pos);
        }
        for sweep in 0..SWEEPS {
            if max_rank == 0 {
                break;
            }
            let downward = sweep & 1 == 0;
            for s in 1..=max_rank {
                let idx = if downward { s } else { max_rank - s };
                let mut row = std::mem::take(&mut rows[idx]);
                let (start, list) = if downward {
                    (&up_start, &up)
                } else {
                    (&down_start, &down)
                };
                for &i in &row {
                    let mut sum = 0.0f64;
                    let count = start[i + 1] - start[i];
                    for k in start[i]..start[i + 1] {
                        sum += pos[list[k]];
                    }
                    key[i] = if count > 0 {
                        sum / count as f64
                    } else {
                        pos[i]
                    };
                }
                settle(&mut row, &lane_of_item, &key, &mut pos);
                rows[idx] = row;
            }
        }

        let opt = self.opt;
        let gap = |a: usize, b: usize| -> f64 {
            if dummy[a] || dummy[b] {
                opt.edge_sep
            } else {
                opt.node_sep
            }
        };
        let run_size = |row: &[usize], from: usize, to: usize, size: &[f64]| -> f64 {
            let mut s = size[row[from]];
            for k in from..to {
                s += gap(row[k], row[k + 1]) + size[row[k + 1]];
            }
            s
        };

        let mut breadth = vec![0.0f64; lane_count];
        let mut head = if !self.lanes.is_empty() {
            LANE_HEAD
        } else {
            0.0
        };
        for k in 0..self.lanes.len() {
            let lane = &self.nodes[self.lanes[k]];
            breadth[k] = js_max(0.0, lane.min_w - 2.0 * pad[k]);
            if lane.pad_top > head {
                head = lane.pad_top;
            }
        }
        for row in &rows {
            for (from, to) in run_bounds(row, &lane_of_item) {
                let s = run_size(row, from, to, &size);
                let lane = lane_of_item[row[from]] as usize;
                if s > breadth[lane] {
                    breadth[lane] = s;
                }
            }
        }
        let mut lane_start = vec![0.0f64; lane_count + 1];
        for k in 0..lane_count {
            lane_start[k + 1] = lane_start[k] + breadth[k] + 2.0 * pad[k];
        }

        let mut c = vec![0.0f64; total];
        let mut want = vec![0.0f64; total];
        for row in &rows {
            for (from, to) in run_bounds(row, &lane_of_item) {
                let lane = lane_of_item[row[from]] as usize;
                let mut at = (lane_start[lane] + pad[lane] + (lane_start[lane + 1] - pad[lane])
                    - run_size(row, from, to, &size))
                    / 2.0;
                for k in from..=to {
                    c[row[k]] = at + size[row[k]] / 2.0;
                    if k < to {
                        at += size[row[k]] + gap(row[k], row[k + 1]);
                    }
                }
            }
        }
        for pass in 0..4 {
            if max_rank == 0 {
                break;
            }
            let downward = pass & 1 == 0;
            for s in 1..=max_rank {
                let idx = if downward { s } else { max_rank - s };
                let row = &rows[idx];
                let (start, list) = if downward {
                    (&up_start, &up)
                } else {
                    (&down_start, &down)
                };
                for &i in row {
                    let mut sum = 0.0f64;
                    let mut count = 0usize;
                    for k in start[i]..start[i + 1] {
                        if lane_of_item[list[k]] != lane_of_item[i] {
                            continue;
                        }
                        sum += c[list[k]];
                        count += 1;
                    }
                    want[i] = if count > 0 { sum / count as f64 } else { c[i] };
                }
                for (from, to) in run_bounds(row, &lane_of_item) {
                    let lane = lane_of_item[row[from]] as usize;
                    let mut floor = lane_start[lane] + pad[lane];
                    for k in from..=to {
                        let i = row[k];
                        c[i] = js_max(want[i], floor + size[i] / 2.0);
                        if k < to {
                            floor = c[i] + size[i] / 2.0 + gap(i, row[k + 1]);
                        }
                    }
                    let mut ceil = lane_start[lane + 1] - pad[lane];
                    for k in (from..=to).rev() {
                        let i = row[k];
                        c[i] = js_min(c[i], ceil - size[i] / 2.0);
                        if k > from {
                            ceil = c[i] - size[i] / 2.0 - gap(row[k - 1], i);
                        }
                    }
                }
            }
        }

        let full = opt.rank_sep / self.step as f64;
        let mut sep = vec![full; max_rank + 1];
        if plain.iter().any(|&on| on) {
            for s in sep.iter_mut() {
                *s = full / CLOSE;
            }
            for k in 0..m {
                let e = self.ledges[k].clone();
                let a = if e.reversed { e.head } else { e.tail };
                let b = if e.reversed { e.tail } else { e.head };
                if plain[k] {
                    if e.label_h <= 0.0 || span[k] as i64 > self.step {
                        continue;
                    }
                    let room = js_min(
                        full,
                        (e.label_h + 2.0 * TURN - js_min(depth[a], depth[b]) / 2.0)
                            / span[k] as f64,
                    );
                    for slot in &mut sep[rnk[a]..rnk[b]] {
                        if room > *slot {
                            *slot = room;
                        }
                    }
                    continue;
                }
                let d0 = first_dummy[k];
                if d0 < 0 || (span[k] as f64) <= (self.step as f64) * e.minlen {
                    sep[rnk[a]..rnk[b]].fill(full);
                    continue;
                }
                let span_k = span[k] as i64;
                let mut d = d0 - 1;
                while d < d0 + span_k - 1 {
                    let u = if d < d0 { a } else { d as usize };
                    let v = if d + 1 < d0 + span_k - 1 {
                        (d + 1) as usize
                    } else {
                        b
                    };
                    if lane_of_item[u] != lane_of_item[v] || (c[u] - c[v]).abs() >= 0.5 {
                        sep[rnk[u]] = full;
                    }
                    d += 1;
                }
            }
        }
        let mut band_start = vec![0.0f64; max_rank + 1];
        let mut band_end = vec![0.0f64; max_rank + 1];
        let mut at = head + self.end_pad;
        for r in 0..=max_rank {
            let mut d = 0.0f64;
            for &i in &rows[r] {
                if depth[i] > d {
                    d = depth[i];
                }
            }
            band_start[r] = at;
            band_end[r] = at + d;
            at += d + if r == max_rank { 0.0 } else { sep[r] };
        }
        let length = at + self.end_pad;
        let breadth_total = lane_start[lane_count];
        let sideways = self.sideways;
        let geo = Geo {
            c: c.clone(),
            size: size.clone(),
            depth: depth.clone(),
            rnk: rnk.clone(),
            band_start: band_start.clone(),
            band_end: band_end.clone(),
            real_len,
            edges: self
                .edges
                .iter()
                .map(|e| (e.src, e.dst, e.label_w, e.label_h))
                .collect(),
            ledge_dims: self
                .ledges
                .iter()
                .map(|e| (e.label_w, e.label_h, e.tail, e.head))
                .collect(),
            sideways,
            flipped: self.flipped,
            length,
            breadth_total,
            head,
            axis: if sideways { 1.0 } else { 0.0 },
        };
        let axis = geo.axis;

        for k in 0..real_len {
            let idx = self.real[k];
            let a = (band_start[rnk[k]] + band_end[rnk[k]]) / 2.0;
            self.nodes[idx].x = geo.px(c[k], a);
            self.nodes[idx].y = geo.py(c[k], a);
        }
        for k in 0..self.lanes.len() {
            let idx = self.lanes[k];
            let cc = (lane_start[k] + lane_start[k + 1]) / 2.0;
            let size_k = lane_start[k + 1] - lane_start[k];
            let lane = &mut self.nodes[idx];
            lane.x = if sideways { length / 2.0 } else { cc };
            lane.y = if sideways { cc } else { length / 2.0 };
            lane.w = if sideways { length } else { size_k };
            lane.h = if sideways { size_k } else { length };
            lane.pad_top = head;
        }

        let n = self.n;
        let mut depth_of = vec![0i64; n];
        let mut inner: Vec<usize> = Vec::new();
        for i in 0..n {
            if !self.nodes[i].is_group || self.lane_slot[i] >= 0 || self.item_of[i] >= 0 {
                continue;
            }
            let mut d = 0i64;
            let mut p = self.nodes[i].parent;
            let mut hops = 0usize;
            while p >= 0 && hops < MAX_HOPS {
                d += 1;
                p = self.nodes[p as usize].parent;
                hops += 1;
            }
            depth_of[i] = d;
            inner.push(i);
        }
        inner.sort_by(|&a, &b| depth_of[b].cmp(&depth_of[a]).then(a.cmp(&b)));
        let mut boxes = vec![
            [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY
            ];
            n
        ];
        for &i in &self.real {
            let parent = self.nodes[i].parent;
            if parent >= 0 && self.lane_slot[parent as usize] < 0 {
                let node = self.nodes[i].clone();
                grow(&mut boxes, parent as usize, &node);
            }
        }
        for &g in &inner {
            if boxes[g][0] == f64::INFINITY {
                continue;
            }
            let b = boxes[g];
            let cn = &mut self.nodes[g];
            let w = js_max(b[2] - b[0] + 2.0 * cn.pad_x, cn.min_w);
            cn.x = (b[0] + b[2]) / 2.0;
            cn.w = w;
            cn.y = (b[1] - cn.pad_top + b[3] + cn.pad_bottom) / 2.0;
            cn.h = b[3] - b[1] + cn.pad_top + cn.pad_bottom;
            let parent = cn.parent;
            let snapshot = cn.clone();
            if parent >= 0 && self.lane_slot[parent as usize] < 0 {
                grow(&mut boxes, parent as usize, &snapshot);
            }
        }

        let mut st = RouteState {
            routes: vec![Vec::new(); self.edges.len()],
            labels: Vec::new(),
            label_out: self.edges.iter().map(|e| (e.label_x, e.label_y)).collect(),
            routed: vec![false; m],
            used,
        };
        for k in 0..m {
            let e = self.ledges[k].clone();
            let out_index = self.edge_of[k];
            let a = if e.reversed { e.head } else { e.tail };
            let b = if e.reversed { e.tail } else { e.head };
            let mut pts = vec![
                c[a],
                band_end[rnk[a]] - (band_end[rnk[a]] - band_start[rnk[a]] - depth[a]) / 2.0,
            ];
            let d0 = first_dummy[k];
            if d0 >= 0 {
                let d0 = d0 as usize;
                for d in d0..d0 + span[k] - 1 {
                    pts.push(c[d]);
                    pts.push(band_start[rnk[d]]);
                    if band_end[rnk[d]] > band_start[rnk[d]] {
                        pts.push(c[d]);
                        pts.push(band_end[rnk[d]]);
                    }
                }
            }
            pts.push(c[b]);
            pts.push(band_start[rnk[b]] + (band_end[rnk[b]] - band_start[rnk[b]] - depth[b]) / 2.0);
            let mut route: Vec<f64> = Vec::new();
            for p in (0..pts.len()).step_by(2) {
                route.push(geo.px(pts[p], pts[p + 1]));
                route.push(geo.py(pts[p], pts[p + 1]));
                route.push(axis);
            }
            if e.reversed {
                let mut i = 0usize;
                let mut j = route.len() as isize - 3;
                while (i as isize) < j {
                    for t in 0..2 {
                        route.swap(i + t, j as usize + t);
                    }
                    i += 3;
                    j -= 3;
                }
            }
            st.routes[out_index] = route.clone();
            let label = label_item[k];
            if label >= 0 {
                let l = label as usize;
                let mid = (band_start[rnk[l]] + band_end[rnk[l]]) / 2.0;
                st.label_out[out_index] = (geo.px(c[l], mid), geo.py(c[l], mid));
            } else {
                let len = route.len();
                st.label_out[out_index] = (
                    (route[0] + route[len - 3]) / 2.0,
                    (route[1] + route[len - 2]) / 2.0,
                );
            }
        }

        for ei in 0..self.edges.len() {
            if !st.routes[ei].is_empty() || self.edges[ei].src == self.edges[ei].dst {
                continue;
            }
            let (src, dst) = (self.edges[ei].src, self.edges[ei].dst);
            let (ax, ay) = border(&self.nodes[src], &self.nodes[dst]);
            let (bx, by) = border(&self.nodes[dst], &self.nodes[src]);
            let flat = if (bx - ax).abs() > (by - ay).abs() {
                1.0
            } else {
                0.0
            };
            st.routes[ei] = vec![ax, ay, flat, bx, by, flat];
            st.label_out[ei] = ((ax + bx) / 2.0, (ay + by) / 2.0);
        }

        let route_points: f64 = st.routes.iter().map(|r| r.len() as f64 / 3.0).sum();
        let work = m as f64 * (real_len as f64 + self.edges.len() as f64 + route_points);
        if work <= ROUTE_BUDGET {
            st.labels = (0..self.edges.len())
                .map(|i| {
                    let (lx, ly) = st.label_out[i];
                    let (x, y) = if sideways { (ly, lx) } else { (lx, ly) };
                    (x, geo.flip(y))
                })
                .collect();
            for k in 0..m {
                let e = self.ledges[k].clone();
                let ei = self.edge_of[k];
                let a = e.tail;
                let b = e.head;
                if lane_of_item[a] == lane_of_item[b] || (c[b] - c[a]).abs() < 1.0 {
                    continue;
                }
                let (ac, bc) = (c[a], c[b]);
                let (aa, ba) = (geo.amid(a), geo.amid(b));
                let dc = sign(bc - ac);
                let da = sign(ba - aa);
                let pts1 = [
                    ac + (dc * size[a]) / 2.0,
                    aa,
                    bc,
                    aa,
                    bc,
                    ba - (da * depth[b]) / 2.0,
                ];
                if geo.attempt(k, ei, (a, b), &pts1, 2.0, &mut st) {
                    continue;
                }
                let pts2 = [
                    ac,
                    aa + (da * depth[a]) / 2.0,
                    ac,
                    ba,
                    bc - (dc * size[b]) / 2.0,
                    ba,
                ];
                if geo.attempt(k, ei, (a, b), &pts2, 2.0, &mut st) {
                    continue;
                }
                if work > ROUTE_BUDGET / RELAY {
                    continue;
                }
                let mut around: Vec<(Vec<f64>, f64, usize)> = Vec::new();
                let lane_a = lane_of_item[a] as usize;
                let lane_b = lane_of_item[b] as usize;
                let lanes_pick = if lane_a < lane_b {
                    [lane_b, lane_a]
                } else {
                    [lane_a, lane_b]
                };
                for lane in lanes_pick {
                    let inset = js_max(pad[lane] - LEG, js_min(pad[lane] / 2.0, MARGIN - LEG));
                    for ch in [lane_start[lane] + inset, lane_start[lane + 1] - inset] {
                        if (ch - ac).abs() < size[a] / 2.0 + 4.0
                            || (ch - bc).abs() < size[b] / 2.0 + 4.0
                        {
                            continue;
                        }
                        let pts = [
                            ac + (sign(ch - ac) * size[a]) / 2.0,
                            aa,
                            ch,
                            aa,
                            ch,
                            ba,
                            bc + (sign(ch - bc) * size[b]) / 2.0,
                            ba,
                        ];
                        let mut cost = (geo.meets(
                            geo.px(pts[0], pts[1]),
                            geo.py(pts[0], pts[1]),
                            ei,
                            &st.routes,
                        ) + geo.meets(
                            geo.px(pts[6], pts[7]),
                            geo.py(pts[6], pts[7]),
                            ei,
                            &st.routes,
                        )) as f64
                            * 1e6;
                        for p in (2..pts.len()).step_by(2) {
                            let x0 = geo.px(pts[p - 2], pts[p - 1]);
                            let y0 = geo.py(pts[p - 2], pts[p - 1]);
                            let x1 = geo.px(pts[p], pts[p + 1]);
                            let y1 = geo.py(pts[p], pts[p + 1]);
                            cost += geo.crossings(
                                (
                                    js_min(x0, x1),
                                    js_min(y0, y1),
                                    js_max(x0, x1),
                                    js_max(y0, y1),
                                ),
                                ei,
                                &st.routes,
                            ) as f64
                                * 1e6
                                + (x1 - x0).abs()
                                + (y1 - y0).abs();
                        }
                        around.push((pts.to_vec(), cost, lane));
                    }
                }
                around.sort_by(|p, q| p.1.partial_cmp(&q.1).unwrap_or(std::cmp::Ordering::Equal));
                for (pts, _, lane) in around {
                    if !geo.attempt(k, ei, (a, b), &pts, 1.0, &mut st) {
                        continue;
                    }
                    st.used[lane] = true;
                    break;
                }
            }
        }

        for ei in 0..self.edges.len() {
            self.edges[ei].route = st.routes[ei].clone();
            self.edges[ei].label_x = st.label_out[ei].0;
            self.edges[ei].label_y = st.label_out[ei].1;
        }
        PlaceOut {
            width: if sideways { length } else { breadth_total },
            height: if sideways { breadth_total } else { length },
            routed: st.routed,
            margins: st.used,
            work,
        }
    }
}

pub(crate) fn lane_layout(
    nodes: &mut [CNode],
    edges: &mut [CEdge],
    dir: Dir,
    opt: &LayeredOptions,
    lane_pad: f64,
) -> CompoundResult {
    let n = nodes.len();
    if n == 0 {
        return CompoundResult {
            width: 0.0,
            height: 0.0,
        };
    }
    let sideways = matches!(dir, Dir::LR | Dir::RL);
    let flipped = matches!(dir, Dir::BT | Dir::RL);

    let mut lanes: Vec<usize> = Vec::new();
    for i in (0..n).rev() {
        if nodes[i].parent < 0 && nodes[i].is_group {
            lanes.push(i);
        }
    }
    let lane_count = js_max(1.0, lanes.len() as f64) as usize;
    let mut lane_slot = vec![-1i64; n];
    for (k, &id) in lanes.iter().enumerate() {
        lane_slot[id] = k as i64;
    }
    let mut has_child = vec![false; n];
    for i in 0..n {
        if nodes[i].parent >= 0 {
            has_child[nodes[i].parent as usize] = true;
        }
    }
    let mut item_of = vec![-1i64; n];
    let mut real: Vec<usize> = Vec::new();
    for i in 0..n {
        if lane_slot[i] >= 0 {
            continue;
        }
        if !nodes[i].is_group || !has_child[i] {
            item_of[i] = real.len() as i64;
            real.push(i);
        }
    }
    let along = |c: &CNode| if sideways { c.w } else { c.h };
    let cross = |c: &CNode| if sideways { c.h } else { c.w };
    let mut lnodes: Vec<LNode> = real
        .iter()
        .map(|&i| lnode(cross(&nodes[i]), along(&nodes[i]), Kind::Node))
        .collect();
    let mut ledges: Vec<LEdge> = Vec::new();
    let mut edge_of: Vec<usize> = Vec::new();
    let mut step: i64 = 1;
    {
        let mut seen = std::collections::HashSet::new();
        for (ei, e) in edges.iter().enumerate() {
            let a = item_of[e.src];
            let b = item_of[e.dst];
            if a < 0 || b < 0 || a == b {
                continue;
            }
            let (a, b) = (a as usize, b as usize);
            let key = a.min(b) * real.len() + a.max(b);
            if e.label_w > 0.0 || seen.contains(&key) {
                step = 2;
            }
            seen.insert(key);
            ledges.push(ledge(
                a,
                b,
                e.minlen,
                if sideways { e.label_h } else { e.label_w },
                if sideways { e.label_w } else { e.label_h },
            ));
            edge_of.push(ei);
        }
    }
    orient(&lnodes, &mut ledges);
    rank(&mut lnodes, &ledges, step);

    let m = ledges.len();
    let mut layout = Layout {
        nodes,
        edges,
        opt,
        lane_pad,
        sideways,
        flipped,
        lanes: lanes.clone(),
        lane_count,
        lane_pads: vec![lane_pad; lane_count],
        end_pad: lane_pad,
        lane_slot,
        item_of,
        real: real.clone(),
        lnodes,
        ledges,
        edge_of,
        step,
        item_lane: Vec::new(),
        n,
    };
    let mut item_lane: Vec<i32> = Vec::with_capacity(real.len());
    for &i in &real {
        item_lane.push(layout.lane_of(i) as i32);
    }
    layout.item_lane = item_lane;

    let mut plain = vec![false; m];
    let mut margins = vec![false; lane_count];
    let mut placed = layout.place(&plain, &margins);
    let mut round = 0usize;
    while round < ROUNDS && placed.work <= ROUTE_BUDGET / RELAY {
        let next: Vec<bool> = if round == 0 {
            placed.routed.clone()
        } else {
            plain
                .iter()
                .zip(placed.routed.iter())
                .map(|(&on, &r)| on && r)
                .collect()
        };
        if next == plain && placed.margins == margins {
            break;
        }
        plain = next;
        margins = placed.margins.clone();
        placed = layout.place(&plain, &margins);
        round += 1;
    }
    if plain
        .iter()
        .zip(placed.routed.iter())
        .any(|(&on, &r)| on && !r)
    {
        placed = layout.place(&vec![false; m], &vec![false; lane_count]);
    }
    CompoundResult {
        width: placed.width,
        height: placed.height,
    }
}
