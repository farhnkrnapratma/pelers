use super::{Graph, is_dummy, sort_range};
use crate::layout::{Kind, LayeredOptions};

pub(crate) fn half_room(g: &Graph, opt: &LayeredOptions, v: usize) -> f64 {
    g.w[v] / 2.0
        + (if g.kind[v] == Kind::Node {
            opt.node_sep
        } else {
            opt.edge_sep
        }) / 2.0
}

pub(crate) fn position(g: &mut Graph, opt: &LayeredOptions) -> Vec<f64> {
    let n = g.n;
    let max_rank = g.max_rank;
    let segments = g.ef.len();
    let gap: Vec<f64> = (0..n).map(|i| half_room(&*g, opt, i)).collect();

    let mut key = vec![0usize; segments];
    for e in 0..segments {
        key[e] = g.pos[g.et[e]];
    }
    for v in 0..n {
        if g.succ_start[v + 1] - g.succ_start[v] > 1 {
            sort_range(&mut g.succ_e, g.succ_start[v], g.succ_start[v + 1], &key);
        }
    }
    for e in 0..segments {
        key[e] = g.pos[g.ef[e]];
    }
    for v in 0..n {
        if g.pred_start[v + 1] - g.pred_start[v] > 1 {
            sort_range(&mut g.pred_e, g.pred_start[v], g.pred_start[v + 1], &key);
        }
    }

    let mut conflict = vec![false; segments];
    for r in 1..=max_rank {
        let prev_len = g.layer_start[r] - g.layer_start[r - 1];
        let s = g.layer_start[r];
        let end = g.layer_start[r + 1];
        let mut k0: i64 = 0;
        let mut scan = s;
        for i in s..end {
            let v = g.order[i];
            let mut inner: Option<usize> = None;
            if is_dummy(g.kind[v]) && g.pred_start[v] < g.pred_start[v + 1] {
                let u = g.ef[g.pred_e[g.pred_start[v]]];
                if is_dummy(g.kind[u]) {
                    inner = Some(u);
                }
            }
            let k1 = match inner {
                Some(u) => g.pos[u] as i64,
                None => prev_len as i64,
            };
            if inner.is_some() || i == end - 1 {
                for j in scan..=i {
                    let w = g.order[j];
                    let w_dummy = is_dummy(g.kind[w]);
                    for k in g.pred_start[w]..g.pred_start[w + 1] {
                        let e = g.pred_e[k];
                        let up = g.pos[g.ef[e]] as i64;
                        if (up < k0 || k1 < up) && !(w_dummy && is_dummy(g.kind[g.ef[e]])) {
                            conflict[e] = true;
                        }
                    }
                }
                scan = i + 1;
                k0 = k1;
            }
        }
    }

    let mut results: Vec<Vec<f64>> = Vec::with_capacity(4);
    let mut root = vec![0usize; n];
    let mut align = vec![0usize; n];
    let mut shift = vec![0.0f64; n];
    let mut indeg = vec![0usize; n];
    let mut x = vec![0.0f64; n];

    for variant in 0..4 {
        let up = variant < 2;
        let right = (variant & 1) == 1;
        let (start, list, other, own_dx, other_dx) = if up {
            (&g.pred_start, &g.pred_e, &g.ef, &g.etd, &g.efd)
        } else {
            (&g.succ_start, &g.succ_e, &g.et, &g.efd, &g.etd)
        };
        for i in 0..n {
            root[i] = i;
            align[i] = i;
            shift[i] = 0.0;
            indeg[i] = 0;
            x[i] = 0.0;
        }
        for step in 0..=max_rank {
            let r = if up { step } else { max_rank - step };
            let s = g.layer_start[r];
            let end = g.layer_start[r + 1];
            let adjacent = if up { r as isize - 1 } else { r as isize + 1 };
            let adjacent_last: i64 = if adjacent >= 0 && adjacent <= max_rank as isize {
                let a = adjacent as usize;
                (g.layer_start[a + 1] - g.layer_start[a]) as i64 - 1
            } else {
                0
            };
            let mut prev_idx: i64 = -1;
            let indices: Vec<usize> = if right {
                (s..end).rev().collect()
            } else {
                (s..end).collect()
            };
            for i in indices {
                let v = g.order[i];
                let from = start[v];
                let d = start[v + 1] - from;
                if d == 0 {
                    continue;
                }
                let lo = from + (d - 1) / 2;
                let hi = from + d / 2;
                for pass in 0..2 {
                    if pass == 1 && lo == hi {
                        break;
                    }
                    let e = list[if (pass == 0) != right { lo } else { hi }];
                    let w = other[e];
                    let idx: i64 = if right {
                        adjacent_last - g.pos[w] as i64
                    } else {
                        g.pos[w] as i64
                    };
                    if align[v] == v && prev_idx < idx && !conflict[e] {
                        align[w] = v;
                        root[v] = root[w];
                        align[v] = root[w];
                        shift[v] = shift[w] + other_dx[e] - own_dx[e];
                        prev_idx = idx;
                    }
                }
            }
        }

        let mut e_from: Vec<usize> = Vec::new();
        let mut e_to: Vec<usize> = Vec::new();
        let mut e_w: Vec<f64> = Vec::new();
        let mut out_start = vec![0usize; n + 1];
        for r in 0..=max_rank {
            let s = g.layer_start[r];
            let end = g.layer_start[r + 1];
            for i in s + 1..end {
                let a = if right { g.order[i] } else { g.order[i - 1] };
                let b = if right { g.order[i - 1] } else { g.order[i] };
                let sep = gap[a] + gap[b];
                e_from.push(root[a]);
                e_to.push(root[b]);
                e_w.push(if right {
                    sep - shift[a] + shift[b]
                } else {
                    sep + shift[a] - shift[b]
                });
                indeg[root[b]] += 1;
                out_start[root[a] + 1] += 1;
            }
        }
        for i in 0..n {
            out_start[i + 1] += out_start[i];
        }
        let block_edges = e_from.len();
        let mut out_e = vec![0usize; block_edges];
        {
            let mut fill = out_start[..n].to_vec();
            for e in 0..block_edges {
                out_e[fill[e_from[e]]] = e;
                fill[e_from[e]] += 1;
            }
        }

        let mut topo = vec![0usize; n];
        let mut count = 0;
        for i in 0..n {
            if root[i] == i && indeg[i] == 0 {
                topo[count] = i;
                count += 1;
            }
        }
        let mut q = 0;
        while q < count {
            let v = topo[q];
            for &e in &out_e[out_start[v]..out_start[v + 1]] {
                let w = e_to[e];
                if x[v] + e_w[e] > x[w] {
                    x[w] = x[v] + e_w[e];
                }
                indeg[w] -= 1;
                if indeg[w] == 0 {
                    topo[count] = w;
                    count += 1;
                }
            }
            q += 1;
        }
        for q in (0..count).rev() {
            let v = topo[q];
            let mut min = f64::INFINITY;
            for &e in &out_e[out_start[v]..out_start[v + 1]] {
                let cand = x[e_to[e]] - e_w[e];
                if cand < min {
                    min = cand;
                }
            }
            if min != f64::INFINITY && min > x[v] {
                x[v] = min;
            }
        }
        let xs: Vec<f64> = (0..n)
            .map(|i| (if right { -x[root[i]] } else { x[root[i]] }) + shift[i])
            .collect();
        results.push(xs);
    }

    let mut narrowest = 0usize;
    let mut best_width = f64::INFINITY;
    let mut mins: Vec<f64> = Vec::with_capacity(4);
    let mut maxs: Vec<f64> = Vec::with_capacity(4);
    for (k, xs) in results.iter().enumerate().take(4) {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for i in 0..n {
            let half = g.w[i] / 2.0;
            if xs[i] - half < lo {
                lo = xs[i] - half;
            }
            if xs[i] + half > hi {
                hi = xs[i] + half;
            }
        }
        mins.push(lo);
        maxs.push(hi);
        if hi - lo < best_width {
            best_width = hi - lo;
            narrowest = k;
        }
    }
    for k in 0..4 {
        let delta = if (k & 1) == 0 {
            mins[narrowest] - mins[k]
        } else {
            maxs[narrowest] - maxs[k]
        };
        if delta != 0.0 {
            for value in results[k].iter_mut().take(n) {
                *value += delta;
            }
        }
    }

    (0..n)
        .map(|i| {
            let mut a = results[0][i];
            let mut b = results[1][i];
            let mut c = results[2][i];
            let mut d = results[3][i];
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            if c > d {
                std::mem::swap(&mut c, &mut d);
            }
            if a > c {
                std::mem::swap(&mut a, &mut c);
            }
            if b > d {
                std::mem::swap(&mut b, &mut d);
            }
            if b > c {
                std::mem::swap(&mut b, &mut c);
            }
            (b + c) / 2.0
        })
        .collect()
}
