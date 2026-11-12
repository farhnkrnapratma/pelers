use super::position::half_room;
use super::{BEND_SNAP, End, Graph, IN_LINE, LEAN_CLEAR, PORT_PITCH, SNAP, TWIN, chain, is_dummy};
use crate::layout::{Kind, LNode, LayeredOptions};
use crate::util::jsnum::{js_max, js_min};

fn dx_get(g: &Graph, side: usize, v: usize) -> f64 {
    if side == 0 { g.etd[v] } else { g.efd[v] }
}

pub(crate) fn side_ends(
    g: &Graph,
    v: usize,
    side: usize,
    span: f64,
    kinds: &[End],
    ids: &mut Vec<usize>,
    room: &mut Vec<f64>,
) -> f64 {
    let (start, list, other) = if side == 0 {
        (&g.pred_start, &g.pred_e, &g.ef)
    } else {
        (&g.succ_start, &g.succ_e, &g.et)
    };
    let pos = &g.pos;
    for k in start[v]..start[v + 1] {
        if kinds[list[k]] != End::Held {
            ids.push(list[k]);
        }
    }
    if ids.len() < 2 {
        return 0.0;
    }
    ids.sort_by(|&a, &b| chain(pos[other[a]] as f64 - pos[other[b]] as f64, a.cmp(&b)));
    let fill = |share: &dyn Fn(bool, bool) -> f64, ids: &[usize], room: &mut Vec<f64>| -> f64 {
        let mut total = 0.0f64;
        room.clear();
        room.resize(ids.len(), 0.0);
        for k in 1..ids.len() {
            let same = kinds[ids[k]] == kinds[ids[k - 1]];
            room[k] = share(same && kinds[ids[k]] == End::Leaves, same);
            total += room[k];
        }
        total
    };
    let mut total = fill(&|_, _| 1.0, ids, room);
    if span / total < super::MIN_PITCH {
        total = fill(&|leaves, _| if leaves { 0.5 } else { 1.0 }, ids, room);
    }
    if span / total < super::MIN_PITCH {
        total = fill(&|_, same| if same { 0.0 } else { 1.0 }, ids, room);
    }
    if total == 0.0 {
        return 0.0;
    }
    let pitch = js_min(PORT_PITCH, span / total);
    for r in &mut room[1..ids.len()] {
        *r *= pitch;
    }
    total * pitch
}

pub(crate) fn spread(g: &mut Graph, nodes: &[LNode], efk: &[End], etk: &[End]) {
    for v in 0..g.real {
        let span = js_min(nodes[v].span, g.w[v]);
        if span <= 0.0 || nodes[v].kind != Kind::Node {
            continue;
        }
        for side in 0..2 {
            let mut ids = Vec::new();
            let mut room = Vec::new();
            let kinds = if side == 0 { etk } else { efk };
            let total = side_ends(g, v, side, span, kinds, &mut ids, &mut room);
            if total == 0.0 {
                continue;
            }
            let dx = if side == 0 { &mut g.etd } else { &mut g.efd };
            let mut at = -total / 2.0;
            for k in 0..ids.len() {
                at += room[k];
                dx[ids[k]] = at;
            }
        }
    }
}

pub(crate) fn follow(g: &Graph, empty: Option<&[bool]>, mut e: usize, side: usize) -> usize {
    let Some(empty) = empty else {
        return e;
    };
    for _ in 0..100000 {
        let u = if side == 0 { g.ef[e] } else { g.et[e] };
        if g.kind[u] != Kind::Dummy || !empty[g.rank[u]] {
            break;
        }
        e = if side == 0 {
            g.pred_e[g.pred_start[u]]
        } else {
            g.succ_e[g.succ_start[u]]
        };
    }
    e
}

