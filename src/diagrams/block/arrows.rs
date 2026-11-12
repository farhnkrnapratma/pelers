use crate::svg::builder::num;
use crate::util::chars::trim;
use crate::util::jsnum::{js_max, js_min};

pub const RIGHT: u32 = 1;
pub const LEFT: u32 = 2;
pub const UP: u32 = 4;
pub const DOWN: u32 = 8;

pub const OVERHANG: f64 = 7.0;
const MAX_DEPTH: f64 = 20.0;
const NUB: f64 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrowSize {
    pub w: f64,
    pub h: f64,
    pub dx: f64,
    pub dy: f64,
}

pub fn arrow_mask(directions: Option<&[String]>) -> u32 {
    let mut mask = 0;
    for direction in directions.unwrap_or(&[]) {
        mask |= match trim(direction) {
            "right" => RIGHT,
            "left" => LEFT,
            "up" => UP,
            "down" => DOWN,
            "x" => RIGHT | LEFT,
            "y" => UP | DOWN,
            _ => 0,
        };
    }
    mask
}

fn bit(mask: u32, flag: u32) -> f64 {
    if mask & flag != 0 { 1.0 } else { 0.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    None,
}

pub fn arrow_axis(mask: u32) -> Axis {
    if mask != 0 && mask & (UP | DOWN) == 0 {
        return Axis::X;
    }
    if mask != 0 && mask & (RIGHT | LEFT) == 0 {
        return Axis::Y;
    }
    Axis::None
}

pub fn arrow_size(mask: u32, tw: f64, th: f64) -> ArrowSize {
    let axis = arrow_axis(mask);
    let across = bit(mask, LEFT) - bit(mask, RIGHT);
    let along = bit(mask, UP) - bit(mask, DOWN);
    match axis {
        Axis::X => {
            let h = js_max(th + 8.0, 14.0) + 2.0 * OVERHANG;
            let depth = js_min(h / 2.0, MAX_DEPTH);
            ArrowSize {
                w: js_max(tw + 12.0, 16.0) + depth * if mask == (RIGHT | LEFT) { 2.0 } else { 1.0 },
                h,
                dx: (across * depth) / 2.0,
                dy: 0.0,
            }
        }
        Axis::Y => {
            let w = js_max(tw + 16.0, 14.0) + 2.0 * OVERHANG;
            let depth = js_min(w / 2.0, MAX_DEPTH);
            ArrowSize {
                w,
                h: js_max(th + 8.0, 16.0) + depth * if mask == (UP | DOWN) { 2.0 } else { 1.0 },
                dx: 0.0,
                dy: (along * depth) / 2.0,
            }
        }
        Axis::None => ArrowSize {
            w: js_max(tw + 16.0, 24.0) + NUB * (bit(mask, LEFT) + bit(mask, RIGHT)),
            h: js_max(th + 8.0, 24.0) + NUB * (bit(mask, UP) + bit(mask, DOWN)),
            dx: (across * NUB) / 2.0,
            dy: (along * NUB) / 2.0,
        },
    }
}

pub fn draw_arrow(mask: u32, w: f64, h: f64, attrs: &str) -> String {
    let axis = arrow_axis(mask);
    let x = w / 2.0;
    let y = h / 2.0;
    let mut p: Vec<f64>;
    match axis {
        Axis::X => {
            let depth = js_min(y, MAX_DEPTH);
            let s = y - OVERHANG;
            let x0 = if mask & LEFT != 0 { -x + depth } else { -x };
            let x1 = if mask & RIGHT != 0 { x - depth } else { x };
            p = vec![x0, -s, x1, -s];
            if mask & RIGHT != 0 {
                p.extend_from_slice(&[x1, -y, x, 0.0, x1, y]);
            }
            p.extend_from_slice(&[x1, s, x0, s]);
            if mask & LEFT != 0 {
                p.extend_from_slice(&[x0, y, -x, 0.0, x0, -y]);
            }
        }
        Axis::Y => {
            let depth = js_min(x, MAX_DEPTH);
            let s = x - OVERHANG;
            let y0 = if mask & UP != 0 { -y + depth } else { -y };
            let y1 = if mask & DOWN != 0 { y - depth } else { y };
            p = vec![s, y0, s, y1];
            if mask & DOWN != 0 {
                p.extend_from_slice(&[x, y1, 0.0, y, -x, y1]);
            }
            p.extend_from_slice(&[-s, y1, -s, y0]);
            if mask & UP != 0 {
                p.extend_from_slice(&[-x, y0, 0.0, -y, x, y0]);
            }
        }
        Axis::None => {
            let x0 = if mask & LEFT != 0 { -x + NUB } else { -x };
            let x1 = if mask & RIGHT != 0 { x - NUB } else { x };
            let y0 = if mask & UP != 0 { -y + NUB } else { -y };
            let y1 = if mask & DOWN != 0 { y - NUB } else { y };
            let cx = (x0 + x1) / 2.0;
            let cy = (y0 + y1) / 2.0;
            let kx = js_min(10.0, (x1 - x0) / 2.0);
            let ky = js_min(10.0, (y1 - y0) / 2.0);
            p = vec![x0, y0];
            if mask & UP != 0 {
                p.extend_from_slice(&[cx - kx, y0, cx, -y, cx + kx, y0]);
            }
            p.extend_from_slice(&[x1, y0]);
            if mask & RIGHT != 0 {
                p.extend_from_slice(&[x1, cy - ky, x, cy, x1, cy + ky]);
            }
            p.extend_from_slice(&[x1, y1]);
            if mask & DOWN != 0 {
                p.extend_from_slice(&[cx + kx, y1, cx, y, cx - kx, y1]);
            }
            p.extend_from_slice(&[x0, y1]);
            if mask & LEFT != 0 {
                p.extend_from_slice(&[x0, cy + ky, -x, cy, x0, cy - ky]);
            }
        }
    }
    let mut points = String::new();
    for (k, pair) in p.as_chunks::<2>().0.iter().enumerate() {
        if k > 0 {
            points.push(' ');
        }
        points.push_str(&format!("{},{}", num(pair[0]), num(pair[1])));
    }
    format!("<polygon points=\"{points}\" stroke-linejoin=\"round\"{attrs}/>")
}
