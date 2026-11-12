use std::collections::HashMap;
use std::fmt::Write as _;

use crate::layout::layered::ease;
use crate::svg::builder::{esc_text, label_svg_with, num};
use crate::svg::ink::ink;
use crate::text::label::Label;
use crate::util::jsnum::{js_hypot, js_max, js_min};

pub const ARROW: f64 = 8.0;
const CORNER: f64 = 12.0;
const HANDLE: f64 = 0.62;
pub const LEAD: f64 = 6.0;
const CASE: f64 = 4.0;

#[derive(Clone, Debug, PartialEq)]
pub struct EdgePath {
    pub d: String,
    pub tokens: Option<Vec<f64>>,
    pub sx: f64,
    pub sy: f64,
    pub sdx: f64,
    pub sdy: f64,
    pub ex: f64,
    pub ey: f64,
    pub edx: f64,
    pub edy: f64,
}

fn js_sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

fn or_one(x: f64) -> f64 {
    if x == 0.0 || x.is_nan() { 1.0 } else { x }
}

#[derive(Default)]
pub struct SideReach {
    reach: HashMap<i64, f64>,
}

impl SideReach {
    pub fn add(&mut self, node: i64, side: i64, kind: &str) {
        let trim = marker_trim(kind);
        if side >= 0 && trim > 0.0 {
            let key = node * 4 + side;
            let current = self.reach.get(&key).copied().unwrap_or(0.0);
            if trim + LEAD > current {
                self.reach.insert(key, trim + LEAD);
            }
        }
    }

    pub fn of(&self, node: i64, side: i64) -> f64 {
        if side >= 0 {
            self.reach.get(&(node * 4 + side)).copied().unwrap_or(0.0)
        } else {
            0.0
        }
    }
}

