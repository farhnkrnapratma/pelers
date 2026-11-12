use std::collections::{HashMap, HashSet};

use super::actors::{ActorSize, actor_kind, actor_size, draw_actor, is_figure};
use super::db::SeqDb;
use super::types::{MessageBody, SeqActor, SeqMessage, linetype, placement};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{tighten, title_room};
use crate::diagrams::common::literal_label::break_label;
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg_with, num};
use crate::svg::edges::{ARROW, marker};
use crate::svg::ink::ink;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ROUNDED, class_names, resolve_style, rounded};
use crate::svg::title::title_label;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, MEASURE_MONO, width as measure};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Head {
    None,
    Arrow,
    Cross,
    Open,
    HalfSolid,
    HalfStick,
}

#[derive(Clone, Copy, Debug)]
struct LineStyle {
    dotted: bool,
    head: Head,
    at_start: bool,
    at_end: bool,
    bottom: bool,
}

fn line_style(type_: i32) -> Option<LineStyle> {
    use linetype::*;
    let (dotted, head, ends, bottom) = match type_ {
        SOLID => (false, Head::Arrow, 1, false),
        DOTTED => (true, Head::Arrow, 1, false),
        SOLID_CROSS => (false, Head::Cross, 1, false),
        DOTTED_CROSS => (true, Head::Cross, 1, false),
        SOLID_OPEN => (false, Head::None, 1, false),
        DOTTED_OPEN => (true, Head::None, 1, false),
        SOLID_POINT => (false, Head::Open, 1, false),
        DOTTED_POINT => (true, Head::Open, 1, false),
        BIDIRECTIONAL_SOLID => (false, Head::Arrow, 2, false),
        BIDIRECTIONAL_DOTTED => (true, Head::Arrow, 2, false),
        _ => {
            let (dotted, d) = match type_ {
                41..=48 => (false, 0),
                51..=58 => (true, 10),
                _ => return None,
            };
            let base = type_ - d;
            let (head, ends, bottom) = match base {
                SOLID_TOP => (Head::HalfSolid, 1, false),
                SOLID_BOTTOM => (Head::HalfSolid, 1, true),
                STICK_TOP => (Head::HalfStick, 1, false),
                STICK_BOTTOM => (Head::HalfStick, 1, true),
                SOLID_ARROW_TOP_REVERSE => (Head::HalfSolid, 0, false),
                SOLID_ARROW_BOTTOM_REVERSE => (Head::HalfSolid, 0, true),
                STICK_ARROW_TOP_REVERSE => (Head::HalfStick, 0, false),
                _ => (Head::HalfStick, 0, true),
            };
            (dotted, head, ends, bottom)
        }
    };
    Some(LineStyle {
        dotted,
        head,
        at_start: ends != 1,
        at_end: ends != 0,
        bottom,
    })
}

fn frame_kind(type_: i32) -> Option<&'static str> {
    use linetype::*;
    Some(match type_ {
        LOOP_START => "loop",
        ALT_START => "alt",
        OPT_START => "opt",
        PAR_START | PAR_OVER_START => "par",
        CRITICAL_START => "critical",
        BREAK_START => "break",
        RECT_START => "rect",
        _ => return None,
    })
}

const MSG_PAD: f64 = 12.0;
const SELF_W: f64 = 28.0;
const SELF_H: f64 = 22.0;
const ACT_W: f64 = 10.0;
const ACT_STEP: f64 = 4.0;
const ACT_MIN: f64 = 14.0;
const NOTE_GAP: f64 = 10.0;
const NOTE_OVER: f64 = 16.0;
const FRAME_PAD: f64 = 20.0;
const TAB_H: f64 = 20.0;
const FRAME_TEXT: f64 = 240.0;
const BADGE_H: f64 = 16.0;
const CENTRAL_R: f64 = 4.0;
const DASH: &str = " stroke-dasharray=\"4 4\"";
const FONT_MONO: &str = "var(--_fm)";

fn is_line_message(type_: Option<i32>) -> bool {
    type_.is_some_and(|t| line_style(t).is_some())
}

#[derive(Clone, Debug)]
struct Numbering {
    index: Vec<f64>,
    shown: Vec<bool>,
}

fn number_messages(messages: &[SeqMessage], always: bool) -> Numbering {
    let mut index = vec![f64::NAN; messages.len()];
    let mut shown = vec![false; messages.len()];
    let mut current = 1.0f64;
    let mut step = 1.0f64;
    let mut visible = false;
    for (i, m) in messages.iter().enumerate() {
        if m.type_ == Some(linetype::AUTONUMBER) {
            if let MessageBody::Numbering(spec) = &m.message {
                if let Some(start) = spec.start.filter(|v| *v != 0.0 && !v.is_nan()) {
                    current = start;
                }
                if let Some(s) = spec.step.filter(|v| *v != 0.0 && !v.is_nan()) {
                    step = s;
                }
                visible = spec.visible;
            }
        } else if is_line_message(m.type_) {
            index[i] = current;
            shown[i] = visible || always;
            current = js_round((current + step) * 100.0) / 100.0;
        }
    }
    Numbering { index, shown }
}

fn fill_ink(fill: &str) -> Option<String> {
    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<String, Option<String>>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    CACHE.with(|cache| {
        if let Some(color) = cache.borrow().get(fill) {
            return color.clone();
        }
        let shape = resolve_style(&[format!("fill:{fill}")]).shape;
        let color = ink(&shape, "").map(str::to_owned);
        let mut cache = cache.borrow_mut();
        if cache.len() >= 1 << 12 {
            cache.clear();
        }
        cache.insert(fill.to_owned(), color.clone());
        color
    })
}

#[derive(Clone, Debug)]
struct Column {
    id: String,
    actor: SeqActor,
    kind: &'static str,
    label: Label,
    size: ActorSize,
    x: f64,
    create_at: Option<usize>,
    destroy_at: Option<usize>,
    top: f64,
    life_start: f64,
    life_end: Option<f64>,
    activations: Vec<f64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ViewKind {
    Line,
    SelfLoop,
    Note,
}

#[derive(Clone, Debug)]
struct View {
    kind: ViewKind,
    a: usize,
    b: usize,
    label: Label,
    x: f64,
    w: f64,
}

#[derive(Clone, Debug)]
struct Frame {
    kind: &'static str,
    x0: f64,
    x1: f64,
    top: f64,
    title: Label,
    tab_w: f64,
    fill: String,
    dividers: Vec<usize>,
    divider_y: Vec<f64>,
    divider_labels: Vec<Label>,
}

#[derive(Clone, Debug)]
struct BoxRun {
    box_index: usize,
    first: usize,
    last: usize,
    title: Label,
    grow: f64,
}

fn no_label() -> Label {
    Label {
        lines: Vec::new(),
        widths: Vec::new(),
        width: 0.0,
        height: 0.0,
        size: 0.0,
        line_height: 0.0,
        heights: None,
    }
}

fn option(conf: Option<&Value>, key: &str, fallback: f64) -> f64 {
    match conf.and_then(Value::as_object).and_then(|o| o.get(key)) {
        Some(Value::Number(value)) if *value >= 0.0 && *value <= 2000.0 => *value,
        _ => fallback,
    }
}

fn conf_string<'a>(conf: Option<&'a Value>, key: &str) -> Option<&'a str> {
    conf.and_then(Value::as_object)
        .and_then(|o| o.get(key))
        .and_then(Value::as_str)
}

