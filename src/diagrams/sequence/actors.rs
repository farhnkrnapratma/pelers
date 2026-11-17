use crate::svg::builder::{label_svg_with, num};
use crate::svg::theme::ROUNDED;
use crate::text::label::Label;

const PAD_Y: f64 = 10.0;
const GLYPH: f64 = 34.0;
const GLYPH_GAP: f64 = 4.0;
const STACK: f64 = 5.0;
const CAP: f64 = 6.0;
const SHAPE: &str = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";
const LINE: &str = " fill=\"none\" stroke=\"var(--_b)\"";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorSize {
    pub w: f64,
    pub h: f64,
}

pub fn actor_kind(type_: &str) -> &'static str {
    match type_ {
        "participant" => "participant",
        "actor" => "actor",
        "boundary" => "boundary",
        "control" => "control",
        "entity" => "entity",
        "database" => "database",
        "collections" => "collections",
        "queue" => "queue",
        _ => "participant",
    }
}

pub fn is_figure(kind: &str) -> bool {
    matches!(
        kind,
        "actor" | "boundary" | "control" | "entity" | "database"
    )
}

pub fn actor_size(kind: &str, label: &Label, min_w: f64, min_h: f64, padding_x: f64) -> ActorSize {
    if is_figure(kind) {
        return ActorSize {
            w: crate::util::jsnum::js_max(40.0, label.width + 8.0),
            h: GLYPH
                + if label.height > 0.0 {
                    GLYPH_GAP + label.height
                } else {
                    0.0
                },
        };
    }
    let extra = match kind {
        "collections" => STACK,
        "queue" => 2.0 * CAP,
        _ => 0.0,
    };
    ActorSize {
        w: crate::util::jsnum::js_max(min_w, label.width + 2.0 * padding_x) + extra,
        h: crate::util::jsnum::js_max(min_h, label.height + 2.0 * PAD_Y)
            + if kind == "collections" { STACK } else { 0.0 },
    }
}

fn figure(kind: &str, cx: f64, top: f64) -> String {
    let x = num(cx);
    let y = |dy: f64| num(top + dy);
    match kind {
        "actor" => format!(
            "<circle cx=\"{x}\" cy=\"{}\" r=\"6\"{SHAPE}/><path d=\"M{x},{}V{}M{},{}H{}M{},{}L{x},{}L{},{}\"{LINE}/>",
            y(7.0),
            y(13.0),
            y(24.0),
            num(cx - 10.0),
            y(17.0),
            num(cx + 10.0),
            num(cx - 8.0),
            y(34.0),
            y(24.0),
            num(cx + 8.0),
            y(34.0)
        ),
        "boundary" => format!(
            "<path d=\"M{},{}V{}M{},{}H{}\"{LINE}/><circle cx=\"{}\" cy=\"{}\" r=\"11\"{SHAPE}/>",
            num(cx - 19.0),
            y(6.0),
            y(28.0),
            num(cx - 19.0),
            y(17.0),
            num(cx - 8.0),
            num(cx + 3.0),
            y(17.0)
        ),
        "control" => format!(
            "<circle cx=\"{x}\" cy=\"{}\" r=\"11\"{SHAPE}/><path d=\"M{},{}L{},{}L{},{}\"{LINE}/>",
            y(21.0),
            num(cx + 5.0),
            y(5.0),
            num(cx - 1.0),
            y(10.0),
            num(cx + 5.0),
            y(15.0)
        ),
        "entity" => format!(
            "<circle cx=\"{x}\" cy=\"{}\" r=\"11\"{SHAPE}/><path d=\"M{},{}H{}\"{LINE}/>",
            y(15.0),
            num(cx - 11.0),
            y(30.0),
            num(cx + 11.0)
        ),
        _ => {
            let l = num(cx - 14.0);
            let r = num(cx + 14.0);
            format!(
                "<path d=\"M{l},{}V{}A14,5 0 0 0 {r},{}V{}A14,5 0 0 0 {l},{}Z\"{SHAPE}/><path d=\"M{l},{}A14,5 0 0 0 {r},{}\"{LINE}/>",
                y(7.0),
                y(27.0),
                y(27.0),
                y(7.0),
                y(7.0),
                y(7.0),
                y(7.0)
            )
        }
    }
}

pub fn draw_actor(
    kind: &str,
    cx: f64,
    top: f64,
    size: ActorSize,
    label: &Label,
    icons: bool,
    ink: &str,
) -> String {
    let ActorSize { w, h } = size;
    let text = " class=\"pelers-label\"";
    if is_figure(kind) {
        return figure(kind, cx, top)
            + &label_svg_with(
                label,
                cx,
                top + GLYPH + GLYPH_GAP + label.height / 2.0,
                &format!("{text}{ink}"),
                icons,
            );
    }
    let left = cx - w / 2.0;
    match kind {
        "collections" => {
            let bw = w - STACK;
            let bh = h - STACK;
            let back = format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{SHAPE}{ROUNDED}/>",
                num(left + STACK),
                num(top),
                num(bw),
                num(bh)
            );
            let front = format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{SHAPE}{ROUNDED}/>",
                num(left),
                num(top + STACK),
                num(bw),
                num(bh)
            );
            back + &front
                + &label_svg_with(label, left + bw / 2.0, top + STACK + bh / 2.0, text, icons)
        }
        "queue" => {
            let l = num(left + CAP);
            let r = num(left + w - CAP);
            let b = num(top + h);
            let ry = num(h / 2.0);
            format!(
                "<path d=\"M{l},{}H{r}A{CAP},{ry} 0 0 1 {r},{b}H{l}A{CAP},{ry} 0 0 1 {l},{}Z\"{SHAPE}/><path d=\"M{r},{}A{CAP},{ry} 0 0 0 {r},{b}\"{LINE}/>{}",
                num(top),
                num(top),
                num(top),
                label_svg_with(label, cx - CAP / 2.0, top + h / 2.0, text, icons)
            )
        }
        _ => format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{SHAPE}{ROUNDED}/>{}",
            num(left),
            num(top),
            num(w),
            num(h),
            label_svg_with(label, cx, top + h / 2.0, text, icons)
        ),
    }
}
