use super::{Graph, LNode, MAX_SWEEPS, chain, sort_by_bary};

struct Counter {
    offsets: bool,
    place: Vec<usize>,
    place_in: Vec<usize>,
    places: Vec<usize>,
    leaving: Vec<usize>,
    mixed: Vec<bool>,
    targets: Vec<usize>,
    tree: Vec<i64>,
}

impl Counter {
    fn count(&mut self, g: &Graph) -> i64 {
        let mut total: i64 = 0;
        for r in 0..g.max_rank {
            let mut south_len = g.layer_start[r + 2] - g.layer_start[r + 1];
            if self.offsets {
                let mut next = 0usize;
                for i in g.layer_start[r + 1]..g.layer_start[r + 2] {
                    let v = g.order[i];
                    for k in g.pred_start[v]..g.pred_start[v + 1] {
                        self.place[g.pred_e[k]] = next + self.place_in[g.pred_e[k]];
                    }
                    next += self.places[v];
                }
                south_len = next;
            }
            if south_len < 2 {
                continue;
            }
            let mut t = 0usize;
            for i in g.layer_start[r]..g.layer_start[r + 1] {
                let v = g.order[i];
                let from = g.succ_start[v];
                let to = g.succ_start[v + 1];
                let t0 = t;
                for k in from..to {
                    self.targets[t] = if self.offsets {
                        self.place[self.leaving[k]]
                    } else {
                        g.pos[g.et[g.succ_e[k]]]
                    };
                    t += 1;
                }
                let d = t - t0;
                if d < 2 {
                    continue;
                }
                if self.offsets && self.mixed[v] {
                    for a in t0 + 1..t {
                        let item = self.targets[a];
                        let dx = g.efd[self.leaving[from + a - t0]];
                        let mut b = a as isize - 1;
                        while b >= t0 as isize
                            && g.efd[self.leaving[from + b as usize - t0]] == dx
                            && self.targets[b as usize] > item
                        {
                            self.targets[(b + 1) as usize] = self.targets[b as usize];
                            b -= 1;
                        }
                        self.targets[(b + 1) as usize] = item;
                    }
                } else if d <= 12 {
                    for a in t0 + 1..t {
                        let item = self.targets[a];
                        let mut b = a as isize - 1;
                        while b >= t0 as isize && self.targets[b as usize] > item {
                            self.targets[(b + 1) as usize] = self.targets[b as usize];
                            b -= 1;
                        }
                        self.targets[(b + 1) as usize] = item;
                    }
                } else {
                    self.targets[t0..t].sort();
                }
            }
            if t < 2 {
                continue;
            }
            let mut leaves = 1usize;
            while leaves < south_len {
                leaves <<= 1;
            }
            for x in self.tree.iter_mut().take(2 * leaves) {
                *x = 0;
            }
            for k in 0..t {
                let mut i = self.targets[k] + leaves;
                self.tree[i] += 1;
                while i > 1 {
                    if (i & 1) == 0 {
                        total += self.tree[i + 1];
                    }
                    i >>= 1;
                    self.tree[i] += 1;
                }
            }
        }
        total
    }
}

struct Groups {
    sum: Vec<f64>,
    count: Vec<i64>,
    first: Vec<usize>,
    depth: Vec<usize>,
}