pub(crate) fn align_ports(g: &Graph, xs: &mut [f64], empty: Option<&[bool]>) {
    let max_rank = g.max_rank;
    for (r, kind) in [(0usize, Kind::StartPort), (max_rank, Kind::EndPort)] {
        let s = g.layer_start[r];
        let end = g.layer_start[r + 1];
        let mut to: Vec<f64> = Vec::new();
        for i in s..end {
            let v = g.order[i];
            let start = kind == Kind::StartPort;
            let degree = if start {
                g.succ_start[v + 1] - g.succ_start[v]
            } else {
                g.pred_start[v + 1] - g.pred_start[v]
            };
            if g.kind[v] != kind || degree != 1 {
                break;
            }
            let e = follow(
                g,
                empty,
                if start {
                    g.succ_e[g.succ_start[v]]
                } else {
                    g.pred_e[g.pred_start[v]]
                },
                if start { 1 } else { 0 },
            );
            let at = if start {
                xs[g.et[e]] + g.etd[e]
            } else {
                xs[g.ef[e]] + g.efd[e]
            };
            if let Some(&last) = to.last()
                && at < last + TWIN
            {
                break;
            }
            to.push(at);
        }
        if to.is_empty() || to.len() != end - s {
            continue;
        }
        for i in s..end {
            xs[g.order[i]] = to[i - s];
        }
    }
}

pub(crate) fn free_ends(g: &Graph, efk: &[End], etk: &[End]) -> [Vec<i32>; 2] {
    let mut free: [Vec<i32>; 2] = [vec![0; g.n], vec![0; g.n]];
    for e in 0..g.ef.len() {
        if etk[e] != End::Held {
            free[0][g.et[e]] += 1;
        }
        if efk[e] != End::Held {
            free[1][g.ef[e]] += 1;
        }
    }
    free
}

fn end_at(
    g: &Graph,
    nodes: &[LNode],
    xs: &[f64],
    free: &[Vec<i32>; 2],
    e: usize,
    side: usize,
) -> (f64, i32) {
    let u = if side == 0 { g.ef[e] } else { g.et[e] };
    let x = if side == 0 {
        xs[u] + g.efd[e]
    } else {
        xs[u] + g.etd[e]
    };
    let count = if u < g.real && nodes[u].kind == Kind::Node {
        free[1 - side][u]
    } else {
        0
    };
    (x, count)
}

fn choose(x: f64, up: (f64, i32), down: (f64, i32), lo: f64, hi: f64) -> f64 {
    let up_first = if up.1 != down.1 {
        up.1 < down.1
    } else {
        (up.0 - x).abs() <= (down.0 - x).abs()
    };
    let choices = if up_first {
        [up.0, down.0]
    } else {
        [down.0, up.0]
    };
    for c in choices {
        if c >= lo && c <= hi {
            return c;
        }
    }
    js_max(lo, js_min(hi, choices[0]))
}

