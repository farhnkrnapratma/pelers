use std::collections::{HashMap, HashSet};

use super::arrows::{Axis, OVERHANG, arrow_axis, arrow_mask, arrow_size, draw_arrow};
use super::db::BlockDb;
use super::layout::{Cell, layout};
use super::types::{Block, BlockClassDef};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::title_room;
use crate::errors::PelersError;
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, label_svg_with, num};
use crate::svg::edges::{
    ARROW, EdgePath, Fill, casings_svg, edge_casing, edge_label_svg, marker, marker_trim,
    over_fills,
};
use crate::svg::ink::{ink_fill, paints};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::shapes::{draw_shape, shape_filled, shape_inset, shape_size};
use crate::svg::theme::{ResolvedStyle, class_names, resolve_style, rounded, sets_fill};
use crate::svg::title::title_label;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, ITALIC};
use crate::util::jsnum::{js_hypot, js_max, js_min, js_number_to_string, js_round};
use crate::util::ordered::OrderedMap;
use crate::util::value::Value;

const GAP: f64 = 12.0;
const WRAP: f64 = 200.0;
const CORNER: f64 = 8.0;
const SHAPE_ATTRS: &str = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";
const LINE_ATTRS: &str = " stroke=\"var(--_b)\"";
const OBSTACLE_BUDGET: f64 = 2_000_000.0;
const LABEL_GAP: f64 = 4.0;
const SPOTS: [f64; 5] = [0.5, 0.3, 0.7, 0.15, 0.85];
const TAU: f64 = 2.0 * std::f64::consts::PI;

fn shape_of(ty: &str) -> &'static str {
    match ty {
        "square" => "rect",
        "round" => "rounded",
        "circle" => "circle",
        "doublecircle" => "dbl-circ",
        "diamond" => "diam",
        "hexagon" => "hex",
        "stadium" => "stadium",
        "subroutine" => "fr-rect",
        "cylinder" => "cyl",
        "rect_left_inv_arrow" => "odd",
        "lean_right" => "lean-r",
        "lean_left" => "lean-l",
        "trapezoid" => "trap-b",
        "inv_trapezoid" => "trap-t",
        _ => "rect",
    }
}

fn is_fixed(shape: &str) -> bool {
    matches!(shape, "circle" | "dbl-circ" | "diam")
}

