use std::collections::{BTreeMap, HashSet};

use super::{
    CROSSED_SLOPE, IN_LINE, LEVEL_CLEAR, LEVEL_LEAD, LEVEL_TOUCH, MAX_WEIGHED, Pieces, SAMPLES,
    chain, ease,
};
use crate::util::jsnum::{js_hypot, js_max, js_min};

fn within(x: f64, lo: f64, hi: f64) -> i32 {
    (x >= lo - IN_LINE && x <= hi + IN_LINE) as i32
}

fn sample(from: f64, to: f64, h: f64, out: &mut [f64]) {
    let c = ease(to - from, h);
    for i in 0..=SAMPLES {
        let t = i as f64 / SAMPLES as f64;
        let u = 1.0 - t;
        out[2 * i] = from + (to - from) * t * t * (3.0 - 2.0 * t);
        out[2 * i + 1] = h * (3.0 * c * t * u * u + 3.0 * (1.0 - c) * t * t * u + t * t * t);
    }
}

fn reaches(points: &[f64], line: &[f64], limit: f64) -> bool {
    let bound = limit * limit * (1.0 + 1e-9);
    for i in 0..=SAMPLES {
        let px = points[2 * i];
        let py = points[2 * i + 1];
        for j in 0..SAMPLES {
            let dx = line[2 * j + 2] - line[2 * j];
            let dy = line[2 * j + 3] - line[2 * j + 1];
            let len = dx * dx + dy * dy;
            let len = if len == 0.0 || len.is_nan() { 1.0 } else { len };
            let t = js_max(
                0.0,
                js_min(
                    1.0,
                    ((px - line[2 * j]) * dx + (py - line[2 * j + 1]) * dy) / len,
                ),
            );
            let ex = px - line[2 * j] - t * dx;
            let ey = py - line[2 * j + 1] - t * dy;
            if ex * ex + ey * ey < bound && js_hypot(ex, ey) < limit {
                return true;
            }
        }
    }
    false
}

fn near(a: (f64, f64), b: (f64, f64), height: f64, limit: f64) -> bool {
    let h = js_max(height / 2.0, height - 2.0 * LEVEL_LEAD);
    let mut line_a = [0.0f64; 2 * SAMPLES + 2];
    let mut line_b = [0.0f64; 2 * SAMPLES + 2];
    sample(a.0, a.1, h, &mut line_a);
    sample(b.0, b.1, h, &mut line_b);
    reaches(&line_a, &line_b, limit) || reaches(&line_b, &line_a, limit)
}

fn find(group: &mut [usize], mut i: usize) -> usize {
    while group[i] != i {
        group[i] = group[group[i]];
        i = group[i];
    }
    i
}

