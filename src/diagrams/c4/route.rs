use crate::svg::builder::num;
use crate::util::jsnum::js_hypot;

#[derive(Clone, Debug, PartialEq)]
pub struct Box {
    pub id: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub form: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub d: String,
    pub sx: f64,
    pub sy: f64,
    pub sdx: f64,
    pub sdy: f64,
    pub ex: f64,
    pub ey: f64,
    pub edx: f64,
    pub edy: f64,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub nx: f64,
    pub ny: f64,
    pub lean: f64,
    pub room: f64,
    pub points: Vec<f64>,
}

const LOOP: f64 = 28.0;
pub const LANE: f64 = 20.0;
const LANE_STEP: f64 = 7.0;
const CORNER: f64 = 8.0;
const LEAN: f64 = 30.0;
pub const NARROW: f64 = 88.0;
const WIDE: f64 = 160.0;
pub const LABEL_PAD: f64 = 8.0;
pub const HEAD: f64 = 26.0;

pub fn cyl_ry(w: f64) -> f64 {
    (w / 14.0).clamp(6.0, 12.0)
}

pub fn queue_rx(h: f64) -> f64 {
    (h / 4.0).min(12.0)
}

fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

#[allow(clippy::too_many_arguments)]
fn hit_rect(x0: f64, y0: f64, x1: f64, y1: f64, px: f64, py: f64, dx: f64, dy: f64) -> f64 {
    let mut enter: f64 = 0.0;
    let mut leave: f64 = 1.0;
    if dx == 0.0 {
        if px < x0 || px > x1 {
            return f64::INFINITY;
        }
    } else {
        let a = (x0 - px) / dx;
        let b = (x1 - px) / dx;
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
    }
    if dy == 0.0 {
        if py < y0 || py > y1 {
            return f64::INFINITY;
        }
    } else {
        let a = (y0 - py) / dy;
        let b = (y1 - py) / dy;
        enter = enter.max(a.min(b));
        leave = leave.min(a.max(b));
    }
    if enter <= leave { enter } else { f64::INFINITY }
}

#[allow(clippy::too_many_arguments)]
fn hit_ellipse(cx: f64, cy: f64, rx: f64, ry: f64, px: f64, py: f64, dx: f64, dy: f64) -> f64 {
    let ux = dx / rx;
    let uy = dy / ry;
    let vx = (px - cx) / rx;
    let vy = (py - cy) / ry;
    let a = ux * ux + uy * uy;
    let b = 2.0 * (ux * vx + uy * vy);
    let c = vx * vx + vy * vy - 1.0;
    let disc = b * b - 4.0 * a * c;
    if a == 0.0 || disc < 0.0 {
        return f64::INFINITY;
    }
    let t = (-b - disc.sqrt()) / (2.0 * a);
    if (0.0..=1.0).contains(&t) {
        t
    } else if c <= 0.0 {
        0.0
    } else {
        f64::INFINITY
    }
}

fn hit(b: &Box, px: f64, py: f64, qx: f64, qy: f64) -> f64 {
    let (x, y, w, h) = (b.x, b.y, b.w, b.h);
    let dx = qx - px;
    let dy = qy - py;
    match b.form {
        "person" => hit_rect(x, y + HEAD, x + w, y + h, px, py, dx, dy).min(hit_ellipse(
            x + w / 2.0,
            y + 14.0,
            13.0,
            13.0,
            px,
            py,
            dx,
            dy,
        )),
        "cyl" => {
            let ry = cyl_ry(w);
            hit_rect(x, y + ry, x + w, y + h - ry, px, py, dx, dy)
                .min(hit_ellipse(
                    x + w / 2.0,
                    y + ry,
                    w / 2.0,
                    ry,
                    px,
                    py,
                    dx,
                    dy,
                ))
                .min(hit_ellipse(
                    x + w / 2.0,
                    y + h - ry,
                    w / 2.0,
                    ry,
                    px,
                    py,
                    dx,
                    dy,
                ))
        }
        "h-cyl" => {
            let rx = queue_rx(h);
            hit_rect(x + rx, y, x + w - rx, y + h, px, py, dx, dy)
                .min(hit_ellipse(
                    x + rx,
                    y + h / 2.0,
                    rx,
                    h / 2.0,
                    px,
                    py,
                    dx,
                    dy,
                ))
                .min(hit_ellipse(
                    x + w - rx,
                    y + h / 2.0,
                    rx,
                    h / 2.0,
                    px,
                    py,
                    dx,
                    dy,
                ))
        }
        _ => hit_rect(x, y, x + w, y + h, px, py, dx, dy),
    }
}