pub(crate) fn align_labels(
    g: &Graph,
    nodes: &[LNode],
    xs: &mut [f64],
    free: &[Vec<i32>; 2],
    opt: &LayeredOptions,
    empty: Option<&[bool]>,
) {
    let sep = opt.edge_sep;
    let placed = |v: usize| -> bool {
        g.kind[v] == Kind::Label
            || (g.kind[v] == Kind::Dummy && !(empty.is_some_and(|e| e[g.rank[v]])))
    };
    let room = |xs: &[f64], v: usize| -> (f64, f64) {
        let s = g.layer_start[g.rank[v]];
        let i = s + g.pos[v];
        let clear = |other: usize| -> f64 {
            g.w[other] / 2.0
                + g.w[v] / 2.0
                + if g.kind[other] == Kind::Node {
                    opt.node_sep / 2.0 + sep / 2.0
                } else if g.kind[other] == g.kind[v] {
                    sep
                } else {
                    sep / 2.0
                }
        };
        let least = if i > s {
            xs[g.order[i - 1]] + clear(g.order[i - 1])
        } else {
            f64::NEG_INFINITY
        };
        let most = if i + 1 < g.layer_start[g.rank[v] + 1] {
            xs[g.order[i + 1]] - clear(g.order[i + 1])
        } else {
            f64::INFINITY
        };
        (least, most)
    };
    let done_len = g.n;
    let mut done = vec![false; done_len];
    let mut chain_v: Vec<usize> = Vec::new();
    for r in 1..g.max_rank {
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            let v0 = g.order[i];
            if !placed(v0) || done[v0] {
                continue;
            }
            let mut top = v0;
            while is_dummy(g.kind[g.ef[g.pred_e[g.pred_start[top]]]]) {
                top = g.ef[g.pred_e[g.pred_start[top]]];
            }
            let mut bottom = top;
            let mut lo = f64::NEG_INFINITY;
            let mut hi = f64::INFINITY;
            chain_v.clear();
            let mut d = top;
            while is_dummy(g.kind[d]) {
                bottom = d;
                if placed(d) {
                    done[d] = true;
                    chain_v.push(d);
                    let (least, most) = room(xs, d);
                    if least > lo {
                        lo = least;
                    }
                    if most < hi {
                        hi = most;
                    }
                }
                d = g.et[g.succ_e[g.succ_start[d]]];
            }
            if lo <= hi {
                let x = choose(
                    xs[chain_v[0]],
                    end_at(g, nodes, xs, free, g.pred_e[g.pred_start[top]], 0),
                    end_at(g, nodes, xs, free, g.succ_e[g.succ_start[bottom]], 1),
                    lo,
                    hi,
                );
                for &d in &chain_v {
                    xs[d] = x;
                }
                continue;
            }
            for &d in &chain_v {
                let (least, most) = room(xs, d);
                if least > most {
                    continue;
                }
                let up = end_at(
                    g,
                    nodes,
                    xs,
                    free,
                    follow(g, empty, g.pred_e[g.pred_start[d]], 0),
                    0,
                );
                let down = end_at(
                    g,
                    nodes,
                    xs,
                    free,
                    follow(g, empty, g.succ_e[g.succ_start[d]], 1),
                    1,
                );
                xs[d] = choose(xs[d], up, down, least, most);
            }
        }
    }
}

fn wanted(
    g: &Graph,
    nodes: &[LNode],
    state: (&[f64], &[Vec<i32>; 2], Option<&[bool]>),
    ends: (&[End], &[End]),
    at: (usize, usize, usize),
    span: f64,
    again: bool,
) -> f64 {
    let (xs, free, empty) = state;
    let (efk, etk) = ends;
    let (e0, side, v) = at;
    if free[side][v] < 2 {
        return 0.0;
    }
    let e = follow(g, empty, e0, side);
    let u = if side == 0 { g.ef[e] } else { g.et[e] };
    let point = if side == 0 {
        xs[u] + g.efd[e]
    } else {
        xs[u] + g.etd[e]
    };
    if again && (point - xs[v]).abs() <= span / 2.0 + IN_LINE {
        return js_max(-span / 2.0, js_min(span / 2.0, point - xs[v]));
    }
    let mut middle = point;
    let mut width = 0.0f64;
    let held = if side == 0 { efk[e] } else { etk[e] };
    if u < g.real && nodes[u].kind == Kind::Node && held != End::Held && free[1 - side][u] >= 2 {
        width = js_max(0.0, js_min(nodes[u].span, g.w[u]));
        if width > 0.0 {
            middle = xs[u];
        }
    }
    let lo = js_max(xs[v] - span / 2.0, middle - width / 2.0);
    let hi = js_min(xs[v] + span / 2.0, middle + width / 2.0);
    if lo > hi + IN_LINE {
        let lean = js_max(0.0, span / 2.0 - LEAN_CLEAR);
        return js_max(-lean, js_min(lean, point - xs[v]));
    }
    let place = (xs[v] * width + middle * span) / (width + span);
    js_max(lo, js_min(hi, place)) - xs[v]
}

