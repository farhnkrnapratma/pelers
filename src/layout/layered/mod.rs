use std::cmp::Ordering;
use std::collections::HashSet;

use super::{Kind, LEdge, LNode, LayeredOptions, LayeredResult};
use crate::util::jsnum::{js_max, js_min};

mod crossings;
mod position;
mod spread;
mod tracks;

pub(crate) use crossings::reduce_crossings;
pub(crate) use position::position;
pub(crate) use spread::{
    align_labels, align_ports, free_ends, past_labels, settle, snap, straighten,
};
pub(crate) use tracks::assign_tracks;

const MAX_SWEEPS: usize = 24;
const MAX_DUMMIES: usize = 20000;
const PORT_PITCH: f64 = 16.0;
const MIN_PITCH: f64 = 8.0;
const MIN_SLOPE: f64 = 0.125;
const MAX_GAP: f64 = 3.0;
const TRACK_MARGIN: f64 = 18.0;
const TRACK_PITCH: f64 = 10.0;
const TRACK_SPREAD: f64 = 14.0;
const LEVEL_WIDE: f64 = 2.0;
const LEVEL_MARGIN: f64 = 28.0;
const LEVEL_PITCH: f64 = 12.0;
const LEVEL_LEAD: f64 = 12.0;
const LEVEL_CLEAR: f64 = 8.0;
const LEVEL_TOUCH: f64 = 4.0;
const CROSSED_SLOPE: f64 = 0.5;
const IN_LINE: f64 = 0.5;
const TWIN: f64 = 1.0;
const SNAP: f64 = 12.0;
const BEND_SNAP: f64 = 24.0;
const LEAN_CLEAR: f64 = 40.0;
const MAX_WEIGHED: usize = 400;
const SAMPLES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    Held,
    Leaves,
    Arrives,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Pieces {
    pub from: Vec<f64>,
    pub to: Vec<f64>,
    pub rank: Vec<usize>,
    pub lower: Vec<Option<usize>>,
    pub track: Vec<i32>,
    pub levels: Vec<i32>,
    pub wide: Vec<bool>,
}

#[derive(Clone, Debug)]
pub(crate) struct Graph {
    pub n: usize,
    pub real: usize,
    pub max_rank: usize,
    pub w: Vec<f64>,
    pub kind: Vec<Kind>,
    pub rank: Vec<usize>,
    pub ef: Vec<usize>,
    pub et: Vec<usize>,
    pub efd: Vec<f64>,
    pub etd: Vec<f64>,
    pub succ_start: Vec<usize>,
    pub succ_e: Vec<usize>,
    pub pred_start: Vec<usize>,
    pub pred_e: Vec<usize>,
    pub layer_start: Vec<usize>,
    pub order: Vec<usize>,
    pub pos: Vec<usize>,
    pub offsets: bool,
    pub fixed: Vec<bool>,
    pub grp: Option<Vec<i32>>,
    pub group_parent: Option<Vec<i32>>,
}