pub fn crosses(points: &[f64], x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
    let mut i = 2;
    while i < points.len() {
        let px = points[i - 2];
        let py = points[i - 1];
        if hit_rect(x0, y0, x1, y1, px, py, points[i] - px, points[i + 1] - py) <= 1.0 {
            return true;
        }
        i += 2;
    }
    false
}

fn contains(outer: &Box, inner: &Box) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.w >= inner.x + inner.w
        && outer.y + outer.h >= inner.y + inner.h
}

fn connect(a: &Box, b: &Box, shift: f64) -> [f64; 4] {
    let ax = a.x + a.w / 2.0;
    let ay = a.y + a.h / 2.0;
    let bx = b.x + b.w / 2.0;
    let by = b.y + b.h / 2.0;
    let down = contains(a, b);
    if down || contains(b, a) {
        let (outer, inner) = if down { (a, b) } else { (b, a) };
        let ix = inner.x + inner.w / 2.0;
        let iy = inner.y + inner.h / 2.0;
        let left = inner.x - outer.x;
        let right = outer.x + outer.w - inner.x - inner.w;
        let top = inner.y - outer.y;
        let bottom = outer.y + outer.h - inner.y - inner.h;
        let least = left.min(right).min(top).min(bottom);
        let ox = if least == left {
            outer.x
        } else if least == right {
            outer.x + outer.w
        } else {
            ix
        };
        let oy = if least == left || least == right {
            iy
        } else if least == bottom {
            outer.y + outer.h
        } else {
            outer.y
        };
        let t = hit(inner, ox, oy, ix, iy).min(1.0);
        let mx = ox + (ix - ox) * t;
        let my = oy + (iy - oy) * t;
        return if down {
            [ox, oy, mx, my]
        } else {
            [mx, my, ox, oy]
        };
    }

    let (mut px, mut py, mut qx, mut qy) = (ax, ay, bx, by);
    let x0 = a.x.max(b.x);
    let x1 = (a.x + a.w).min(b.x + b.w);
    let y0 = a.y.max(b.y);
    let y1 = (a.y + a.h).min(b.y + b.h);
    if x0 < x1 {
        px = (x0 + x1) / 2.0;
        qx = px;
    } else if y0 < y1 {
        py = (y0 + y1) / 2.0;
        qy = py;
    }
    let len = {
        let l = js_hypot(qx - px, qy - py);
        if l == 0.0 || l.is_nan() { 1.0 } else { l }
    };
    let nx = (-(qy - py) / len) * shift;
    let ny = ((qx - px) / len) * shift;
    let mut ta = hit(a, qx + nx, qy + ny, px + nx, py + ny);
    let mut tb = hit(b, px + nx, py + ny, qx + nx, qy + ny);
    if ta <= 1.0 && tb <= 1.0 {
        px += nx;
        py += ny;
        qx += nx;
        qy += ny;
    } else {
        px = ax;
        py = ay;
        qx = bx;
        qy = by;
        ta = hit(a, qx, qy, px, py).min(1.0);
        tb = hit(b, px, py, qx, qy).min(1.0);
    }
    [
        qx + (px - qx) * ta,
        qy + (py - qy) * ta,
        px + (qx - px) * tb,
        py + (qy - py) * tb,
    ]
}

const NEAR: f64 = 6.0;
const FRAME_NEAR: f64 = 3.0;
const PORT_STEP: f64 = 12.0;