fn conf_bool(conf: Option<&Value>, key: &str) -> Option<bool> {
    match conf.and_then(Value::as_object).and_then(|o| o.get(key)) {
        Some(Value::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn half(x: f64, y: f64, dir: f64, up: f64, stick: bool, color: &str) -> String {
    let bx = num(x - dir * ARROW);
    if stick {
        format!(
            "<path class=\"pelers-marker\" d=\"M{},{}L{bx},{}\"/>",
            num(x),
            num(y),
            num(y + up * 5.0)
        )
    } else {
        format!(
            "<path class=\"pelers-marker\" d=\"M{},{}L{bx},{}V{}Z\" fill=\"{color}\" stroke=\"none\"/>",
            num(x),
            num(y),
            num(y + up * 4.5),
            num(y)
        )
    }
}

fn head(type_: Head, x: f64, y: f64, dir: f64, up: f64, color: &str) -> String {
    match type_ {
        Head::Arrow => marker("arrow_point", x, y, dir, 0.0, color, "none"),
        Head::Cross => marker("arrow_cross", x, y, dir, 0.0, color, "none"),
        Head::Open => format!(
            "<path class=\"pelers-marker\" d=\"M{},{}L{},{}L{},{}\" stroke-linejoin=\"round\"/>",
            num(x - dir * ARROW),
            num(y - 4.5),
            num(x),
            num(y),
            num(x - dir * ARROW),
            num(y + 4.5)
        ),
        Head::HalfSolid => half(x, y, dir, up, false, color),
        Head::HalfStick => half(x, y, dir, up, true, color),
        Head::None => String::new(),
    }
}

fn head_trim(type_: Head) -> f64 {
    match type_ {
        Head::Arrow => ARROW - 1.0,
        Head::Cross => 4.0,
        _ => 0.0,
    }
}

struct Ctx<'a> {
    db: &'a SeqDb,
    messages: &'a [SeqMessage],
    small: f64,
    tiny: f64,
    tight: bool,
    base_style: u32,
    icons: bool,
    wrap_w: f64,
    msg_gap: f64,
    note_pad: f64,
    box_pad: f64,
    align: i32,
    msg_pad: f64,
    note_over: f64,
    frame_pad: f64,
    frame_nest: f64,
    numbering: Numbering,
    cols: Vec<Column>,
    col_of: HashMap<String, usize>,
    needs: HashMap<(usize, usize), f64>,
    views: Vec<Option<View>>,
    sized: Vec<Vec<(f64, Label)>>,
    runs: Vec<BoxRun>,
    gaps: Vec<f64>,
    frames: Vec<Frame>,
    frame_at: Vec<Option<usize>>,
    min_x: f64,
    max_x: f64,
    inks: HashMap<String, Option<String>>,
    stack: Vec<usize>,
    xs: Vec<f64>,
}

impl Ctx<'_> {
    fn crosses(&self, x0: f64, x1: f64) -> bool {
        let (mut lo, mut hi) = (0usize, self.xs.len());
        while lo < hi {
            let mid = (lo + hi) >> 1;
            if self.xs[mid] <= x0 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo < self.xs.len() && self.xs[lo] < x1
    }

    fn fill_under(&self, l: f64, r: f64) -> Option<String> {
        for &k in self.stack.iter().rev() {
            if self.frames[k].kind == "rect" {
                return Some(self.frames[k].fill.clone());
            }
        }
        for run in &self.runs {
            let (x0, x1) = self.box_edges(run);
            if l >= x0 && r <= x1 {
                return Some(self.db.boxes[run.box_index].fill.clone());
            }
        }
        None
    }

    fn backed(&mut self, label: &Label, cx: f64, cy: f64, frame: bool) -> String {
        if label.width == 0.0 {
            return String::new();
        }
        let l = cx - label.width / 2.0;
        let plate = self.crosses(l, l + label.width);
        let back = if plate {
            format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"var(--_bg)\" stroke=\"none\"/>",
                num(l - 3.0),
                num(cy - label.height / 2.0 + 2.0),
                num(label.width + 6.0),
                num(label.height - 4.0)
            )
        } else {
            String::new()
        };
        let fill = if plate {
            String::new()
        } else {
            self.under_color(l, l + label.width)
        };
        let attrs = if frame {
            format!(
                " class=\"pelers-frame-label\"{} stroke=\"none\"",
                if fill.is_empty() {
                    " fill=\"var(--_m)\"".to_owned()
                } else {
                    fill.clone()
                }
            )
        } else {
            fill
        };
        back + &label_svg_with(label, cx, cy, &attrs, self.icons)
    }

    fn under_color(&mut self, l: f64, r: f64) -> String {
        match self.fill_under(l, r) {
            Some(fill) => self.ink_on(&fill),
            None => String::new(),
        }
    }

    fn last(&self) -> isize {
        self.cols.len() as isize - 1
    }

    fn message_text(&self, raw: &str, wrap: bool, width: f64) -> Label {
        self.text(raw, self.small, wrap, width, self.base_style)
    }

    fn text(&self, raw: &str, font_size: f64, wrap: bool, width: f64, base: u32) -> Label {
        let label = layout_label(
            Some(raw),
            false,
            font_size,
            if self.tight || wrap {
                width
            } else {
                f64::INFINITY
            },
            base,
        );
        if self.tight {
            break_label(&label, js_max(120.0, width))
        } else {
            label
        }
    }

    fn plain_text(&self, raw: Option<&str>, font_size: f64, wrap: bool, width: f64) -> Label {
        self.text(raw.unwrap_or(""), font_size, wrap, width, 0)
    }

    fn badge_width(&self, i: usize) -> f64 {
        if self.numbering.shown[i] {
            js_max(
                BADGE_H,
                measure(&js_number_to_string(self.numbering.index[i]), self.tiny, 0) + 8.0,
            )
        } else {
            0.0
        }
    }

    fn sized_text(&mut self, i: usize, room: f64) -> Label {
        if self.sized[i].is_empty() {
            if room == self.wrap_w {
                return self.views[i]
                    .as_ref()
                    .map(|v| v.label.clone())
                    .unwrap_or_else(no_label);
            }
            let base = self.views[i]
                .as_ref()
                .map(|v| v.label.clone())
                .unwrap_or_else(no_label);
            self.sized[i].push((self.wrap_w, base));
        }
        if let Some((_, label)) = self.sized[i].iter().find(|(key, _)| *key == room) {
            return label.clone();
        }
        let raw = match &self.messages[i].message {
            MessageBody::Text(text) => text.clone(),
            MessageBody::Numbering(_) => String::new(),
        };
        let label = self.message_text(&raw, true, room);
        self.sized[i].push((room, label.clone()));
        label
    }

    fn ink_of(&mut self, fill: &str) -> Option<String> {
        if fill.is_empty() {
            return None;
        }
        if let Some(cached) = self.inks.get(fill) {
            return cached.clone();
        }
        let color = fill_ink(fill);
        self.inks.insert(fill.to_owned(), color.clone());
        color
    }

    fn ink_on(&mut self, fill: &str) -> String {
        match self.ink_of(fill) {
            Some(color) => format!(" fill=\"{color}\""),
            None => String::new(),
        }
    }

    fn extend(&mut self, x0: f64, x1: f64) {
        if x0 < self.min_x {
            self.min_x = x0;
        }
        if x1 > self.max_x {
            self.max_x = x1;
        }
    }

    fn place(&mut self) {
        for i in 1..self.cols.len() {
            self.cols[i].x = self.cols[i - 1].x + self.gaps[i - 1];
        }
    }

    fn box_edges(&self, run: &BoxRun) -> (f64, f64) {
        let first = &self.cols[run.first];
        let last = &self.cols[run.last];
        (
            first.x - first.size.w / 2.0 - self.box_pad - run.grow,
            last.x + last.size.w / 2.0 + self.box_pad + run.grow,
        )
    }

    fn span_all(&self) -> (f64, f64) {
        if self.last() < 0 {
            (0.0, 120.0)
        } else {
            let last = self.cols.len() - 1;
            (
                self.cols[0].x - self.frame_pad,
                self.cols[last].x + self.frame_pad,
            )
        }
    }

    fn layout(&mut self) {
        self.place();
        self.min_x = f64::INFINITY;
        self.max_x = f64::NEG_INFINITY;
        for i in 0..self.cols.len() {
            let (x, w) = (self.cols[i].x, self.cols[i].size.w);
            self.extend(x - w / 2.0, x + w / 2.0);
        }
        for run in self.runs.clone() {
            let (x0, x1) = self.box_edges(&run);
            self.extend(x0, x1);
        }

        let mut open: Vec<usize> = Vec::new();
        for i in 0..self.messages.len() {
            let m = self.messages[i].clone();
            let kind = m.type_.and_then(frame_kind);
            if let Some(kind) = kind {
                let fill = match (&m.message, kind) {
                    (MessageBody::Text(text), "rect") => text.clone(),
                    _ => String::new(),
                };
                self.frames.push(Frame {
                    kind,
                    x0: f64::INFINITY,
                    x1: f64::NEG_INFINITY,
                    top: 0.0,
                    title: no_label(),
                    tab_w: 0.0,
                    fill,
                    dividers: vec![i],
                    divider_y: Vec::new(),
                    divider_labels: Vec::new(),
                });
                let index = self.frames.len() - 1;
                self.frame_at[i] = Some(index);
                open.push(index);
            } else if matches!(
                m.type_,
                Some(linetype::ALT_ELSE | linetype::PAR_AND | linetype::CRITICAL_OPTION)
            ) {
                if let Some(&top) = open.last() {
                    self.frames[top].dividers.push(i);
                }
            } else if matches!(
                m.type_,
                Some(
                    linetype::LOOP_END
                        | linetype::ALT_END
                        | linetype::OPT_END
                        | linetype::PAR_END
                        | linetype::CRITICAL_END
                        | linetype::BREAK_END
                        | linetype::RECT_END
                )
            ) {
                let Some(frame_index) = open.pop() else {
                    continue;
                };
                self.frame_at[i] = Some(frame_index);
                if self.frames[frame_index].x0 > self.frames[frame_index].x1 {
                    let (x0, x1) = self.span_all();
                    self.frames[frame_index].x0 = x0;
                    self.frames[frame_index].x1 = x1;
                }
                if self.frames[frame_index].kind != "rect" {
                    let kind = self.frames[frame_index].kind;
                    self.frames[frame_index].tab_w = measure(kind, self.tiny, BOLD_STYLE) + 14.0;
                    let dividers = self.frames[frame_index].dividers.clone();
                    for (k, &message_index) in dividers.iter().enumerate() {
                        let raw = match &self.messages[message_index].message {
                            MessageBody::Text(text) => text.clone(),
                            MessageBody::Numbering(_) => String::new(),
                        };
                        let tab = if k == 0 {
                            self.frames[frame_index].tab_w + 8.0
                        } else {
                            0.0
                        };
                        let room =
                            self.frames[frame_index].x1 - self.frames[frame_index].x0 - 16.0 - tab;
                        let label = if raw.is_empty() {
                            no_label()
                        } else {
                            let wrap_width = js_max(
                                room,
                                if self.tight {
                                    js_min(FRAME_TEXT, self.wrap_w)
                                } else {
                                    FRAME_TEXT
                                },
                            );
                            self.text(&format!("[{raw}]"), self.small, true, wrap_width, 0)
                        };
                        if label.width > room {
                            self.frames[frame_index].x1 += label.width - room;
                        }
                        if k == 0 {
                            self.frames[frame_index].title = label;
                        } else {
                            self.frames[frame_index].divider_labels.push(label);
                        }
                    }
                }
                let (fx0, fx1) = (self.frames[frame_index].x0, self.frames[frame_index].x1);
                self.extend(fx0, fx1);
                if let Some(&parent) = open.last() {
                    self.frames[parent].x0 = js_min(self.frames[parent].x0, fx0 - self.frame_nest);
                    self.frames[parent].x1 = js_max(self.frames[parent].x1, fx1 + self.frame_nest);
                }
            } else if let Some(view) = self.views[i].clone() {
                let a_x = self.cols[view.a].x;
                let b_x = self.cols[view.b].x;
                let mut x0: f64;
                let mut x1: f64;
                if view.kind == ViewKind::Note {
                    let mut new_view = view.clone();
                    if m.placement == Some(placement::LEFTOF) {
                        new_view.x = a_x - NOTE_GAP - new_view.w;
                    } else if m.placement == Some(placement::RIGHTOF) {
                        new_view.x = a_x + NOTE_GAP;
                    } else if view.a == view.b {
                        new_view.x = a_x - new_view.w / 2.0;
                    } else {
                        let lo = js_min(a_x, b_x) - self.note_over;
                        let hi = js_max(a_x, b_x) + self.note_over;
                        if self.tight || m.wrap {
                            let raw = match &m.message {
                                MessageBody::Text(text) => text.clone(),
                                MessageBody::Numbering(_) => String::new(),
                            };
                            new_view.label = self.text(
                                &raw,
                                self.small,
                                true,
                                js_max(self.wrap_w, hi - lo - 2.0 * self.note_pad),
                                self.base_style,
                            );
                        }
                        new_view.w = js_max(hi - lo, new_view.label.width + 2.0 * self.note_pad);
                        new_view.x = (lo + hi - new_view.w) / 2.0;
                    }
                    x0 = new_view.x;
                    x1 = new_view.x + new_view.w;
                    self.views[i] = Some(new_view);
                } else if view.kind == ViewKind::SelfLoop {
                    x0 = a_x;
                    x1 = a_x + ACT_W + SELF_W + 8.0 + view.label.width + self.badge_width(i);
                } else {
                    let mut lo = js_min(a_x, b_x);
                    let mut hi = js_max(a_x, b_x);
                    x0 = lo;
                    x1 = hi;
                    if self.tight || m.wrap {
                        let room = js_max(self.wrap_w, x1 - x0 - 2.0 * self.msg_pad - ACT_W);
                        let label = self.sized_text(i, room);
                        if let Some(v) = self.views[i].as_mut() {
                            v.label = label;
                        }
                    }
                    if self.cols[view.b].create_at == Some(i) {
                        if b_x > a_x {
                            hi += self.cols[view.b].size.w / 2.0;
                        } else {
                            lo -= self.cols[view.b].size.w / 2.0;
                        }
                    }
                    let (nx0, nx1) = (lo, hi);
                    x1 = nx1;
                    x0 = nx0;
                }
                self.extend(x0, x1);
                if let Some(&frame_index) = open.last() {
                    self.frames[frame_index].x0 =
                        js_min(self.frames[frame_index].x0, x0 - self.frame_pad);
                    self.frames[frame_index].x1 =
                        js_max(self.frames[frame_index].x1, x1 + self.frame_pad);
                }
            }
        }
    }
}

pub fn render_sequence(db: &SeqDb, config: &Config, options: &RenderOptions) -> Drawn {
    tighten(options, |tight| draw(db, config, options, tight, 0.0, true))
}

fn draw(
    db: &SeqDb,
    config: &Config,
    options: &RenderOptions,
    tight: bool,
    hold: f64,
    by_need: bool,
) -> Drawn {
    let conf = config.get("sequence");
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let tiny = js_round(size * 0.75);
    let monospaced = options
        .monospace
        .as_ref()
        .is_some_and(|list| list.iter().any(|item| item == "messages"));
    let base_style = if monospaced { MEASURE_MONO } else { 0 };
    let icons = options.icons.is_some();
    let pad = options.padding.unwrap_or(8.0);
    let mirror = conf_bool(conf, "mirrorActors").unwrap_or(true);
    let right_angles = conf_bool(conf, "rightAngles") == Some(true);
    let actor_margin = js_min(
        option(conf, "actorMargin", 40.0),
        if tight { 8.0 } else { f64::INFINITY },
    );
    let min_w = js_min(
        option(conf, "width", 64.0),
        if tight { 48.0 } else { f64::INFINITY },
    );
    let min_h = option(conf, "height", js_round(size * 1.5) + 20.0);
    let wrap_w = js_min(
        option(conf, "width", 160.0),
        if tight {
            js_max(
                32.0,
                (options.max_width.unwrap_or(320.0) - 2.0 * pad)
                    / js_max(1.0, db.actors.len() as f64)
                    - actor_margin
                    - 24.0,
            )
        } else {
            f64::INFINITY
        },
    );
    let msg_gap = js_max(4.0, 14.0 + option(conf, "messageMargin", 35.0) - 35.0);
    let note_pad = js_min(
        option(conf, "noteMargin", 10.0),
        if tight { 6.0 } else { f64::INFINITY },
    );
    let box_pad = js_min(
        option(conf, "boxMargin", 10.0),
        if tight { 6.0 } else { f64::INFINITY },
    );
    let align = match conf_string(conf, "messageAlign") {
        Some("left") => -1,
        Some("right") => 1,
        _ => 0,
    };
    let msg_pad = if tight { 6.0 } else { MSG_PAD };
    let note_over = if tight { 8.0 } else { NOTE_OVER };
    let frame_pad = if tight { 12.0 } else { FRAME_PAD };
    let frame_nest = if tight { 6.0 } else { 10.0 };
    let messages = db.messages.as_slice();
    let count = messages.len();
    let numbering = number_messages(
        messages,
        conf_bool(conf, "showSequenceNumbers") == Some(true),
    );

    let used: Option<HashSet<String>> = if conf_bool(conf, "hideUnusedParticipants") == Some(true) {
        let mut set = HashSet::new();
        for m in messages {
            if let Some(from) = &m.from {
                set.insert(from.clone());
            }
            if let Some(to) = &m.to {
                set.insert(to.clone());
            }
        }
        Some(set)
    } else {
        None
    };

    let mut next_line = vec![None::<usize>; count + 1];
    for i in (0..count).rev() {
        next_line[i] = if is_line_message(messages[i].type_) {
            Some(i)
        } else {
            next_line[i + 1]
        };
    }

    let mut ctx = Ctx {
        db,
        messages,
        small,
        tiny,
        tight,
        base_style,
        icons,
        wrap_w,
        msg_gap,
        note_pad,
        box_pad,
        align,
        msg_pad,
        note_over,
        frame_pad,
        frame_nest,
        numbering,
        cols: Vec::new(),
        col_of: HashMap::new(),
        needs: HashMap::new(),
        views: vec![None; count],
        sized: vec![Vec::new(); count],
        runs: Vec::new(),
        gaps: Vec::new(),
        frames: Vec::new(),
        frame_at: vec![None; count],
        min_x: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        inks: HashMap::new(),
        stack: Vec::new(),
        xs: Vec::new(),
    };

    for (id, actor) in db.actors.iter() {
        if let Some(used) = &used
            && !used.contains(id)
        {
            continue;
        }
        let kind = actor_kind(&actor.type_);
        let label = ctx.plain_text(Some(&actor.description), size, actor.wrap, wrap_w);
        let created = db.created_actors.get(id).copied();
        let destroyed = db.destroyed_actors.get(id).copied();
        let mut create_at =
            created.and_then(|c| next_line[js_min(c as f64, count as f64) as usize]);
        if let Some(at) = create_at
            && messages[at].to.as_deref() != Some(id)
        {
            create_at = None;
        }
        let mut destroy_at =
            destroyed.and_then(|d| next_line[js_min(d as f64, count as f64) as usize]);
        if let Some(at) = destroy_at
            && messages[at].to.as_deref() != Some(id)
            && messages[at].from.as_deref() != Some(id)
        {
            destroy_at = None;
        }
        let size_of = actor_size(kind, &label, min_w, min_h, if tight { 8.0 } else { 16.0 });
        ctx.col_of.insert(id.to_owned(), ctx.cols.len());
        ctx.cols.push(Column {
            id: id.to_owned(),
            actor: actor.clone(),
            kind,
            label,
            size: size_of,
            x: 0.0,
            create_at,
            destroy_at,
            top: 0.0,
            life_start: 0.0,
            life_end: None,
            activations: Vec::new(),
        });
    }
    let n_cols = ctx.cols.len();
    let last = ctx.last();

    let need = |ctx: &mut Ctx, lo: isize, hi: isize, width: f64| {
        if lo < 0 || hi > last {
            return;
        }
        let key = (lo as usize, hi as usize);
        let current = ctx.needs.get(&key).copied().unwrap_or(0.0);
        if current < width {
            ctx.needs.insert(key, width);
        }
    };

    for i in 0..count {
        let m = messages[i].clone();
        let a = m.from.as_ref().and_then(|f| ctx.col_of.get(f).copied());
        let b = m.to.as_ref().and_then(|t| ctx.col_of.get(t).copied());
        let (Some(a), Some(b)) = (a, b) else {
            continue;
        };
        let MessageBody::Text(raw) = &m.message else {
            continue;
        };
        if m.type_ == Some(linetype::NOTE) {
            let label = ctx.text(raw, small, m.wrap, wrap_w, 0);
            let w = label.width + 2.0 * note_pad;
            let (ai, bi) = (a as isize, b as isize);
            if m.placement == Some(placement::LEFTOF) {
                need(&mut ctx, ai - 1, ai, w + 2.0 * NOTE_GAP + ACT_W);
            } else if m.placement == Some(placement::RIGHTOF) {
                need(&mut ctx, ai, ai + 1, w + 2.0 * NOTE_GAP + ACT_W);
            } else if a == b {
                need(&mut ctx, ai - 1, ai, w / 2.0 + NOTE_GAP);
                need(&mut ctx, ai, ai + 1, w / 2.0 + NOTE_GAP);
            } else {
                need(
                    &mut ctx,
                    js_min(ai as f64, bi as f64) as isize,
                    js_max(ai as f64, bi as f64) as isize,
                    w - 2.0 * note_over,
                );
            }
            ctx.views[i] = Some(View {
                kind: ViewKind::Note,
                a,
                b,
                label,
                x: 0.0,
                w,
            });
        } else if is_line_message(m.type_) {
            let label = ctx.message_text(raw, m.wrap, wrap_w);
            let badge = ctx.badge_width(i);
            if a == b {
                need(
                    &mut ctx,
                    a as isize,
                    a as isize + 1,
                    SELF_W + 8.0 + label.width + 16.0 + ACT_W + badge,
                );
                ctx.views[i] = Some(View {
                    kind: ViewKind::SelfLoop,
                    a,
                    b,
                    label,
                    x: 0.0,
                    w: 0.0,
                });
            } else {
                let created = if ctx.cols[b].create_at == Some(i) {
                    ctx.cols[b].size.w / 2.0
                } else {
                    0.0
                };
                let central = if m.central_connection.unwrap_or(0) != 0 {
                    4.0 * CENTRAL_R
                } else {
                    0.0
                };
                let label_w = if tight {
                    js_min(label.width, js_max(16.0, wrap_w - 16.0))
                } else {
                    label.width
                };
                need(
                    &mut ctx,
                    js_min(a as f64, b as f64) as isize,
                    js_max(a as f64, b as f64) as isize,
                    label_w + 2.0 * msg_pad + ACT_W + badge + created + central,
                );
                ctx.views[i] = Some(View {
                    kind: ViewKind::Line,
                    a,
                    b,
                    label,
                    x: 0.0,
                    w: 0.0,
                });
            }
        }
    }

    for i in 0..n_cols {
        let box_index = ctx.cols[i].actor.box_index;
        let extends = match (box_index, ctx.runs.last()) {
            (Some(bi), Some(run)) => run.box_index == bi && run.last + 1 == i,
            _ => false,
        };
        if extends {
            if let Some(run) = ctx.runs.last_mut() {
                run.last = i;
            }
        } else if let Some(bi) = box_index {
            let bx = &db.boxes[bi];
            let title = ctx.plain_text(bx.name.as_deref(), small, bx.wrap, wrap_w);
            ctx.runs.push(BoxRun {
                box_index: bi,
                first: i,
                last: i,
                title,
                grow: 0.0,
            });
        }
    }

    ctx.gaps = vec![0.0; n_cols.saturating_sub(1)];
    for i in 0..n_cols.saturating_sub(1) {
        let left = &ctx.cols[i];
        let right = &ctx.cols[i + 1];
        let boxed = if left.actor.box_index != right.actor.box_index {
            (if left.actor.box_index.is_some() {
                box_pad
            } else {
                0.0
            }) + (if right.actor.box_index.is_some() {
                box_pad
            } else {
                0.0
            })
        } else {
            0.0
        };
        let needed = ctx.needs.get(&(i, i + 1)).copied().unwrap_or(0.0);
        ctx.gaps[i] = js_max(
            left.size.w / 2.0 + right.size.w / 2.0 + actor_margin + boxed,
            needed,
        );
    }
    let mut spans: Vec<(usize, usize, f64)> = ctx
        .needs
        .iter()
        .filter(|((lo, hi), _)| hi - lo > 1)
        .map(|((lo, hi), width)| (*lo, *hi, *width))
        .collect();
    spans.sort_by(|p, q| (p.1 - p.0).cmp(&(q.1 - q.0)).then(p.0.cmp(&q.0)));
    for (lo, hi, width) in spans {
        let total: f64 = ctx.gaps[lo..hi].iter().sum();
        if total >= width {
            continue;
        }
        let share = (width - total) / (hi - lo) as f64;
        for gap in &mut ctx.gaps[lo..hi] {
            *gap += share;
        }
    }
    ctx.place();
    for r in 0..ctx.runs.len() {
        let run = ctx.runs[r].clone();
        let first = ctx.cols[run.first].clone();
        let lastc = ctx.cols[run.last].clone();
        let inner = lastc.x + lastc.size.w / 2.0 - (first.x - first.size.w / 2.0);
        let grow = js_max(0.0, (run.title.width - inner) / 2.0);
        ctx.runs[r].grow = grow;
        if grow == 0.0 {
            continue;
        }
        if run.first > 0 {
            ctx.gaps[run.first - 1] += grow;
        }
        if (run.last as isize) < last {
            ctx.gaps[run.last] += grow;
        }
    }

    ctx.layout();

    let mut grown = false;
    let mut uneven = false;
    if tight && options.max_width.is_some() {
        let max_width = options.max_width.unwrap_or(0.0);
        let spare = max_width.floor() - (ctx.max_x - ctx.min_x + 2.0 * pad).ceil() - hold;
        let mut stretch = vec![0i64; n_cols];
        let mut want = vec![0.0f64; ctx.gaps.len()];
        let mut wrapped: Vec<usize> = Vec::new();
        let mut lines = 0usize;
        if spare > 0.0 {
            for i in 0..ctx.views.len() {
                let Some(view) = ctx.views[i].clone() else {
                    continue;
                };
                if view.kind != ViewKind::Line {
                    continue;
                }
                let raw = match &messages[i].message {
                    MessageBody::Text(text) => text.clone(),
                    MessageBody::Numbering(_) => String::new(),
                };
                let natural = ctx.message_text(&raw, false, f64::INFINITY);
                if view.label.lines.len() <= natural.lines.len() {
                    continue;
                }
                wrapped.push(i);
                lines += view.label.lines.len();
                let lo = js_min(view.a as f64, view.b as f64) as usize;
                let hi = js_max(view.a as f64, view.b as f64) as usize;
                stretch[lo] += 1;
                stretch[hi] -= 1;
                let short = (natural.width
                    - (ctx.cols[hi].x - ctx.cols[lo].x - 2.0 * msg_pad - ACT_W))
                    / (hi - lo) as f64;
                for slot in &mut want[lo..hi] {
                    *slot = js_max(*slot, short);
                }
            }
        }
        let mut active = 0i64;
        let mut crossed = 0i64;
        for slot in stretch.iter_mut().take(ctx.gaps.len()) {
            active += *slot;
            *slot = if active > 0 { 1 } else { 0 };
            crossed += *slot;
        }
        if crossed > 0 {
            let before = ctx.gaps.clone();
            let spread = |ctx: &mut Ctx, share: &dyn Fn(usize) -> f64| -> Vec<usize> {
                for g in 0..ctx.gaps.len() {
                    ctx.gaps[g] = before[g] + share(g);
                }
                ctx.layout();
                wrapped
                    .iter()
                    .map(|&i| ctx.views[i].as_ref().map_or(0, |v| v.label.lines.len()))
                    .collect()
            };
            let even = |g: usize| (stretch[g] as f64 * spare) / crossed as f64;
            let sum = |counts: &[usize]| counts.iter().sum::<usize>();
            let mut counts = spread(&mut ctx, &even);
            let missing: f64 = want.iter().sum();
            let needs_fn = |g: usize| (want[g] * (spare - 0.01)) / missing;
            if by_need
                && crossed > 1
                && missing > 0.0
                && (0..ctx.gaps.len()).any(|g| needs_fn(g) > even(g))
            {
                let first = counts.clone();
                counts = spread(&mut ctx, &needs_fn);
                uneven = sum(&counts) < sum(&first)
                    && counts.iter().enumerate().all(|(k, n)| *n <= first[k]);
                if !uneven {
                    counts = spread(&mut ctx, &even);
                }
            }
            grown = sum(&counts) < lines;
            if !grown {
                ctx.gaps = before.clone();
                ctx.layout();
            }
        }
    }

    let mut box_title_h = 0.0f64;
    for run in &ctx.runs {
        box_title_h = js_max(box_title_h, run.title.height);
    }
    let box_top = if ctx.runs.is_empty() {
        0.0
    } else {
        ctx.box_pad
            + if box_title_h > 0.0 {
                box_title_h + 4.0
            } else {
                0.0
            }
    };
    let mut row_h = 0.0f64;
    for col in &ctx.cols {
        if col.create_at.is_none() {
            row_h = js_max(row_h, col.size.h);
        }
    }
    let head_bottom = box_top + row_h;
    for col in ctx.cols.iter_mut() {
        col.top = head_bottom - col.size.h;
        col.life_start = head_bottom;
    }

    ctx.xs = ctx.cols.iter().map(|c| c.x).collect();

    let mut rects_out: Vec<String> = Vec::new();
    let mut activations_out: Vec<String> = Vec::new();
    let mut frames_out = String::new();
    let mut edges_out = String::new();
    let mut labels_out = String::new();
    let mut notes_out = String::new();
    let mut actors_out = String::new();
    let mut crosses_out = String::new();

    let mut y = head_bottom + 6.0;

    for i in 0..count {
        let m = messages[i].clone();
        let view = ctx.views[i].clone();
        let frame = ctx.frame_at[i];
        let type_ = m.type_;
        if type_ == Some(linetype::ACTIVE_START) {
            if let Some(col) = m.from.as_ref().and_then(|f| ctx.col_of.get(f).copied()) {
                ctx.cols[col].activations.push(y);
            }
        } else if type_ == Some(linetype::ACTIVE_END) {
            if let Some(col) = m.from.as_ref().and_then(|f| ctx.col_of.get(f).copied())
                && !ctx.cols[col].activations.is_empty()
            {
                let bottom = end_activation(&mut ctx, col, y, &mut activations_out);
                y = js_max(y, bottom);
            }
        } else if let Some(fi) = frame.filter(|&fi| ctx.frames[fi].dividers[0] == i) {
            ctx.stack.push(fi);
            if ctx.frames[fi].kind == "rect" {
                ctx.frames[fi].top = y + 8.0;
                y = ctx.frames[fi].top;
            } else {
                ctx.frames[fi].top = y + 12.0;
                y = ctx.frames[fi].top + js_max(TAB_H, ctx.frames[fi].title.height + 2.0);
            }
        } else if let Some(fi) = frame {
            ctx.stack.pop();
            let bottom = y + 12.0;
            let frame_x = num(ctx.frames[fi].x0);
            let frame_w = num(ctx.frames[fi].x1 - ctx.frames[fi].x0);
            if ctx.frames[fi].kind == "rect" {
                let fill = if ctx.frames[fi].fill.is_empty() {
                    String::new()
                } else {
                    resolve_style(&[format!("fill:{}", ctx.frames[fi].fill)]).shape
                };
                let attrs = format!(
                    "{}{}",
                    if fill.is_empty() {
                        " fill-opacity=\"0.5\""
                    } else {
                        ""
                    },
                    fill
                );
                rects_out.push(format!(
                    "<rect class=\"pelers-rect\" x=\"{frame_x}\" y=\"{}\" width=\"{frame_w}\" height=\"{}\"{}/>",
                    num(ctx.frames[fi].top),
                    num(bottom - ctx.frames[fi].top),
                    rounded(&attrs)
                ));
            } else {
                let mut inner = format!(
                    "<rect x=\"{frame_x}\" y=\"{}\" width=\"{frame_w}\" height=\"{}\" fill=\"none\"{ROUNDED}/>",
                    num(ctx.frames[fi].top),
                    num(bottom - ctx.frames[fi].top)
                );
                let dividers = ctx.frames[fi].divider_y.clone();
                let labels = ctx.frames[fi].divider_labels.clone();
                for (k, dy) in dividers.iter().enumerate() {
                    let label = labels.get(k).cloned().unwrap_or_else(no_label);
                    let fx0 = ctx.frames[fi].x0;
                    let fx1 = ctx.frames[fi].x1;
                    inner.push_str(&format!(
                        "<path d=\"M{frame_x},{}H{}\"{DASH}/>",
                        num(*dy),
                        num(fx1)
                    ));
                    inner.push_str(&ctx.backed(
                        &label,
                        fx0 + 8.0 + label.width / 2.0,
                        dy + 2.0 + label.height / 2.0,
                        true,
                    ));
                }
                let kind = ctx.frames[fi].kind;
                let tab_w = ctx.frames[fi].tab_w;
                let fx0 = ctx.frames[fi].x0;
                let top = ctx.frames[fi].top;
                inner.push_str(&format!(
                    "<rect x=\"{frame_x}\" y=\"{}\" width=\"{}\" height=\"{TAB_H}\" fill=\"var(--_s)\"{ROUNDED}/>",
                    num(top),
                    num(tab_w)
                ));
                inner.push_str(&format!(
                    "<text class=\"pelers-frame-kind\" x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" font-weight=\"var(--_hw)\" fill=\"var(--_m)\" stroke=\"none\">{kind}</text>",
                    num(fx0 + tab_w / 2.0),
                    num(top + TAB_H / 2.0 + tiny * 0.35),
                    js_number_to_string(tiny)
                ));
                let title = ctx.frames[fi].title.clone();
                inner.push_str(&ctx.backed(
                    &title,
                    fx0 + tab_w + 8.0 + title.width / 2.0,
                    top + js_max(TAB_H, title.height) / 2.0,
                    true,
                ));
                frames_out.push_str(&format!(
                    "<g class=\"pelers-frame pelers-frame-{kind}\">{inner}</g>"
                ));
            }
            y = bottom;
        } else if matches!(
            type_,
            Some(linetype::ALT_ELSE | linetype::PAR_AND | linetype::CRITICAL_OPTION)
        ) {
            if let Some(&current) = ctx.stack.last() {
                let index = ctx.frames[current].divider_y.len();
                let label = ctx.frames[current]
                    .divider_labels
                    .get(index)
                    .cloned()
                    .unwrap_or_else(no_label);
                ctx.frames[current].divider_y.push(y + 12.0);
                y += 12.0
                    + if label.height > 0.0 {
                        label.height
                    } else {
                        0.0
                    };
            }
        } else if let Some(view) = view.clone().filter(|v| v.kind == ViewKind::Note) {
            let a_index = view.a;
            let h = view.label.height + 2.0 * js_max(4.0, ctx.note_pad - 4.0);
            let top = y + 10.0;
            let mut x = view.x;
            if m.placement == Some(placement::RIGHTOF) {
                x += right_edge(&ctx.cols[a_index], 0) - ctx.cols[a_index].x;
            } else if m.placement == Some(placement::LEFTOF) {
                x += left_edge(&ctx.cols[a_index], 0) - ctx.cols[a_index].x;
            }
            ctx.extend(x, x + view.w);
            notes_out.push_str(&format!(
                "<g class=\"pelers-note\" data-id=\"i{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{ROUNDED}/>{}</g>",
                esc_text(&m.id),
                num(x),
                num(top),
                num(view.w),
                num(h),
                label_svg_with(
                    &view.label,
                    x + view.w / 2.0,
                    top + h / 2.0,
                    " class=\"pelers-label\" stroke=\"none\" fill=\"var(--_fg)\"",
                    icons
                )
            ));
            y = top + h;
        } else if let Some(view) = view {
            let st = line_style(type_.unwrap_or(0)).unwrap_or(LineStyle {
                dotted: false,
                head: Head::None,
                at_start: false,
                at_end: false,
                bottom: false,
            });
            let a = ctx.cols[view.a].clone();
            let b = ctx.cols[view.b].clone();
            let label = view.label.clone();
            let bw = ctx.badge_width(i);
            let mut starting = 0usize;
            for next in &messages[i + 1..] {
                let t = next.type_;
                if t == Some(linetype::ACTIVE_START) {
                    if next.from == m.to {
                        starting += 1;
                    }
                } else if t != Some(linetype::ACTIVE_END)
                    && t != Some(linetype::CENTRAL_CONNECTION)
                    && t != Some(linetype::CENTRAL_CONNECTION_REVERSE)
                {
                    break;
                }
            }
            let up = if st.bottom { 1.0 } else { -1.0 };
            let attrs = if st.dotted { DASH } else { "" };
            let cc = m.central_connection.unwrap_or(0);
            let circle_to =
                cc == linetype::CENTRAL_CONNECTION || cc == linetype::CENTRAL_CONNECTION_DUAL;
            let circle_from = cc == linetype::CENTRAL_CONNECTION_REVERSE
                || cc == linetype::CENTRAL_CONNECTION_DUAL;
            let tail_at_end = st.at_start && !st.at_end;
            let trim_len = head_trim(st.head);
            let mut inner = String::new();
            let mut color = "var(--_l)".to_owned();
            let line: Option<String>;
            let line_y = if view.kind == ViewKind::SelfLoop {
                let top = y + ctx.msg_gap + js_max(0.0, (label.height - SELF_H) / 2.0);
                let bottom = top + SELF_H;
                let mut x0 = if circle_from {
                    a.x + CENTRAL_R
                } else {
                    right_edge(&a, 0)
                };
                let mut x1 = if circle_to {
                    a.x + CENTRAL_R
                } else {
                    right_edge(&a, starting)
                };
                let out = js_max(x0, x1) + SELF_W + bw / 2.0;
                line = ctx.ink_of(&ctx.fill_under(a.x, out).unwrap_or_default());
                if let Some(color_ink) = &line {
                    color = color_ink.clone();
                }
                if circle_from {
                    inner.push_str(&central_circle(a.x, top, &color));
                }
                if circle_to {
                    inner.push_str(&central_circle(a.x, bottom, &color));
                }
                if bw > 0.0 {
                    let (bx, by) = if tail_at_end { (x1, bottom) } else { (x0, top) };
                    inner.push_str(&badge(&ctx, i, bx, by, bw));
                    if tail_at_end {
                        x1 += bw / 2.0;
                    } else {
                        x0 += bw / 2.0;
                    }
                }
                let heads = (if st.at_start {
                    head(st.head, x0, top, -1.0, up, &color)
                } else {
                    String::new()
                }) + &(if st.at_end {
                    head(st.head, x1, bottom, -1.0, -up, &color)
                } else {
                    String::new()
                });
                let from = num(x0 + if st.at_start { trim_len } else { 0.0 });
                let to = num(x1 + if st.at_end { trim_len } else { 0.0 });
                let d = if right_angles {
                    format!("M{from},{}H{}V{}H{to}", num(top), num(out), num(bottom))
                } else {
                    format!(
                        "M{from},{}H{}A6,6 0 0 1 {},{}V{}A6,6 0 0 1 {},{}H{to}",
                        num(top),
                        num(out - 6.0),
                        num(out),
                        num(top + 6.0),
                        num(bottom - 6.0),
                        num(out - 6.0),
                        num(bottom)
                    )
                };
                inner = format!("<path d=\"{d}\"{attrs}/>{heads}{inner}");
                ctx.extend(a.x, out + 8.0 + label.width);
                if label.width > 0.0 {
                    let fill_attr = {
                        let fill = ctx
                            .fill_under(out + 8.0, out + 8.0 + label.width)
                            .unwrap_or_default();
                        ctx.ink_on(&fill)
                    };
                    labels_out.push_str(&format!(
                        "<g class=\"pelers-edge-label\" data-id=\"i{}\">{}</g>",
                        esc_text(&m.id),
                        label_svg_with(
                            &label,
                            out + 8.0 + label.width / 2.0,
                            (top + bottom) / 2.0,
                            &fill_attr,
                            icons
                        )
                    ));
                }
                js_max(bottom, (top + bottom) / 2.0 + label.height / 2.0)
            } else {
                let dir = if b.x > a.x { 1.0 } else { -1.0 };
                let creating = ctx.cols[view.b].create_at == Some(i);
                line = ctx.ink_of(
                    &ctx.fill_under(js_min(a.x, b.x), js_max(a.x, b.x))
                        .unwrap_or_default(),
                );
                if let Some(color_ink) = &line {
                    color = color_ink.clone();
                }
                let ly = y
                    + ctx.msg_gap
                    + js_max(
                        if label.height > 0.0 {
                            label.height + 2.0
                        } else {
                            6.0
                        },
                        if creating {
                            b.size.h / 2.0 - ctx.msg_gap + 6.0
                        } else {
                            0.0
                        },
                    );
                let mut x0 = if dir > 0.0 {
                    right_edge(&a, 0)
                } else {
                    left_edge(&a, 0)
                };
                let mut x1 = if dir > 0.0 {
                    left_edge(&b, starting)
                } else {
                    right_edge(&b, starting)
                };
                if creating {
                    let bi = view.b;
                    ctx.cols[bi].top = ly - ctx.cols[bi].size.h / 2.0;
                    ctx.cols[bi].life_start = ctx.cols[bi].top + ctx.cols[bi].size.h;
                    x1 = ctx.cols[bi].x
                        - dir
                            * (if is_figure(ctx.cols[bi].kind) {
                                18.0
                            } else {
                                ctx.cols[bi].size.w / 2.0 + 1.0
                            });
                }
                if circle_from {
                    inner.push_str(&central_circle(a.x, ly, &color));
                    x0 = a.x + dir * CENTRAL_R;
                }
                if circle_to {
                    inner.push_str(&central_circle(b.x, ly, &color));
                    x1 = b.x - dir * CENTRAL_R;
                }
                if bw > 0.0 {
                    if tail_at_end {
                        let cx = if circle_to {
                            x1 - dir * (bw / 2.0 + 2.0)
                        } else {
                            x1
                        };
                        inner.push_str(&badge(&ctx, i, cx, ly, bw));
                        x1 = cx - (dir * bw) / 2.0;
                    } else {
                        let cx = if circle_from {
                            x0 + dir * (bw / 2.0 + 2.0)
                        } else {
                            x0
                        };
                        inner.push_str(&badge(&ctx, i, cx, ly, bw));
                        x0 = cx + (dir * bw) / 2.0;
                    }
                }
                let heads = (if st.at_start {
                    head(st.head, x0, ly, -dir, up * dir, &color)
                } else {
                    String::new()
                }) + &(if st.at_end {
                    head(st.head, x1, ly, dir, up * dir, &color)
                } else {
                    String::new()
                });
                let from = x0 + if st.at_start { dir * trim_len } else { 0.0 };
                let to = x1 - if st.at_end { dir * trim_len } else { 0.0 };
                inner = format!(
                    "<path d=\"M{},{}H{}\"{attrs}/>{heads}{inner}",
                    num(from),
                    num(ly),
                    num(to)
                );
                if label.width > 0.0 {
                    let l = js_min(x0, x1);
                    let r = js_max(x0, x1);
                    let cx = if ctx.align < 0 {
                        l + ctx.msg_pad + label.width / 2.0
                    } else if ctx.align > 0 {
                        r - ctx.msg_pad - label.width / 2.0
                    } else {
                        (l + r) / 2.0
                    };
                    ctx.extend(cx - label.width / 2.0, cx + label.width / 2.0);
                    let cy = ly - 4.0 - label.height / 2.0;
                    let back = ctx.backed(&label, cx, cy, false);
                    labels_out.push_str(&format!(
                        "<g class=\"pelers-edge-label\" data-id=\"i{}\">{back}</g>",
                        esc_text(&m.id)
                    ));
                }
                if creating {
                    y = ly + ctx.cols[view.b].size.h / 2.0 - 6.0;
                }
                ly
            };
            edges_out.push_str(&format!(
                "<g class=\"pelers-edge pelers-message\" data-id=\"i{}\"{}>{inner}</g>",
                esc_text(&m.id),
                match &line {
                    Some(l) => format!(" stroke=\"{l}\""),
                    None => String::new(),
                }
            ));
            y = js_max(y, line_y);
            if ctx.cols[view.a].destroy_at == Some(i) {
                let d = destroy(&mut ctx, view.a, line_y, &mut crosses_out);
                y = js_max(y, d);
            }
            if ctx.cols[view.b].destroy_at == Some(i) && view.b != view.a {
                let d = destroy(&mut ctx, view.b, line_y, &mut crosses_out);
                y = js_max(y, d);
            }
        }
    }

    y += 20.0;
    for col in 0..ctx.cols.len() {
        while !ctx.cols[col].activations.is_empty() {
            end_activation(&mut ctx, col, y, &mut activations_out);
        }
    }
    let mut bottom = y;
    if mirror {
        let mut foot_h = 0.0f64;
        for col in ctx.cols.clone() {
            if col.life_end.is_some() {
                continue;
            }
            foot_h = js_max(foot_h, col.size.h);
            actors_out.push_str(&actor_svg(&mut ctx, &col, y, true));
        }
        bottom = y + foot_h;
    }
    let mut lifelines = String::new();
    for col in ctx.cols.clone() {
        actors_out.push_str(&actor_svg(&mut ctx, &col, col.top, false));
        let end = col.life_end.unwrap_or(y);
        if end > col.life_start {
            lifelines.push_str(&format!(
                "<path class=\"pelers-lifeline\" data-id=\"{}\" d=\"M{},{}V{}\"/>",
                esc_text(&col.id),
                num(col.x),
                num(col.life_start),
                num(end)
            ));
        }
    }

    let mut boxes_out = String::new();
    for run in ctx.runs.clone() {
        let (x0, x1) = ctx.box_edges(&run);
        let bx = db.boxes[run.box_index].clone();
        let fill = if bx.fill == "transparent" {
            String::new()
        } else {
            resolve_style(&[format!("fill:{}", bx.fill)]).shape
        };
        let attrs = format!(
            "{}{}",
            if fill.is_empty() {
                " fill-opacity=\"0.5\""
            } else {
                ""
            },
            fill
        );
        let name_attr = match &bx.name {
            Some(name) => format!(" data-id=\"{}\"", esc_text(name)),
            None => String::new(),
        };
        let fill_attr = {
            let color = ctx.ink_on(&bx.fill);
            if color.is_empty() {
                " fill=\"var(--_m)\"".to_owned()
            } else {
                color
            }
        };
        boxes_out.push_str(&format!(
            "<g class=\"pelers-cluster pelers-box\"{name_attr}><rect x=\"{}\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\" stroke=\"var(--_b)\"{}/>{}</g>",
            num(x0),
            num(x1 - x0),
            num(bottom + ctx.box_pad),
            rounded(&attrs),
            label_svg_with(
                &run.title,
                (x0 + x1) / 2.0,
                ctx.box_pad / 2.0 + 2.0 + run.title.height / 2.0,
                &format!(" class=\"pelers-cluster-label\"{fill_attr}"),
                icons
            )
        ));
    }
    if !ctx.runs.is_empty() {
        bottom += ctx.box_pad;
    }

    let mut min_y = 0.0f64;
    let mut title_out = String::new();
    let room = title_room(options);
    let plain_title = title_label(
        db.title.as_deref(),
        size,
        if tight { room } else { f64::INFINITY },
    );
    let title = if tight {
        break_label(&plain_title, js_max(120.0, room))
    } else {
        plain_title
    };
    if ctx.min_x > ctx.max_x {
        let (x0, x1) = (0.0, js_max(title.width, 40.0));
        ctx.extend(x0, x1);
    }
    if title.width > 0.0 {
        let cx = (ctx.min_x + ctx.max_x) / 2.0;
        ctx.extend(cx - title.width / 2.0, cx + title.width / 2.0);
        min_y = -title.height - 12.0;
        title_out = label_svg_with(
            &title,
            cx,
            min_y + title.height / 2.0,
            " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
            icons,
        );
    }

    let width = (ctx.max_x - ctx.min_x + 2.0 * pad).ceil();
    if grown && options.max_width.is_some_and(|max_width| width > max_width) {
        let max_width = options.max_width.unwrap_or(0.0);
        let next_hold = if uneven {
            hold
        } else if hold > 0.0 {
            f64::INFINITY
        } else {
            width - max_width.floor()
        };
        return draw(db, config, options, tight, next_hold, false);
    }
    let height = (bottom - min_y + 2.0 * pad).ceil();

    let mut content = format!(
        "<g transform=\"translate({},{})\">",
        num(pad - ctx.min_x),
        num(pad - min_y)
    );
    content.push_str(&title_out);
    if !boxes_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-boxes\" font-size=\"{}\">{boxes_out}</g>",
            js_number_to_string(small)
        ));
    }
    if !rects_out.is_empty() {
        rects_out.reverse();
        content.push_str(&format!(
            "<g class=\"pelers-rects\" fill=\"var(--_a)\">{}</g>",
            rects_out.concat()
        ));
    }
    if !lifelines.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-lifelines\" fill=\"none\" stroke=\"var(--_b)\">{lifelines}</g>"
        ));
    }
    if !activations_out.is_empty() {
        activations_out.reverse();
        content.push_str(&format!(
            "<g class=\"pelers-activations\" fill=\"var(--_s)\" stroke=\"var(--_b)\">{}</g>",
            activations_out.concat()
        ));
    }
    if !frames_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-frames\" font-size=\"{}\" stroke=\"var(--_b)\">{frames_out}</g>",
            js_number_to_string(small)
        ));
    }
    if !edges_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{edges_out}</g>"
        ));
    }
    if !crosses_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-destroys\" fill=\"none\" stroke=\"var(--_l)\" stroke-width=\"1.5\" stroke-linecap=\"round\">{crosses_out}</g>"
        ));
    }
    if !labels_out.is_empty() {
        let mono_attr = if monospaced {
            format!(" font-family=\"{FONT_MONO}\"")
        } else {
            String::new()
        };
        content.push_str(&format!(
            "<g class=\"pelers-edge-labels\" font-size=\"{}\"{mono_attr}>{labels_out}</g>",
            js_number_to_string(small)
        ));
    }
    if !notes_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-notes\" font-size=\"{}\" fill=\"var(--_a)\" stroke=\"var(--_b)\">{notes_out}</g>",
            js_number_to_string(small)
        ));
    }
    if !actors_out.is_empty() {
        content.push_str(&format!("<g class=\"pelers-nodes\">{actors_out}</g>"));
    }
    content.push_str("</g>");

    let svg = svg_document(
        "sequence",
        width,
        height,
        size,
        options,
        Accessible {
            acc_title: db.acc_title.as_deref(),
            acc_descr: db.acc_descr.as_deref(),
        },
        &content,
    );
    Drawn {
        svg,
        width,
        height,
        links: Vec::new(),
    }
}

