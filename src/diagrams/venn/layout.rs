use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;

use super::db::Subset;
use crate::util::jsnum::{js_hypot, js_max, js_min};

const TAU: f64 = PI * 2.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Circle {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub x: f64,
    pub y: f64,
    pub margin: f64,
}

fn sorted_members(sets: &[String]) -> Vec<String> {
    let mut members = sets.to_vec();
    members.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    members
}

pub fn ensure_pairwise_subsets(subsets: &[Subset]) -> Vec<Subset> {
    let mut present: HashSet<String> = HashSet::new();
    let mut sizes: Vec<(String, f64)> = Vec::new();
    for subset in subsets {
        present.insert(sorted_members(&subset.sets).join("\u{0}"));
        if subset.sets.len() == 1 {
            set_value(&mut sizes, &subset.sets[0], subset.size);
        }
    }
    let mut added: Vec<Subset> = Vec::new();
    for subset in subsets {
        if subset.sets.len() < 3 {
            continue;
        }
        let members = sorted_members(&subset.sets);
        for i in 0..members.len() - 1 {
            for j in i + 1..members.len() {
                let pair = vec![members[i].clone(), members[j].clone()];
                let key = pair.join("\u{0}");
                if present.contains(&key) {
                    continue;
                }
                present.insert(key);
                let a = get_value(&sizes, &pair[0]);
                let b = get_value(&sizes, &pair[1]);
                added.push(Subset {
                    sets: pair,
                    size: match (a, b) {
                        (Some(a), Some(b)) => js_min(a, b) / 4.0,
                        _ => 2.5,
                    },
                    label: Some(String::new()),
                });
            }
        }
    }
    if added.is_empty() {
        subsets.to_vec()
    } else {
        let mut all = subsets.to_vec();
        all.extend(added);
        all
    }
}

fn get_value(map: &[(String, f64)], key: &str) -> Option<f64> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| *v)
}

fn set_value(map: &mut Vec<(String, f64)>, key: &str, value: f64) {
    match map.iter_mut().find(|(k, _)| k == key) {
        Some(entry) => entry.1 = value,
        None => map.push((key.to_owned(), value)),
    }
}

pub fn lens_area(r1: f64, r2: f64, d: f64) -> f64 {
    if d >= r1 + r2 {
        return 0.0;
    }
    let small = js_min(r1, r2);
    if d <= (r1 - r2).abs() {
        return PI * small * small;
    }
    let a1 = r1 * r1 * ((d * d + r1 * r1 - r2 * r2) / (2.0 * d * r1)).acos();
    let a2 = r2 * r2 * ((d * d + r2 * r2 - r1 * r1) / (2.0 * d * r2)).acos();
    let a3 = 0.5
        * js_max(
            (-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2),
            0.0,
        )
        .sqrt();
    a1 + a2 - a3
}

pub fn distance_for_overlap(r1: f64, r2: f64, area: f64) -> f64 {
    let mut low = (r1 - r2).abs();
    let mut high = r1 + r2;
    if area.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
        return high;
    }
    if area >= PI * js_min(r1, r2) * js_min(r1, r2) {
        return low;
    }
    for _ in 0..48 {
        let middle = (low + high) / 2.0;
        if lens_area(r1, r2, middle) > area {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low + high) / 2.0
}

fn margin(
    circles: &[Circle],
    inside: &[usize],
    outside: &[usize],
    x: f64,
    y: f64,
    floor: f64,
) -> f64 {
    let mut best = f64::INFINITY;
    for &i in inside {
        let c = &circles[i];
        let dx = x - c.x;
        let dy = y - c.y;
        let room = c.r - (dx * dx + dy * dy).sqrt();
        if room < best {
            if room < floor {
                return room;
            }
            best = room;
        }
    }
    for &i in outside {
        let c = &circles[i];
        let dx = x - c.x;
        let dy = y - c.y;
        let room = (dx * dx + dy * dy).sqrt() - c.r;
        if room < best {
            if room < floor {
                return room;
            }
            best = room;
        }
    }
    best
}