pub fn route_path(
    route: &[f64],
    curve: Option<&str>,
    start_trim: f64,
    end_trim: f64,
    start_reach: Option<f64>,
    end_reach: Option<f64>,
) -> EdgePath {
    let start_reach = start_reach.unwrap_or(if start_trim > 0.0 {
        start_trim + LEAD
    } else {
        0.0
    });
    let end_reach = end_reach.unwrap_or(if end_trim > 0.0 { end_trim + LEAD } else { 0.0 });
    let linear = curve == Some("linear");
    let stepped = matches!(curve, Some("step") | Some("stepBefore") | Some("stepAfter"));
    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    let mut axes: Vec<f64> = Vec::new();
    let near =
        |ax: f64, ay: f64, bx: f64, by: f64| (ax - bx).abs() < 0.01 && (ay - by).abs() < 0.01;
    for i in (0..route.len()).step_by(3) {
        let x = route[i];
        let y = route[i + 1];
        let axis = route[i + 2];
        let last = xs.len() as isize - 1;
        if stepped && last >= 0 {
            let l = last as usize;
            if (x - xs[l]).abs() >= 0.01 && (y - ys[l]).abs() >= 0.01 {
                let px = xs[l];
                let py = ys[l];
                let my = match curve {
                    Some("stepBefore") => py,
                    Some("stepAfter") => y,
                    _ => (py + y) / 2.0,
                };
                let mx = match curve {
                    Some("stepBefore") => px,
                    Some("stepAfter") => x,
                    _ => (px + x) / 2.0,
                };
                let turns = if axis == 0.0 {
                    [px, my, x, my]
                } else {
                    [mx, py, mx, y]
                };
                for k in (0..4).step_by(2) {
                    if near(turns[k], turns[k + 1], px, py) || near(turns[k], turns[k + 1], x, y) {
                        continue;
                    }
                    xs.push(turns[k]);
                    ys.push(turns[k + 1]);
                    axes.push(axis);
                }
            }
        }
        xs.push(x);
        ys.push(y);
        axes.push(axis);
    }
    let mut last = xs.len() - 1;
    let aligned = |xs: &[f64], ys: &[f64], i: usize| {
        (xs[i] - xs[i - 1]).abs() < 0.01 || (ys[i] - ys[i - 1]).abs() < 0.01
    };
    let straight = |xs: &[f64], ys: &[f64], i: usize| linear || aligned(xs, ys, i);
    let unit = |xs: &[f64], ys: &[f64], i: usize, j: usize, axis: f64| -> (f64, f64) {
        if !straight(xs, ys, i.max(j)) {
            return if axis == 0.0 {
                (0.0, or_one(js_sign(ys[j] - ys[i])))
            } else {
                (or_one(js_sign(xs[j] - xs[i])), 0.0)
            };
        }
        let dx = xs[j] - xs[i];
        let dy = ys[j] - ys[i];
        let len = or_one(js_hypot(dx, dy));
        (dx / len, dy / len)
    };
    let (sdx, sdy) = unit(&xs, &ys, 0, 1, axes[1]);
    let (edx, edy) = unit(&xs, &ys, last - 1, last, axes[last]);
    let sx = xs[0];
    let sy = ys[0];
    let ex = xs[last];
    let ey = ys[last];
    xs[0] += sdx * start_trim;
    ys[0] += sdy * start_trim;
    xs[last] -= edx * end_trim;
    ys[last] -= edy * end_trim;
    let length =
        |xs: &[f64], ys: &[f64], i: usize| (xs[i] - xs[i - 1]).abs() + (ys[i] - ys[i - 1]).abs();
    let turns = |xs: &[f64], ys: &[f64], last: usize, i: isize| -> bool {
        if linear || i <= 0 {
            return false;
        }
        let i = i as usize;
        !linear
            && i > 0
            && i < last
            && aligned(xs, ys, i)
            && aligned(xs, ys, i + 1)
            && length(xs, ys, i) >= 0.01
            && length(xs, ys, i + 1) >= 0.01
            && ((xs[i] - xs[i - 1]).abs() < 0.01) != ((xs[i + 1] - xs[i]).abs() < 0.01)
    };
    let along = |xs: &[f64], ys: &[f64], i: usize, ux: f64, uy: f64| -> f64 {
        (xs[i] - xs[i - 1]).abs() * ux.abs() + (ys[i] - ys[i - 1]).abs() * uy.abs()
    };
    let lead = |reach: f64, trim: f64, run: f64| -> f64 {
        let room = js_min(js_max(0.0, reach - LEAD - trim), run / 4.0);
        room + js_min(reach - trim - room, (run - room) / 4.0)
    };
    let arrives = if end_reach > end_trim
        && (!straight(&xs, &ys, last) || turns(&xs, &ys, last, last as isize - 1))
    {
        lead(end_reach, end_trim, along(&xs, &ys, last, edx, edy))
    } else {
        0.0
    };
    let leaves = if start_reach > start_trim && (!straight(&xs, &ys, 1) || turns(&xs, &ys, last, 1))
    {
        lead(start_reach, start_trim, along(&xs, &ys, 1, sdx, sdy))
    } else {
        0.0
    };
    if arrives >= 1.0 {
        let (x, y, a) = (
            xs[last] - edx * arrives,
            ys[last] - edy * arrives,
            axes[last],
        );
        xs.insert(last, x);
        ys.insert(last, y);
        axes.insert(last, a);
        last += 1;
    }
    if leaves >= 1.0 {
        let (x, y, a) = (xs[0] + sdx * leaves, ys[0] + sdy * leaves, axes[1]);
        xs.insert(1, x);
        ys.insert(1, y);
        axes.insert(1, a);
        last += 1;
    }

    let mut tokens: Vec<f64> = Vec::new();
    let mut d = String::new();
    let mut tok = |v: f64, tokens: &mut Vec<f64>| -> String {
        tokens.push(written(v));
        num(v).to_string()
    };
    tokens.push(77.0);
    tokens.push(2.0);
    d.push('M');
    let first = tok(xs[0], &mut tokens);
    d.push_str(&first);
    d.push(',');
    let second = tok(ys[0], &mut tokens);
    d.push_str(&second);
    let mut at_x = xs[0];
    let mut at_y = ys[0];
    for i in 1..=last {
        let x = xs[i];
        let y = ys[i];
        if !straight(&xs, &ys, i) {
            let down = axes[i] == 0.0;
            let along_v = if down { y - at_y } else { x - at_x };
            let k = along_v.abs() * ease(if down { x - at_x } else { y - at_y }, along_v);
            let ux = if down { 0.0 } else { js_sign(along_v) * k };
            let uy = if down { js_sign(along_v) * k } else { 0.0 };
            tokens.push(67.0);
            tokens.push(6.0);
            let p1 = (at_x + ux, at_y + uy);
            let p2 = (x - ux, y - uy);
            let p3 = (x, y);
            let s = [
                tok(p1.0, &mut tokens),
                tok(p1.1, &mut tokens),
                tok(p2.0, &mut tokens),
                tok(p2.1, &mut tokens),
                tok(p3.0, &mut tokens),
                tok(p3.1, &mut tokens),
            ];
            let _ = write!(d, "C{},{} {},{} {},{}", s[0], s[1], s[2], s[3], s[4], s[5]);
            at_x = x;
            at_y = y;
            continue;
        }
        if !turns(&xs, &ys, last, i as isize) {
            if i < last
                && aligned(&xs, &ys, i)
                && aligned(&xs, &ys, i + 1)
                && (x - xs[i - 1]) * (xs[i + 1] - x) + (y - ys[i - 1]) * (ys[i + 1] - y) > 0.0
            {
                continue;
            }
            line_to(&mut d, &mut tokens, &mut at_x, &mut at_y, x, y, &mut tok);
            continue;
        }
        let mut before = if turns(&xs, &ys, last, i as isize - 1) {
            length(&xs, &ys, i) / 2.0
        } else {
            length(&xs, &ys, i)
        };
        let mut after = if turns(&xs, &ys, last, i as isize + 1) {
            length(&xs, &ys, i + 1) / 2.0
        } else {
            length(&xs, &ys, i + 1)
        };
        if stepped {
            let m = js_min(CORNER, js_min(before, after));
            before = m;
            after = m;
        }
        let (ux, uy) = unit(&xs, &ys, i - 1, i, axes[i]);
        let (vx, vy) = unit(&xs, &ys, i, i + 1, axes[i + 1]);
        line_to(
            &mut d,
            &mut tokens,
            &mut at_x,
            &mut at_y,
            x - ux * before,
            y - uy * before,
            &mut tok,
        );
        at_x = x + vx * after;
        at_y = y + vy * after;
        if stepped {
            tokens.push(81.0);
            tokens.push(4.0);
            let a = tok(x, &mut tokens);
            let b = tok(y, &mut tokens);
            let c = tok(at_x, &mut tokens);
            let e = tok(at_y, &mut tokens);
            let _ = write!(d, "Q{a},{b} {c},{e}");
        } else {
            tokens.push(67.0);
            tokens.push(6.0);
            let s = [
                tok(x - ux * before * (1.0 - HANDLE), &mut tokens),
                tok(y - uy * before * (1.0 - HANDLE), &mut tokens),
                tok(x + vx * after * (1.0 - HANDLE), &mut tokens),
                tok(y + vy * after * (1.0 - HANDLE), &mut tokens),
                tok(at_x, &mut tokens),
                tok(at_y, &mut tokens),
            ];
            let _ = write!(d, "C{},{} {},{} {},{}", s[0], s[1], s[2], s[3], s[4], s[5]);
        }
    }
    EdgePath {
        d,
        tokens: Some(tokens),
        sx,
        sy,
        sdx: -sdx,
        sdy: -sdy,
        ex,
        ey,
        edx,
        edy,
    }
}