fn left_edge(col: &Column, extra: usize) -> f64 {
    if !col.activations.is_empty() || extra > 0 {
        col.x - ACT_W / 2.0
    } else {
        col.x
    }
}

fn right_edge(col: &Column, extra: usize) -> f64 {
    let depth = col.activations.len() + extra;
    if depth > 0 {
        col.x + ACT_W / 2.0 + (depth as f64 - 1.0) * ACT_STEP
    } else {
        col.x
    }
}

fn end_activation(ctx: &mut Ctx, col: usize, end: f64, out: &mut Vec<String>) -> f64 {
    let start = ctx.cols[col].activations.pop().unwrap_or(end);
    let bottom = js_max(end, start + ACT_MIN);
    let x = ctx.cols[col].x - ACT_W / 2.0 + ctx.cols[col].activations.len() as f64 * ACT_STEP;
    ctx.extend(x, x + ACT_W);
    out.push(format!(
        "<rect class=\"pelers-activation\" data-id=\"{}\" x=\"{}\" y=\"{}\" width=\"{ACT_W}\" height=\"{}\" rx=\"2\"/>",
        esc_text(&ctx.cols[col].id),
        num(x),
        num(start),
        num(bottom - start)
    ));
    bottom
}

fn destroy(ctx: &mut Ctx, col: usize, line_y: f64, out: &mut String) -> f64 {
    let cx = ctx.cols[col].x;
    let cy = line_y + 16.0;
    ctx.cols[col].life_end = Some(cy);
    out.push_str(&format!(
        "<path class=\"pelers-destroy\" data-id=\"{}\" d=\"M{},{}L{},{}M{},{}L{},{}\"/>",
        esc_text(&ctx.cols[col].id),
        num(cx - 6.0),
        num(cy - 6.0),
        num(cx + 6.0),
        num(cy + 6.0),
        num(cx + 6.0),
        num(cy - 6.0),
        num(cx - 6.0),
        num(cy + 6.0)
    ));
    cy + 6.0
}