#[derive(Default)]
pub struct Lanes {
    upright: Vec<f64>,
    level: Vec<f64>,
    frames: [Vec<f64>; 2],
    taken: std::collections::HashMap<usize, Vec<f64>>,
    pub squeezed: usize,
}

impl Lanes {
    pub fn free(&self, upright: bool, at: f64, a0: f64, a1: f64) -> bool {
        let lo = a0.min(a1);
        let hi = a0.max(a1);
        let near = |list: &[f64], gap: f64| {
            list.as_chunks::<3>()
                .0
                .iter()
                .any(|s| (s[0] - at).abs() < gap && s[1] < hi && s[2] > lo)
        };
        let lanes = if upright { &self.upright } else { &self.level };
        let frames = &self.frames[usize::from(!upright)];
        !near(lanes, NEAR) && !near(frames, FRAME_NEAR)
    }

    pub fn crossings(&self, points: &[f64]) -> usize {
        let mut count = 0;
        let mut i = 2;
        while i < points.len() {
            let upright = points[i] == points[i - 2];
            let at = if upright { points[i] } else { points[i + 1] };
            let (p, q) = if upright {
                (points[i - 1], points[i + 1])
            } else {
                (points[i - 2], points[i])
            };
            let lo = p.min(q);
            let hi = p.max(q);
            let list = if upright { &self.level } else { &self.upright };
            for s in list.as_chunks::<3>().0 {
                if s[0] > lo && s[0] < hi && s[1] < at && s[2] > at {
                    count += 1;
                }
            }
            i += 2;
        }
        count
    }

    pub fn frame(&mut self, b: &Box) {
        self.frames[0].extend_from_slice(&[b.x, b.y, b.y + b.h, b.x + b.w, b.y, b.y + b.h]);
        self.frames[1].extend_from_slice(&[b.y, b.x, b.x + b.w, b.y + b.h, b.x, b.x + b.w]);
    }

    pub fn take(&mut self, points: &[f64]) {
        let mut i = 2;
        while i < points.len() {
            if points[i] == points[i - 2] {
                self.upright.extend_from_slice(&[
                    points[i],
                    points[i - 1].min(points[i + 1]),
                    points[i - 1].max(points[i + 1]),
                ]);
            } else if points[i + 1] == points[i - 1] {
                self.level.extend_from_slice(&[
                    points[i + 1],
                    points[i - 2].min(points[i]),
                    points[i - 2].max(points[i]),
                ]);
            }
            i += 2;
        }
    }

    pub fn ports(&self, b: &Box, side: usize, want: f64, lo: f64, hi: f64) -> Vec<f64> {
        let used = self.taken.get(&b.id).map_or(&[][..], Vec::as_slice);
        let mut out = Vec::new();
        for k in 0..9 {
            if out.len() >= 4 {
                break;
            }
            let step = if k & 1 == 1 { 1.0 } else { -1.0 };
            let at = want + step * ((k as f64) / 2.0).ceil() * PORT_STEP;
            if at < lo || at > hi {
                continue;
            }
            let mut clear = true;
            for u in used.as_chunks::<2>().0 {
                if u[0] == side as f64 && (u[1] - at).abs() < PORT_STEP {
                    clear = false;
                }
            }
            if clear {
                out.push(at);
            }
        }
        if out.is_empty() { vec![want] } else { out }
    }

    pub fn claim(&mut self, b: &Box, side: usize, at: f64) {
        self.taken
            .entry(b.id)
            .or_default()
            .extend_from_slice(&[side as f64, at]);
    }
}