fn line_to(
    d: &mut String,
    tokens: &mut Vec<f64>,
    at_x: &mut f64,
    at_y: &mut f64,
    x: f64,
    y: f64,
    tok: &mut impl FnMut(f64, &mut Vec<f64>) -> String,
) {
    if (x - *at_x).abs() < 0.01 && (y - *at_y).abs() < 0.01 {
        return;
    }
    tokens.push(76.0);
    tokens.push(2.0);
    let a = tok(x, tokens);
    let b = tok(y, tokens);
    let _ = write!(d, "L{a},{b}");
    *at_x = x;
    *at_y = y;
}

pub fn marker_trim(kind: &str) -> f64 {
    match kind {
        "arrow_point" => ARROW - 1.0,
        "arrow_circle" => ARROW,
        "arrow_cross" => 4.0,
        _ => 0.0,
    }
}

pub fn marker(kind: &str, x: f64, y: f64, dx: f64, dy: f64, color: &str, stroke: &str) -> String {
    let nx = -dy;
    let ny = dx;
    match kind {
        "arrow_point" => {
            let bx = x - dx * ARROW;
            let by = y - dy * ARROW;
            let half = ARROW * 0.42;
            format!(
                "<path class=\"pelers-marker\" d=\"M{},{}L{},{}L{},{}Z\" fill=\"{color}\" stroke=\"{stroke}\"/>",
                num(x),
                num(y),
                num(bx + nx * half),
                num(by + ny * half),
                num(bx - nx * half),
                num(by - ny * half)
            )
        }
        "arrow_circle" => format!(
            "<circle class=\"pelers-marker\" cx=\"{}\" cy=\"{}\" r=\"3.5\" fill=\"{color}\" stroke=\"{stroke}\"/>",
            num(x - dx * 4.0),
            num(y - dy * 4.0)
        ),
        "arrow_cross" => {
            let cx = x - dx * 4.0;
            let cy = y - dy * 4.0;
            let a = 3.5;
            let ux = (dx + nx) * a;
            let uy = (dy + ny) * a;
            let vx = (dx - nx) * a;
            let vy = (dy - ny) * a;
            format!(
                "<path class=\"pelers-marker\" d=\"M{},{}L{},{}M{},{}L{},{}\"/>",
                num(cx - ux),
                num(cy - uy),
                num(cx + ux),
                num(cy + uy),
                num(cx - vx),
                num(cy - vy),
                num(cx + vx),
                num(cy + vy)
            )
        }
        _ => String::new(),
    }
}