fn central_circle(cx: f64, cy: f64, color: &str) -> String {
    format!(
        "<circle class=\"pelers-central\" cx=\"{}\" cy=\"{}\" r=\"{CENTRAL_R}\" fill=\"{color}\" stroke=\"none\"/>",
        num(cx),
        num(cy)
    )
}

fn badge(ctx: &Ctx, i: usize, cx: f64, cy: f64, w: f64) -> String {
    format!(
        "<g class=\"pelers-sequence-number\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{BADGE_H}\" rx=\"{}\" fill=\"var(--_l)\" stroke=\"none\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\" fill=\"var(--_bg)\" stroke=\"none\">{}</text></g>",
        num(cx - w / 2.0),
        num(cy - BADGE_H / 2.0),
        num(w),
        BADGE_H / 2.0,
        num(cx),
        num(cy + ctx.tiny * 0.35),
        js_number_to_string(ctx.tiny),
        esc(&js_number_to_string(ctx.numbering.index[i]))
    )
}

fn actor_svg(ctx: &mut Ctx, col: &Column, top: f64, footer: bool) -> String {
    let cls = col.actor.properties.get("class");
    let class_attr = match cls {
        Some(Value::String(text)) => class_names(text),
        _ => String::new(),
    };
    let ink_attr = if is_figure(col.kind) {
        let fill = col
            .actor
            .box_index
            .map(|bi| ctx.db.boxes[bi].fill.clone())
            .unwrap_or_default();
        ctx.ink_on(&fill)
    } else {
        String::new()
    };
    format!(
        "<g class=\"pelers-node pelers-actor pelers-actor-{}{}{}\" data-id=\"{}\">{}</g>",
        col.kind,
        if footer { " pelers-actor-footer" } else { "" },
        class_attr,
        esc_text(&col.id),
        draw_actor(
            col.kind, col.x, top, col.size, &col.label, ctx.icons, &ink_attr
        )
    )
}