pub(crate) fn settle(
    g: &mut Graph,
    nodes: &[LNode],
    xs: &mut [f64],
    efk: &[End],
    etk: &[End],
    free: &[Vec<i32>; 2],
    empty: Option<&[bool]>,
) {
    struct Entry {
        v: usize,
        side: usize,
        span: f64,
        ids: Vec<usize>,
        room: Vec<f64>,
        all: Vec<usize>,
    }
    let mut sides: Vec<Entry> = Vec::new();
    for v in 0..g.real {
        let span = js_min(nodes[v].span, g.w[v]);
        if span <= 0.0 || nodes[v].kind != Kind::Node {
            continue;
        }
        for side in 0..2 {
            let kinds = if side == 0 { etk } else { efk };
            let mut ids = Vec::new();
            let mut room = Vec::new();
            side_ends(g, v, side, span, kinds, &mut ids, &mut room);
            if ids.is_empty() {
                continue;
            }
            if ids.len() == 1 {
                room.clear();
                room.push(0.0);
            }
            let (start, list, other) = if side == 0 {
                (&g.pred_start, &g.pred_e, &g.ef)
            } else {
                (&g.succ_start, &g.succ_e, &g.et)
            };
            let mut all: Vec<usize> = (start[v]..start[v + 1]).map(|k| list[k]).collect();
            let pos = &g.pos;
            all.sort_by(|&a, &b| chain(pos[other[a]] as f64 - pos[other[b]] as f64, a.cmp(&b)));
            sides.push(Entry {
                v,
                side,
                span,
                ids,
                room,
                all,
            });
        }
    }

    let mut moved = true;
    let mut pass = 0usize;
    while moved && pass < 6 {
        moved = false;
        for entry in sides.iter() {
            let Entry {
                v,
                side,
                span,
                ids,
                room,
                all,
            } = entry;
            let (v, side, span) = (*v, *side, *span);
            let kinds = if side == 0 { etk } else { efk };
            let mut k = 0usize;
            let mut lo = -span / 2.0;
            for i in 0..=all.len() {
                let held = i < all.len() && kinds[all[i]] == End::Held;
                if i < all.len() && !held {
                    continue;
                }
                let mut size = 0usize;
                let mut j = i as isize - 1;
                while j >= 0 && kinds[all[j as usize]] != End::Held {
                    size += 1;
                    j -= 1;
                }
                let hi = if held {
                    dx_get(g, side, all[i]) - PORT_PITCH
                } else {
                    span / 2.0
                };
                if size > 0 {
                    let mut sum: Vec<f64> = Vec::new();
                    let mut count: Vec<f64> = Vec::new();
                    let mut before = 0.0f64;
                    for jj in 0..size {
                        if jj > 0 {
                            before += room[k + jj];
                        }
                        let w = wanted(
                            g,
                            nodes,
                            (xs, free, empty),
                            (efk, etk),
                            (ids[k + jj], side, v),
                            span,
                            pass > 0,
                        );
                        sum.push(w - before);
                        count.push(1.0);
                        let mut together = jj > 0 && room[k + jj] == 0.0;
                        while sum.len() > 1
                            && (together
                                || sum[sum.len() - 2] / count[count.len() - 2]
                                    > sum[sum.len() - 1] / count[count.len() - 1])
                        {
                            together = false;
                            let top = sum.pop().unwrap();
                            let pooled = count.pop().unwrap();
                            let last = sum.len() - 1;
                            sum[last] += top;
                            let last = count.len() - 1;
                            count[last] += pooled;
                        }
                    }
                    let total = before;
                    let mut jj = 0usize;
                    let mut before = 0.0f64;
                    for b in 0..sum.len() {
                        let level = if hi - total < lo {
                            (lo + hi - total) / 2.0
                        } else {
                            js_max(lo, js_min(hi - total, sum[b] / count[b]))
                        };
                        let mut c = 0usize;
                        while (c as f64) < count[b] {
                            if jj > 0 {
                                before += room[k + jj];
                            }
                            let target = ids[k + jj];
                            let dx: &mut Vec<f64> = if side == 0 { &mut g.etd } else { &mut g.efd };
                            if (dx[target] - level - before).abs() > 0.01 {
                                moved = true;
                            }
                            dx[target] = level + before;
                            c += 1;
                            jj += 1;
                        }
                    }
                    k += size;
                }
                if held {
                    lo = dx_get(g, side, all[i]) + PORT_PITCH;
                }
            }
        }
        pass += 1;
    }
}