pub fn find_spot(circles: &[Circle], inside: &[usize], outside: &[usize]) -> Spot {
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut smallest = inside[0];
    for &i in inside {
        let c = &circles[i];
        cx += c.x / inside.len() as f64;
        cy += c.y / inside.len() as f64;
        if c.r < circles[smallest].r {
            smallest = i;
        }
    }
    let r = circles[smallest].r;
    let mut x = cx;
    let mut y = cy;
    let mut best = margin(circles, inside, outside, x, y, f64::NEG_INFINITY);
    let mut off = 0.0f64;
    let mut consider = |px: f64, py: f64, best: &mut f64, x: &mut f64, y: &mut f64| -> bool {
        let m = margin(circles, inside, outside, px, py, *best - r * 1e-9);
        if m.partial_cmp(&(*best - r * 1e-9)) != Some(std::cmp::Ordering::Greater) {
            return false;
        }
        let distance = js_hypot(px - cx, py - cy);
        if m > *best + r * 1e-9 || distance < off - r * 1e-9 {
            *best = m;
            off = distance;
            *x = px;
            *y = py;
            return true;
        }
        false
    };
    const GRID: i64 = 10;
    for gy in -GRID..=GRID {
        for gx in -GRID..=GRID {
            if gx * gx + gy * gy <= GRID * GRID {
                consider(
                    circles[smallest].x + (gx as f64 * r) / GRID as f64,
                    circles[smallest].y + (gy as f64 * r) / GRID as f64,
                    &mut best,
                    &mut x,
                    &mut y,
                );
            }
        }
    }
    let mut step = r / GRID as f64;
    let mut round = 0;
    while round < 40 && step > r * 1e-6 {
        let mut moved = false;
        for k in 0..8 {
            let angle = (k as f64 * TAU) / 8.0;
            if consider(
                x + angle.cos() * step,
                y + angle.sin() * step,
                &mut best,
                &mut x,
                &mut y,
            ) {
                moved = true;
            }
        }
        if !moved {
            step /= 2.0;
        }
        round += 1;
    }
    Spot { x, y, margin: best }
}