fn is_wide(shape: &str) -> bool {
    matches!(
        shape,
        "stadium" | "hex" | "odd" | "lean-r" | "lean-l" | "trap-b" | "trap-t"
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Leaf,
    Arrow,
    Composite,
    Space,
}

#[derive(Clone)]
struct View {
    cell: Cell,
    block: usize,
    kind: Kind,
    shape: String,
    label: Label,
    style: ResolvedStyle,
    mask: u32,
    dx: f64,
    dy: f64,
    bw: f64,
    bh: f64,
}

#[derive(Clone)]
struct Route {
    edge: usize,
    points: Vec<f64>,
    loop_index: usize,
    label: Label,
    lx: f64,
    ly: f64,
}

#[derive(Clone, Copy)]
struct Box2 {
    x: f64,
    y: f64,
    bw: f64,
    bh: f64,
    owner: usize,
}

fn is_round(v: &View) -> bool {
    v.kind == Kind::Leaf && (v.shape == "circle" || v.shape == "dbl-circ")
}

fn edge_y(v: &View, x: f64, sign: f64) -> f64 {
    let hw = v.bw / 2.0;
    let hh = v.bh / 2.0;
    let dx = (x - v.cell.x).abs();
    let reach = if is_round(v) {
        js_max(hw * hw - dx * dx, 0.0).sqrt()
    } else if v.kind == Kind::Leaf && v.shape == "diam" {
        hh * js_max(1.0 - dx / hw, 0.0)
    } else if v.kind == Kind::Leaf {
        hh - shape_inset(&v.shape, v.bw, v.bh, if sign > 0.0 { 2 } else { 0 }, 0.0)
    } else if v.kind == Kind::Arrow && arrow_axis(v.mask) == Axis::X {
        hh - OVERHANG
    } else {
        hh
    };
    v.cell.y + sign * reach
}

fn edge_x(v: &View, y: f64, sign: f64) -> f64 {
    let hw = v.bw / 2.0;
    let hh = v.bh / 2.0;
    let dy = (y - v.cell.y).abs();
    let reach = if is_round(v) {
        js_max(hh * hh - dy * dy, 0.0).sqrt()
    } else if v.kind == Kind::Leaf && v.shape == "diam" {
        hw * js_max(1.0 - dy / hh, 0.0)
    } else if v.kind == Kind::Leaf {
        hw - shape_inset(&v.shape, v.bw, v.bh, if sign > 0.0 { 1 } else { 3 }, 0.0)
    } else if v.kind == Kind::Arrow && arrow_axis(v.mask) == Axis::Y {
        hw - OVERHANG
    } else {
        hw
    };
    v.cell.x + sign * reach
}

fn border(v: &View, tx: f64, ty: f64, out: &mut Vec<f64>) {
    let dx = tx - v.cell.x;
    let dy = ty - v.cell.y;
    let hw = v.bw / 2.0;
    let hh = v.bh / 2.0;
    let mut k = if is_round(v) {
        let len = js_hypot(dx, dy);
        hw / if len == 0.0 || len.is_nan() { 1.0 } else { len }
    } else if v.kind == Kind::Leaf && v.shape == "diam" {
        let sum = dx.abs() / hw + dy.abs() / hh;
        1.0 / if sum == 0.0 || sum.is_nan() { 1.0 } else { sum }
    } else {
        let a = if dx == 0.0 {
            f64::INFINITY
        } else {
            hw / dx.abs()
        };
        let b = if dy == 0.0 {
            f64::INFINITY
        } else {
            hh / dy.abs()
        };
        js_min(a, b)
    };
    if k == f64::INFINITY {
        k = 0.0;
    }
    out.push(v.cell.x + dx * k);
    out.push(v.cell.y + dy * k);
}

fn crosses_box(x1: f64, y1: f64, x2: f64, y2: f64, v: &View) -> bool {
    let l = v.cell.x - v.bw / 2.0 + 1.0;
    let r = v.cell.x + v.bw / 2.0 - 1.0;
    let t = v.cell.y - v.bh / 2.0 + 1.0;
    let b = v.cell.y + v.bh / 2.0 - 1.0;
    if (x1 < l && x2 < l) || (x1 > r && x2 > r) || (y1 < t && y2 < t) || (y1 > b && y2 > b) {
        return false;
    }
    if x1 == x2 || y1 == y2 {
        return true;
    }
    let dx = x2 - x1;
    let dy = y2 - y1;
    let side = |x: f64, y: f64| -> f64 {
        let value = dx * (y - y1) - dy * (x - x1);
        if value > 0.0 {
            1.0
        } else if value < 0.0 {
            -1.0
        } else {
            value
        }
    };
    let first = side(l, t);
    first == 0.0 || side(r, t) != first || side(r, b) != first || side(l, b) != first
}

fn hits(points: &[f64], s: usize, t: usize, obstacles: &[usize], views: &[View]) -> usize {
    let mut count = 0;
    for &v in obstacles {
        if v == s || v == t {
            continue;
        }
        let mut i = 0;
        while i + 3 < points.len() {
            if crosses_box(
                points[i],
                points[i + 1],
                points[i + 2],
                points[i + 3],
                &views[v],
            ) {
                count += 1;
                break;
            }
            i += 2;
        }
    }
    count
}

fn route_edge(
    views: &[View],
    s: usize,
    t: usize,
    shift: f64,
    obstacles: Option<&[usize]>,
) -> Vec<f64> {
    let (sv, tv) = (&views[s], &views[t]);
    let sl = sv.cell.x - sv.bw / 2.0;
    let sr = sv.cell.x + sv.bw / 2.0;
    let st = sv.cell.y - sv.bh / 2.0;
    let sb = sv.cell.y + sv.bh / 2.0;
    let tl = tv.cell.x - tv.bw / 2.0;
    let tr = tv.cell.x + tv.bw / 2.0;
    let tt = tv.cell.y - tv.bh / 2.0;
    let tb = tv.cell.y + tv.bh / 2.0;
    let x1 = js_max(sl, tl);
    let x2 = js_min(sr, tr);
    let y1 = js_max(st, tt);
    let y2 = js_min(sb, tb);
    let over_x = x2 - x1 > 6.0;
    let over_y = y2 - y1 > 6.0;

    if over_x && over_y {
        let inner = if sv.bw * sv.bh <= tv.bw * tv.bh { s } else { t };
        let outer = if inner == s { t } else { s };
        let (iv, ov) = (&views[inner], &views[outer]);
        let up = iv.cell.y - iv.bh / 2.0 - (ov.cell.y - ov.bh / 2.0);
        let down = ov.cell.y + ov.bh / 2.0 - (iv.cell.y + iv.bh / 2.0);
        let left = iv.cell.x - iv.bw / 2.0 - (ov.cell.x - ov.bw / 2.0);
        let right = ov.cell.x + ov.bw / 2.0 - (iv.cell.x + iv.bw / 2.0);
        let least = js_min(js_min(up, down), js_min(left, right));
        let points = if least == down {
            vec![
                iv.cell.x,
                edge_y(iv, iv.cell.x, 1.0),
                iv.cell.x,
                ov.cell.y + ov.bh / 2.0,
            ]
        } else if least == up {
            vec![
                iv.cell.x,
                edge_y(iv, iv.cell.x, -1.0),
                iv.cell.x,
                ov.cell.y - ov.bh / 2.0,
            ]
        } else if least == right {
            vec![
                edge_x(iv, iv.cell.y, 1.0),
                iv.cell.y,
                ov.cell.x + ov.bw / 2.0,
                iv.cell.y,
            ]
        } else {
            vec![
                edge_x(iv, iv.cell.y, -1.0),
                iv.cell.y,
                ov.cell.x - ov.bw / 2.0,
                iv.cell.y,
            ]
        };
        return if inner == s {
            points
        } else {
            vec![points[2], points[3], points[0], points[1]]
        };
    }
    if over_x {
        let margin = js_min(6.0, (x2 - x1) / 2.0);
        let x = js_min(js_max((x1 + x2) / 2.0 + shift, x1 + margin), x2 - margin);
        let down = if tv.cell.y > sv.cell.y { 1.0 } else { -1.0 };
        return vec![x, edge_y(sv, x, down), x, edge_y(tv, x, -down)];
    }
    if over_y {
        let margin = js_min(6.0, (y2 - y1) / 2.0);
        let y = js_min(js_max((y1 + y2) / 2.0 + shift, y1 + margin), y2 - margin);
        let right = if tv.cell.x > sv.cell.x { 1.0 } else { -1.0 };
        return vec![edge_x(sv, y, right), y, edge_x(tv, y, -right), y];
    }

    let right = if tv.cell.x > sv.cell.x { 1.0 } else { -1.0 };
    let down = if tv.cell.y > sv.cell.y { 1.0 } else { -1.0 };
    let across = vec![
        edge_x(sv, sv.cell.y, right),
        sv.cell.y,
        tv.cell.x,
        sv.cell.y,
        tv.cell.x,
        edge_y(tv, tv.cell.x, -down),
    ];
    let along = vec![
        sv.cell.x,
        edge_y(sv, sv.cell.x, down),
        sv.cell.x,
        tv.cell.y,
        edge_x(tv, tv.cell.y, -right),
        tv.cell.y,
    ];
    let wide = (tv.cell.x - sv.cell.x).abs() >= (tv.cell.y - sv.cell.y).abs();
    let mut best = if wide { across.clone() } else { along.clone() };
    let Some(obstacles) = obstacles else {
        return best;
    };
    let mut least = hits(&best, s, t, obstacles, views);
    if least == 0 {
        return best;
    }
    let other = if wide { along } else { across };
    let count = hits(&other, s, t, obstacles, views);
    if count < least {
        best = other;
        least = count;
    }
    if least == 0 {
        return best;
    }
    let mut direct = Vec::new();
    border(sv, tv.cell.x, tv.cell.y, &mut direct);
    border(tv, sv.cell.x, sv.cell.y, &mut direct);
    if hits(&direct, s, t, obstacles, views) < least {
        direct
    } else {
        best
    }
}

fn trace(p: &[f64], start_trim: f64, end_trim: f64) -> EdgePath {
    let n = p.len();
    let sx = p[0];
    let sy = p[1];
    let ex = p[n - 2];
    let ey = p[n - 1];
    let mut ax = p[2] - sx;
    let mut ay = p[3] - sy;
    let first = js_hypot(ax, ay);
    let first_div = if first == 0.0 || first.is_nan() {
        1.0
    } else {
        first
    };
    ax /= first_div;
    ay /= first_div;
    let mut bx = ex - p[n - 4];
    let mut by = ey - p[n - 3];
    let last = js_hypot(bx, by);
    let last_div = if last == 0.0 || last.is_nan() {
        1.0
    } else {
        last
    };
    bx /= last_div;
    by /= last_div;
    let (mut start_trim, mut end_trim) = (start_trim, end_trim);
    if n == 4 && start_trim + end_trim > first {
        let scale = first / (start_trim + end_trim);
        start_trim *= scale;
        end_trim *= scale;
    }
    start_trim = js_min(start_trim, first);
    end_trim = js_min(end_trim, last);
    let mut d = format!(
        "M{},{}",
        num(sx + ax * start_trim),
        num(sy + ay * start_trim)
    );
    if n == 6 {
        let r = js_max(
            0.0,
            js_min(js_min(CORNER, first - start_trim), last - end_trim),
        );
        d.push_str(&format!(
            "L{},{}Q{},{} {},{}",
            num(p[2] - ax * r),
            num(p[3] - ay * r),
            num(p[2]),
            num(p[3]),
            num(p[2] + bx * r),
            num(p[3] + by * r)
        ));
    }
    d.push_str(&format!(
        "L{},{}",
        num(ex - bx * end_trim),
        num(ey - by * end_trim)
    ));
    EdgePath {
        d,
        tokens: None,
        sx,
        sy,
        sdx: -ax,
        sdy: -ay,
        ex,
        ey,
        edx: bx,
        edy: by,
    }
}

fn loop_arc(v: &View, k: f64) -> (f64, f64, f64, f64, f64) {
    let hw = v.bw / 2.0;
    let hh = v.bh / 2.0;
    let r = js_max(1.0, js_min(js_min(10.0 + 5.0 * (k - 1.0), hw), hh));
    if is_round(v) {
        let px = v.cell.x + hw * std::f64::consts::FRAC_1_SQRT_2;
        let py = v.cell.y - hw * std::f64::consts::FRAC_1_SQRT_2;
        let half = 2.0 * js_min(1.0, r / (2.0 * hw)).asin();
        let angle = |at: f64| -> f64 {
            (v.cell.y - hw * at.sin() - py).atan2(v.cell.x + hw * at.cos() - px)
        };
        return (
            px,
            py,
            r,
            angle(std::f64::consts::FRAC_PI_4 + half),
            angle(std::f64::consts::FRAC_PI_4 - half),
        );
    }
    if v.kind == Kind::Leaf && v.shape == "diam" {
        let slope = hh.atan2(hw);
        return (
            v.cell.x + hw / 2.0,
            v.cell.y - hh / 2.0,
            r,
            slope + std::f64::consts::PI,
            slope,
        );
    }
    (
        v.cell.x + hw,
        v.cell.y - hh,
        r,
        std::f64::consts::PI,
        std::f64::consts::FRAC_PI_2,
    )
}

fn loop_path(v: &View, k: f64, start_trim: f64, end_trim: f64) -> EdgePath {
    let (px, py, r, a1, a2) = loop_arc(v, k);
    let from = a1 + start_trim / r;
    let to = a2 - end_trim / r;
    let mut sweep = (to - from) % TAU;
    if sweep <= 0.0 {
        sweep += TAU;
    }
    let x1 = px + r * from.cos();
    let y1 = py + r * from.sin();
    let x2 = px + r * to.cos();
    let y2 = py + r * to.sin();
    let sx = px + r * a1.cos();
    let sy = py + r * a1.sin();
    let ex = px + r * a2.cos();
    let ey = py + r * a2.sin();
    let sl = js_hypot(sx - x1, sy - y1);
    let el = js_hypot(ex - x2, ey - y2);
    EdgePath {
        d: format!(
            "M{},{}A{},{} 0 {} 1 {},{}",
            num(x1),
            num(y1),
            num(r),
            num(r),
            if sweep > std::f64::consts::PI { 1 } else { 0 },
            num(x2),
            num(y2)
        ),
        tokens: None,
        sx,
        sy,
        sdx: if sl > 0.0 { (sx - x1) / sl } else { a1.sin() },
        sdy: if sl > 0.0 { (sy - y1) / sl } else { -a1.cos() },
        ex,
        ey,
        edx: if el > 0.0 { (ex - x2) / el } else { -a2.sin() },
        edy: if el > 0.0 { (ey - y2) / el } else { a2.cos() },
    }
}

fn covers(x: f64, y: f64, w: f64, h: f64, s: usize, t: usize, obstacles: Option<&[Box2]>) -> bool {
    let Some(obstacles) = obstacles else {
        return false;
    };
    obstacles.iter().any(|v| {
        v.owner != s
            && v.owner != t
            && (x - v.x).abs() < (w + v.bw) / 2.0
            && (y - v.y).abs() < (h + v.bh) / 2.0
    })
}

fn label_spot(
    p: &[f64],
    at: usize,
    w: f64,
    h: f64,
    s: usize,
    t: usize,
    obstacles: Option<&[Box2]>,
) -> f64 {
    if obstacles.is_none() {
        return 0.5;
    }
    for spot in SPOTS {
        let x = p[at] + (p[at + 2] - p[at]) * spot;
        let y = p[at + 1] + (p[at + 3] - p[at + 1]) * spot;
        if !covers(x, y, w, h, s, t, obstacles) {
            return spot;
        }
    }
    0.5
}

fn stack_labels(
    stack: &[usize],
    routes: &mut [Route],
    legs: &[Vec<f64>],
    s: usize,
    t: usize,
    blocked: Option<&[Box2]>,
) -> f64 {
    let first_leg = &legs[stack[0]];
    let vertical = first_leg[0] == first_leg[2];
    let mut lo = f64::NEG_INFINITY;
    let mut hi = f64::INFINITY;
    let mut need = -LABEL_GAP;
    let mut crowded = false;
    for a in 0..stack.len() {
        let leg = &legs[stack[a]];
        let parallel = if vertical {
            leg[0] == leg[2]
        } else {
            leg[1] == leg[3]
        };
        if !parallel {
            return 0.0;
        }
        lo = js_max(
            lo,
            if vertical {
                js_min(leg[1], leg[3])
            } else {
                js_min(leg[0], leg[2])
            },
        );
        hi = js_min(
            hi,
            if vertical {
                js_max(leg[1], leg[3])
            } else {
                js_max(leg[0], leg[2])
            },
        );
        let p = &routes[stack[a]];
        need += (if vertical {
            p.label.height
        } else {
            p.label.width + 8.0
        }) + LABEL_GAP;
        for b in 0..a {
            let q = &routes[stack[b]];
            if (p.lx - q.lx).abs() < (p.label.width + q.label.width) / 2.0 + 8.0
                && (p.ly - q.ly).abs() < (p.label.height + q.label.height) / 2.0
            {
                crowded = true;
            }
        }
    }
    if !crowded {
        return 0.0;
    }
    if need > hi - lo - 2.0 * ARROW {
        return need - (hi - lo - 2.0 * ARROW);
    }
    let mut along: Vec<f64> = Vec::new();
    let mut next = (lo + hi - need) / 2.0;
    for &i in stack {
        let label = routes[i].label.clone();
        let (lx, ly) = (routes[i].lx, routes[i].ly);
        let size = if vertical {
            label.height
        } else {
            label.width + 8.0
        };
        let at = next + size / 2.0;
        let (cx, cy) = if vertical { (lx, at) } else { (at, ly) };
        if covers(cx, cy, label.width + 8.0, label.height, s, t, blocked) {
            return 0.0;
        }
        along.push(at);
        next += size + LABEL_GAP;
    }
    for (k, &i) in stack.iter().enumerate() {
        if vertical {
            routes[i].ly = along[k];
        } else {
            routes[i].lx = along[k];
        }
    }
    0.0
}

fn widen(
    views: &mut [View],
    parents: &[usize],
    s: usize,
    t: usize,
    vertical: bool,
    short: f64,
    base: &mut HashMap<(usize, bool), Vec<f64>>,
) -> bool {
    let mut chain: HashSet<usize> = HashSet::new();
    let mut at_view = Some(s);
    while let Some(at) = at_view {
        chain.insert(at);
        at_view = if at == 0 { None } else { Some(parents[at]) };
    }
    let mut tb: Option<usize> = None;
    let mut boxed: Option<usize> = Some(t);
    while let Some(b) = boxed {
        if chain.contains(&b) {
            break;
        }
        tb = Some(b);
        boxed = if b == 0 { None } else { Some(parents[b]) };
    }
    let (Some(boxed), Some(tb)) = (boxed, tb) else {
        return false;
    };
    if boxed == s {
        return false;
    }
    let mut sb = s;
    while parents[sb] != boxed {
        sb = parents[sb];
    }
    let at: usize = if vertical {
        if views[sb].cell.row == views[tb].cell.row {
            return false;
        }
        (js_max(views[sb].cell.row, views[tb].cell.row) - 1.0) as usize
    } else {
        let (l, r) = if views[sb].cell.col < views[tb].cell.col {
            (sb, tb)
        } else {
            (tb, sb)
        };
        if views[r].cell.col < views[l].cell.col + views[l].cell.span {
            return false;
        }
        (views[r].cell.col - 1.0) as usize
    };
    let key = (boxed, vertical);
    let gaps = if vertical {
        &mut views[boxed].cell.row_gaps
    } else {
        &mut views[boxed].cell.col_gaps
    };
    let original = base.entry(key).or_insert_with(|| gaps.clone()).clone();
    if gaps.len() <= at {
        gaps.resize(at + 1, 0.0);
    }
    let kept = original.get(at).copied().unwrap_or(0.0);
    gaps[at] = js_max(gaps[at], kept + short);
    true
}

fn place(
    views: &mut [View],
    order: &[usize],
    widths: &[f64],
    gap: f64,
    obstacles: &mut Vec<usize>,
    titles: &mut Vec<Box2>,
) -> Result<(f64, f64), PelersError> {
    for &v in order {
        views[v].cell.nw = widths[v];
    }
    let mut cells: Vec<Cell> = views.iter().map(|v| v.cell.clone()).collect();
    let (width, height) = layout(&mut cells, 0, gap)?;
    for (v, cell) in views.iter_mut().zip(cells) {
        v.cell = cell;
    }
    obstacles.clear();
    titles.clear();
    for &index in order {
        let v = &mut views[index];
        if v.kind == Kind::Leaf && is_fixed(&v.shape) {
            v.bw = v.cell.nw;
            v.bh = v.cell.nh;
        } else if v.kind == Kind::Arrow {
            v.bw = if v.cell.span > 1.0 {
                v.cell.w
            } else {
                v.cell.nw
            };
            v.bh = v.cell.nh;
        } else {
            v.bw = v.cell.w;
            v.bh = if v.kind == Kind::Leaf && is_wide(&v.shape) {
                v.cell.nh
            } else {
                v.cell.h
            };
        }
        if v.kind == Kind::Leaf && v.shape == "cyl" {
            v.dy = js_min(12.0, js_max(6.0, v.bw / 14.0)) / 2.0;
        }
        if v.kind == Kind::Leaf || v.kind == Kind::Arrow {
            obstacles.push(index);
        } else if v.cell.head > 0.0 {
            titles.push(Box2 {
                x: v.cell.x,
                y: v.cell.y - v.cell.h / 2.0 + (v.cell.head + gap) / 2.0 + 2.0,
                bw: v.label.width,
                bh: v.label.height,
                owner: usize::MAX,
            });
        }
    }
    Ok((width, height))
}

fn boxes_of(views: &[View], list: &[usize], titles: &[Box2], with_titles: bool) -> Vec<Box2> {
    let mut boxes: Vec<Box2> = list
        .iter()
        .map(|&v| Box2 {
            x: views[v].cell.x,
            y: views[v].cell.y,
            bw: views[v].bw,
            bh: views[v].bh,
            owner: v,
        })
        .collect();
    if with_titles {
        boxes.extend(titles.iter().copied());
    }
    boxes
}

pub fn render_block(
    db: &BlockDb,
    config: &Config,
    options: &RenderOptions,
) -> Result<Drawn, PelersError> {
    let loose = draw(db, config, options, false)?;
    let Some(available) = options.max_width.filter(|a| *a > 0.0) else {
        return Ok(loose);
    };
    if loose.width <= available {
        return Ok(loose);
    }
    let narrow = draw(db, config, options, true)?;
    Ok(if narrow.width < loose.width {
        narrow
    } else {
        loose
    })
}

fn style_of(block: &Block, classes: &OrderedMap<BlockClassDef>) -> ResolvedStyle {
    let mut declarations: Vec<String> = Vec::new();
    let default_class = vec!["default".to_owned()];
    let names: &[String] = match &block.classes {
        Some(list) if !list.is_empty() => list,
        _ => &default_class,
    };
    for name in names {
        if let Some(def) = classes.get(name) {
            for style in &def.styles {
                declarations.extend(style.split(';').map(str::to_owned));
            }
        }
    }
    if let Some(styles) = &block.styles {
        for style in styles {
            declarations.extend(style.split(';').map(str::to_owned));
        }
    }
    resolve_style(&declarations)
}

fn title_room_for_block(tight: bool) -> f64 {
    if tight { 80.0 } else { 4000.0 }
}

fn new_view(db: &BlockDb, block_index: usize, tight: bool, size: f64, gap: f64, wrap: f64) -> View {
    let block = &db.arena[block_index];
    let ty = block.kind.clone().unwrap_or_default();
    let style = if ty == "space" {
        resolve_style(&[])
    } else {
        style_of(block, &db.classes)
    };
    let text_style =
        (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
    let font_size = style.font_size.unwrap_or(size);
    let mut kind = Kind::Leaf;
    let mut shape = String::new();
    let mut mask = 0u32;
    let mut dx = 0.0;
    let mut dy = 0.0;
    let mut nw = 0.0;
    let mut nh = 0.0;
    let mut head = 0.0;
    let label: Label;
    if ty == "space" {
        kind = Kind::Space;
        label = layout_label(None, false, font_size, wrap, 0);
    } else if ty == "composite" {
        kind = Kind::Composite;
        label = layout_label(
            block.label.as_deref(),
            false,
            font_size,
            title_room_for_block(tight),
            text_style,
        );
        head = if label.height > 0.0 {
            label.height + 4.0
        } else {
            0.0
        };
        nw = label.width + 2.0 * gap;
        nh = head + 2.0 * gap;
    } else if ty == "block_arrow" {
        kind = Kind::Arrow;
        label = layout_label(block.label.as_deref(), false, font_size, wrap, text_style);
        let blank = label.lines.iter().all(|line| {
            line.iter()
                .all(|part| part.icon.is_none() && part.text.trim().is_empty())
        });
        mask = arrow_mask(block.directions.as_deref());
        let s = if blank {
            arrow_size(mask, 0.0, 0.0)
        } else {
            arrow_size(mask, label.width, label.height)
        };
        nw = s.w;
        nh = s.h;
        dx = s.dx;
        dy = s.dy;
    } else {
        shape = shape_of(&ty).to_owned();
        label = layout_label(block.label.as_deref(), false, font_size, wrap, text_style);
        let s = shape_size(&shape, label.width, label.height);
        nw = s.w;
        nh = if shape == "cyl" {
            js_max(s.h, label.height + 40.0)
        } else {
            s.h
        };
        dy = s.dy;
    }
    let cell = Cell {
        columns: if kind == Kind::Composite {
            block.columns.unwrap_or(-1.0)
        } else {
            -1.0
        },
        span: if kind == Kind::Space {
            1.0
        } else {
            block.width_in_columns.unwrap_or(1.0)
        },
        space: kind == Kind::Space,
        head,
        nw,
        nh,
        ..Cell::new(0.0, 0.0, 1.0)
    };
    View {
        cell,
        block: block_index,
        kind,
        shape,
        label,
        style,
        mask,
        dx,
        dy,
        bw: 0.0,
        bh: 0.0,
    }
}

struct Plan {
    drawn: Vec<usize>,
    ends: Vec<(usize, usize)>,
    pairs: HashMap<usize, usize>,
    labelled: HashSet<usize>,
    check: Option<Vec<usize>>,
    order_len: usize,
    edge_size: f64,
}

impl Plan {
    fn key_of(&self, s: usize, t: usize) -> usize {
        s.min(t) * self.order_len + s.max(t)
    }
}

fn route_all(
    db: &BlockDb,
    plan: &Plan,
    views: &[View],
    legs: &mut [Vec<f64>],
) -> (Vec<Route>, Vec<(usize, Vec<usize>)>) {
    let mut seen: HashMap<usize, usize> = HashMap::new();
    let mut routes: Vec<Route> = Vec::new();
    let mut stacks: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..plan.drawn.len() {
        let (s, t) = plan.ends[i];
        let key = plan.key_of(s, t);
        let total = plan.pairs.get(&key).copied().unwrap_or(1);
        let k = seen.get(&key).copied().unwrap_or(0);
        seen.insert(key, k + 1);
        let edge = &db.arena[plan.drawn[i]];
        let label = layout_label(edge.label.as_deref(), false, plan.edge_size, WRAP, 0);
        let (points, lx, ly, loop_index) = if s == t {
            let (px, py, r, _, _) = loop_arc(&views[s], (k + 1) as f64);
            (
                Vec::new(),
                px + r + 6.0 + label.width / 2.0,
                py - r / 2.0,
                k + 1,
            )
        } else {
            let step = if plan.labelled.contains(&key) {
                js_round(plan.edge_size * 1.5) + 2.0
            } else {
                14.0
            };
            let shift = (k as f64 - (total as f64 - 1.0) / 2.0) * step;
            let pts = route_edge(views, s, t, shift, plan.check.as_deref());
            let n = pts.len();
            let first_leg = (pts[2] - pts[0]).abs() + (pts[3] - pts[1]).abs();
            let last_leg = (pts[n - 2] - pts[n - 4]).abs() + (pts[n - 1] - pts[n - 3]).abs();
            let at = if n == 6 && last_leg > first_leg { 2 } else { 0 };
            let spot = if label.width > 0.0 {
                let boxes = plan
                    .check
                    .as_ref()
                    .map(|list| boxes_of(views, list, &[], false));
                label_spot(
                    &pts,
                    at,
                    label.width + 8.0,
                    label.height,
                    s,
                    t,
                    boxes.as_deref(),
                )
            } else {
                0.5
            };
            let lx = pts[at] + (pts[at + 2] - pts[at]) * spot;
            let ly = pts[at + 1] + (pts[at + 3] - pts[at + 1]) * spot;
            if label.width > 0.0 && total > 1 {
                legs[i] = pts[at..at + 4].to_vec();
                match stacks.iter_mut().find(|(stack_key, _)| *stack_key == key) {
                    Some((_, stack)) => stack.push(i),
                    None => stacks.push((key, vec![i])),
                }
            }
            (pts, lx, ly, 0)
        };
        routes.push(Route {
            edge: plan.drawn[i],
            points,
            loop_index,
            label,
            lx,
            ly,
        });
    }
    (routes, stacks)
}

#[allow(clippy::too_many_arguments)]
fn settle(
    db: &BlockDb,
    plan: &Plan,
    views: &mut [View],
    parents: &[usize],
    titles: &[Box2],
    legs: &mut [Vec<f64>],
    routes: &mut Vec<Route>,
    grow: bool,
    tight: bool,
) -> bool {
    let (fresh, stacks) = route_all(db, plan, views, legs);
    *routes = fresh;
    let blocked: Option<Vec<Box2>> = plan
        .check
        .as_ref()
        .map(|list| boxes_of(views, list, titles, true));
    let mut base: HashMap<(usize, bool), Vec<f64>> = HashMap::new();
    let mut grown = false;
    for (_, stack) in stacks {
        if stack.len() < 2 {
            continue;
        }
        let (s, t) = plan.ends[stack[0]];
        let short = stack_labels(&stack, routes, legs, s, t, blocked.as_deref());
        let vertical = legs[stack[0]][0] == legs[stack[0]][2];
        if grow
            && short > 0.0
            && (vertical || !tight)
            && widen(views, parents, s, t, vertical, short.ceil(), &mut base)
        {
            grown = true;
        }
    }
    grown
}

fn overlap(routes: &[Route]) -> f64 {
    let mut area = 0.0;
    for i in 0..routes.len() {
        let p = &routes[i];
        for q in &routes[i + 1..] {
            let pw = (p.label.width + 8.0) / 2.0;
            let qw = (q.label.width + 8.0) / 2.0;
            let w = js_min(p.lx + pw, q.lx + qw) - js_max(p.lx - pw, q.lx - qw);
            let h = js_min(p.ly + p.label.height / 2.0, q.ly + q.label.height / 2.0)
                - js_max(p.ly - p.label.height / 2.0, q.ly - q.label.height / 2.0);
            if p.label.width > 0.0 && q.label.width > 0.0 && w > 0.0 && h > 0.0 {
                area += w * h;
            }
        }
    }
    area
}

#[derive(Clone, Copy)]
struct Extent {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Extent {
    fn grow(&mut self, x: f64, y: f64, w: f64, h: f64) {
        if x - w / 2.0 < self.min_x {
            self.min_x = x - w / 2.0;
        }
        if x + w / 2.0 > self.max_x {
            self.max_x = x + w / 2.0;
        }
        if y - h / 2.0 < self.min_y {
            self.min_y = y - h / 2.0;
        }
        if y + h / 2.0 > self.max_y {
            self.max_y = y + h / 2.0;
        }
    }
}

fn bound(views: &[View], plan: &Plan, routes: &[Route], content: (f64, f64)) -> (f64, Extent) {
    let mut extent = Extent {
        min_x: 0.0,
        min_y: 0.0,
        max_x: content.0,
        max_y: content.1,
    };
    for (i, route) in routes.iter().enumerate() {
        if route.loop_index > 0 {
            let (px, py, r, _, _) = loop_arc(&views[plan.ends[i].0], route.loop_index as f64);
            extent.grow(px, py, 2.0 * r + 2.0, 2.0 * r + 2.0);
        }
        if route.label.width > 0.0 {
            extent.grow(
                route.lx,
                route.ly,
                route.label.width + 8.0,
                route.label.height,
            );
        }
    }
    (extent.max_x - extent.min_x, extent)
}

fn draw(
    db: &BlockDb,
    config: &Config,
    options: &RenderOptions,
    tight: bool,
) -> Result<Drawn, PelersError> {
    let size = options.font_size.unwrap_or(16.0);
    let edge_size = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();
    let configured = config
        .get("block")
        .and_then(Value::as_object)
        .and_then(|section| section.get("padding"));
    let gap = js_min(
        match configured {
            Some(Value::Number(value)) if *value >= 0.0 => js_min(*value, 200.0),
            _ => GAP,
        },
        if tight { 6.0 } else { f64::INFINITY },
    );
    let wrap = if tight { 80.0 } else { WRAP };

    let mut views: Vec<View> = vec![new_view(db, db.root, tight, size, gap, wrap)];
    views[0].cell.nw = 0.0;
    views[0].cell.nh = 0.0;
    let mut order: Vec<usize> = vec![0];
    let mut parents: Vec<usize> = vec![0];
    let mut by_id: HashMap<String, usize> = HashMap::new();
    let mut i = 0;
    while i < order.len() {
        let parent = order[i];
        i += 1;
        if views[parent].kind != Kind::Composite {
            continue;
        }
        let children = db.arena[views[parent].block].children.clone();
        for child in children {
            let index = views.len();
            let view = new_view(db, child, tight, size, gap, wrap);
            let kind = view.kind;
            let child_id = db.arena[child].id.clone();
            views.push(view);
            views[parent].cell.children.push(index);
            parents.push(parent);
            order.push(index);
            if let (Some(id), false) = (child_id, kind == Kind::Space) {
                by_id.insert(id, index);
            }
        }
    }

    let mut obstacles: Vec<usize> = Vec::new();
    let mut titles: Vec<Box2> = Vec::new();
    let widths: Vec<f64> = views.iter().map(|v| v.cell.nw).collect();
    let mut content = place(
        &mut views,
        &order,
        &widths,
        gap,
        &mut obstacles,
        &mut titles,
    )?;

    let mut plan = Plan {
        drawn: Vec::new(),
        ends: Vec::new(),
        pairs: HashMap::new(),
        labelled: HashSet::new(),
        check: None,
        order_len: order.len(),
        edge_size,
    };
    for &edge in &db.edges {
        let start = db.arena[edge].start.clone().unwrap_or_default();
        let end = db.arena[edge].end.clone().unwrap_or_default();
        let (Some(&s), Some(&t)) = (by_id.get(&start), by_id.get(&end)) else {
            continue;
        };
        let key = plan.key_of(s, t);
        *plan.pairs.entry(key).or_insert(0) += 1;
        if db.arena[edge]
            .label
            .as_deref()
            .is_some_and(|l| !l.is_empty())
        {
            plan.labelled.insert(key);
        }
        plan.ends.push((s, t));
        plan.drawn.push(edge);
    }
    plan.check = if (plan.drawn.len() * obstacles.len()) as f64 <= OBSTACLE_BUDGET {
        Some(obstacles.clone())
    } else {
        None
    };
    let mut legs: Vec<Vec<f64>> = vec![Vec::new(); plan.drawn.len()];
    let mut routes: Vec<Route> = Vec::new();

    let grow_first = ((plan.drawn.len() * plan.drawn.len()) as f64) <= OBSTACLE_BUDGET;
    if settle(
        db,
        &plan,
        &mut views,
        &parents,
        &titles,
        &mut legs,
        &mut routes,
        grow_first,
        tight,
    ) {
        let before = overlap(&routes);
        let (narrow, _) = bound(&views, &plan, &routes, content);
        let mut grown = true;
        let mut round = 0;
        while round < 3 && grown {
            content = place(
                &mut views,
                &order,
                &widths,
                gap,
                &mut obstacles,
                &mut titles,
            )?;
            grown = settle(
                db,
                &plan,
                &mut views,
                &parents,
                &titles,
                &mut legs,
                &mut routes,
                round < 2,
                tight,
            );
            round += 1;
        }
        let (wide, _) = bound(&views, &plan, &routes, content);
        let room = match options.max_width {
            Some(max) if max > 0.0 => max,
            _ => f64::INFINITY,
        };
        if overlap(&routes) > before - 1.0 || (wide > narrow && (wide + 2.0 * pad).ceil() > room) {
            for v in views.iter_mut() {
                v.cell.row_gaps = Vec::new();
                v.cell.col_gaps = Vec::new();
            }
            content = place(
                &mut views,
                &order,
                &widths,
                gap,
                &mut obstacles,
                &mut titles,
            )?;
            settle(
                db,
                &plan,
                &mut views,
                &parents,
                &titles,
                &mut legs,
                &mut routes,
                false,
                tight,
            );
        }
    }

    let (_, extent) = bound(&views, &plan, &routes, content);
    let (min_x, min_y, max_x, max_y) = (extent.min_x, extent.min_y, extent.max_x, extent.max_y);

    let title = title_label(
        db.title.as_deref(),
        size,
        if tight { title_room(options) } else { 4000.0 },
    );
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let inner = js_max(max_x - min_x, title.width);
    let ox = pad - min_x + (inner - (max_x - min_x)) / 2.0;
    let oy = pad + title_height - min_y;
    for v in views.iter_mut() {
        v.cell.x += ox;
        v.cell.y += oy;
    }
    let width = (inner + 2.0 * pad).ceil();
    let height = (max_y - min_y + title_height + 2.0 * pad).ceil();

    let mut clusters = String::new();
    let mut nodes = String::new();
    let mut fills: Vec<Fill> = Vec::new();
    for v in views.iter().skip(1) {
        if v.kind == Kind::Space {
            continue;
        }
        let block = &db.arena[v.block];
        let classes = class_names(&block.classes.clone().unwrap_or_default().join(" "));
        let id = esc_text(block.id.as_deref().unwrap_or(""));
        if v.kind == Kind::Composite {
            let top = v.cell.y - v.cell.h / 2.0;
            if paints(&v.style.shape) {
                fills.push(Fill {
                    box_: [
                        v.cell.x - v.cell.w / 2.0,
                        top,
                        v.cell.x + v.cell.w / 2.0,
                        top + v.cell.h,
                    ],
                    shape: v.style.shape.clone(),
                });
            }
            clusters.push_str(&format!(
                "<g class=\"pelers-cluster{classes}\" data-id=\"{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\"{} stroke=\"var(--_b)\"{}/>{}</g>",
                num(v.cell.x - v.cell.w / 2.0),
                num(top),
                num(v.cell.w),
                num(v.cell.h),
                if sets_fill(&v.style.shape) { "" } else { " fill-opacity=\"0.5\"" },
                rounded(&v.style.shape),
                label_svg_with(
                    &v.label,
                    v.cell.x,
                    top + (v.cell.head + gap) / 2.0 + 2.0,
                    &format!(
                        " class=\"pelers-cluster-label\"{}{}",
                        ink_fill(&v.style.shape, &v.style.text, Some("var(--_m)")),
                        v.style.text
                    ),
                    icons
                )
            ));
            continue;
        }
        let arrow = v.kind == Kind::Arrow;
        let mut label = v.label.clone();
        if v.cell.span > 1.0
            && v.label.width > 0.0
            && v.label.lines.len() > 1
            && v.bw > v.cell.nw
            && (arrow || !is_fixed(&v.shape))
        {
            let text_style = (if v.style.bold { BOLD_STYLE } else { 0 })
                | (if v.style.italic { ITALIC } else { 0 });
            let room = v.label.width + (v.bw - v.cell.nw);
            label = layout_label(
                block.label.as_deref(),
                false,
                v.label.size,
                room,
                text_style,
            );
        }
        let shape_svg = if arrow {
            draw_arrow(
                v.mask,
                v.bw,
                v.bh,
                &format!("{SHAPE_ATTRS}{}", v.style.shape),
            )
        } else {
            draw_shape(
                &v.shape,
                v.bw,
                v.bh,
                &format!("{SHAPE_ATTRS}{}", v.style.shape),
                &format!("{LINE_ATTRS}{}", v.style.line),
            )
        };
        let label_attrs = format!(
            " class=\"pelers-label\"{}{}",
            if arrow || shape_filled(&v.shape) {
                ink_fill(&v.style.shape, &v.style.text, None)
            } else {
                String::new()
            },
            v.style.text
        );
        nodes.push_str(&format!(
            "<g class=\"pelers-node pelers-shape-{}{classes}\" data-id=\"{id}\" transform=\"translate({},{})\">{shape_svg}{}</g>",
            if arrow { "block-arrow" } else { v.shape.as_str() },
            num(v.cell.x),
            num(v.cell.y),
            label_svg_with(&label, v.dx, v.dy, &label_attrs, icons)
        ));
    }

    let mut casings = String::new();
    let mut edges = String::new();
    let mut labels = String::new();
    for (i, route) in routes.iter().enumerate() {
        let edge = &db.arena[route.edge];
        let start_type = edge.arrow_type_start.clone().unwrap_or_default();
        let end_type = edge.arrow_type_end.clone().unwrap_or_default();
        let thick = edge.thickness.as_deref() == Some("thick");
        let dotted = edge.pattern.as_deref() == Some("dotted");
        let path = if route.loop_index > 0 {
            loop_path(
                &views[plan.ends[i].0],
                route.loop_index as f64,
                marker_trim(&start_type),
                marker_trim(&end_type),
            )
        } else {
            let shifted: Vec<f64> = route
                .points
                .as_chunks::<2>()
                .0
                .iter()
                .flat_map(|p| [p[0] + ox, p[1] + oy])
                .collect();
            trace(&shifted, marker_trim(&start_type), marker_trim(&end_type))
        };
        let over_found = over_fills(
            &path.d,
            &[path.sx, path.sy, path.ex, path.ey],
            &fills,
            "",
            None,
        );
        let over = over_found.clone().unwrap_or_default();
        if over_found.as_deref() == Some("") {
            casings.push_str(&edge_casing(&path, &start_type, &end_type, "", thick));
        }
        let id = esc_text(edge.id.as_deref().unwrap_or(""));
        let over_attr = if over.is_empty() {
            String::new()
        } else {
            format!(" stroke=\"{over}\"")
        };
        let ink = if over.is_empty() {
            "var(--_l)".to_owned()
        } else {
            over.clone()
        };
        edges.push_str(&format!(
            "<g class=\"pelers-edge\" data-id=\"{id}\"{over_attr}><path d=\"{}\"{}{}/>{}{}</g>",
            path.d,
            if thick { " stroke-width=\"2.5\"" } else { "" },
            if dotted {
                " stroke-dasharray=\"3 4\""
            } else {
                ""
            },
            marker(
                &start_type,
                path.sx,
                path.sy,
                path.sdx,
                path.sdy,
                &ink,
                "none"
            ),
            marker(
                &end_type, path.ex, path.ey, path.edx, path.edy, &ink, "none"
            ),
        ));
        if route.label.width > 0.0 {
            labels.push_str(&edge_label_svg(
                edge.id.as_deref().unwrap_or(""),
                &route.label,
                route.lx + ox,
                route.ly + oy,
                "",
                icons,
            ));
        }
    }

    let mut body = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    if !clusters.is_empty() {
        body.push_str(&format!("<g class=\"pelers-clusters\">{clusters}</g>"));
    }
    body.push_str(&casings_svg(&casings));
    if !edges.is_empty() {
        body.push_str(&format!(
            "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{edges}</g>"
        ));
    }
    body.push_str(&format!("<g class=\"pelers-nodes\">{nodes}</g>"));
    if !labels.is_empty() {
        body.push_str(&format!(
            "<g class=\"pelers-edge-labels\" font-size=\"{}\">{labels}</g>",
            js_number_to_string(edge_size)
        ));
    }
    let svg = svg_document(
        "block",
        width,
        height,
        size,
        options,
        Accessible {
            acc_title: None,
            acc_descr: None,
        },
        &body,
    );
    Ok(Drawn {
        svg,
        width,
        height,
        links: Vec::new(),
    })
}