pub(crate) fn snap(g: &Graph, xs: &mut [f64], empty: Option<&[bool]>, opt: &LayeredOptions) {
    let n = g.n;
    let passed =
        |u: usize| -> bool { empty.is_some_and(|e| g.kind[u] == Kind::Dummy && e[g.rank[u]]) };
    let mut a: Vec<usize> = Vec::new();
    let mut b: Vec<usize> = Vec::new();
    let mut da: Vec<f64> = Vec::new();
    let mut db: Vec<f64> = Vec::new();
    for e in 0..g.ef.len() {
        if passed(g.ef[e]) {
            continue;
        }
        let f = follow(g, empty, e, 1);
        a.push(g.ef[e]);
        b.push(g.et[f]);
        da.push(g.efd[e]);
        db.push(g.etd[f]);
    }
    let mut block: Vec<usize> = (0..n).collect();
    let mut members: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();
    fn join(block: &mut [usize], members: &mut [Vec<usize>], x: usize, y: usize) {
        let mut keep = block[x];
        let mut drop = block[y];
        if keep == drop {
            return;
        }
        if members[keep].len() < members[drop].len() {
            std::mem::swap(&mut keep, &mut drop);
        }
        let moved = std::mem::take(&mut members[drop]);
        for v in moved {
            block[v] = keep;
            members[keep].push(v);
        }
    }
    let off = |xs: &[f64], k: usize| -> f64 { xs[b[k]] + db[k] - xs[a[k]] - da[k] };
    let mut near: Vec<usize> = Vec::new();
    for k in 0..a.len() {
        let d = off(xs, k).abs();
        if d < 0.01 {
            join(&mut block, &mut members, a[k], b[k]);
        } else if d <= BEND_SNAP {
            near.push(k);
        }
    }
    if near.is_empty() {
        return;
    }
    near.sort_by(|&x, &y| chain((off(xs, x).abs()) - (off(xs, y).abs()), x.cmp(&y)));
    let gap = |v: usize| half_room(g, opt, v);
    let free =
        |block: &[usize], members: &[Vec<usize>], xs: &[f64], id: usize, shift: f64| -> bool {
            for &v in &members[id] {
                let s = g.layer_start[g.rank[v]];
                let end = g.layer_start[g.rank[v] + 1];
                let i = s + g.pos[v];
                if i > s
                    && block[g.order[i - 1]] != id
                    && xs[v] + shift - xs[g.order[i - 1]] < gap(v) + gap(g.order[i - 1]) - 0.01
                {
                    return false;
                }
                if i + 1 < end
                    && block[g.order[i + 1]] != id
                    && xs[g.order[i + 1]] - xs[v] - shift < gap(v) + gap(g.order[i + 1]) - 0.01
                {
                    return false;
                }
            }
            true
        };
    for &k in &near {
        let d = off(xs, k);
        if d.abs() < 0.01 {
            join(&mut block, &mut members, a[k], b[k]);
            continue;
        }
        if d.abs() > BEND_SNAP || block[a[k]] == block[b[k]] {
            continue;
        }
        let upper = block[a[k]];
        let lower = block[b[k]];
        let first = if members[upper].len() <= members[lower].len() {
            upper
        } else {
            lower
        };
        let second = if first == upper { lower } else { upper };
        for id in [first, second] {
            let shift = if id == upper { d } else { -d };
            if d.abs() > SNAP && !members[id].iter().all(|&v| g.kind[v] == Kind::Dummy) {
                continue;
            }
            if !free(&block, &members, xs, id, shift) {
                continue;
            }
            for &v in &members[id] {
                xs[v] += shift;
            }
            join(&mut block, &mut members, a[k], b[k]);
            break;
        }
    }
}