pub(crate) fn chain(diff: f64, rest: Ordering) -> Ordering {
    if diff.is_nan() || diff == 0.0 {
        rest
    } else if diff < 0.0 {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

pub(crate) fn sort_range(list: &mut [usize], from: usize, to: usize, key: &[usize]) {
    for i in from + 1..to {
        let item = list[i];
        let k = key[item];
        let mut j = i as isize - 1;
        while j >= from as isize {
            let cur = list[j as usize];
            if key[cur] > k || (key[cur] == k && cur > item) {
                list[(j + 1) as usize] = cur;
                j -= 1;
            } else {
                break;
            }
        }
        list[(j + 1) as usize] = item;
    }
}

pub(crate) fn sort_by_bary(list: &mut [usize], bary: &[f64], pos: &[usize], right: bool) {
    let count = list.len();
    let before = |a: usize, b: usize| {
        bary[a] < bary[b]
            || (bary[a] == bary[b]
                && (if right {
                    pos[a] > pos[b]
                } else {
                    pos[a] < pos[b]
                }))
    };
    const RUN: usize = 8;
    let mut s = 0;
    while s < count {
        let end = count.min(s + RUN);
        for i in s + 1..end {
            let item = list[i];
            let mut j = i as isize - 1;
            while j >= s as isize && before(item, list[j as usize]) {
                list[(j + 1) as usize] = list[j as usize];
                j -= 1;
            }
            list[(j + 1) as usize] = item;
        }
        s += RUN;
    }
    let mut from = list.to_vec();
    let mut to = vec![0usize; count];
    let mut width = RUN;
    while width < count {
        let mut s = 0;
        while s < count {
            let mid = count.min(s + width);
            let end = count.min(s + 2 * width);
            let (mut i, mut j, mut k) = (s, mid, s);
            while i < mid && j < end {
                if before(from[j], from[i]) {
                    to[k] = from[j];
                    j += 1;
                } else {
                    to[k] = from[i];
                    i += 1;
                }
                k += 1;
            }
            while i < mid {
                to[k] = from[i];
                i += 1;
                k += 1;
            }
            while j < end {
                to[k] = from[j];
                j += 1;
                k += 1;
            }
            s += 2 * width;
        }
        std::mem::swap(&mut from, &mut to);
        width *= 2;
    }
    list.copy_from_slice(&from);
}

pub(crate) fn ease(across: f64, along: f64) -> f64 {
    let d = across.abs();
    let d = if d == 0.0 || d.is_nan() { 1.0 } else { d };
    js_max(0.25, js_min(0.5, (4.0 * along.abs()) / d))
}

pub(crate) fn orient(nodes: &[LNode], edges: &mut [LEdge]) {
    let n = nodes.len();
    let m = edges.len();
    let mut start = vec![0usize; n + 1];
    for e in edges.iter_mut().take(m) {
        e.reversed = nodes[e.head].kind == Kind::StartPort || nodes[e.tail].kind == Kind::EndPort;
        if !e.reversed {
            start[e.tail + 1] += 1;
        }
    }
    for i in 0..n {
        start[i + 1] += start[i];
    }
    let mut out = vec![0usize; start[n]];
    {
        let mut fill = start[..n].to_vec();
        for (i, e) in edges.iter().enumerate() {
            if !e.reversed {
                out[fill[e.tail]] = i;
                fill[e.tail] += 1;
            }
        }
    }
    let mut state = vec![0u8; n];
    let mut cursor = start[..n].to_vec();
    let mut stack: Vec<usize> = Vec::new();
    for root in 0..n {
        if state[root] != 0 {
            continue;
        }
        stack.push(root);
        state[root] = 1;
        while let Some(&v) = stack.last() {
            if cursor[v] < start[v + 1] {
                let ei = out[cursor[v]];
                cursor[v] += 1;
                let w = edges[ei].head;
                if state[w] == 1 {
                    edges[ei].reversed = true;
                } else if state[w] == 0 {
                    state[w] = 1;
                    stack.push(w);
                }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
}

pub(crate) fn rank(nodes: &mut [LNode], edges: &[LEdge], step: i64) {
    let n = nodes.len();
    let m = edges.len();
    let mut from = vec![0usize; m];
    let mut to = vec![0usize; m];
    let mut len = vec![0i64; m];
    let mut out_start = vec![0usize; n + 1];
    let mut in_start = vec![0usize; n + 1];
    for (i, e) in edges.iter().enumerate() {
        let a = if e.reversed { e.head } else { e.tail };
        let b = if e.reversed { e.tail } else { e.head };
        from[i] = a;
        to[i] = b;
        len[i] = (js_max(1.0, e.minlen) * step as f64) as i64;
        out_start[a + 1] += 1;
        in_start[b + 1] += 1;
    }
    for i in 0..n {
        out_start[i + 1] += out_start[i];
        in_start[i + 1] += in_start[i];
    }
    let mut out_e = vec![0usize; m];
    let mut in_e = vec![0usize; m];
    {
        let mut of = out_start[..n].to_vec();
        let mut inf = in_start[..n].to_vec();
        for i in 0..m {
            out_e[of[from[i]]] = i;
            of[from[i]] += 1;
            in_e[inf[to[i]]] = i;
            inf[to[i]] += 1;
        }
    }

    let mut r = vec![0i64; n];
    for (i, node) in nodes.iter().enumerate() {
        if node.kind == Kind::StartPort {
            r[i] = -step;
        }
    }
    let mut indeg: Vec<usize> = (0..n).map(|i| in_start[i + 1] - in_start[i]).collect();
    let mut topo = vec![0usize; n];
    let mut count = 0;
    for (i, &deg) in indeg.iter().enumerate().take(n) {
        if deg == 0 {
            topo[count] = i;
            count += 1;
        }
    }
    let mut q = 0;
    while q < count {
        let v = topo[q];
        for &ei in &out_e[out_start[v]..out_start[v + 1]] {
            let w = to[ei];
            if r[v] + len[ei] > r[w] {
                r[w] = r[v] + len[ei];
            }
            indeg[w] -= 1;
            if indeg[w] == 0 {
                topo[count] = w;
                count += 1;
            }
        }
        q += 1;
    }

    for _pass in 0..8 {
        let mut changed = false;
        for q in (0..count).rev() {
            let v = topo[q];
            let ins = in_start[v + 1] - in_start[v];
            let outs = out_start[v + 1] - out_start[v];
            if ins == 0 && outs == 0 {
                continue;
            }
            let mut lo: Option<i64> = None;
            let mut hi: Option<i64> = None;
            for &ei in &in_e[in_start[v]..in_start[v + 1]] {
                let x = r[from[ei]] + len[ei];
                lo = Some(lo.map_or(x, |l| l.max(x)));
            }
            for &ei in &out_e[out_start[v]..out_start[v + 1]] {
                let x = r[to[ei]] - len[ei];
                hi = Some(hi.map_or(x, |h| h.min(x)));
            }
            let mut next = r[v];
            if let (true, Some(h)) = (outs > ins, hi) {
                next = h;
            } else if let (true, Some(l)) = (ins > outs, lo) {
                next = l;
            }
            if next != r[v] {
                r[v] = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut min = i64::MAX;
    let mut max = i64::MIN;
    let mut start_ports = false;
    for i in 0..n {
        let kind = nodes[i].kind;
        if kind == Kind::StartPort {
            start_ports = true;
        }
        if kind == Kind::StartPort || kind == Kind::EndPort {
            continue;
        }
        min = min.min(r[i]);
        max = max.max(r[i]);
    }
    if min == i64::MAX {
        min = 0;
        max = 0;
    }
    for i in 0..n {
        if nodes[i].kind == Kind::StartPort {
            r[i] = min - step;
        } else if nodes[i].kind == Kind::EndPort {
            r[i] = max + step;
        }
    }
    if start_ports {
        min -= step;
    }
    for i in 0..n {
        nodes[i].rank = (r[i] - min) as usize;
    }
}

pub(crate) fn layered(
    nodes: &mut [LNode],
    edges: &mut [LEdge],
    opt: &LayeredOptions,
) -> LayeredResult {
    let real = nodes.len();
    if real == 0 {
        return LayeredResult {
            width: 0.0,
            height: 0.0,
            ..Default::default()
        };
    }
    let m = edges.len();
    if real == 1 && m == 0 && nodes[0].kind == Kind::Node {
        let node = &mut nodes[0];
        node.rank = 0;
        node.x = node.w / 2.0;
        node.y = node.h / 2.0;
        return LayeredResult {
            width: node.w,
            height: node.h,
            ..Default::default()
        };
    }

    let mut step: i64 = 1;
    if m > 0 {
        let mut seen = HashSet::new();
        for e in edges.iter() {
            let a = e.tail.min(e.head);
            let b = e.tail.max(e.head);
            let key = a * real + b;
            if e.label_w > 0.0 || seen.contains(&key) {
                step = 2;
                break;
            }
            seen.insert(key);
        }
    }

    orient(nodes, edges);
    rank(nodes, edges, step);

    let mut span = vec![0usize; m];
    let mut dummies = 0usize;
    for ei in 0..m {
        let e = &edges[ei];
        span[ei] = nodes[e.head].rank.abs_diff(nodes[e.tail].rank);
        if span[ei] > 1 {
            dummies += span[ei] - 1;
        }
    }
    let mut direct = vec![false; m];
    if dummies > MAX_DUMMIES {
        let mut longest: Vec<usize> = (0..m).filter(|&ei| span[ei] > 1).collect();
        longest.sort_by(|&a, &b| span[b].cmp(&span[a]).then(a.cmp(&b)));
        for ei in longest {
            if dummies <= MAX_DUMMIES {
                break;
            }
            direct[ei] = true;
            dummies -= span[ei] - 1;
        }
    }

    let n = real + dummies;
    let mut w = vec![0.0f64; n];
    let mut h = vec![0.0f64; n];
    let mut kind = vec![Kind::Node; n];
    let mut rnk = vec![0usize; n];
    let mut max_rank = 0usize;
    for i in 0..real {
        let node = &nodes[i];
        w[i] = node.w;
        h[i] = node.h;
        kind[i] = node.kind;
        rnk[i] = node.rank;
        if node.rank > max_rank {
            max_rank = node.rank;
        }
    }

    let mut segments = 0usize;
    for ei in 0..m {
        if !direct[ei] {
            segments += span[ei];
        }
    }
    let mut ef = vec![0usize; segments];
    let mut et = vec![0usize; segments];
    let mut efd = vec![0.0f64; segments];
    let mut etd = vec![0.0f64; segments];
    let mut first_dummy: Vec<Option<usize>> = vec![None; m];
    let mut heads = vec![0u8; max_rank + 1];
    let mut label_node: Vec<Option<usize>> = vec![None; m];
    let mut spans = false;
    for node in &nodes[..real] {
        if node.span > 0.0 && node.kind == Kind::Node {
            spans = true;
        }
    }
    let mut efk: Option<Vec<End>> = if spans {
        Some(vec![End::Held; segments])
    } else {
        None
    };
    let mut etk: Option<Vec<End>> = if spans {
        Some(vec![End::Held; segments])
    } else {
        None
    };
    let mut first_seg = vec![0usize; m];
    let mut last_seg = vec![0usize; m];
    let mut key = vec![f64::NAN; n];
    for i in 0..real {
        key[i] = nodes[i].key;
    }
    let mut offsets = false;
    {
        let mut next = real;
        let mut ce = 0usize;
        for ei in 0..m {
            if direct[ei] {
                continue;
            }
            let e = &edges[ei];
            let a = if e.reversed { e.head } else { e.tail };
            let b = if e.reversed { e.tail } else { e.head };
            let a_dx = if e.reversed { e.head_dx } else { e.tail_dx };
            let b_dx = if e.reversed { e.tail_dx } else { e.head_dx };
            if a_dx != 0.0 || b_dx != 0.0 {
                offsets = true;
            }
            let ra = rnk[a] as i64;
            let rb = rnk[b] as i64;
            let mut label_rank: i64 = -1;
            if e.label_w > 0.0 {
                label_rank = ra + ((rb - ra) >> 1);
                if step == 2 && rb - ra >= 4 {
                    if (label_rank & 1) == 1 {
                        label_rank += if e.reversed { 1 } else { -1 };
                    }
                } else if step == 2 && (label_rank & 1) == 0 {
                    label_rank += if e.reversed { 1 } else { -1 };
                }
            }
            let mut prev = a;
            if rb - ra > 1 {
                first_dummy[ei] = Some(next);
            }
            first_seg[ei] = ce;
            if let Some(efk) = efk.as_mut()
                && !(if e.reversed {
                    e.head_fixed
                } else {
                    e.tail_fixed
                })
            {
                efk[ce] = if e.reversed {
                    End::Arrives
                } else {
                    End::Leaves
                };
            }
            for r in ra + 1..rb {
                let d = next;
                next += 1;
                rnk[d] = r as usize;
                key[d] = key[a] + ((key[b] - key[a]) * (r - ra) as f64) / (rb - ra) as f64;
                if r == label_rank {
                    w[d] = e.label_w;
                    h[d] = e.label_h;
                    kind[d] = Kind::Label;
                    label_node[ei] = Some(d);
                    heads[r as usize] |= if e.reversed { 2 } else { 1 };
                } else {
                    kind[d] = Kind::Dummy;
                }
                ef[ce] = prev;
                et[ce] = d;
                efd[ce] = if prev == a { a_dx } else { 0.0 };
                ce += 1;
                prev = d;
            }
            ef[ce] = prev;
            et[ce] = b;
            efd[ce] = if prev == a { a_dx } else { 0.0 };
            etd[ce] = b_dx;
            if let Some(etk) = etk.as_mut()
                && !(if e.reversed {
                    e.tail_fixed
                } else {
                    e.head_fixed
                })
            {
                etk[ce] = if e.reversed {
                    End::Leaves
                } else {
                    End::Arrives
                };
            }
            last_seg[ei] = ce;
            ce += 1;
        }
    }

    let mut grp: Option<Vec<i32>> = None;
    if let Some(groups) = &opt.groups {
        let mut g = vec![0i32; n];
        g[..real].copy_from_slice(&groups.of[..real]);
        let depth = |mut gr: i32| -> usize {
            let mut d = 0;
            while gr >= 0 {
                d += 1;
                gr = groups.parent[gr as usize];
            }
            d
        };
        let lca = |mut x: i32, mut y: i32| -> i32 {
            let mut dx = depth(x);
            let mut dy = depth(y);
            while dx > dy {
                x = groups.parent[x as usize];
                dx -= 1;
            }
            while dy > dx {
                y = groups.parent[y as usize];
                dy -= 1;
            }
            while x != y {
                x = groups.parent[x as usize];
                y = groups.parent[y as usize];
            }
            x
        };
        for ei in 0..m {
            let Some(first) = first_dummy[ei] else {
                continue;
            };
            let e = &edges[ei];
            let gr = lca(groups.of[e.tail], groups.of[e.head]);
            g[first..first + span[ei] - 1].fill(gr);
        }
        grp = Some(g);
    }

    let mut succ_start = vec![0usize; n + 1];
    let mut pred_start = vec![0usize; n + 1];
    for e in 0..segments {
        succ_start[ef[e] + 1] += 1;
        pred_start[et[e] + 1] += 1;
    }
    for i in 0..n {
        succ_start[i + 1] += succ_start[i];
        pred_start[i + 1] += pred_start[i];
    }
    let mut succ_e = vec![0usize; segments];
    let mut pred_e = vec![0usize; segments];
    {
        let mut sf = succ_start[..n].to_vec();
        let mut pf = pred_start[..n].to_vec();
        for e in 0..segments {
            succ_e[sf[ef[e]]] = e;
            sf[ef[e]] += 1;
            pred_e[pf[et[e]]] = e;
            pf[et[e]] += 1;
        }
    }

    let mut layer_start = vec![0usize; max_rank + 2];
    for i in 0..n {
        layer_start[rnk[i] + 1] += 1;
    }
    for r in 0..=max_rank {
        layer_start[r + 1] += layer_start[r];
    }
    let mut order = vec![0usize; n];
    let mut pos = vec![0usize; n];

    {
        let mut by_rank = vec![0usize; n];
        let mut fill: Vec<usize> = layer_start[..=max_rank].to_vec();
        for i in 0..n {
            by_rank[fill[rnk[i]]] = i;
            fill[rnk[i]] += 1;
        }
        fill[..=max_rank].copy_from_slice(&layer_start[..=max_rank]);
        let mut visited = vec![false; n];
        let mut stack: Vec<usize> = Vec::new();
        for k in 0..n {
            if visited[by_rank[k]] {
                continue;
            }
            stack.push(by_rank[k]);
            while let Some(v) = stack.pop() {
                if visited[v] {
                    continue;
                }
                visited[v] = true;
                order[fill[rnk[v]]] = v;
                fill[rnk[v]] += 1;
                for j in (succ_start[v]..succ_start[v + 1]).rev() {
                    let wv = et[succ_e[j]];
                    if !visited[wv] {
                        stack.push(wv);
                    }
                }
            }
        }
    }

    let sort_rank = |order: &mut [usize], r: usize, key: &dyn Fn(usize) -> f64| -> bool {
        let s = layer_start[r];
        let e = layer_start[r + 1];
        if e - s < 2 {
            return false;
        }
        for &o in &order[s..e] {
            if key(o).is_nan() {
                return false;
            }
        }
        let mut layer: Vec<usize> = order[s..e].to_vec();
        let index: std::collections::HashMap<usize, usize> =
            layer.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        layer.sort_by(|&a, &b| chain(key(a) - key(b), index[&a].cmp(&index[&b])));
        order[s..e].copy_from_slice(&layer);
        true
    };
    for r in 0..=max_rank {
        sort_rank(&mut order, r, &|v| key[v]);
    }

    let mut pinned = false;
    for node in &nodes[..real] {
        if node.pin != 0 {
            pinned = true;
        }
    }
    if pinned {
        for r in 0..=max_rank {
            let s = layer_start[r];
            let e = layer_start[r + 1];
            let mut layer: Vec<usize> = order[s..e].to_vec();
            let index: std::collections::HashMap<usize, usize> =
                layer.iter().enumerate().map(|(i, &v)| (v, i)).collect();
            let pin_of = |v: usize| -> i32 { if v < real { nodes[v].pin } else { 0 } };
            layer.sort_by(|&a, &b| {
                let d = pin_of(a) - pin_of(b);
                if d != 0 {
                    d.cmp(&0)
                } else {
                    index[&a].cmp(&index[&b])
                }
            });
            order[s..e].copy_from_slice(&layer);
        }
    }

    let mut fixed = vec![false; max_rank + 1];
    let hint = |v: usize| -> f64 {
        if v < real && (nodes[v].kind == Kind::StartPort || nodes[v].kind == Kind::EndPort) {
            nodes[v].hint
        } else {
            f64::NAN
        }
    };
    for r in [0, max_rank] {
        if sort_rank(&mut order, r, &hint) {
            fixed[r] = true;
        }
    }
    let set_positions = |order: &[usize], pos: &mut [usize], layer_start: &[usize]| {
        for r in 0..=max_rank {
            let s = layer_start[r];
            for i in s..layer_start[r + 1] {
                pos[order[i]] = i - s;
            }
        }
    };
    set_positions(&order, &mut pos, &layer_start);

    let mut g = Graph {
        n,
        real,
        max_rank,
        w,
        kind,
        rank: rnk,
        ef,
        et,
        efd,
        etd,
        succ_start,
        succ_e,
        pred_start,
        pred_e,
        layer_start,
        order,
        pos,
        offsets,
        fixed,
        grp,
        group_parent: opt.groups.as_ref().map(|gr| gr.parent.clone()),
    };

    let mut crossings_count = 0;
    let mut declared = 0;
    if segments > 0 && max_rank > 0 {
        let (c, d) = reduce_crossings(&mut g, nodes);
        crossings_count = c;
        declared = d;
        set_positions(&g.order, &mut g.pos, &g.layer_start);
    }
    if opt.order_only {
        let at = position(&mut g, opt);
        for i in 0..real {
            nodes[i].x = at[i];
        }
        for ei in 0..m {
            let e = &mut edges[ei];
            let a = if e.reversed { e.head } else { e.tail };
            let b = if e.reversed { e.tail } else { e.head };
            let mut pts = vec![g.rank[a] as f64, at[a]];
            if !direct[ei]
                && let Some(first) = first_dummy[ei]
            {
                for d in first..first + span[ei] - 1 {
                    pts.push(g.rank[d] as f64);
                    pts.push(at[d]);
                }
            }
            pts.push(g.rank[b] as f64);
            pts.push(at[b]);
            e.points = pts;
        }
        return LayeredResult {
            width: 0.0,
            height: 0.0,
            crossings: Some(crossings_count),
            declared: Some(declared),
        };
    }

    if let (Some(efk_v), Some(etk_v)) = (efk.as_ref(), etk.as_ref()) {
        spread::spread(&mut g, nodes, efk_v, etk_v);
    }

    let mut xs = position(&mut g, opt);

    let empty = empty_ranks(&g);
    if let (Some(efk_v), Some(etk_v)) = (efk.as_ref(), etk.as_ref()) {
        let free = free_ends(&g, efk_v, etk_v);
        align_labels(&g, nodes, &mut xs, &free, opt, empty.as_deref());
        settle(
            &mut g,
            nodes,
            &mut xs,
            efk_v,
            etk_v,
            &free,
            empty.as_deref(),
        );
        snap(&g, &mut xs, empty.as_deref(), opt);
        for ei in 0..m {
            if direct[ei] {
                continue;
            }
            let a_dx = g.efd[first_seg[ei]];
            let b_dx = g.etd[last_seg[ei]];
            let e = &mut edges[ei];
            e.tail_dx = if e.reversed { b_dx } else { a_dx };
            e.head_dx = if e.reversed { a_dx } else { b_dx };
        }
    }
    align_ports(&g, &mut xs, empty.as_deref());

    let mut on_line = vec![false; n];
    if let (Some(empty_v), false) = (empty.as_deref(), opt.tracks) {
        let sep = if opt.apart { 0.0 } else { opt.edge_sep };
        straighten(&g, &mut xs, empty_v, &first_dummy, &span, sep, &mut on_line);
    }
    if !opt.tracks {
        past_labels(&g, &mut xs, &first_dummy, &span, opt.edge_sep, &mut on_line);
    }

    let mut piece_start = vec![0usize; m + 1];
    let has_pieces = opt.tracks || (spans && !opt.straight);
    let mut pieces: Option<Pieces> = if has_pieces {
        Some(Pieces::default())
    } else {
        None
    };
    let mut track_count = vec![0i32; max_rank + 1];
    if let Some(p) = pieces.as_mut() {
        for ei in 0..m {
            piece_start[ei] = p.from.len();
            if direct[ei] {
                continue;
            }
            let d0 = first_dummy[ei];
            let mut twin = false;
            if let Some(d0) = d0 {
                for d in d0..d0 + span[ei] - 1 {
                    let bare = empty.as_deref().is_some_and(|e| e[g.rank[d]]);
                    if !opt.tracks && bare != on_line[d] {
                        twin = true;
                    }
                }
            }
            if twin {
                continue;
            }
            let mut x = xs[g.ef[first_seg[ei]]] + g.efd[first_seg[ei]];
            let mut rank_v = g.rank[g.ef[first_seg[ei]]];
            if let Some(d0) = d0 {
                for d in d0..d0 + span[ei] - 1 {
                    if empty.as_deref().is_some_and(|e| e[g.rank[d]]) {
                        continue;
                    }
                    p.from.push(x);
                    p.to.push(xs[d]);
                    p.rank.push(rank_v);
                    p.lower.push(Some(d));
                    x = xs[d];
                    rank_v = g.rank[d];
                }
            }
            p.from.push(x);
            p.to.push(xs[g.et[last_seg[ei]]] + g.etd[last_seg[ei]]);
            p.rank.push(rank_v);
            p.lower.push(None);
        }
        piece_start[m] = p.from.len();
    }

    let port_only = |r: usize, k: Kind| -> bool {
        let mut found = false;
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            let kk = g.kind[g.order[i]];
            if kk == k {
                found = true;
            } else if kk != Kind::Dummy {
                return false;
            }
        }
        found
    };
    let start_ranks = if port_only(0, Kind::StartPort) {
        step as usize
    } else {
        0
    };
    let end_ranks = if port_only(max_rank, Kind::EndPort) {
        step as usize
    } else {
        0
    };
    let mut run = vec![0.0f64; max_rank + 1];
    for e in 0..segments {
        let across = (xs[g.ef[e]] + g.efd[e] - xs[g.et[e]] - g.etd[e]).abs();
        if across > run[g.rank[g.ef[e]]] {
            run[g.rank[g.ef[e]]] = across;
        }
    }
    let mut band_top = vec![0.0f64; max_rank + 1];
    let mut band_bottom = vec![0.0f64; max_rank + 1];
    let stepf = step as f64;
    let usual = |r: usize| -> f64 {
        (if r < start_ranks || r + end_ranks >= max_rank {
            opt.port_sep
        } else {
            opt.rank_sep
        }) / stepf
    };
    let mut gap_under = vec![0.0f64; max_rank + 1];
    for r in 0..max_rank {
        gap_under[r] = if has_pieces {
            usual(r)
        } else {
            js_max(
                usual(r),
                js_min((MAX_GAP * opt.rank_sep) / stepf, run[r] * MIN_SLOPE),
            )
        };
    }
    let mut between = vec![true; max_rank + 1];
    for i in 0..real {
        between[g.rank[i]] = false;
    }
    for r in 1..max_rank {
        if opt.tracks || !between[r] || (heads[r] != 1 && heads[r] != 2) {
            continue;
        }
        let shift = js_min(
            opt.head_room / 2.0,
            gap_under[if heads[r] == 1 { r - 1 } else { r }] / 2.0,
        );
        gap_under[r - 1] += if heads[r] == 1 { -shift } else { shift };
        gap_under[r] += if heads[r] == 1 { shift } else { -shift };
    }
    let margin = if opt.tracks {
        TRACK_MARGIN
    } else {
        LEVEL_MARGIN
    };
    let least_pitch = if opt.tracks { TRACK_PITCH } else { LEVEL_PITCH };
    let most_pitch = if opt.tracks {
        TRACK_SPREAD
    } else {
        LEVEL_PITCH
    };
    let mut below = vec![0usize; max_rank + 1];
    if let Some(p) = pieces.as_mut() {
        let mut room = vec![0.0f64; max_rank + 1];
        for r in 0..max_rank {
            if empty.as_deref().is_some_and(|e| e[r]) {
                continue;
            }
            let mut next = r + 1;
            room[r] = gap_under[r];
            while next < max_rank && empty.as_deref().is_some_and(|e| e[next]) {
                room[r] += gap_under[next];
                next += 1;
            }
            below[r] = next;
        }
        for k in 0..p.from.len() {
            p.wide
                .push((p.from[k] - p.to[k]).abs() > LEVEL_WIDE * room[p.rank[k]]);
        }
        let mut tall = vec![0.0f64; max_rank + 1];
        assign_tracks(p, &mut track_count, !opt.tracks, &room, &mut tall);
        for r in 0..max_rank {
            if track_count[r] > 0 {
                gap_under[r] += js_max(
                    0.0,
                    2.0 * margin + (track_count[r] - 1) as f64 * least_pitch - room[r],
                );
            } else if tall[r] > 0.0 {
                gap_under[r] += js_max(0.0, js_min(MAX_GAP * opt.rank_sep, tall[r]) - room[r]);
            }
        }
    }
    let mut y = 0.0f64;
    for r in 0..=max_rank {
        let mut hh = 0.0f64;
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            if h[g.order[i]] > hh {
                hh = h[g.order[i]];
            }
        }
        band_top[r] = y;
        band_bottom[r] = y + hh;
        y += hh + gap_under[r];
    }
    let height = y;

    let track_y = |p: &Pieces,
                   piece: usize,
                   on_track: i32,
                   band_top: &[f64],
                   band_bottom: &[f64],
                   track_count: &[i32]|
     -> f64 {
        let r = p.rank[piece];
        let top = band_bottom[r];
        let bottom = band_top[below[r]];
        let count = if opt.tracks {
            track_count[r]
        } else {
            p.levels[piece]
        };
        let pitch = if count > 1 {
            js_min(
                most_pitch,
                (bottom - top - 2.0 * margin) / (count - 1) as f64,
            )
        } else {
            0.0
        };
        (top + bottom) / 2.0 + (on_track as f64 - (count - 1) as f64 / 2.0) * pitch
    };

    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    for i in 0..n {
        let half = g.w[i] / 2.0;
        if xs[i] - half < min_x {
            min_x = xs[i] - half;
        }
        if xs[i] + half > max_x {
            max_x = xs[i] + half;
        }
    }
    let mut arrives = vec![false; max_rank + 1];
    let mut leaves = vec![false; max_rank + 1];
    for i in 0..real {
        if g.kind[i] != Kind::Node || !spans {
            continue;
        }
        if g.pred_start[i + 1] > g.pred_start[i] {
            arrives[g.rank[i]] = true;
        }
        if g.succ_start[i + 1] > g.succ_start[i] {
            leaves[g.rank[i]] = true;
        }
    }
    for i in 0..real {
        let r = nodes[i].rank;
        nodes[i].x = xs[i] - min_x;
        if g.kind[i] == Kind::Node && leaves[r] && !arrives[r] {
            nodes[i].y = band_bottom[r] - nodes[i].h / 2.0;
        } else if g.kind[i] == Kind::Node && arrives[r] && !leaves[r] {
            nodes[i].y = band_top[r] + nodes[i].h / 2.0;
        } else {
            nodes[i].y = (band_top[r] + band_bottom[r]) / 2.0;
        }
    }

    let inset = |d: usize| -> f64 {
        if g.kind[d] == Kind::Dummy && between[g.rank[d]] && !opt.tracks {
            (band_bottom[g.rank[d]] - band_top[g.rank[d]]) / 4.0
        } else {
            0.0
        }
    };

    for ei in 0..m {
        let e = &edges[ei];
        let first = &nodes[if e.reversed { e.head } else { e.tail }];
        let last = &nodes[if e.reversed { e.tail } else { e.head }];
        let mut pts = vec![
            first.x + if e.reversed { e.head_dx } else { e.tail_dx },
            first.y + first.h / 2.0,
        ];
        let d0 = first_dummy[ei];
        let piece_range = piece_start[ei]..piece_start[ei + 1];
        if let Some(p) = pieces
            .as_ref()
            .filter(|_| piece_start[ei + 1] > piece_start[ei])
        {
            for k in piece_range {
                if p.track[k] >= 0 && (opt.tracks || p.wide[k]) {
                    let across = track_y(p, k, p.track[k], &band_top, &band_bottom, &track_count);
                    pts.push(p.from[k] - min_x);
                    pts.push(across);
                    pts.push(p.to[k] - min_x);
                    pts.push(across);
                } else if p.track[k] > 0 {
                    let across =
                        track_y(p, k, p.track[k] - 1, &band_top, &band_bottom, &track_count);
                    pts.push(p.from[k] - min_x);
                    pts.push(across);
                }
                let Some(d) = p.lower[k] else {
                    continue;
                };
                pts.push(xs[d] - min_x);
                pts.push(band_top[g.rank[d]] + inset(d));
                if band_bottom[g.rank[d]] > band_top[g.rank[d]] {
                    pts.push(xs[d] - min_x);
                    pts.push(band_bottom[g.rank[d]] - inset(d));
                }
            }
        } else if let Some(d0) = d0 {
            for d in d0..d0 + span[ei] - 1 {
                if on_line[d] {
                    continue;
                }
                let x = xs[d] - min_x;
                let top = band_top[g.rank[d]];
                let bottom = band_bottom[g.rank[d]];
                pts.push(x);
                pts.push(top + inset(d));
                if bottom > top {
                    pts.push(x);
                    pts.push(bottom - inset(d));
                }
            }
        }
        pts.push(last.x + if e.reversed { e.tail_dx } else { e.head_dx });
        pts.push(last.y - last.h / 2.0);
        if e.reversed {
            let mut i = 0usize;
            let mut j = pts.len() - 2;
            while i < j {
                pts.swap(i, j);
                pts.swap(i + 1, j + 1);
                i += 2;
                j -= 2;
            }
        }
        let label_at = label_node[ei];
        let label_w = e.label_w;
        let label_pos = match label_at {
            Some(ln) => Some((
                xs[ln] - min_x,
                (band_top[g.rank[ln]] + band_bottom[g.rank[ln]]) / 2.0,
            )),
            None if label_w > 0.0 => {
                let len = pts.len();
                Some(((pts[0] + pts[len - 2]) / 2.0, (pts[1] + pts[len - 1]) / 2.0))
            }
            None => None,
        };
        let target = &mut edges[ei];
        target.points = pts;
        if let Some((lx, ly)) = label_pos {
            target.label_x = lx;
            target.label_y = ly;
        }
    }

    LayeredResult {
        width: max_x - min_x,
        height,
        crossings: None,
        declared: None,
    }
}

pub(crate) fn empty_ranks(g: &Graph) -> Option<Vec<bool>> {
    let mut empty = vec![false; g.max_rank + 1];
    let mut any = false;
    for r in 1..g.max_rank {
        let mut only = g.layer_start[r + 1] > g.layer_start[r];
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            if only && g.kind[g.order[i]] != Kind::Dummy {
                only = false;
            }
        }
        if only {
            empty[r] = true;
            any = true;
        }
    }
    if any { Some(empty) } else { None }
}

pub(crate) fn is_dummy(kind: Kind) -> bool {
    kind == Kind::Dummy || kind == Kind::Label
}
