use crate::svg::builder::num;

fn to_int32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    value.trunc().rem_euclid(4_294_967_296.0) as u32 as i32
}

pub fn seeded_random(seed: f64) -> f64 {
    let mut t = to_int32(seed + 1_831_565_813.0);
    t = (t ^ ((t as u32 >> 15) as i32)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ ((t as u32 >> 7) as i32)).wrapping_mul(t | 61));
    f64::from((t ^ ((t as u32 >> 14) as i32)) as u32) / 4_294_967_296.0
}

pub fn hash_string(text: &str) -> i32 {
    let mut hash: i32 = 0;
    for unit in text.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(unit));
    }
    hash
}

pub fn resolve_seed(configured: Option<f64>, id: &str) -> f64 {
    match configured {
        Some(value) if value.is_finite() && value != 0.0 => value,
        _ => f64::from(hash_string(id)),
    }
}

#[derive(Clone, Copy)]
pub struct WaveOptions {
    pub segments: usize,
    pub vertical: bool,
    pub point_step: f64,
    pub control_step: f64,
    pub control_shift: f64,
    pub pin_start: bool,
    pub pin_end: bool,
}

pub const FOLD: WaveOptions = WaveOptions {
    segments: 7,
    vertical: true,
    point_step: 17.0,
    control_step: 31.0,
    control_shift: 7.0,
    pin_start: false,
    pin_end: false,
};

pub const HORIZON: WaveOptions = WaveOptions {
    segments: 7,
    vertical: false,
    point_step: 23.0,
    control_step: 37.0,
    control_shift: 11.0,
    pin_start: false,
    pin_end: false,
};

pub fn wave(
    from: f64,
    to: f64,
    center: f64,
    seed: f64,
    amplitude: f64,
    o: WaveOptions,
) -> Vec<f64> {
    let step = (to - from) / o.segments as f64;
    let mut along = Vec::with_capacity(o.segments + 1);
    let mut across = Vec::with_capacity(o.segments + 1);
    for i in 0..=o.segments {
        let pinned = (i == 0 && o.pin_start) || (i == o.segments && o.pin_end);
        along.push(from + i as f64 * step);
        across.push(if pinned {
            center
        } else {
            center + seeded_random(seed + i as f64 * o.point_step) * amplitude * 2.0 - amplitude
        });
    }
    let mut out = if o.vertical {
        vec![across[0], along[0]]
    } else {
        vec![along[0], across[0]]
    };
    for i in 0..o.segments {
        let mid = (along[i] + along[i + 1]) / 2.0;
        let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
        let offset = amplitude
            * 1.5
            * sign
            * seeded_random(seed + i as f64 * o.control_step + o.control_shift);
        if o.vertical {
            out.extend_from_slice(&[
                across[i] + offset,
                mid,
                across[i + 1] - offset,
                mid,
                across[i + 1],
                along[i + 1],
            ]);
        } else {
            out.extend_from_slice(&[
                mid,
                across[i] + offset,
                mid,
                across[i + 1] - offset,
                along[i + 1],
                across[i + 1],
            ]);
        }
    }
    out
}

pub fn cliff(cx: f64, top: f64, bottom: f64, amplitude: f64) -> Vec<f64> {
    let h = bottom - top;
    vec![
        cx,
        top,
        cx + amplitude,
        top + h * 0.2,
        cx - amplitude * 1.5,
        top + h * 0.55,
        cx + amplitude * 0.5,
        top + h * 0.75,
        cx - amplitude,
        top + h * 0.85,
        cx + amplitude * 0.3,
        top + h * 0.95,
        cx,
        bottom,
    ]
}

fn push_point(out: &mut String, x: f64, y: f64) {
    out.push_str(&num(x));
    out.push(',');
    out.push_str(&num(y));
}

pub fn forward(curve: &[f64]) -> String {
    let mut d = String::with_capacity(curve.len() * 12);
    let mut i = 2;
    while i < curve.len() {
        d.push('C');
        push_point(&mut d, curve[i], curve[i + 1]);
        d.push(' ');
        push_point(&mut d, curve[i + 2], curve[i + 3]);
        d.push(' ');
        push_point(&mut d, curve[i + 4], curve[i + 5]);
        i += 6;
    }
    d
}

pub fn backward(curve: &[f64]) -> String {
    let mut d = String::with_capacity(curve.len() * 12);
    let mut i = curve.len() as isize - 6;
    while i >= 2 {
        let at = i as usize;
        d.push('C');
        push_point(&mut d, curve[at + 2], curve[at + 3]);
        d.push(' ');
        push_point(&mut d, curve[at], curve[at + 1]);
        d.push(' ');
        push_point(&mut d, curve[at - 2], curve[at - 1]);
        i -= 6;
    }
    d
}

pub fn curve_path(curve: &[f64]) -> String {
    let body = forward(curve);
    let mut out = String::with_capacity(body.len() + 24);
    out.push('M');
    push_point(&mut out, curve[0], curve[1]);
    out.push_str(&body);
    out
}

pub fn ellipse_path(cx: f64, cy: f64, rx: f64, ry: f64) -> String {
    let r = format!("{},{}", num(rx), num(ry));
    format!(
        "M{},{}A{r} 0 1,1 {},{}A{r} 0 1,1 {},{}Z",
        num(cx - rx),
        num(cy),
        num(cx + rx),
        num(cy),
        num(cx - rx),
        num(cy)
    )
}