pub fn layout_circles(subsets: &[Subset]) -> Vec<Circle> {
    let mut sizes: Vec<(String, f64)> = Vec::new();
    for subset in subsets {
        for id in &subset.sets {
            if get_value(&sizes, id).is_none() {
                set_value(&mut sizes, id, 10.0);
            }
        }
    }
    for subset in subsets {
        if subset.sets.len() == 1 {
            set_value(&mut sizes, &subset.sets[0], subset.size);
        }
    }

    let mut index: HashMap<String, usize> = HashMap::new();
    let mut circles: Vec<Circle> = Vec::new();
    let mut largest = 0.0f64;
    for (_, size) in &sizes {
        if size.is_finite() && *size > largest {
            largest = *size;
        }
    }
    if largest == 0.0 {
        largest = 10.0;
    }
    let widest = (largest / PI).sqrt();
    for (id, size) in &sizes {
        index.insert(id.clone(), circles.len());
        let r = if size.is_finite() && *size > 0.0 {
            (size / PI).sqrt()
        } else {
            0.0
        };
        circles.push(Circle {
            id: id.clone(),
            x: 0.0,
            y: 0.0,
            r: js_max(r, widest * 0.3),
        });
    }
    let n = circles.len();
    if n < 2 {
        return circles;
    }

    let mut overlaps: Vec<(usize, f64)> = Vec::new();
    for subset in ensure_pairwise_subsets(subsets) {
        if subset.sets.len() != 2 {
            continue;
        }
        let a = index[&subset.sets[0]];
        let b = index[&subset.sets[1]];
        if a != b {
            let key = js_min(a as f64, b as f64) as usize * n + js_max(a as f64, b as f64) as usize;
            match overlaps.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = subset.size,
                None => overlaps.push((key, subset.size)),
            }
        }
    }
    let mut wanted: HashMap<usize, f64> = HashMap::new();
    for &(key, area) in &overlaps {
        wanted.insert(
            key,
            distance_for_overlap(circles[key / n].r, circles[key % n].r, area),
        );
    }
    let gap = widest * 0.15;
    let distance = |a: usize, b: usize| -> f64 {
        wanted
            .get(&(a * n + b))
            .copied()
            .unwrap_or(circles[a].r + circles[b].r + gap)
    };

    if n == 2 {
        circles[1].x = distance(0, 1);
        return circles;
    }

    if n == 3 {
        let mut ab = distance(0, 1);
        let mut ac = distance(0, 2);
        let mut bc = distance(1, 2);
        let triple = subsets
            .iter()
            .any(|subset| subset.sets.iter().collect::<HashSet<_>>().len() == 3);
        for _ in 0..40 {
            let x = if ab > 0.0 {
                (ab * ab + ac * ac - bc * bc) / (2.0 * ab)
            } else {
                0.0
            };
            circles[1].x = ab;
            circles[2].x = js_max(-ac, js_min(ac, x));
            circles[2].y = js_max(ac * ac - x * x, 0.0).sqrt();
            if !triple || find_spot(&circles, &[0, 1, 2], &[]).margin > widest * 0.12 {
                break;
            }
            ab *= 0.94;
            ac *= 0.94;
            bc *= 0.94;
        }
        return circles;
    }

    let mut total = 0.0;
    for c in &circles {
        total += 2.0 * c.r + gap;
    }
    let rounds = js_min(300.0, (3e6 / (n * n) as f64).floor()) as usize;
    if n <= 64 {
        let ring = total / TAU;
        for (i, c) in circles.iter_mut().enumerate() {
            let angle = (i as f64 * TAU) / n as f64 - PI / 2.0;
            c.x = angle.cos() * ring;
            c.y = angle.sin() * ring;
        }
    } else {
        let columns = (n as f64).sqrt().ceil() as usize;
        let cell = 2.0 * widest + gap;
        for (i, c) in circles.iter_mut().enumerate() {
            c.x = (i % columns) as f64 * cell;
            c.y = (i / columns) as f64 * cell;
        }
    }
    let mut dx = vec![0.0f64; n];
    let mut dy = vec![0.0f64; n];
    let mut pulls = vec![0i64; n];
    for _ in 0..rounds {
        dx.iter_mut().for_each(|v| *v = 0.0);
        dy.iter_mut().for_each(|v| *v = 0.0);
        pulls.iter_mut().for_each(|v| *v = 0);
        for a in 0..n {
            for b in a + 1..n {
                let overlap = wanted.get(&(a * n + b)).copied();
                let apart = circles[a].r + circles[b].r + gap;
                let mut ux = circles[b].x - circles[a].x;
                let mut uy = circles[b].y - circles[a].y;
                let mut length = js_hypot(ux, uy);
                if overlap.is_none() && length >= apart {
                    continue;
                }
                if length < 1e-9 {
                    ux = ((a + b) as f64).cos();
                    uy = ((a + b) as f64).sin();
                    length = 1.0;
                }
                let move_by = (length - overlap.unwrap_or(apart)) / 2.0 / length;
                dx[a] += ux * move_by;
                dy[a] += uy * move_by;
                dx[b] -= ux * move_by;
                dy[b] -= uy * move_by;
                pulls[a] += 1;
                pulls[b] += 1;
            }
        }
        for i in 0..n {
            if pulls[i] == 0 {
                continue;
            }
            circles[i].x += dx[i] / pulls[i] as f64;
            circles[i].y += dy[i] / pulls[i] as f64;
        }
    }
    circles
}