pub(crate) fn assign_tracks(
    pieces: &mut Pieces,
    track_count: &mut [i32],
    curved: bool,
    room: &[f64],
    tall: &mut [f64],
) {
    let total = pieces.from.len();
    let from = pieces.from.clone();
    let to = pieces.to.clone();
    let rank = pieces.rank.clone();
    let wide = pieces.wide.clone();
    let mut by_rank: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    pieces.track.clear();
    pieces.levels.clear();
    for k in 0..total {
        pieces.track.push(-1);
        pieces.levels.push(0);
        if (from[k] - to[k]).abs() < IN_LINE {
            continue;
        }
        by_rank.entry(rank[k]).or_default().push(k);
    }
    if by_rank.is_empty() {
        return;
    }
    let lo_v: Vec<f64> = (0..total).map(|k| js_min(from[k], to[k])).collect();
    let hi_v: Vec<f64> = (0..total).map(|k| js_max(from[k], to[k])).collect();
    let mut track = std::mem::take(&mut pieces.track);
    let mut levels = std::mem::take(&mut pieces.levels);

    let within_p = |x: f64, k: usize| within(x, lo_v[k], hi_v[k]);
    let mates =
        |a: usize, b: usize| (from[a] - from[b]).abs() < IN_LINE || (to[a] - to[b]).abs() < IN_LINE;
    let overlap = |a: usize, b: usize| lo_v[a] < hi_v[b] - IN_LINE && lo_v[b] < hi_v[a] - IN_LINE;
    let clash = |a: usize, b: usize| overlap(a, b) && !mates(a, b);
    let meet = |a: usize, b: usize| {
        overlap(a, b)
            || within_p(to[a], b) + within_p(from[b], a) + within_p(to[b], a) + within_p(from[a], b)
                > 0
    };
    let crossings = |a: usize, b: usize| within_p(to[a], b) + within_p(from[b], a);

    let near_pairs = |list: &[usize]| -> Vec<(usize, usize)> {
        let mut by_lo: Vec<usize> = (0..list.len()).collect();
        by_lo.sort_by(|&a, &b| {
            let d = lo_v[list[a]] - lo_v[list[b]];
            if d < 0.0 {
                std::cmp::Ordering::Less
            } else if d > 0.0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        let mut pairs = Vec::new();
        for x in 0..by_lo.len() {
            let i = by_lo[x];
            let reach = hi_v[list[i]] + 4.0 * IN_LINE;
            for y in x + 1..by_lo.len() {
                if lo_v[list[by_lo[y]]] > reach {
                    break;
                }
                let j = by_lo[y];
                pairs.push((i.min(j), i.max(j)));
            }
        }
        pairs
    };

    let lay = |list: &mut Vec<usize>, track: &mut [i32], levels: &mut [i32]| -> i32 {
        list.sort_by(|&a, &b| {
            let d1 = (hi_v[b] - lo_v[b]) - (hi_v[a] - lo_v[a]);
            let d2 = lo_v[a] - lo_v[b];
            let d3 = a as f64 - b as f64;
            chain(d1, chain(d2, chain(d3, std::cmp::Ordering::Equal)))
        });
        let count = list.len();
        let mut above: Vec<Vec<usize>> = Vec::new();
        let mut group: Vec<usize> = (0..count).collect();
        let weighed = count <= MAX_WEIGHED;
        let pairs = if weighed {
            near_pairs(list)
        } else {
            Vec::new()
        };
        let mut clashes: Vec<Vec<usize>> = if weighed {
            vec![Vec::new(); count]
        } else {
            Vec::new()
        };
        for &(i, j) in &pairs {
            if !clash(list[i], list[j]) {
                continue;
            }
            clashes[i].push(j);
            clashes[j].push(i);
        }
        let mut sequence: Vec<usize> = (0..count).collect();
        if weighed {
            let mut waiting = vec![0i32; count];
            let mut after: Vec<Vec<usize>> = vec![Vec::new(); count];
            above = vec![Vec::new(); count];
            for &(i, j) in &pairs {
                let a = list[i];
                let b = list[j];
                if !meet(a, b) {
                    continue;
                }
                let ri = find(&mut group, i);
                let rj = find(&mut group, j);
                group[ri] = rj;
                let ab = crossings(a, b);
                let ba = crossings(b, a);
                if ab == ba {
                    continue;
                }
                let (first, second) = if ab < ba { (i, j) } else { (j, i) };
                above[second].push(first);
                after[first].push(second);
                waiting[second] += 1;
            }
            let mut placed = vec![false; count];
            sequence = Vec::with_capacity(count);
            for _done in 0..count {
                let mut pick: Option<usize> = None;
                for i in 0..count {
                    if placed[i] {
                        continue;
                    }
                    match pick {
                        None => pick = Some(i),
                        Some(p) if waiting[i] < waiting[p] => pick = Some(i),
                        _ => {}
                    }
                    if waiting[i] == 0 {
                        break;
                    }
                }
                let pick = pick.expect("a piece is always left to place");
                placed[pick] = true;
                sequence.push(pick);
                for &j in &after[pick] {
                    waiting[j] -= 1;
                }
            }
        }
        let mut taken: HashSet<i32> = HashSet::new();
        let mut on_track: Vec<Vec<usize>> = Vec::new();
        let mut rows = 0i32;
        for &at in &sequence {
            let k = list[at];
            let mut t = 0i32;
            if let Some(ab) = above.get(at) {
                for &i in ab {
                    if track[list[i]] >= t {
                        t = track[list[i]] + 1;
                    }
                }
            }
            if weighed {
                taken.clear();
                for &i in &clashes[at] {
                    taken.insert(track[list[i]]);
                }
                while taken.contains(&t) {
                    t += 1;
                }
            } else {
                loop {
                    let t_us = t as usize;
                    let busy = on_track
                        .get(t_us)
                        .is_some_and(|others| others.iter().any(|&other| clash(k, other)));
                    if !busy {
                        break;
                    }
                    t += 1;
                }
                let t_us = t as usize;
                if on_track.len() <= t_us {
                    on_track.resize(t_us + 1, Vec::new());
                }
                on_track[t_us].push(k);
            }
            track[k] = t;
            if t >= rows {
                rows = t + 1;
            }
        }
        let mut first = vec![rows; count];
        let mut last = vec![-1i32; count];
        for i in 0..count {
            let root = find(&mut group, i);
            first[root] = first[root].min(track[list[i]]);
            last[root] = last[root].max(track[list[i]]);
        }
        for i in 0..count {
            let root = find(&mut group, i);
            levels[list[i]] = last[root] - first[root] + 1;
            if curved {
                track[list[i]] -= first[root];
            }
        }
        rows
    };

    for (r, all) in by_rank.iter_mut() {
        let r = *r;
        if !curved {
            track_count[r] = lay(all, &mut track, &mut levels);
            continue;
        }
        if all.len() > MAX_WEIGHED {
            continue;
        }
        let mut far: Vec<usize> = all.iter().copied().filter(|&k| wide[k]).collect();
        lay(&mut far, &mut track, &mut levels);
        let mut level: HashSet<usize> = far.iter().copied().filter(|&k| levels[k] > 1).collect();
        let pairs = near_pairs(all);
        for &(pi, pj) in &pairs {
            let a = all[pi];
            let b = all[pj];
            if (level.contains(&a) && level.contains(&b))
                || !clash(a, b)
                || ((from[a] < to[a]) != (from[b] < to[b]))
            {
                continue;
            }
            let limit = if wide[a] || wide[b] {
                LEVEL_CLEAR
            } else {
                LEVEL_TOUCH
            };
            if near((from[a], to[a]), (from[b], to[b]), room[r], limit) {
                level.insert(a);
                level.insert(b);
            }
        }
        for &k in &far {
            track[k] = -1;
            levels[k] = 0;
        }
        if level.is_empty() {
            continue;
        }
        let mut crossing = 0usize;
        let mut alongside = 0usize;
        let mut beside: HashSet<usize> = HashSet::new();
        for &(pi, pj) in &pairs {
            let a = all[pi];
            let b = all[pj];
            if level.contains(&a) && level.contains(&b) {
                if !overlap(a, b) {
                    continue;
                }
                if (from[a] < to[a]) != (from[b] < to[b]) {
                    crossing += 1;
                } else if !mates(a, b) {
                    alongside += 1;
                }
            } else if (level.contains(&a) || level.contains(&b)) && meet(a, b) {
                beside.insert(a);
                beside.insert(b);
            }
        }
        if crossing > alongside {
            for &k in all.iter() {
                let slope = (hi_v[k] - lo_v[k]) * CROSSED_SLOPE;
                if slope > tall[r] {
                    tall[r] = slope;
                }
            }
            continue;
        }
        let mut members: Vec<usize> = all
            .iter()
            .copied()
            .filter(|&k| level.contains(&k) || (!wide[k] && beside.contains(&k)))
            .collect();
        lay(&mut members, &mut track, &mut levels);
        let mut most = 0i32;
        for &k in &members {
            if levels[k] > most {
                most = levels[k];
            }
        }
        track_count[r] = most;
    }
    pieces.track = track;
    pieces.levels = levels;
}