fn path(points: &[f64], start_trim: f64, end_trim: f64) -> String {
    let n = points.len();
    let ux = sign(points[2] - points[0]);
    let uy = sign(points[3] - points[1]);
    let mut d = format!(
        "M{},{}",
        num(points[0] + ux * start_trim),
        num(points[1] + uy * start_trim)
    );
    let mut i = 2;
    while i < n - 2 {
        let x = points[i];
        let y = points[i + 1];
        let ix = sign(x - points[i - 2]);
        let iy = sign(y - points[i - 1]);
        let ox = sign(points[i + 2] - x);
        let oy = sign(points[i + 3] - y);
        let r = CORNER
            .min((x - points[i - 2] + y - points[i - 1]).abs() / 2.0)
            .min((points[i + 2] - x + points[i + 3] - y).abs() / 2.0);
        if ix != 0.0 {
            d.push_str(&format!("H{}", num(x - ix * r)));
        } else {
            d.push_str(&format!("V{}", num(y - iy * r)));
        }
        d.push_str(&format!(
            "Q{},{} {},{}",
            num(x),
            num(y),
            num(x + ox * r),
            num(y + oy * r)
        ));
        i += 2;
    }
    let lx = sign(points[n - 2] - points[n - 4]);
    let ly = sign(points[n - 1] - points[n - 3]);
    d.push_str(&if lx != 0.0 {
        format!("H{}", num(points[n - 2] - lx * end_trim))
    } else {
        format!("V{}", num(points[n - 1] - ly * end_trim))
    });
    d
}

fn clear_of(points: &[f64], others: &[Box], a: &Box, b: &Box) -> bool {
    others.iter().all(|other| {
        other.id == a.id
            || other.id == b.id
            || !crosses(
                points,
                other.x,
                other.y,
                other.x + other.w,
                other.y + other.h,
            )
    })
}

fn bent(
    points: Vec<f64>,
    along: usize,
    start_trim: f64,
    end_trim: f64,
    room: f64,
    nx: f64,
    lean: f64,
) -> Route {
    let n = points.len();
    Route {
        d: path(&points, start_trim, end_trim),
        sx: points[0],
        sy: points[1],
        sdx: -sign(points[2] - points[0]),
        sdy: -sign(points[3] - points[1]),
        ex: points[n - 2],
        ey: points[n - 1],
        edx: sign(points[n - 2] - points[n - 4]),
        edy: sign(points[n - 1] - points[n - 3]),
        x0: points[along],
        y0: points[along + 1],
        x1: points[along + 2],
        y1: points[along + 3],
        nx,
        ny: 0.0,
        lean,
        room,
        points,
    }
}

fn across(lo: f64, hi: f64) -> Vec<f64> {
    let mid = (lo + hi) / 2.0;
    let step = LANE_STEP.max((hi - lo) / 4.0);
    let mut out = Vec::new();
    for k in 0..9 {
        let sgn = if k & 1 == 1 { 1.0 } else { -1.0 };
        let at = mid + sgn * ((k as f64) / 2.0).ceil() * step;
        if at >= lo + NEAR && at <= hi - NEAR {
            out.push(at);
        }
    }
    out
}

#[derive(Clone)]
struct Best {
    points: Vec<f64>,
    cost: f64,
    upright: bool,
    sides: [usize; 2],
}

#[allow(clippy::too_many_arguments)]
fn consider(
    best: &mut Option<Best>,
    points: Vec<f64>,
    upright: bool,
    sides: [usize; 2],
    others: &[Box],
    a: &Box,
    b: &Box,
    lanes: &Lanes,
) -> bool {
    if !clear_of(&points, others, a, b) {
        return false;
    }
    let mut i = 2;
    while i < points.len() {
        let up = points[i] == points[i - 2];
        let free = lanes.free(
            up,
            if up { points[i] } else { points[i + 1] },
            if up { points[i - 1] } else { points[i - 2] },
            if up { points[i + 1] } else { points[i] },
        );
        if !free {
            return false;
        }
        i += 2;
    }
    let mut cost = lanes.crossings(&points) as f64 * 1e6;
    let mut j = 2;
    while j < points.len() {
        cost += (points[j] - points[j - 2]).abs() + (points[j + 1] - points[j - 1]).abs();
        j += 2;
    }
    if best.as_ref().is_none_or(|current| cost < current.cost) {
        *best = Some(Best {
            points,
            cost,
            upright,
            sides,
        });
        return true;
    }
    false
}