fn written(v: f64) -> f64 {
    let n = js_round_100(v);
    if n == 0.0 { 0.0 } else { n / 100.0 }
}

fn js_round_100(v: f64) -> f64 {
    crate::util::jsnum::js_round(v * 100.0)
}

fn tenth_power(places: usize) -> f64 {
    format!("1e{places}").parse::<f64>().unwrap_or(f64::NAN)
}

fn tokenize(d: &str) -> Vec<f64> {
    let bytes = d.as_bytes();
    let code_at = |i: usize| -> u32 { bytes.get(i).map_or(u32::MAX, |&b| b as u32) };
    let mut out: Vec<f64> = Vec::new();
    let mut at: isize = -1;
    let mut i = 0usize;
    while i < bytes.len() {
        let c = code_at(i);
        if (65..=90).contains(&c) && c != 69 {
            at = out.len() as isize + 1;
            out.push(c as f64);
            out.push(0.0);
            i += 1;
        } else if c == 45 || c == 46 || (48..=57).contains(&c) {
            let mut j = if c == 45 { i + 1 } else { i };
            let mut m = 0.0f64;
            let mut places: i64 = -1;
            let mut k = code_at(j);
            while (48..=57).contains(&k) || (k == 46 && places < 0) {
                if k == 46 {
                    places = 0;
                } else {
                    m = m * 10.0 + (k - 48) as f64;
                    if places >= 0 {
                        places += 1;
                    }
                }
                j += 1;
                k = code_at(j);
            }
            let mut v: f64;
            if (code_at(j) | 32) == 101 || m > 9007199254740991.0 || places > 22 {
                let mut k = code_at(j);
                while k == 46
                    || k == 101
                    || k == 69
                    || (48..=57).contains(&k)
                    || ((k == 45 || k == 43) && (code_at(j - 1) | 32) == 101)
                {
                    j += 1;
                    k = code_at(j);
                }
                let slice = std::str::from_utf8(&bytes[i..j]).unwrap_or("");
                v = slice.parse::<f64>().unwrap_or(f64::NAN);
            } else {
                v = if places > 0 {
                    m / tenth_power(places as usize)
                } else {
                    m
                };
                if c == 45 {
                    v = -v;
                }
            }
            if at >= 0 {
                out.push(v);
                let idx = at as usize;
                out[idx] += 1.0;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn sample(tokens: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::new();
    let mut x = 0.0f64;
    let mut y = 0.0f64;
    let mut i = 0usize;
    while i < tokens.len() {
        let cmd = tokens[i];
        let count = tokens[i + 1] as usize;
        let s = i + 2;
        let e = s + count;
        i = e;
        if cmd == 72.0 || cmd == 86.0 {
            for &token in &tokens[s..e] {
                if cmd == 72.0 {
                    x = token;
                } else {
                    y = token;
                }
                out.push(x);
                out.push(y);
            }
        } else if cmd == 67.0 || cmd == 81.0 {
            let cubic = cmd == 67.0;
            let step = if cubic { 6 } else { 4 };
            let mut k = s;
            while k + step <= e {
                let x1 = tokens[k];
                let y1 = tokens[k + 1];
                let x2 = tokens[k + 2];
                let y2 = tokens[k + 3];
                let x3 = if cubic { tokens[k + 4] } else { x2 };
                let y3 = if cubic { tokens[k + 5] } else { y2 };
                let mut t = 0.125f64;
                while t <= 1.0 {
                    let mut ax = x + (x1 - x) * t;
                    let mut ay = y + (y1 - y) * t;
                    let mut bx = x1 + (x2 - x1) * t;
                    let mut by = y1 + (y2 - y1) * t;
                    if cubic {
                        let cx = x2 + (x3 - x2) * t;
                        let cy = y2 + (y3 - y2) * t;
                        ax += (bx - ax) * t;
                        ay += (by - ay) * t;
                        bx += (cx - bx) * t;
                        by += (cy - by) * t;
                    }
                    out.push(ax + (bx - ax) * t);
                    out.push(ay + (by - ay) * t);
                    t += 0.125;
                }
                x = x3;
                y = y3;
                k += step;
            }
        } else if cmd == 65.0 {
            let mut k = s;
            while k + 7 <= e {
                arc(
                    &mut out,
                    x,
                    y,
                    tokens[k],
                    tokens[k + 1],
                    tokens[k + 3],
                    tokens[k + 4],
                    tokens[k + 5],
                    tokens[k + 6],
                );
                x = tokens[k + 5];
                y = tokens[k + 6];
                k += 7;
            }
        } else {
            let mut k = s;
            while k + 2 <= e {
                if cmd == 77.0 && k == s && !out.is_empty() {
                    out.push(f64::NAN);
                    out.push(f64::NAN);
                }
                x = tokens[k];
                y = tokens[k + 1];
                out.push(x);
                out.push(y);
                k += 2;
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn arc(
    out: &mut Vec<f64>,
    x1: f64,
    y1: f64,
    rx: f64,
    ry: f64,
    large: f64,
    sweep: f64,
    x2: f64,
    y2: f64,
) {
    let mut rx = rx;
    let mut ry = ry;
    let hx = (x1 - x2) / 2.0;
    let hy = (y1 - y2) / 2.0;
    if rx == 0.0 || ry == 0.0 || (hx == 0.0 && hy == 0.0) {
        out.push(x2);
        out.push(y2);
        return;
    }
    let scale = js_max(1.0, ((hx * hx) / (rx * rx) + (hy * hy) / (ry * ry)).sqrt());
    rx *= scale;
    ry *= scale;
    let sign = if large == sweep { -1.0 } else { 1.0 };
    let k = sign
        * js_max(
            0.0,
            (rx * rx * ry * ry - rx * rx * hy * hy - ry * ry * hx * hx)
                / (rx * rx * hy * hy + ry * ry * hx * hx),
        )
        .sqrt();
    let cx = (k * rx * hy) / ry + (x1 + x2) / 2.0;
    let cy = (-k * ry * hx) / rx + (y1 + y2) / 2.0;
    let from = ((y1 - cy) / ry).atan2((x1 - cx) / rx);
    let mut turn = ((y2 - cy) / ry).atan2((x2 - cx) / rx) - from;
    if sweep != 0.0 && turn < 0.0 {
        turn += 2.0 * std::f64::consts::PI;
    } else if sweep == 0.0 && turn > 0.0 {
        turn -= 2.0 * std::f64::consts::PI;
    }
    for i in 1..=8 {
        let a = from + (turn * i as f64) / 8.0;
        out.push(cx + rx * a.cos());
        out.push(cy + ry * a.sin());
    }
}

fn path_points(d: &str) -> Vec<f64> {
    sample(&tokenize(d))
}

fn clips(origin: (f64, f64), delta: (f64, f64), bounds: (f64, f64, f64, f64)) -> bool {
    let (x, y) = origin;
    let (dx, dy) = delta;
    let (l, t, r, b) = bounds;
    let mut lo = 0.0f64;
    let mut hi = 1.0f64;
    for side in 0..4 {
        let p = match side {
            0 => -dx,
            1 => dx,
            2 => -dy,
            _ => dy,
        };
        let q = match side {
            0 => x - l,
            1 => r - x,
            2 => y - t,
            _ => b - y,
        };
        if p == 0.0 {
            if q < 0.0 {
                hi = -1.0;
            }
        } else if p < 0.0 {
            lo = js_max(lo, q / p);
        } else {
            hi = js_min(hi, q / p);
        }
    }
    lo < hi
}

fn enters(points: &[f64], l: f64, t: f64, r: f64, b: f64) -> bool {
    let mut k = 0usize;
    while k + 2 < points.len() {
        let x = points[k];
        let y = points[k + 1];
        let x2 = points[k + 2];
        let y2 = points[k + 3];
        if !((x < l && x2 < l) || (x > r && x2 > r) || (y < t && y2 < t) || (y > b && y2 > b))
            && clips((x, y), (x2 - x, y2 - y), (l, t, r, b))
        {
            return true;
        }
        k += 2;
    }
    false
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    pub box_: [f64; 4],
    pub shape: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FillUnder {
    Off,
    Crosses,
    Holder(usize),
}

pub fn fill_under(d: &str, tips: &[f64], fills: &[Fill], tokens: Option<&[f64]>) -> FillUnder {
    if fills.is_empty() {
        return FillUnder::Off;
    }
    let points = match tokens {
        Some(t) => sample(t),
        None => path_points(d),
    };
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let total = points.len() + tips.len();
    let mut k = 0usize;
    while k < total {
        let x = if k < points.len() {
            points[k]
        } else {
            tips[k - points.len()]
        };
        let y = if k < points.len() {
            points[k + 1]
        } else {
            tips[k + 1 - points.len()]
        };
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
        k += 2;
    }
    let mut holder: Option<usize> = None;
    let mut least = f64::INFINITY;
    for (idx, fill) in fills.iter().enumerate() {
        let [l, t, r, b] = fill.box_;
        if max_x < l + 0.5 || min_x > r - 0.5 || max_y < t + 0.5 || min_y > b - 0.5 {
            continue;
        }
        let mut touched = enters(&points, l + 0.5, t + 0.5, r - 0.5, b - 0.5);
        let mut q = 0usize;
        while q < tips.len() && !touched {
            touched = tips[q] > l + 0.5
                && tips[q] < r - 0.5
                && tips[q + 1] > t + 0.5
                && tips[q + 1] < b - 0.5;
            q += 2;
        }
        if !touched {
            continue;
        }
        if min_x < l || max_x > r || min_y < t || max_y > b {
            return FillUnder::Crosses;
        }
        let area = (r - l) * (b - t);
        if area < least {
            least = area;
            holder = Some(idx);
        }
    }
    match holder {
        Some(idx) => FillUnder::Holder(idx),
        None => FillUnder::Off,
    }
}

fn has_stroke_declaration(line: &str) -> bool {
    line.match_indices("stroke:")
        .any(|(at, _)| at > 0 && matches!(line.as_bytes()[at - 1], b'"' | b';'))
}

pub fn edge_ink(fill: &Fill, line: &str) -> String {
    if has_stroke_declaration(line) {
        return String::new();
    }
    ink(&fill.shape, "").map_or(String::new(), |c| c.to_owned())
}

fn fill_color(shape: &str) -> Option<String> {
    for (at, _) in shape.match_indices("fill:") {
        if at == 0 {
            continue;
        }
        if !matches!(shape.as_bytes()[at - 1], b'"' | b';') {
            continue;
        }
        let rest = &shape[at + 5..];
        let end = rest.find([';', '"']).unwrap_or(rest.len());
        return Some(rest[..end].trim().to_owned());
    }
    None
}

pub fn hollow_on(markers: &str, fill: &Fill) -> String {
    match fill_color(&fill.shape) {
        Some(color) if !color.is_empty() => {
            markers.replace(" fill=\"var(--_bg)\"", &format!(" fill=\"{color}\""))
        }
        _ => markers.to_owned(),
    }
}

pub fn over_fills(
    d: &str,
    tips: &[f64],
    fills: &[Fill],
    line: &str,
    tokens: Option<&[f64]>,
) -> Option<String> {
    match fill_under(d, tips, fills, tokens) {
        FillUnder::Off => None,
        FillUnder::Crosses => Some(String::new()),
        FillUnder::Holder(idx) => Some(edge_ink(&fills[idx], line)),
    }
}

fn decl(line: &str, prop: &str) -> Option<String> {
    if !line.contains(prop) {
        return None;
    }
    let bytes = line.as_bytes();
    for (at, _) in line.match_indices(prop) {
        if at == 0 || !matches!(bytes[at - 1], b'"' | b';') {
            continue;
        }
        let after = at + prop.len();
        if bytes.get(after) != Some(&b':') {
            continue;
        }
        let start = after + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end] != b';' && bytes[end] != b'"' {
            end += 1;
        }
        if end == start || bytes.get(end) != Some(&b';') {
            continue;
        }
        return Some(line[start..end].trim().to_owned());
    }
    None
}

fn is_plain_width(width: &str) -> bool {
    let body = width.strip_suffix("px").unwrap_or(width);
    !body.is_empty() && body.bytes().all(|b| b.is_ascii_digit() || b == b'.')
}

fn parse_float(text: &str) -> f64 {
    let bytes = text.as_bytes();
    let mut end = 0usize;
    let mut seen_dot = false;
    let mut seen_digit = false;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_digit() {
            seen_digit = true;
            end += 1;
        } else if b == b'.' && !seen_dot {
            seen_dot = true;
            end += 1;
        } else {
            break;
        }
    }
    if !seen_digit {
        return f64::NAN;
    }
    text[..end].parse::<f64>().unwrap_or(f64::NAN)
}

pub fn casing(d: &str, markers: &str, line: &str, thick: bool) -> String {
    let stroke = decl(line, "stroke");
    if stroke.as_deref() == Some("none") || stroke.as_deref() == Some("transparent") {
        return String::new();
    }
    let width: Option<String> = if thick {
        Some("2.5".to_owned())
    } else {
        decl(line, "stroke-width")
    };
    let mut attrs = String::new();
    let mut style = String::new();
    if let Some(w) = width.as_deref() {
        if is_plain_width(w) {
            let value = parse_float(w);
            if value != 1.0 {
                attrs = format!(" stroke-width=\"{}\"", num(value + CASE));
            }
        } else {
            let _ = write!(style, "stroke-width:calc({w} + {}px);", num(CASE));
        }
    }
    for prop in ["opacity", "stroke-opacity"] {
        if let Some(value) = decl(line, prop) {
            let _ = write!(style, "{prop}:{value};");
        }
    }
    let body = format!("<path d=\"{d}\"/>") + &markers.replace(" class=\"pelers-marker\"", "");
    if attrs.is_empty() && style.is_empty() {
        body
    } else {
        let style_attr = if style.is_empty() {
            String::new()
        } else {
            format!(" style=\"{style}\"")
        };
        format!("<g{attrs}{style_attr}>{body}</g>")
    }
}

pub fn edge_casing(
    path: &EdgePath,
    start_type: &str,
    end_type: &str,
    line: &str,
    thick: bool,
) -> String {
    let markers = marker(
        start_type,
        path.sx,
        path.sy,
        path.sdx,
        path.sdy,
        "var(--_bg)",
        "var(--_bg)",
    ) + &marker(
        end_type,
        path.ex,
        path.ey,
        path.edx,
        path.edy,
        "var(--_bg)",
        "var(--_bg)",
    );
    casing(&path.d, &markers, line, thick)
}

pub fn casings_svg(casings: &str) -> String {
    if casings.is_empty() {
        String::new()
    } else {
        format!(
            "<g class=\"pelers-casings\" fill=\"none\" stroke=\"var(--_bg)\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" pointer-events=\"none\">{casings}</g>",
            1.0 + CASE
        )
    }
}

pub fn edge_label_svg(id: &str, label: &Label, x: f64, y: f64, attrs: &str, icons: bool) -> String {
    let w = label.width + 8.0;
    let h = label.height;
    format!(
        "<g class=\"pelers-edge-label\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"var(--_bg)\"/>{}</g>",
        esc_text(id),
        num(x - w / 2.0),
        num(y - h / 2.0),
        num(w),
        num(h),
        label_svg_with(label, x, y, attrs, icons)
    )
}

pub struct LoopNode {
    pub x: f64,
    pub y: f64,
}

pub struct LoopEdge {
    pub label_w: f64,
    pub label_h: f64,
    pub label_x: f64,
    pub label_y: f64,
}

pub fn loop_path(
    c: &LoopNode,
    (w, h): (f64, f64),
    k: f64,
    sideways: bool,
    base: f64,
    trim: f64,
    e: &mut LoopEdge,
) -> EdgePath {
    let half = (if sideways { w } else { h }) / 2.0;
    let spread = js_min(half - 4.0, 8.0 + k * 6.0);
    let reach = base + k * 8.0;
    let len = or_one(js_hypot(reach, spread));
    let at = |out: f64, along: f64| -> String {
        if sideways {
            format!("{},{}", num(c.x + along), num(c.y + h / 2.0 + out))
        } else {
            format!("{},{}", num(c.x + w / 2.0 + out), num(c.y + along))
        }
    };
    let tx = (reach / len) * trim;
    let ty = (spread / len) * trim;
    if sideways {
        e.label_x = c.x;
        e.label_y = c.y + h / 2.0 + reach + 4.0 + e.label_h / 2.0;
    } else {
        e.label_x = c.x + w / 2.0 + reach + 4.0 + e.label_w / 2.0;
        e.label_y = c.y;
    }
    EdgePath {
        d: format!(
            "M{}C{} {} {}",
            at(0.0, -spread),
            at(reach, -spread * 2.0),
            at(reach, spread * 2.0),
            at(tx, spread + ty)
        ),
        tokens: None,
        sx: if sideways {
            c.x - spread
        } else {
            c.x + w / 2.0
        },
        sy: if sideways {
            c.y + h / 2.0
        } else {
            c.y - spread
        },
        sdx: if sideways { 0.0 } else { -1.0 },
        sdy: if sideways { -1.0 } else { 0.0 },
        ex: if sideways {
            c.x + spread
        } else {
            c.x + w / 2.0
        },
        ey: if sideways {
            c.y + h / 2.0
        } else {
            c.y + spread
        },
        edx: if sideways {
            -spread / len
        } else {
            -reach / len
        },
        edy: if sideways {
            -reach / len
        } else {
            -spread / len
        },
    }
}
