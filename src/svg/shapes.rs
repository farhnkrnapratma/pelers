use crate::layout::CNode;
use crate::svg::builder::num;
use crate::svg::theme::rounded;
use crate::util::jsnum::{js_max, js_min};
use std::fmt::Write as _;

pub const PAD_X: f64 = 18.0;
pub const PAD_Y: f64 = 12.0;
const CORNER: f64 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeSize {
    pub w: f64,
    pub h: f64,
    pub dy: f64,
}

type Sizer = fn(f64, f64) -> ShapeSize;
type Drawer = fn(f64, f64, &str, &str) -> String;
type Inset = fn(f64, f64, i32, f64) -> f64;
type Span = fn(f64, f64, bool) -> f64;

#[derive(Clone, Copy)]
struct Def {
    size: Sizer,
    draw: Drawer,
    inset: Inset,
    span: Span,
    no_label: bool,
    bare: bool,
}

fn no_inset(_w: f64, _h: f64, _side: i32, _t: f64) -> f64 {
    0.0
}

fn no_span(_w: f64, _h: f64, _sideways: bool) -> f64 {
    0.0
}

fn base(size: Sizer, draw: Drawer) -> Def {
    Def {
        size,
        draw,
        inset: no_inset,
        span: no_span,
        no_label: false,
        bare: false,
    }
}

fn flat(w: f64, h: f64, sideways: bool) -> f64 {
    (if sideways { h } else { w }) - 2.0 * CORNER
}

fn flat_across_zero(w: f64, _h: f64, sideways: bool) -> f64 {
    if sideways { 0.0 } else { w - 2.0 * CORNER }
}

fn flat_across_quarter(w: f64, h: f64, sideways: bool) -> f64 {
    if sideways {
        0.0
    } else {
        w - 2.0 * (h / 4.0) - 2.0 * CORNER
    }
}

fn bulge(t: f64, half: f64, depth: f64) -> f64 {
    let k = js_min(1.0, t.abs() / half);
    depth * (1.0 - (1.0 - k * k).sqrt())
}

fn round_inset(w: f64, h: f64, side: i32, t: f64) -> f64 {
    if side & 1 == 1 {
        bulge(t, h / 2.0, w / 2.0)
    } else {
        bulge(t, w / 2.0, h / 2.0)
    }
}

fn round_span(w: f64, h: f64, sideways: bool) -> f64 {
    (if sideways { h } else { w }) * 0.6
}

fn cap_inset(w: f64, _h: f64, side: i32, t: f64) -> f64 {
    if side & 1 == 1 {
        0.0
    } else {
        bulge(t, w / 2.0, js_min(12.0, js_max(6.0, w / 14.0)))
    }
}

fn lean_inset(_w: f64, h: f64, side: i32, _t: f64) -> f64 {
    if side & 1 == 1 { h / 8.0 } else { 0.0 }
}

fn box_size(tw: f64, th: f64) -> ShapeSize {
    ShapeSize {
        w: js_max(tw + 2.0 * PAD_X, 2.0 * PAD_X + 12.0),
        h: th + 2.0 * PAD_Y,
        dy: 0.0,
    }
}

fn tri_size(tw: f64, th: f64, flip: bool) -> ShapeSize {
    let w = js_max(1.6 * tw + 40.0, 80.0);
    let h = (th + 10.0) / (1.0 - (tw + 8.0) / w);
    let dy = h / 2.0 - 10.0 - th / 2.0;
    ShapeSize {
        w,
        h,
        dy: if flip { -dy } else { dy },
    }
}

fn path(d: &str, a: &str) -> String {
    format!("<path d=\"{d}\"{a}/>")
}

fn poly(points: &[f64], a: &str) -> String {
    let mut d = String::new();
    for i in (0..points.len()).step_by(2) {
        if i > 0 {
            d.push(' ');
        }
        d.push_str(&num(points[i]));
        d.push(',');
        d.push_str(&num(points[i + 1]));
    }
    format!("<polygon points=\"{d}\"{a}/>")
}

fn rect(w: f64, h: f64, a: &str, rx: &str) -> String {
    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{rx}{a}/>",
        num(-w / 2.0),
        num(-h / 2.0),
        num(w),
        num(h)
    )
}

fn brace(x: f64, h: f64, dir: f64, line: &str) -> String {
    let y = h / 2.0;
    let k = 6.0 * dir;
    format!(
        "<path d=\"M{},{}q{},0 {},6V-6q0,6 {},6q{},0 {},6V{}q0,6 {},6\" fill=\"none\"{line}/>",
        num(x),
        num(-y),
        num(k),
        num(k),
        num(k),
        num(-k),
        num(-k),
        num(y - 6.0),
        num(-k)
    )
}

fn wave(w: f64, h: f64, a: &str) -> String {
    let x = w / 2.0;
    let top = -h / 2.0;
    let base = h / 2.0 - 10.0;
    let q = w / 4.0;
    format!(
        "<path d=\"M{},{}H{}V{}q{},-12 {},0t{},0Z\"{a}/>",
        num(-x),
        num(top),
        num(x),
        num(base),
        num(-q),
        num(-2.0 * q),
        num(-2.0 * q)
    )
}