fn elbow(
    a: &Box,
    b: &Box,
    others: &[Box],
    lanes: &mut Lanes,
    start_trim: f64,
    end_trim: f64,
) -> Option<Route> {
    let ax = a.x + a.w / 2.0;
    let ay = a.y + a.h / 2.0;
    let bx = b.x + b.w / 2.0;
    let by = b.y + b.h / 2.0;
    let mut best: Option<Best> = None;
    if a.x + a.w <= b.x || b.x + b.w <= a.x {
        let right = bx > ax;
        let down = sign(by - ay);
        let sides = if right { [1, 0] } else { [0, 1] };
        let lanes_across = across(
            if right { a.x + a.w } else { b.x + b.w },
            if right { b.x } else { a.x },
        );
        'search: for y0 in lanes.ports(
            a,
            sides[0],
            ay + (down * a.h) / 4.0,
            a.y + 8.0,
            a.y + a.h - 8.0,
        ) {
            for y1 in lanes.ports(
                b,
                sides[1],
                by - (down * b.h) / 4.0,
                b.y + 8.0,
                b.y + b.h - 8.0,
            ) {
                for &lane in &lanes_across {
                    let x0 = lane + (ax - lane) * hit(a, lane, y0, ax, y0).min(1.0);
                    let x1 = lane + (bx - lane) * hit(b, lane, y1, bx, y1).min(1.0);
                    let points = vec![x0, y0, lane, y0, lane, y1, x1, y1];
                    if consider(&mut best, points, true, sides, others, a, b, lanes) {
                        break 'search;
                    }
                }
            }
        }
    }
    if a.y + a.h <= b.y || b.y + b.h <= a.y {
        let below = by > ay;
        let sideways = sign(bx - ax);
        let sides = if below { [3, 2] } else { [2, 3] };
        let lanes_across = across(
            if below { a.y + a.h } else { b.y + b.h },
            if below { b.y } else { a.y },
        );
        'search2: for x0 in lanes.ports(
            a,
            sides[0],
            ax + (sideways * a.w) / 4.0,
            a.x + 8.0,
            a.x + a.w - 8.0,
        ) {
            for x1 in lanes.ports(
                b,
                sides[1],
                bx - (sideways * b.w) / 4.0,
                b.x + 8.0,
                b.x + b.w - 8.0,
            ) {
                for &lane in &lanes_across {
                    let y0 = lane + (ay - lane) * hit(a, x0, lane, x0, ay).min(1.0);
                    let y1 = lane + (by - lane) * hit(b, x1, lane, x1, by).min(1.0);
                    let points = vec![x0, y0, x0, lane, x1, lane, x1, y1];
                    if consider(&mut best, points, false, sides, others, a, b, lanes) {
                        break 'search2;
                    }
                }
            }
        }
    }
    let Best {
        points,
        upright,
        sides,
        ..
    } = best?;
    lanes.take(&points);
    lanes.claim(a, sides[0], if upright { points[1] } else { points[0] });
    lanes.claim(b, sides[1], if upright { points[7] } else { points[6] });
    let room = if upright { NARROW } else { WIDE };
    Some(bent(points, 2, start_trim, end_trim, room, 0.0, 0.0))
}