fn arrange(
    n: usize,
    s: usize,
    seq: (&mut [usize], &mut [usize]),
    grp: &[i32],
    parent: &[i32],
    key: &[f64],
    groups: &mut Groups,
) {
    let (order, pos) = seq;
    for &v in order.iter() {
        let mut gr = grp[v];
        while gr >= 0 {
            groups.sum[gr as usize] = 0.0;
            groups.count[gr as usize] = 0;
            gr = parent[gr as usize];
        }
    }
    for (idx, &v) in order.iter().enumerate() {
        let mut gr = grp[v];
        while gr >= 0 {
            if groups.count[gr as usize] == 0 {
                groups.first[gr as usize] = s + idx;
            }
            groups.sum[gr as usize] += key[v];
            groups.count[gr as usize] += 1;
            gr = parent[gr as usize];
        }
    }
    let value = |item: usize, groups: &Groups| -> f64 {
        if item >= n {
            groups.sum[item - n] / groups.count[item - n] as f64
        } else {
            key[item]
        }
    };
    let first = |item: usize, groups: &Groups, pos: &[usize]| -> f64 {
        if item >= n {
            groups.first[item - n] as f64
        } else {
            (s + pos[item]) as f64
        }
    };
    let compare = |a: usize, b: usize, groups: &Groups, pos: &[usize]| -> std::cmp::Ordering {
        let mut ia = a;
        let mut ib = b;
        let mut ca = grp[a];
        let mut cb = grp[b];
        let mut da = if ca < 0 { 0 } else { groups.depth[ca as usize] };
        let mut db = if cb < 0 { 0 } else { groups.depth[cb as usize] };
        while da > db {
            ia = n + ca as usize;
            ca = parent[ca as usize];
            da -= 1;
        }
        while db > da {
            ib = n + cb as usize;
            cb = parent[cb as usize];
            db -= 1;
        }
        while ca != cb {
            ia = n + ca as usize;
            ca = parent[ca as usize];
            ib = n + cb as usize;
            cb = parent[cb as usize];
        }
        let by_value = value(ia, groups) - value(ib, groups);
        let by_first = first(ia, groups, pos) - first(ib, groups, pos);
        chain(
            by_value,
            by_first
                .partial_cmp(&0.0)
                .unwrap_or(std::cmp::Ordering::Equal),
        )
    };
    let mut row: Vec<usize> = order.to_vec();
    row.sort_by(|&a, &b| compare(a, b, groups, pos));
    order.copy_from_slice(&row);
    for (i, &v) in order.iter().enumerate() {
        pos[v] = i;
    }
}

struct Scratch {
    groups: Groups,
    bary: Vec<f64>,
    moving: Vec<usize>,
}

fn sweep(g: &mut Graph, nodes: &[LNode], sc: &mut Scratch, down: bool, bias_right: bool) {
    let max_rank = g.max_rank;
    let real = g.real;
    let n = g.n;
    for step in 1..=max_rank {
        let r = if down { step } else { max_rank - step };
        if g.fixed[r] {
            continue;
        }
        let s = g.layer_start[r];
        let e = g.layer_start[r + 1];
        let mut movable = 0usize;
        for i in s..e {
            let v = g.order[i];
            let pin = if v < real { nodes[v].pin } else { 0 };
            if pin != 0 {
                sc.bary[v] = pin as f64 * 1e9;
                sc.moving[movable] = v;
                movable += 1;
                continue;
            }
            let (from, to) = if down {
                (g.pred_start[v], g.pred_start[v + 1])
            } else {
                (g.succ_start[v], g.succ_start[v + 1])
            };
            if from == to {
                if g.grp.is_some() {
                    let across = if down {
                        g.layer_start[r] - g.layer_start[r - 1]
                    } else {
                        g.layer_start[r + 2] - g.layer_start[r + 1]
                    };
                    sc.bary[v] = ((i - s) * across) as f64 / (e - s) as f64;
                    continue;
                }
                sc.bary[v] = -1.0;
                continue;
            }
            let mut sum = 0.0f64;
            for k in from..to {
                let (u, dx) = if down {
                    (g.ef[g.pred_e[k]], g.efd[g.pred_e[k]])
                } else {
                    (g.et[g.succ_e[k]], g.etd[g.succ_e[k]])
                };
                sum += g.pos[u] as f64;
                if g.offsets && g.w[u] > 0.0 {
                    sum += dx / g.w[u];
                }
            }
            sc.bary[v] = sum / (to - from) as f64;
            sc.moving[movable] = v;
            movable += 1;
        }
        if let Some(grp) = g.grp.as_ref() {
            let parent = g
                .group_parent
                .as_ref()
                .expect("groups come with a parent list");
            arrange(
                n,
                s,
                (&mut g.order[s..e], &mut g.pos),
                grp,
                parent,
                &sc.bary,
                &mut sc.groups,
            );
            continue;
        }
        if movable < 2 {
            continue;
        }
        sort_by_bary(&mut sc.moving[..movable], &sc.bary, &g.pos, bias_right);
        let mut k = 0usize;
        for i in s..e {
            if sc.bary[g.order[i]] != -1.0 {
                g.order[i] = sc.moving[k];
                k += 1;
            }
        }
        for i in s..e {
            g.pos[g.order[i]] = i - s;
        }
    }
}