fn def(shape: &str) -> Option<Def> {
    let d = match shape {
        "rect" => Def {
            span: flat,
            ..base(box_size, |w, h, a, _| rect(w, h, &rounded(a), ""))
        },
        "rounded" => Def {
            span: flat,
            ..base(box_size, |w, h, a, _| {
                rect(w, h, a, &format!(" rx=\"{}\"", num(js_min(12.0, h / 2.0))))
            })
        },
        "stadium" => Def {
            inset: |_w, h, side, t| {
                if side & 1 == 1 {
                    bulge(t, h / 2.0, h / 2.0)
                } else {
                    0.0
                }
            },
            span: |w, h, sideways| if sideways { h * 0.6 } else { w - h },
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + h * 0.5 + 2.0 * PAD_X - 8.0,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| rect(w, h, a, &format!(" rx=\"{}\"", num(h / 2.0))),
            )
        },
        "fr-rect" => Def {
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 16.0,
                    h: th + 2.0 * PAD_Y,
                    dy: 0.0,
                },
                |w, h, a, line| {
                    let x = w / 2.0 - 8.0;
                    rect(w, h, &rounded(a), "")
                        + &format!(
                            "<path d=\"M{},{}V{}M{},{}V{}\" fill=\"none\"{line}/>",
                            num(-x),
                            num(-h / 2.0),
                            num(h / 2.0),
                            num(x),
                            num(-h / 2.0),
                            num(h / 2.0)
                        )
                },
            )
        },
        "cyl" => Def {
            inset: cap_inset,
            span: |w, h, sideways| {
                if sideways {
                    h - 2.0 * CORNER - 12.0
                } else {
                    w * 0.6
                }
            },
            ..base(
                |tw, th| {
                    let w = js_max(tw + 2.0 * PAD_X, 56.0);
                    let ry = js_min(12.0, js_max(6.0, w / 14.0));
                    ShapeSize {
                        w,
                        h: th + 2.0 * PAD_Y + 2.0 * ry,
                        dy: ry / 2.0,
                    }
                },
                |w, h, a, line| {
                    let rx = w / 2.0;
                    let ry = js_min(12.0, js_max(6.0, w / 14.0));
                    let top = -h / 2.0 + ry;
                    let bottom = h / 2.0 - ry;
                    format!(
                        "<path d=\"M{},{}A{},{} 0 0 1 {},{}V{}A{},{} 0 0 1 {},{}Z\"{a}/>",
                        num(-rx),
                        num(top),
                        num(rx),
                        num(ry),
                        num(rx),
                        num(top),
                        num(bottom),
                        num(rx),
                        num(ry),
                        num(-rx),
                        num(bottom)
                    ) + &format!(
                        "<path d=\"M{},{}A{},{} 0 0 0 {},{}\" fill=\"none\"{line}/>",
                        num(-rx),
                        num(top),
                        num(rx),
                        num(ry),
                        num(rx),
                        num(top)
                    )
                },
            )
        },
        "circle" => Def {
            inset: round_inset,
            span: round_span,
            ..base(
                |tw, th| {
                    let d = js_max((tw * tw + th * th).sqrt() + 16.0, 44.0);
                    ShapeSize {
                        w: d,
                        h: d,
                        dy: 0.0,
                    }
                },
                |w, _h, a, _| format!("<circle r=\"{}\"{a}/>", num(w / 2.0)),
            )
        },
        "dbl-circ" => Def {
            inset: round_inset,
            span: round_span,
            ..base(
                |tw, th| {
                    let d = js_max((tw * tw + th * th).sqrt() + 26.0, 54.0);
                    ShapeSize {
                        w: d,
                        h: d,
                        dy: 0.0,
                    }
                },
                |w, _h, a, line| {
                    format!(
                        "<circle r=\"{}\"{a}/><circle r=\"{}\" fill=\"none\"{line}/>",
                        num(w / 2.0),
                        num(w / 2.0 - 5.0)
                    )
                },
            )
        },
        "ellipse" => Def {
            inset: round_inset,
            span: round_span,
            ..base(
                |tw, th| ShapeSize {
                    w: tw * 1.3 + 2.0 * PAD_X,
                    h: th * 1.2 + 2.0 * PAD_Y,
                    dy: 0.0,
                },
                |w, h, a, _| {
                    format!(
                        "<ellipse rx=\"{}\" ry=\"{}\"{a}/>",
                        num(w / 2.0),
                        num(h / 2.0)
                    )
                },
            )
        },
        "diam" => Def {
            inset: |w, h, side, t| {
                if side & 1 == 1 {
                    (t.abs() * w) / h
                } else {
                    (t.abs() * h) / w
                }
            },
            span: |w, h, sideways| (if sideways { h } else { w }) * 0.6,
            ..base(
                |tw, th| {
                    let a = tw + 16.0;
                    let b = th + 8.0;
                    ShapeSize {
                        w: a + 1.6 * b,
                        h: a / 1.6 + b,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    poly(
                        &[0.0, -h / 2.0, w / 2.0, 0.0, 0.0, h / 2.0, -w / 2.0, 0.0],
                        a,
                    )
                },
            )
        },
        "hex" => Def {
            inset: |_w, _h, side, t| if side & 1 == 1 { t.abs() / 2.0 } else { 0.0 },
            span: |w, h, sideways| {
                if sideways {
                    h * 0.5
                } else {
                    w - h / 2.0 - CORNER
                }
            },
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + 2.0 * PAD_X + h / 2.0,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    let m = h / 4.0;
                    poly(
                        &[
                            -w / 2.0 + m,
                            -h / 2.0,
                            w / 2.0 - m,
                            -h / 2.0,
                            w / 2.0,
                            0.0,
                            w / 2.0 - m,
                            h / 2.0,
                            -w / 2.0 + m,
                            h / 2.0,
                            -w / 2.0,
                            0.0,
                        ],
                        a,
                    )
                },
            )
        },
        "odd" => Def {
            inset: |_w, h, side, _t| if side == 3 { h / 4.0 } else { 0.0 },
            span: flat_across_zero,
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + 2.0 * PAD_X + h / 4.0,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    let notch = h / 4.0;
                    poly(
                        &[
                            -w / 2.0,
                            -h / 2.0,
                            w / 2.0,
                            -h / 2.0,
                            w / 2.0,
                            h / 2.0,
                            -w / 2.0,
                            h / 2.0,
                            -w / 2.0 + notch,
                            0.0,
                        ],
                        a,
                    )
                },
            )
        },
        "lean-r" => Def {
            inset: lean_inset,
            span: flat_across_quarter,
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + 2.0 * PAD_X + h / 2.0,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    let s = h / 4.0;
                    poly(
                        &[
                            -w / 2.0 + s,
                            -h / 2.0,
                            w / 2.0,
                            -h / 2.0,
                            w / 2.0 - s,
                            h / 2.0,
                            -w / 2.0,
                            h / 2.0,
                        ],
                        a,
                    )
                },
            )
        },
        "lean-l" => Def {
            inset: lean_inset,
            span: flat_across_quarter,
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + 2.0 * PAD_X + h / 2.0,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    let s = h / 4.0;
                    poly(
                        &[
                            -w / 2.0,
                            -h / 2.0,
                            w / 2.0 - s,
                            -h / 2.0,
                            w / 2.0,
                            h / 2.0,
                            -w / 2.0 + s,
                            h / 2.0,
                        ],
                        a,
                    )
                },
            )
        },
        "trap-b" => Def {
            inset: lean_inset,
            span: flat_across_quarter,
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + 2.0 * PAD_X + h / 2.0,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    let s = h / 4.0;
                    poly(
                        &[
                            -w / 2.0 + s,
                            -h / 2.0,
                            w / 2.0 - s,
                            -h / 2.0,
                            w / 2.0,
                            h / 2.0,
                            -w / 2.0,
                            h / 2.0,
                        ],
                        a,
                    )
                },
            )
        },
        "trap-t" => Def {
            inset: lean_inset,
            span: flat_across_quarter,
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + 2.0 * PAD_X + h / 2.0,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    let s = h / 4.0;
                    poly(
                        &[
                            -w / 2.0,
                            -h / 2.0,
                            w / 2.0,
                            -h / 2.0,
                            w / 2.0 - s,
                            h / 2.0,
                            -w / 2.0 + s,
                            h / 2.0,
                        ],
                        a,
                    )
                },
            )
        },
        "text" => Def {
            bare: true,
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 8.0,
                    h: th + 8.0,
                    dy: 0.0,
                },
                |w, h, _, _| rect(w, h, " fill=\"none\" stroke=\"none\"", ""),
            )
        },
        "sm-circ" => Def {
            no_label: true,
            inset: round_inset,
            span: round_span,
            ..base(
                |_, _| ShapeSize {
                    w: 14.0,
                    h: 14.0,
                    dy: 0.0,
                },
                |w, _h, a, _| format!("<circle r=\"{}\"{a}/>", num(w / 2.0)),
            )
        },
        "f-circ" => Def {
            no_label: true,
            inset: round_inset,
            span: round_span,
            ..base(
                |_, _| ShapeSize {
                    w: 14.0,
                    h: 14.0,
                    dy: 0.0,
                },
                |w, _h, _a, line| {
                    format!("<circle r=\"{}\" fill=\"var(--_l)\"{line}/>", num(w / 2.0))
                },
            )
        },
        "fr-circ" => Def {
            no_label: true,
            inset: round_inset,
            span: round_span,
            ..base(
                |_, _| ShapeSize {
                    w: 18.0,
                    h: 18.0,
                    dy: 0.0,
                },
                |w, _h, a, line| {
                    format!(
                        "<circle r=\"{}\"{a}/><circle r=\"{}\" fill=\"var(--_l)\"{line}/>",
                        num(w / 2.0),
                        num(w / 2.0 - 4.0)
                    )
                },
            )
        },
        "fork" => Def {
            no_label: true,
            span: |w, h, sideways| (if sideways { h } else { w }) - 8.0,
            ..base(
                |_, _| ShapeSize {
                    w: 72.0,
                    h: 8.0,
                    dy: 0.0,
                },
                |w, h, _a, line| rect(w, h, &format!(" fill=\"var(--_l)\"{line}"), " rx=\"2\""),
            )
        },
        "notch-rect" => Def {
            span: flat,
            ..base(box_size, |w, h, a, _| {
                let c = 10.0;
                poly(
                    &[
                        -w / 2.0 + c,
                        -h / 2.0,
                        w / 2.0,
                        -h / 2.0,
                        w / 2.0,
                        h / 2.0,
                        -w / 2.0,
                        h / 2.0,
                        -w / 2.0,
                        -h / 2.0 + c,
                    ],
                    a,
                )
            })
        },
        "lin-rect" => Def {
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 8.0,
                    h: th + 2.0 * PAD_Y,
                    dy: 0.0,
                },
                |w, h, a, line| {
                    rect(w, h, &rounded(a), "")
                        + &format!(
                            "<path d=\"M{},{}V{}\" fill=\"none\"{line}/>",
                            num(-w / 2.0 + 8.0),
                            num(-h / 2.0),
                            num(h / 2.0)
                        )
                },
            )
        },
        "div-rect" => Def {
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X,
                    h: th + 2.0 * PAD_Y + 8.0,
                    dy: 4.0,
                },
                |w, h, a, line| {
                    rect(w, h, &rounded(a), "")
                        + &format!(
                            "<path d=\"M{},{}H{}\" fill=\"none\"{line}/>",
                            num(-w / 2.0),
                            num(-h / 2.0 + 8.0),
                            num(w / 2.0)
                        )
                },
            )
        },
        "win-pane" => Def {
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 8.0,
                    h: th + 2.0 * PAD_Y + 8.0,
                    dy: 4.0,
                },
                |w, h, a, line| {
                    rect(w, h, &rounded(a), "")
                        + &format!(
                            "<path d=\"M{},{}V{}M{},{}H{}\" fill=\"none\"{line}/>",
                            num(-w / 2.0 + 8.0),
                            num(-h / 2.0),
                            num(h / 2.0),
                            num(-w / 2.0),
                            num(-h / 2.0 + 8.0),
                            num(w / 2.0)
                        )
                },
            )
        },
        "tri" => Def {
            inset: |w, _h, side, _t| if side & 1 == 1 { w / 4.0 } else { 0.0 },
            ..base(
                |tw, th| tri_size(tw, th, false),
                |w, h, a, _| poly(&[0.0, -h / 2.0, w / 2.0, h / 2.0, -w / 2.0, h / 2.0], a),
            )
        },
        "flip-tri" => Def {
            inset: |w, _h, side, _t| if side & 1 == 1 { w / 4.0 } else { 0.0 },
            ..base(
                |tw, th| tri_size(tw, th, true),
                |w, h, a, _| poly(&[-w / 2.0, -h / 2.0, w / 2.0, -h / 2.0, 0.0, h / 2.0], a),
            )
        },
        "sl-rect" => Def {
            inset: |_w, _h, side, _t| if side == 0 { 5.0 } else { 0.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X,
                    h: th + 2.0 * PAD_Y + 10.0,
                    dy: 5.0,
                },
                |w, h, a, _| {
                    poly(
                        &[
                            -w / 2.0,
                            -h / 2.0 + 10.0,
                            w / 2.0,
                            -h / 2.0,
                            w / 2.0,
                            h / 2.0,
                            -w / 2.0,
                            h / 2.0,
                        ],
                        a,
                    )
                },
            )
        },
        "notch-pent" => Def {
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X,
                    h: th + 2.0 * PAD_Y,
                    dy: 0.0,
                },
                |w, h, a, _| {
                    let c = 10.0;
                    poly(
                        &[
                            -w / 2.0 + c,
                            -h / 2.0,
                            w / 2.0 - c,
                            -h / 2.0,
                            w / 2.0,
                            -h / 2.0 + c,
                            w / 2.0,
                            h / 2.0,
                            -w / 2.0,
                            h / 2.0,
                            -w / 2.0,
                            -h / 2.0 + c,
                        ],
                        a,
                    )
                },
            )
        },
        "hourglass" => Def {
            no_label: true,
            inset: |w, _h, side, _t| if side & 1 == 1 { w / 2.0 } else { 0.0 },
            ..base(
                |_, _| ShapeSize {
                    w: 32.0,
                    h: 40.0,
                    dy: 0.0,
                },
                |w, h, a, _| {
                    poly(
                        &[
                            -w / 2.0,
                            -h / 2.0,
                            w / 2.0,
                            -h / 2.0,
                            -w / 2.0,
                            h / 2.0,
                            w / 2.0,
                            h / 2.0,
                        ],
                        a,
                    )
                },
            )
        },
        "delay" => base(
            |tw, th| {
                let h = th + 2.0 * PAD_Y;
                ShapeSize {
                    w: tw + 2.0 * PAD_X + h / 4.0,
                    h,
                    dy: 0.0,
                }
            },
            |w, h, a, _| {
                let r = h / 2.0;
                format!(
                    "<path d=\"M{},{}H{}A{},{} 0 0 1 {},{}H{}Z\"{a}/>",
                    num(-w / 2.0),
                    num(-r),
                    num(w / 2.0 - r),
                    num(r),
                    num(r),
                    num(w / 2.0 - r),
                    num(r),
                    num(-w / 2.0)
                )
            },
        ),
        "h-cyl" => base(
            |tw, th| {
                let h = th + 2.0 * PAD_Y;
                let rx = js_min(12.0, h / 4.0);
                ShapeSize {
                    w: tw + 2.0 * PAD_X + 2.0 * rx,
                    h,
                    dy: 0.0,
                }
            },
            |w, h, a, line| {
                let ry = h / 2.0;
                let rx = js_min(12.0, h / 4.0);
                let l = -w / 2.0 + rx;
                let r = w / 2.0 - rx;
                format!(
                    "<path d=\"M{},{}H{}A{},{} 0 0 1 {},{}H{}A{},{} 0 0 1 {},{}Z\"{a}/>",
                    num(l),
                    num(-ry),
                    num(r),
                    num(rx),
                    num(ry),
                    num(r),
                    num(ry),
                    num(l),
                    num(rx),
                    num(ry),
                    num(l),
                    num(-ry)
                ) + &format!(
                    "<path d=\"M{},{}A{},{} 0 0 0 {},{}\" fill=\"none\"{line}/>",
                    num(r),
                    num(-ry),
                    num(rx),
                    num(ry),
                    num(r),
                    num(ry)
                )
            },
        ),
        "cross-circ" => Def {
            no_label: true,
            inset: round_inset,
            span: round_span,
            ..base(
                |_, _| ShapeSize {
                    w: 44.0,
                    h: 44.0,
                    dy: 0.0,
                },
                |w, _h, a, line| {
                    let r = w / 2.0;
                    let k = r * std::f64::consts::FRAC_1_SQRT_2;
                    format!("<circle r=\"{}\"{a}/>", num(r))
                        + &format!(
                            "<path d=\"M{},{}L{},{}M{},{}L{},{}\" fill=\"none\"{line}/>",
                            num(-k),
                            num(-k),
                            num(k),
                            num(k),
                            num(k),
                            num(-k),
                            num(-k),
                            num(k)
                        )
                },
            )
        },
        "st-rect" => Def {
            span: |w, h, sideways| (if sideways { h } else { w }) - 2.0 * CORNER - 8.0,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 8.0,
                    h: th + 2.0 * PAD_Y + 8.0,
                    dy: 4.0,
                },
                |w, h, a, _| {
                    let bw = w - 8.0;
                    let bh = h - 8.0;
                    let mut out = String::new();
                    for i in 0..3 {
                        let x = -w / 2.0 + (2 - i) as f64 * 4.0;
                        let y = -h / 2.0 + i as f64 * 4.0;
                        let _ = write!(
                            out,
                            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>",
                            num(x),
                            num(y),
                            num(bw),
                            num(bh),
                            rounded(a)
                        );
                    }
                    out
                },
            )
        },
        "doc" => Def {
            inset: |_w, _h, side, _t| if side == 2 { 10.0 } else { 0.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X,
                    h: th + 2.0 * PAD_Y + 10.0,
                    dy: -5.0,
                },
                |w, h, a, _| wave(w, h, a),
            )
        },
        "flag" => Def {
            inset: |_w, _h, side, _t| if side & 1 == 1 { 0.0 } else { 8.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X,
                    h: th + 2.0 * PAD_Y + 16.0,
                    dy: 0.0,
                },
                |w, h, a, _| {
                    let x = w / 2.0;
                    let y = h / 2.0 - 8.0;
                    let q = w / 4.0;
                    format!(
                        "<path d=\"M{},{}q{},-10 {},0t{},0V{}q{},10 {},0t{},0Z\"{a}/>",
                        num(-x),
                        num(-y),
                        num(q),
                        num(2.0 * q),
                        num(2.0 * q),
                        num(y),
                        num(-q),
                        num(-2.0 * q),
                        num(-2.0 * q)
                    )
                },
            )
        },
        "datastore" => Def {
            span: flat,
            ..base(box_size, |w, h, a, line| {
                rect(
                    w,
                    h,
                    &a.replacen("stroke=\"var(--_b)\"", "stroke=\"none\"", 1),
                    "",
                ) + &format!(
                    "<path d=\"M{},{}H{}M{},{}H{}\" fill=\"none\"{line}/>",
                    num(-w / 2.0),
                    num(-h / 2.0),
                    num(w / 2.0),
                    num(-w / 2.0),
                    num(h / 2.0),
                    num(w / 2.0)
                )
            })
        },
        "folder" => base(
            |tw, th| ShapeSize {
                w: js_max(tw + 2.0 * PAD_X, 64.0),
                h: th + 2.0 * PAD_Y + 8.0,
                dy: 4.0,
            },
            |w, h, a, _| {
                let x = w / 2.0;
                let y = h / 2.0;
                let tab = js_min(w * 0.4, 48.0);
                path(
                    &format!(
                        "M{},{}H{}L{},{}H{}V{}H{}Z",
                        num(-x),
                        num(-y),
                        num(-x + tab),
                        num(-x + tab + 8.0),
                        num(-y + 8.0),
                        num(x),
                        num(y),
                        num(-x)
                    ),
                    a,
                )
            },
        ),
        "bucket" => Def {
            inset: |_w, h, side, _t| if side & 1 == 1 { h / 12.0 } else { 0.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 20.0,
                    h: th + 2.0 * PAD_Y + 10.0,
                    dy: 3.0,
                },
                |w, h, a, line| {
                    let x = w / 2.0;
                    let y = h / 2.0;
                    let ry = 5.0;
                    let taper = h / 6.0;
                    path(
                        &format!(
                            "M{},{}A{},{} 0 0 1 {},{}L{},{}A{},{} 0 0 1 {},{}Z",
                            num(-x),
                            num(-y + ry),
                            num(x),
                            num(ry),
                            num(x),
                            num(-y + ry),
                            num(x - taper),
                            num(y - ry),
                            num(x - taper),
                            num(ry),
                            num(-x + taper),
                            num(y - ry)
                        ),
                        a,
                    ) + &format!(
                        "<path d=\"M{},{}A{},{} 0 0 0 {},{}\" fill=\"none\"{line}/>",
                        num(-x),
                        num(-y + ry),
                        num(x),
                        num(ry),
                        num(x),
                        num(-y + ry)
                    )
                },
            )
        },
        "console" => Def {
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: js_max(tw + 2.0 * PAD_X, 72.0),
                    h: th + 2.0 * PAD_Y + 12.0,
                    dy: 6.0,
                },
                |w, h, a, line| {
                    rect(w, h, &rounded(a), "")
                        + &format!(
                            "<path d=\"M{},{}H{}M{},{}l3,2.5l-3,2.5M{},{}h4\" fill=\"none\"{line}/>",
                            num(-w / 2.0),
                            num(-h / 2.0 + 12.0),
                            num(w / 2.0),
                            num(-w / 2.0 + 7.0),
                            num(-h / 2.0 + 4.0),
                            num(-w / 2.0 + 13.0),
                            num(-h / 2.0 + 9.0)
                        )
                },
            )
        },
        "browser" => Def {
            span: flat,
            ..base(
                |tw, th| ShapeSize {
                    w: js_max(tw + 2.0 * PAD_X, 72.0),
                    h: th + 2.0 * PAD_Y + 12.0,
                    dy: 6.0,
                },
                |w, h, a, line| {
                    let mut dots = String::new();
                    for i in 0..3 {
                        let _ = write!(
                            dots,
                            "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"none\"{line}/>",
                            num(-w / 2.0 + 8.0 + i as f64 * 7.0),
                            num(-h / 2.0 + 6.0)
                        );
                    }
                    rect(w, h, &rounded(a), "")
                        + &format!(
                            "<path d=\"M{},{}H{}\" fill=\"none\"{line}/>",
                            num(-w / 2.0),
                            num(-h / 2.0 + 12.0),
                            num(w / 2.0)
                        )
                        + &dots
                },
            )
        },
        "person" => base(
            |tw, th| ShapeSize {
                w: js_max(tw + 2.0 * PAD_X, 56.0),
                h: th + 2.0 * PAD_Y + 26.0,
                dy: 13.0,
            },
            |w, h, a, _| {
                let top = -h / 2.0;
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\"{a}/>",
                    num(-w / 2.0),
                    num(top + 26.0),
                    num(w),
                    num(h - 26.0)
                ) + &format!("<circle cy=\"{}\" r=\"13\"{a}/>", num(top + 14.0))
            },
        ),
        "bang" => Def {
            inset: |_w, _h, _side, _t| 3.0,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 28.0,
                    h: th + 2.0 * PAD_Y + 24.0,
                    dy: 0.0,
                },
                |w, h, a, _| {
                    let spikes = 14usize;
                    let mut pts: Vec<f64> = Vec::new();
                    for i in 0..spikes * 2 {
                        let angle = (std::f64::consts::PI * i as f64) / spikes as f64
                            - std::f64::consts::PI / 2.0;
                        let k = if i % 2 == 0 { 1.0 } else { 0.82 };
                        pts.push(angle.cos() * (w / 2.0) * k);
                        pts.push(angle.sin() * (h / 2.0) * k);
                    }
                    poly(&pts, a)
                },
            )
        },
        "cloud" => Def {
            inset: |_w, _h, side, _t| if side & 1 == 1 { 2.0 } else { 1.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 32.0,
                    h: th + 2.0 * PAD_Y + 24.0,
                    dy: 0.0,
                },
                |w, h, a, _| {
                    let x = w / 2.0;
                    let y = h / 2.0;
                    let r = y * 0.55;
                    path(
                        &format!(
                            "M{},{}A{},{} 0 0 1 {},{}A{},{} 0 0 1 {},{}A{},{} 0 0 1 {},{}A{},{} 0 0 1 {},{}Z",
                            num(-x + r),
                            num(y),
                            num(r),
                            num(r),
                            num(-x + r * 0.6),
                            num(-y * 0.1),
                            num(x * 0.42),
                            num(y * 0.7),
                            num(-x * 0.1),
                            num(-y * 0.72),
                            num(x * 0.4),
                            num(y * 0.62),
                            num(x * 0.62),
                            num(-y * 0.42),
                            num(r * 1.05),
                            num(r * 1.05),
                            num(x - r),
                            num(y)
                        ),
                        a,
                    )
                },
            )
        },
        "brace" => Def {
            bare: true,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 12.0,
                    h: th + 2.0 * PAD_Y,
                    dy: 0.0,
                },
                |w, h, _a, line| {
                    rect(w, h, " fill=\"none\" stroke=\"none\"", "")
                        + &brace(-w / 2.0 + 12.0, h, -1.0, line)
                },
            )
        },
        "brace-r" => Def {
            bare: true,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 12.0,
                    h: th + 2.0 * PAD_Y,
                    dy: 0.0,
                },
                |w, h, _a, line| {
                    rect(w, h, " fill=\"none\" stroke=\"none\"", "")
                        + &brace(w / 2.0 - 12.0, h, 1.0, line)
                },
            )
        },
        "braces" => Def {
            bare: true,
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 24.0,
                    h: th + 2.0 * PAD_Y,
                    dy: 0.0,
                },
                |w, h, _a, line| {
                    rect(w, h, " fill=\"none\" stroke=\"none\"", "")
                        + &brace(-w / 2.0 + 12.0, h, -1.0, line)
                        + &brace(w / 2.0 - 12.0, h, 1.0, line)
                },
            )
        },
        "bolt" => Def {
            no_label: true,
            inset: |w, _h, side, _t| if side & 1 == 1 { w / 4.0 } else { 0.0 },
            ..base(
                |_, _| ShapeSize {
                    w: 36.0,
                    h: 56.0,
                    dy: 0.0,
                },
                |w, h, a, _| {
                    poly(
                        &[
                            w * 0.15,
                            -h / 2.0,
                            -w / 2.0,
                            h * 0.08,
                            -w * 0.05,
                            h * 0.08,
                            -w * 0.15,
                            h / 2.0,
                            w / 2.0,
                            -h * 0.08,
                            w * 0.05,
                            -h * 0.08,
                        ],
                        a,
                    )
                },
            )
        },
        "lin-cyl" => Def {
            inset: cap_inset,
            span: |w, h, sideways| {
                if sideways {
                    h - 2.0 * CORNER - 20.0
                } else {
                    w * 0.6
                }
            },
            ..base(
                |tw, th| {
                    let w = js_max(tw + 2.0 * PAD_X, 56.0);
                    let ry = js_min(12.0, js_max(6.0, w / 14.0));
                    ShapeSize {
                        w,
                        h: th + 2.0 * PAD_Y + 2.0 * ry + 8.0,
                        dy: ry / 2.0 + 4.0,
                    }
                },
                |w, h, a, line| {
                    let rx = w / 2.0;
                    let ry = js_min(12.0, js_max(6.0, w / 14.0));
                    let top = -h / 2.0 + ry;
                    let bottom = h / 2.0 - ry;
                    let arc = |y: f64| {
                        format!(
                            "M{},{}A{},{} 0 0 0 {},{}",
                            num(-rx),
                            num(y),
                            num(rx),
                            num(ry),
                            num(rx),
                            num(y)
                        )
                    };
                    path(
                        &format!(
                            "M{},{}A{},{} 0 0 1 {},{}V{}A{},{} 0 0 1 {},{}Z",
                            num(-rx),
                            num(top),
                            num(rx),
                            num(ry),
                            num(rx),
                            num(top),
                            num(bottom),
                            num(rx),
                            num(ry),
                            num(-rx),
                            num(bottom)
                        ),
                        a,
                    ) + &format!(
                        "<path d=\"{}{}\" fill=\"none\"{line}/>",
                        arc(top),
                        arc(top + 8.0)
                    )
                },
            )
        },
        "curv-trap" => base(
            |tw, th| {
                let h = th + 2.0 * PAD_Y;
                ShapeSize {
                    w: tw + 2.0 * PAD_X + h * 0.6,
                    h,
                    dy: 0.0,
                }
            },
            |w, h, a, _| {
                let x = w / 2.0;
                let y = h / 2.0;
                let k = h * 0.3;
                path(
                    &format!(
                        "M{},{}H{}A{},{} 0 0 1 {},{}H{}L{},0Z",
                        num(-x + k),
                        num(-y),
                        num(x - k),
                        num(k),
                        num(y),
                        num(x - k),
                        num(y),
                        num(-x + k),
                        num(-x)
                    ),
                    a,
                )
            },
        ),
        "docs" => Def {
            inset: |_w, _h, side, _t| if side == 2 { 10.0 } else { 0.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 8.0,
                    h: th + 2.0 * PAD_Y + 18.0,
                    dy: -1.0,
                },
                |w, h, a, _| {
                    let mut out = String::new();
                    for i in 0..3 {
                        let _ = write!(
                            out,
                            "<g transform=\"translate({},{})\">{}</g>",
                            num((1 - i) as f64 * 4.0),
                            num((i - 1) as f64 * 4.0),
                            wave(w - 8.0, h - 8.0, a)
                        );
                    }
                    out
                },
            )
        },
        "bow-rect" => Def {
            inset: |_w, h, side, _t| if side == 1 { h * 0.2 } else { 0.0 },
            ..base(
                |tw, th| {
                    let h = th + 2.0 * PAD_Y;
                    ShapeSize {
                        w: tw + 2.0 * PAD_X + h * 0.4,
                        h,
                        dy: 0.0,
                    }
                },
                |w, h, a, _| {
                    let x = w / 2.0;
                    let y = h / 2.0;
                    let k = h * 0.2;
                    path(
                        &format!(
                            "M{},{}H{}A{},{} 0 0 0 {},{}H{}A{},{} 0 0 1 {},{}Z",
                            num(-x + k),
                            num(-y),
                            num(x),
                            num(k),
                            num(y),
                            num(x),
                            num(y),
                            num(-x + k),
                            num(k),
                            num(y),
                            num(-x + k),
                            num(-y)
                        ),
                        a,
                    )
                },
            )
        },
        "tag-rect" => Def {
            span: flat,
            ..base(box_size, |w, h, a, line| {
                rect(w, h, &rounded(a), "")
                    + &format!(
                        "<path d=\"M{},{}L{},{}\" fill=\"none\"{line}/>",
                        num(w / 2.0 - 12.0),
                        num(h / 2.0),
                        num(w / 2.0),
                        num(h / 2.0 - 12.0)
                    )
            })
        },
        "tag-doc" => Def {
            inset: |_w, _h, side, _t| if side == 2 { 10.0 } else { 0.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X,
                    h: th + 2.0 * PAD_Y + 10.0,
                    dy: -5.0,
                },
                |w, h, a, line| {
                    wave(w, h, a)
                        + &format!(
                            "<path d=\"M{},{}L{},{}\" fill=\"none\"{line}/>",
                            num(w / 2.0 - 12.0),
                            num(h / 2.0 - 14.0),
                            num(w / 2.0),
                            num(h / 2.0 - 26.0)
                        )
                },
            )
        },
        "lin-doc" => Def {
            inset: |_w, _h, side, _t| if side == 2 { 10.0 } else { 0.0 },
            ..base(
                |tw, th| ShapeSize {
                    w: tw + 2.0 * PAD_X + 8.0,
                    h: th + 2.0 * PAD_Y + 10.0,
                    dy: -5.0,
                },
                |w, h, a, line| {
                    wave(w, h, a)
                        + &format!(
                            "<path d=\"M{},{}V{}\" fill=\"none\"{line}/>",
                            num(-w / 2.0 + 8.0),
                            num(-h / 2.0),
                            num(h / 2.0 - 6.0)
                        )
                },
            )
        },
        "collapsedGroup" => base(
            |tw, th| ShapeSize {
                w: tw + 2.0 * PAD_X + 8.0,
                h: th + 2.0 * PAD_Y + 8.0,
                dy: -2.0,
            },
            |w, h, a, _| {
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>",
                    num(-w / 2.0 + 6.0),
                    num(-h / 2.0 + 8.0),
                    num(w - 12.0),
                    num(h - 8.0),
                    rounded(a)
                ) + &format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>",
                    num(-w / 2.0),
                    num(-h / 2.0),
                    num(w),
                    num(h - 4.0),
                    rounded(a)
                )
            },
        ),
        _ => return None,
    };
    Some(d)
}