#[allow(clippy::too_many_arguments)]
pub fn route(
    a: &Box,
    b: &Box,
    shift: f64,
    others: Option<&[Box]>,
    lanes: &mut Lanes,
    start_trim: f64,
    end_trim: f64,
) -> Route {
    if a.id == b.id {
        let spread = (a.h / 2.0 - 4.0).min(10.0);
        let len = js_hypot(LOOP, spread);
        let x = a.x + a.w;
        let cy = a.y + a.h / 2.0;
        let ux = LOOP / len;
        let uy = spread / len;
        return Route {
            d: format!(
                "M{},{}C{},{} {},{} {},{}",
                num(x + ux * start_trim),
                num(cy - spread - uy * start_trim),
                num(x + LOOP),
                num(cy - spread * 2.0),
                num(x + LOOP),
                num(cy + spread * 2.0),
                num(x + ux * end_trim),
                num(cy + spread + uy * end_trim)
            ),
            sx: x,
            sy: cy - spread,
            sdx: -ux,
            sdy: uy,
            ex: x,
            ey: cy + spread,
            edx: -ux,
            edy: -uy,
            x0: x + LOOP + 4.0,
            y0: cy,
            x1: x + LOOP + 4.0,
            y1: cy,
            nx: 1.0,
            ny: 0.0,
            lean: 0.0,
            room: 64.0,
            points: Vec::new(),
        };
    }

    let [sx, sy, ex, ey] = connect(a, b, shift);
    let upright = sx == ex;
    let mut blocked = false;
    let others_list = others.unwrap_or(&[]);
    if others.is_some() && !contains(a, b) && !contains(b, a) {
        for other in others_list {
            if other.id != a.id
                && other.id != b.id
                && hit_rect(
                    other.x,
                    other.y,
                    other.x + other.w,
                    other.y + other.h,
                    sx,
                    sy,
                    ex - sx,
                    ey - sy,
                ) <= 1.0
            {
                blocked = true;
            }
        }
    }
    if others.is_some() && blocked && !upright && sy != ey {
        if let Some(around) = elbow(a, b, others_list, lanes, start_trim, end_trim) {
            return around;
        }
        lanes.squeezed += 1;
    }
    if others.is_some() && blocked && (upright || sy == ey) {
        return detour(
            a,
            b,
            others_list,
            lanes,
            start_trim,
            end_trim,
            (sx, sy, ex, ey),
            upright,
        );
    }

    let len = {
        let l = js_hypot(ex - sx, ey - sy);
        if l == 0.0 || l.is_nan() { 1.0 } else { l }
    };
    let dx = (ex - sx) / len;
    let dy = (ey - sy) / len;
    if upright || sy == ey {
        lanes.take(&[sx, sy, ex, ey]);
    }
    Route {
        d: format!(
            "M{},{}L{},{}",
            num(sx + dx * start_trim),
            num(sy + dy * start_trim),
            num(ex - dx * end_trim),
            num(ey - dy * end_trim)
        ),
        sx,
        sy,
        sdx: -dx,
        sdy: -dy,
        ex,
        ey,
        edx: dx,
        edy: dy,
        x0: sx,
        y0: sy,
        x1: ex,
        y1: ey,
        nx: if shift != 0.0 { -dy } else { 0.0 },
        ny: if shift != 0.0 { dx } else { 0.0 },
        lean: 0.0,
        room: if dx.abs() > dy.abs() { NARROW } else { WIDE },
        points: vec![sx, sy, ex, ey],
    }
}