fn out_of_order(g: &Graph) -> i64 {
    let mut total = 0i64;
    for r in 0..=g.max_rank {
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            if g.order[i] >= g.real {
                continue;
            }
            for j in i + 1..g.layer_start[r + 1] {
                if g.order[j] < g.order[i] {
                    total += 1;
                }
            }
        }
    }
    total
}

fn settle_positions(g: &mut Graph) {
    for r in 0..=g.max_rank {
        for i in g.layer_start[r]..g.layer_start[r + 1] {
            g.pos[g.order[i]] = i - g.layer_start[r];
        }
    }
}

fn pin_of(nodes: &[LNode], real: usize, v: usize) -> i32 {
    if v < real { nodes[v].pin } else { 0 }
}

fn at(g: &Graph, e: usize, up: bool) -> f64 {
    let u = if up { g.ef[e] } else { g.et[e] };
    if g.offsets && g.w[u] > 0.0 {
        g.pos[u] as f64 + (if up { g.efd[e] } else { g.etd[e] }) / g.w[u]
    } else {
        g.pos[u] as f64
    }
}

fn between(g: &Graph, a: usize, b: usize) -> i64 {
    let mut c = 0i64;
    for k in g.pred_start[a]..g.pred_start[a + 1] {
        let x = at(g, g.pred_e[k], true);
        for j in g.pred_start[b]..g.pred_start[b + 1] {
            if x > at(g, g.pred_e[j], true) {
                c += 1;
            }
        }
    }
    for k in g.succ_start[a]..g.succ_start[a + 1] {
        let x = at(g, g.succ_e[k], false);
        for j in g.succ_start[b]..g.succ_start[b + 1] {
            if x > at(g, g.succ_e[j], false) {
                c += 1;
            }
        }
    }
    c
}

fn transpose(g: &mut Graph, nodes: &[LNode]) {
    let real = g.real;
    for _pass in 0..8 {
        let mut improved = false;
        for r in 0..=g.max_rank {
            if g.fixed[r] {
                continue;
            }
            let s = g.layer_start[r];
            let mut i = s;
            while i + 1 < g.layer_start[r + 1] {
                let a = g.order[i];
                let b = g.order[i + 1];
                let same_grp = match g.grp.as_ref() {
                    Some(grp) => grp[a] == grp[b],
                    None => true,
                };
                if pin_of(nodes, real, a) != pin_of(nodes, real, b) || !same_grp {
                    i += 1;
                    continue;
                }
                if between(g, b, a) >= between(g, a, b) {
                    i += 1;
                    continue;
                }
                g.order[i] = b;
                g.order[i + 1] = a;
                g.pos[b] = i - s;
                g.pos[a] = i + 1 - s;
                improved = true;
                i += 1;
            }
        }
        if !improved {
            break;
        }
    }
}