fn lookup(shape: &str) -> Option<Def> {
    def(shape)
}

pub fn shape_size(shape: &str, tw: f64, th: f64) -> ShapeSize {
    let d = lookup(shape).unwrap_or_else(|| def("rect").expect("rect is defined"));
    (d.size)(tw, th)
}

pub fn draw_shape(shape: &str, w: f64, h: f64, a: &str, line: &str) -> String {
    let d = lookup(shape).unwrap_or_else(|| def("rect").expect("rect is defined"));
    (d.draw)(w, h, a, line)
}

pub fn shape_inset(shape: &str, w: f64, h: f64, side: i32, t: f64) -> f64 {
    lookup(shape).map_or(0.0, |d| (d.inset)(w, h, side, t))
}

pub fn shape_span(shape: &str, w: f64, h: f64, sideways: bool) -> f64 {
    js_max(0.0, lookup(shape).map_or(0.0, |d| (d.span)(w, h, sideways)))
}

pub fn shape_has_label(shape: &str) -> bool {
    lookup(shape).is_none_or(|d| !d.no_label)
}

pub fn shape_filled(shape: &str) -> bool {
    lookup(shape).is_none_or(|d| !d.bare)
}

pub fn end_side(route: &[f64], at: usize, c: &CNode) -> i32 {
    let dx = route[at] - c.x;
    let dy = route[at + 1] - c.y;
    if (dy.abs() - c.h / 2.0).abs() < 0.5 {
        return if dy < 0.0 { 0 } else { 2 };
    }
    if (dx.abs() - c.w / 2.0).abs() < 0.5 {
        return if dx < 0.0 { 3 } else { 1 };
    }
    -1
}

pub struct View<'a> {
    pub shape: &'a str,
    pub w: f64,
    pub h: f64,
}

pub fn inset_route(route: &mut [f64], at: usize, view: &View, c: &CNode) {
    if c.is_group {
        return;
    }
    let side = end_side(route, at, c);
    if side == 0 || side == 2 {
        let amount =
            shape_inset(view.shape, view.w, view.h, side, route[at] - c.x) + (c.h - view.h) / 2.0;
        route[at + 1] += if side == 0 { amount } else { -amount };
    } else if side >= 0 {
        let amount = shape_inset(view.shape, view.w, view.h, side, route[at + 1] - c.y)
            + (c.w - view.w) / 2.0;
        route[at] += if side == 3 { amount } else { -amount };
    }
}