#[allow(clippy::too_many_arguments)]
fn detour(
    a: &Box,
    b: &Box,
    others: &[Box],
    lanes: &mut Lanes,
    start_trim: f64,
    end_trim: f64,
    (sx, sy, ex, ey): (f64, f64, f64, f64),
    upright: bool,
) -> Route {
    let blocking = |other: &Box| {
        other.id != a.id
            && other.id != b.id
            && hit_rect(
                other.x,
                other.y,
                other.x + other.w,
                other.y + other.h,
                sx,
                sy,
                ex - sx,
                ey - sy,
            ) <= 1.0
    };
    let lane_on = |forward: bool| -> f64 {
        let mut lane = if forward {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        let mut widen = |b: &Box| {
            let edge = if upright { b.x } else { b.y };
            let size = if upright { b.w } else { b.h };
            lane = if forward {
                lane.max(edge + size)
            } else {
                lane.min(edge)
            };
        };
        widen(a);
        widen(b);
        for other in others {
            if blocking(other) {
                widen(other);
            }
        }
        lane
    };
    let lo = if upright { a.y.min(b.y) } else { a.x.min(b.x) };
    let hi = if upright {
        (a.y + a.h).max(b.y + b.h)
    } else {
        (a.x + a.w).max(b.x + b.w)
    };
    let outside = |forward: bool, lane: f64| -> bool {
        others.iter().all(|other| {
            if other.id == a.id || other.id == b.id {
                return true;
            }
            let near = if upright {
                other.y < hi && other.y + other.h > lo
            } else {
                other.x < hi && other.x + other.w > lo
            };
            let start = if upright { other.x } else { other.y };
            let end = start + if upright { other.w } else { other.h };
            !(near && (if forward { end > lane } else { start < lane }))
        })
    };
    let ax = a.x + a.w / 2.0;
    let ay = a.y + a.h / 2.0;
    let bx = b.x + b.w / 2.0;
    let by = b.y + b.h / 2.0;
    let towards = if upright {
        sign(by - ay)
    } else {
        sign(bx - ax)
    };
    let along = |forward: bool| -> (Vec<f64>, usize, f64, f64, usize) {
        let lane = lane_on(forward);
        let sgn = if forward { 1.0 } else { -1.0 };
        let side = if upright {
            if forward { 1 } else { 0 }
        } else if forward {
            3
        } else {
            2
        };
        let p0 = if upright {
            lanes.ports(
                a,
                side,
                ay + (towards * a.h) / 4.0,
                a.y + 8.0,
                a.y + a.h - 8.0,
            )[0]
        } else {
            lanes.ports(
                a,
                side,
                ax + (towards * a.w) / 4.0,
                a.x + 8.0,
                a.x + a.w - 8.0,
            )[0]
        };
        let p1 = if upright {
            lanes.ports(
                b,
                side,
                by - (towards * b.h) / 4.0,
                b.y + 8.0,
                b.y + b.h - 8.0,
            )[0]
        } else {
            lanes.ports(
                b,
                side,
                bx - (towards * b.w) / 4.0,
                b.x + 8.0,
                b.x + b.w - 8.0,
            )[0]
        };
        let points_at = |at: f64| -> Vec<f64> {
            if upright {
                let x0 = at + (ax - at) * hit(a, at, p0, ax, p0).min(1.0);
                let x1 = at + (bx - at) * hit(b, at, p1, bx, p1).min(1.0);
                vec![x0, p0, at, p0, at, p1, x1, p1]
            } else {
                let y0 = at + (ay - at) * hit(a, p0, at, p0, ay).min(1.0);
                let y1 = at + (by - at) * hit(b, p1, at, p1, by).min(1.0);
                vec![p0, y0, p0, at, p1, at, p1, y1]
            }
        };
        let mut points = points_at(lane + sgn * LANE);
        for k in 0..=4 {
            let at = lane + sgn * (LANE + k as f64 * LANE_STEP);
            let tried = points_at(at);
            if !clear_of(&tried, others, a, b) {
                continue;
            }
            points = tried;
            if lanes.free(upright, at, p0, p1) {
                return (points, side, p0, p1, k);
            }
        }
        (points, side, p0, p1, 5)
    };
    let natural = if upright { ey > sy } else { ex < sx };
    let out = outside(natural, lane_on(natural));
    let back = outside(!natural, lane_on(!natural));
    let first = along(natural);
    let other = if out == back {
        Some(along(!natural))
    } else {
        None
    };
    let cost = |way: &(Vec<f64>, usize, f64, f64, usize)| {
        lanes.crossings(&way.0) as f64 * 8.0 + way.4 as f64
    };
    let forward = if (back && !out) || other.as_ref().is_some_and(|o| cost(o) < cost(&first)) {
        !natural
    } else {
        natural
    };
    let chosen = if forward == natural {
        first
    } else {
        other.unwrap_or_else(|| along(!natural))
    };
    let (points, side, p0, p1, step) = chosen;
    let sgn = if forward { 1.0 } else { -1.0 };
    if step > 4 {
        lanes.squeezed += 1;
    }
    lanes.take(&points);
    lanes.claim(a, side, p0);
    lanes.claim(b, side, p1);
    if upright {
        bent(points, 2, start_trim, end_trim, NARROW, sgn, LEAN)
    } else {
        bent(points, 2, start_trim, end_trim, WIDE, 0.0, 0.0)
    }
}