pub(crate) fn reduce_crossings(g: &mut Graph, nodes: &[LNode]) -> (i32, i32) {
    let n = g.n;
    let max_rank = g.max_rank;
    let mut widest = 0usize;
    for r in 0..=max_rank {
        widest = widest.max(g.layer_start[r + 1] - g.layer_start[r]);
    }
    if widest < 2 {
        return (0, 0);
    }
    let offsets = g.offsets;
    let seg = g.ef.len();
    let mut counter = Counter {
        offsets,
        place: vec![0usize; if offsets { seg } else { 0 }],
        place_in: vec![0usize; if offsets { seg } else { 0 }],
        places: vec![0usize; if offsets { n } else { 0 }],
        leaving: g.succ_e.clone(),
        mixed: vec![false; if offsets { n } else { 0 }],
        targets: vec![0usize; seg],
        tree: Vec::new(),
    };
    if offsets {
        let mut arriving = g.pred_e.clone();
        for v in 0..n {
            counter.places[v] = 1;
            if sort_by_offset(&mut arriving, g.pred_start[v], g.pred_start[v + 1], &g.etd) {
                for k in g.pred_start[v] + 1..g.pred_start[v + 1] {
                    if g.etd[arriving[k]] != g.etd[arriving[k - 1]] {
                        counter.places[v] += 1;
                    }
                    counter.place_in[arriving[k]] = counter.places[v] - 1;
                }
            }
            if sort_by_offset(
                &mut counter.leaving,
                g.succ_start[v],
                g.succ_start[v + 1],
                &g.efd,
            ) {
                counter.mixed[v] = true;
            }
        }
    }
    let mut size = 1usize;
    let need = if offsets { widest + seg } else { widest };
    while size < need {
        size <<= 1;
    }
    counter.tree = vec![0i64; 2 * size];

    let declared = counter.count(g);

    let groups_n = g.group_parent.as_ref().map_or(0, |p| p.len());
    let mut depth = vec![0usize; groups_n];
    if let Some(parent) = g.group_parent.as_ref() {
        for (gr, slot) in depth.iter_mut().enumerate().take(groups_n) {
            let mut p = gr as i32;
            while p >= 0 {
                *slot += 1;
                p = parent[p as usize];
            }
        }
    }
    let mut sc = Scratch {
        groups: Groups {
            sum: vec![0.0f64; groups_n],
            count: vec![0i64; groups_n],
            first: vec![0usize; groups_n],
            depth,
        },
        bary: vec![0.0f64; n],
        moving: vec![0usize; widest],
    };

    if let (Some(grp), Some(parent)) = (g.grp.as_ref(), g.group_parent.as_ref()) {
        let mut at_v = vec![0.0f64; n];
        for r in 0..=max_rank {
            for i in g.layer_start[r]..g.layer_start[r + 1] {
                at_v[g.order[i]] = (i - g.layer_start[r]) as f64;
            }
            let s = g.layer_start[r];
            let e = g.layer_start[r + 1];
            arrange(
                n,
                s,
                (&mut g.order[s..e], &mut g.pos),
                grp,
                parent,
                &at_v,
                &mut sc.groups,
            );
        }
    }

    let mut best = if g.grp.is_some() {
        counter.count(g)
    } else {
        declared
    };
    if best == 0 {
        return (0, declared as i32);
    }
    let mut best_order = g.order.clone();
    let start = best.min(declared);

    let initial = g.order.clone();
    let small = n <= 600;
    let mut chosen = best_order.clone();
    let mut chosen_out = i64::MAX;
    let attempts = if small { 2 } else { 1 };
    for attempt in 0..attempts {
        if attempt > 0 {
            g.order.copy_from_slice(&initial);
            settle_positions(g);
        }
        let mut found = start;
        let mut found_order = initial.clone();
        let mut i = 0usize;
        let mut stale = 0usize;
        while stale < 4 && i < MAX_SWEEPS {
            sweep(g, nodes, &mut sc, (i + attempt) % 2 == 0, i % 4 >= 2);
            let c = counter.count(g);
            if c < found {
                found = c;
                found_order.copy_from_slice(&g.order);
                stale = 0;
                if c == 0 {
                    break;
                }
            }
            i += 1;
            stale += 1;
        }
        g.order.copy_from_slice(&found_order);
        if small && found > 0 {
            settle_positions(g);
            transpose(g, nodes);
            let c = counter.count(g);
            if c < found {
                found = c;
                found_order.copy_from_slice(&g.order);
            }
        }
        let out = if small { out_of_order(g) } else { 0 };
        if attempt == 0 || found < best || (found == best && out < chosen_out) {
            best = found;
            chosen = found_order;
            chosen_out = out;
        }
    }
    g.order.copy_from_slice(&chosen);
    settle_positions(g);
    best_order.copy_from_slice(&chosen);
    if g.grp.is_some() {
        best = counter.count(g);
    }

    if !small {
        return (best as i32, declared as i32);
    }
    if nodes[..g.real].iter().any(|node| node.pin != 0) {
        return (best as i32, declared as i32);
    }
    if chosen_out == 0 || g.fixed[0] || g.fixed[max_rank] {
        return (best as i32, declared as i32);
    }
    for r in 0..=max_rank {
        let (s, e) = (g.layer_start[r], g.layer_start[r + 1]);
        g.order[s..e].reverse();
    }
    settle_positions(g);
    if counter.count(g) > best || out_of_order(g) >= chosen_out {
        g.order.copy_from_slice(&best_order);
    }
    (best as i32, declared as i32)
}

fn sort_by_offset(list: &mut [usize], from: usize, to: usize, dx: &[f64]) -> bool {
    for k in from + 1..to {
        if dx[list[k]] == dx[list[from]] {
            continue;
        }
        list[from..to].sort_by(|&a, &b| {
            let d = dx[a] - dx[b];
            if d < 0.0 {
                std::cmp::Ordering::Less
            } else if d > 0.0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        return true;
    }
    false
}