pub fn region_path(circles: &[Circle], format: &dyn Fn(f64) -> String) -> String {
    let whole = |c: &Circle| -> String {
        format!(
            "M{},{}A{},{} 0 1 1 {},{}A{},{} 0 1 1 {},{}Z",
            format(c.x - c.r),
            format(c.y),
            format(c.r),
            format(c.r),
            format(c.x + c.r),
            format(c.y),
            format(c.r),
            format(c.r),
            format(c.x - c.r),
            format(c.y)
        )
    };
    if circles.len() == 1 {
        return whole(&circles[0]);
    }
    let within = |x: f64, y: f64, skip: Option<usize>, other: Option<usize>| -> bool {
        for (k, c) in circles.iter().enumerate() {
            if Some(k) != skip
                && Some(k) != other
                && js_hypot(x - c.x, y - c.y) > c.r * (1.0 + 1e-9)
            {
                return false;
            }
        }
        true
    };
    struct Corner {
        x: f64,
        y: f64,
        a: usize,
        b: usize,
    }
    let mut corners: Vec<Corner> = Vec::new();
    for i in 0..circles.len() {
        for j in i + 1..circles.len() {
            let a = &circles[i];
            let b = &circles[j];
            let d = js_hypot(b.x - a.x, b.y - a.y);
            if d >= a.r + b.r || d <= (a.r - b.r).abs() || d == 0.0 {
                continue;
            }
            let along = (a.r * a.r - b.r * b.r + d * d) / (2.0 * d);
            let up = js_max(a.r * a.r - along * along, 0.0).sqrt();
            let mx = a.x + ((b.x - a.x) * along) / d;
            let my = a.y + ((b.y - a.y) * along) / d;
            for sign in [1.0, -1.0] {
                let x = mx + (sign * up * (b.y - a.y)) / d;
                let y = my - (sign * up * (b.x - a.x)) / d;
                if within(x, y, Some(i), Some(j)) {
                    corners.push(Corner { x, y, a: i, b: j });
                }
            }
        }
    }
    if corners.len() < 2 {
        let mut smallest = 0;
        for (k, c) in circles.iter().enumerate() {
            if c.r < circles[smallest].r {
                smallest = k;
            }
        }
        for (k, c) in circles.iter().enumerate() {
            if k != smallest
                && js_hypot(c.x - circles[smallest].x, c.y - circles[smallest].y)
                    + circles[smallest].r
                    > c.r * (1.0 + 1e-9)
            {
                return String::new();
            }
        }
        return whole(&circles[smallest]);
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    for p in &corners {
        cx += p.x / corners.len() as f64;
        cy += p.y / corners.len() as f64;
    }
    corners.sort_by(|p, q| {
        let diff = (p.y - cy).atan2(p.x - cx) - (q.y - cy).atan2(q.x - cx);
        diff.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut d = String::new();
    for i in 0..corners.len() {
        let from = &corners[i];
        let to = &corners[(i + 1) % corners.len()];
        if i == 0 {
            d = format!("M{},{}", format(from.x), format(from.y));
        }
        let mut arc: Option<usize> = None;
        let mut large = 0;
        let mut narrowest = f64::INFINITY;
        for c in [from.a, from.b] {
            if c != to.a && c != to.b {
                continue;
            }
            let circle = &circles[c];
            let start = (from.y - circle.y).atan2(from.x - circle.x);
            let mut sweep = (to.y - circle.y).atan2(to.x - circle.x) - start;
            if sweep <= 0.0 {
                sweep += TAU;
            }
            let mx = circle.x + circle.r * (start + sweep / 2.0).cos();
            let my = circle.y + circle.r * (start + sweep / 2.0).sin();
            let width = js_hypot(mx - (from.x + to.x) / 2.0, my - (from.y + to.y) / 2.0);
            if width < narrowest {
                narrowest = width;
                arc = Some(c);
                large = if sweep > PI { 1 } else { 0 };
            }
        }
        match arc {
            None => d.push_str(&format!("L{},{}", format(to.x), format(to.y))),
            Some(c) => d.push_str(&format!(
                "A{},{} 0 {large} 1 {},{}",
                format(circles[c].r),
                format(circles[c].r),
                format(to.x),
                format(to.y)
            )),
        }
    }
    d.push('Z');
    d
}