pub(crate) fn straighten(
    g: &Graph,
    xs: &mut [f64],
    empty: &[bool],
    first_dummy: &[Option<usize>],
    span: &[usize],
    sep: f64,
    on_line: &mut [bool],
) {
    let len = xs.len();
    let mut want = vec![0.0f64; len];
    let mut from = vec![0.0f64; len];
    let mut to = vec![0.0f64; len];
    for ei in 0..first_dummy.len() {
        let Some(d0) = first_dummy[ei] else {
            continue;
        };
        let end = d0 + span[ei] - 1;
        let base = g.rank[d0];
        let mut d = d0;
        while d < end {
            if !empty[base + d - d0] {
                d += 1;
                continue;
            }
            let mut last = d;
            while last + 1 < end && empty[base + last + 1 - d0] {
                last += 1;
            }
            let above = g.pred_e[g.pred_start[d]];
            let below = g.succ_e[g.succ_start[last]];
            let x0 = xs[g.ef[above]] + g.efd[above];
            let x1 = xs[g.et[below]] + g.etd[below];
            let steps = (last - d + 2) as f64;
            for k in d..=last {
                want[k] = x0 + ((x1 - x0) * (k - d + 1) as f64) / steps;
                from[k] = x0;
                to[k] = x1;
            }
            d = last + 1;
        }
    }

    let mut twins: Vec<usize> = Vec::new();
    for r in 1..g.max_rank {
        if !empty[r] {
            continue;
        }
        let s = g.layer_start[r];
        let end = g.layer_start[r + 1];
        let mut row: Vec<usize> = g.order[s..end].to_vec();
        row.sort_by(|&a, &b| chain(from[a] - from[b], chain(to[a] - to[b], a.cmp(&b))));
        let mut i = 0usize;
        while i < row.len() {
            twins.clear();
            let mut j = i;
            while j < row.len()
                && (from[row[j]] - from[row[i]]).abs() < TWIN
                && (to[row[j]] - to[row[i]]).abs() < TWIN
            {
                twins.push(row[j]);
                j += 1;
            }
            twins.sort_by(|&a, &b| g.pos[a].cmp(&g.pos[b]));
            for k in 0..twins.len() {
                let d = twins[k];
                xs[d] = want[d] + (k as f64 - (twins.len() as f64 - 1.0) / 2.0) * sep;
                if twins.len() == 1 || sep == 0.0 {
                    on_line[d] = true;
                }
            }
            i = j;
        }
    }
}

pub(crate) fn past_labels(
    g: &Graph,
    xs: &mut [f64],
    first_dummy: &[Option<usize>],
    span: &[usize],
    clear: f64,
    on_line: &mut [bool],
) {
    let max_rank = g.max_rank;
    let mut labelled = vec![false; max_rank + 1];
    let mut any = false;
    for r in 1..max_rank {
        let mut labels = false;
        let mut others = false;
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            let kind = g.kind[g.order[i]];
            if kind == Kind::Label {
                labels = true;
            } else if kind != Kind::Dummy {
                others = true;
            }
        }
        if labels && !others {
            labelled[r] = true;
            any = true;
        }
    }
    if !any {
        return;
    }
    for ei in 0..first_dummy.len() {
        let Some(d0) = first_dummy[ei] else {
            continue;
        };
        let end = d0 + span[ei] - 1;
        for d in d0..end {
            if g.kind[d] != Kind::Dummy || !labelled[g.rank[d]] {
                continue;
            }
            let mut first = d;
            let mut last = d;
            while first > d0 && on_line[first - 1] {
                first -= 1;
            }
            while last + 1 < end && on_line[last + 1] {
                last += 1;
            }
            let above = g.pred_e[g.pred_start[first]];
            let below = g.succ_e[g.succ_start[last]];
            let from = xs[g.ef[above]] + g.efd[above];
            let to = xs[g.et[below]] + g.etd[below];
            let lo = js_min(from, to) - clear;
            let hi = js_max(from, to) + clear;
            let mut free = true;
            let r = g.rank[d];
            for i in g.layer_start[r]..g.layer_start[r + 1] {
                if !free {
                    break;
                }
                let label = g.order[i];
                if g.kind[label] == Kind::Label
                    && xs[label] + g.w[label] / 2.0 > lo
                    && xs[label] - g.w[label] / 2.0 < hi
                {
                    free = false;
                }
            }
            if !free {
                continue;
            }
            on_line[d] = true;
            let steps = (last - first + 2) as f64;
            for (offset, x) in xs[first..=last].iter_mut().enumerate() {
                *x = from + ((to - from) * (offset + 1) as f64) / steps;
            }
        }
    }
}
