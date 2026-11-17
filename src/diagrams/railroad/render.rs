use super::types::{RailroadModel, RailroadNode};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{tighten, title_room};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::ROUNDED;
use crate::text::entities::decode_entities;
use crate::text::label::layout_label;
use crate::text::measurer::{BOLD_STYLE, ITALIC, MONO, width as measure};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::Value;

const FONT_MONO: &str = "var(--_fm)";
const LEAD: f64 = 12.0;
const HEAD_GAP: f64 = 6.0;
const RULE_GAP: f64 = 18.0;
const COUNT_SIZE: f64 = 12.0;
const ARROW: f64 = 7.0;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Terminal,
    NonTerminal,
    Special,
    Sequence,
    Choice,
    Bypass,
    Loop,
    Folded,
}

#[derive(Clone)]
struct BoxN {
    kind: Kind,
    width: f64,
    up: f64,
    down: f64,
    kids: Vec<BoxN>,
    text: String,
    id: String,
    reach: f64,
}

#[derive(Clone, Copy)]
struct Metrics {
    size: f64,
    padding: f64,
    height: f64,
    gap: f64,
    sep: f64,
    arc: f64,
}

fn option(config: &Config, key: &str, fallback: f64, max: f64) -> f64 {
    match config
        .get("railroad")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::Number(value)) if *value >= 0.0 => js_min(*value, max),
        _ => fallback,
    }
}

fn shown(text: &str) -> String {
    let s = if text.contains("¶ß") {
        decode_entities(&text.replace('&', "&amp;"))
    } else {
        text.to_owned()
    };
    s.chars()
        .map(|c| match c {
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

fn reverse<T>(list: &mut [T], from: usize) {
    let mut i = from;
    let mut j = list.len() as isize - 1;
    while (i as isize) < j {
        list.swap(i, j as usize);
        i += 1;
        j -= 1;
    }
}

fn even(value: f64) -> f64 {
    (value / 2.0).ceil() * 2.0
}

fn leaf(kind: Kind, text: String, id: &str, style: u32, m: &Metrics) -> BoxN {
    let half = m.height / 2.0;
    let width = js_max(
        even(measure(&text, m.size, style) + 2.0 * m.padding),
        m.height,
    );
    BoxN {
        kind,
        width,
        up: half,
        down: half,
        kids: Vec::new(),
        text,
        id: id.to_owned(),
        reach: 0.0,
    }
}

fn count(min: f64, max: f64) -> String {
    if max.is_infinite() && min <= 1.0 {
        return String::new();
    }
    let text = |value: f64| -> String {
        if value.is_finite() {
            js_number_to_string(value)
        } else {
            "∞".to_owned()
        }
    };
    if min == max {
        return format!("{}×", text(min));
    }
    if max.is_infinite() {
        return format!("≥{}×", text(min));
    }
    if min == 0.0 {
        return format!("≤{}×", text(max));
    }
    format!("{}–{}×", text(min), text(max))
}

fn below(offset: f64, above: &BoxN, branch: &BoxN, m: &Metrics) -> f64 {
    js_max(offset + above.down + m.sep + branch.up, 2.0 * m.arc)
}

fn bypass(kid: BoxN, m: &Metrics) -> BoxN {
    let reach = js_max(kid.up + m.sep, 2.0 * m.arc);
    let width = kid.width + 4.0 * m.arc;
    let down = kid.down;
    BoxN {
        kind: Kind::Bypass,
        width,
        up: reach,
        down,
        kids: vec![kid],
        text: String::new(),
        id: String::new(),
        reach,
    }
}

fn around(node: &RailroadNode, m: &Metrics) -> f64 {
    match node {
        RailroadNode::Choice(_) | RailroadNode::Optional(_) => 4.0 * m.arc,
        RailroadNode::Repetition { min, .. } => (if *min == 0.0 { 6.0 } else { 2.0 }) * m.arc,
        RailroadNode::Sequence(_) => 6.0 * m.arc,
        _ => 0.0,
    }
}

fn measure_node(node: &RailroadNode, m: &Metrics, room: f64) -> BoxN {
    let child_room = if room.is_finite() {
        js_max(40.0, room - around(node, m))
    } else {
        f64::INFINITY
    };
    match node {
        RailroadNode::Terminal(value) => {
            let text = shown(value);
            let display = if !text.is_empty() && text.chars().all(|c| c == ' ') {
                "␣".repeat(text.chars().count())
            } else {
                text
            };
            leaf(Kind::Terminal, display, "", MONO, m)
        }
        RailroadNode::Nonterminal(name) => leaf(Kind::NonTerminal, shown(name), name, 0, m),
        RailroadNode::Special(text) => {
            leaf(Kind::Special, format!("? {} ?", shown(text)), "", ITALIC, m)
        }
        RailroadNode::Sequence(elements) => {
            let kids: Vec<BoxN> = elements
                .iter()
                .map(|e| measure_node(e, m, child_room))
                .collect();
            let mut width = (kids.len().saturating_sub(1)) as f64 * m.gap;
            let mut up = 0.0f64;
            let mut down = 0.0f64;
            for kid in &kids {
                width += kid.width;
                up = js_max(up, kid.up);
                down = js_max(down, kid.down);
            }
            let plain = BoxN {
                kind: Kind::Sequence,
                width,
                up,
                down,
                kids: kids.clone(),
                text: String::new(),
                id: String::new(),
                reach: 0.0,
            };
            if width <= room || kids.len() < 2 {
                return plain;
            }
            let mut rows: Vec<BoxN> = Vec::new();
            let mut row: Vec<BoxN> = Vec::new();
            let mut row_w = 0.0f64;
            let flush = |row: &mut Vec<BoxN>, row_w: &mut f64, rows: &mut Vec<BoxN>| {
                let mut up = 0.0f64;
                let mut down = 0.0f64;
                for kid in row.iter() {
                    up = js_max(up, kid.up);
                    down = js_max(down, kid.down);
                }
                rows.push(BoxN {
                    kind: Kind::Sequence,
                    width: *row_w,
                    up,
                    down,
                    kids: std::mem::take(row),
                    text: String::new(),
                    id: String::new(),
                    reach: 0.0,
                });
                *row_w = 0.0;
            };
            for kid in kids {
                let limit = js_max(40.0, room - 6.0 * m.arc);
                if !row.is_empty() && row_w + m.gap + kid.width > limit {
                    flush(&mut row, &mut row_w, &mut rows);
                }
                row_w += (if row.is_empty() { 0.0 } else { m.gap }) + kid.width;
                row.push(kid);
            }
            flush(&mut row, &mut row_w, &mut rows);
            if rows.len() < 2 {
                return plain;
            }
            let mut folded_width = 0.0f64;
            let mut offset = 0.0f64;
            for i in 0..rows.len() {
                folded_width = js_max(folded_width, rows[i].width);
                if i > 0 {
                    offset += rows[i - 1].down + js_max(m.sep, 2.0 * m.arc) + rows[i].up;
                }
            }
            let up = rows[0].up;
            let down = offset + rows[rows.len() - 1].down;
            BoxN {
                kind: Kind::Folded,
                width: folded_width + 6.0 * m.arc,
                up,
                down,
                kids: rows,
                text: String::new(),
                id: String::new(),
                reach: 0.0,
            }
        }
        RailroadNode::Choice(alternatives) => {
            let kids: Vec<BoxN> = alternatives
                .iter()
                .map(|a| measure_node(a, m, child_room))
                .collect();
            let mut inner = 0.0f64;
            let mut offset = 0.0f64;
            for i in 0..kids.len() {
                inner = js_max(inner, kids[i].width);
                if i > 0 {
                    offset = below(offset, &kids[i - 1], &kids[i], m);
                }
            }
            let up = if kids.is_empty() { 0.0 } else { kids[0].up };
            let down = match kids.last() {
                Some(last) => offset + last.down,
                None => 0.0,
            };
            BoxN {
                kind: Kind::Choice,
                width: inner + 4.0 * m.arc,
                up,
                down,
                kids,
                text: String::new(),
                id: String::new(),
                reach: 0.0,
            }
        }
        RailroadNode::Optional(element) => {
            let kid = measure_node(element, m, child_room);
            bypass(kid, m)
        }
        RailroadNode::Repetition { element, min, max } => {
            let kid = measure_node(element, m, child_room);
            if *min == 1.0 && *max == 1.0 {
                return kid;
            }
            let text = count(*min, *max);
            let reach = js_max(kid.down + m.sep, 2.0 * m.arc);
            let label = if text.is_empty() {
                0.0
            } else {
                even(measure(&text, COUNT_SIZE, 0))
            };
            let width = js_max(kid.width, label) + 2.0 * m.arc;
            let up = kid.up;
            let down = reach
                + if text.is_empty() {
                    ARROW / 2.0 + 1.0
                } else {
                    COUNT_SIZE + 6.0
                };
            let loop_box = BoxN {
                kind: Kind::Loop,
                width,
                up,
                down,
                kids: vec![kid],
                text,
                id: String::new(),
                reach,
            };
            if *min == 0.0 {
                bypass(loop_box, m)
            } else {
                loop_box
            }
        }
    }
}

struct Placer {
    m: Metrics,
    arc: f64,
    box_top: f64,
    text_shift: f64,
    stroke: String,
    turn_east_south: String,
    turn_south_east: String,
    turn_east_north: String,
    turn_north_east: String,
    loop_down: String,
    loop_up: String,
    track: String,
    arrows: String,
    nodes: String,
}

impl Placer {
    fn line(&mut self, x1: f64, y: f64, x2: f64) {
        if x2 > x1 {
            self.track
                .push_str(&format!("M{},{}H{}", num(x1), num(y), num(x2)));
        }
    }

    fn place(&mut self, root: &BoxN, left: f64, baseline: f64) {
        let mut boxes: Vec<BoxN> = vec![root.clone()];
        let mut at: Vec<f64> = vec![left, baseline];
        while let Some(box_n) = boxes.pop() {
            let y = at.pop().expect("a y is pushed with every box");
            let x = at.pop().expect("an x is pushed with every box");
            let right = x + box_n.width;
            let arc = self.arc;
            let gap = self.m.gap;
            match box_n.kind {
                Kind::Sequence => {
                    let kids = &box_n.kids;
                    let mut kx = right;
                    for i in (0..kids.len()).rev() {
                        kx -= kids[i].width;
                        boxes.push(kids[i].clone());
                        at.push(kx);
                        at.push(y);
                        if i > 0 {
                            self.line(kx - gap, y, kx);
                        }
                        kx -= gap;
                    }
                }
                Kind::Folded => {
                    let kids = box_n.kids.clone();
                    let start = x + 2.0 * arc;
                    let from = boxes.len();
                    let mut offset = 0.0f64;
                    self.line(x, y, start);
                    for i in 0..kids.len() {
                        let kid = &kids[i];
                        let ky = y + offset;
                        boxes.push(kid.clone());
                        at.push(ky);
                        at.push(start);
                        if i + 1 < kids.len() {
                            let next = &kids[i + 1];
                            let next_offset =
                                offset + kid.down + js_max(self.m.sep, 2.0 * arc) + next.up;
                            let mid = y + offset + kid.down + js_max(self.m.sep, 2.0 * arc) / 2.0;
                            self.track.push_str(&format!(
                                "M{},{}H{}V{}H{}V{}H{}",
                                num(start + kid.width),
                                num(ky),
                                num(right - 2.0 * arc),
                                num(mid),
                                num(x + arc),
                                num(y + next_offset),
                                num(start)
                            ));
                            self.arrows.push_str(&format!(
                                "M{},{}l{},{}l{},{}z",
                                num(start - ARROW),
                                num(y + next_offset - ARROW / 2.0),
                                num(ARROW),
                                num(ARROW / 2.0),
                                num(-ARROW),
                                num(ARROW / 2.0)
                            ));
                            offset = next_offset;
                        } else {
                            self.track.push_str(&format!(
                                "M{},{}H{}V{}H{}",
                                num(start + kid.width),
                                num(ky),
                                num(right - arc),
                                num(y),
                                num(right)
                            ));
                        }
                    }
                    reverse(&mut boxes[from..], 0);
                    reverse(&mut at, 2 * from);
                }
                Kind::Choice => {
                    let kids = box_n.kids.clone();
                    let inner = box_n.width - 4.0 * arc;
                    let rail_in = x + arc;
                    let rail_out = right - arc;
                    let from = boxes.len();
                    let mut offset = 0.0f64;
                    for i in 0..kids.len() {
                        let kid = &kids[i];
                        if i > 0 {
                            offset = below(offset, &kids[i - 1], kid, &self.m);
                        }
                        let ky = y + offset;
                        let kx = x + 2.0 * arc + (inner - kid.width) / 2.0;
                        let last = i == kids.len() - 1;
                        if i == 0 {
                            self.line(x, y, kx);
                            self.line(kx + kid.width, y, right);
                        } else {
                            let head = if last {
                                format!(
                                    "M{},{}{}V{}",
                                    num(x),
                                    num(y),
                                    self.turn_east_south,
                                    num(ky - arc)
                                )
                            } else {
                                format!("M{},{}", num(rail_in), num(ky - arc))
                            };
                            let middle = if kx > rail_in + arc {
                                format!("H{}", num(kx))
                            } else {
                                String::new()
                            };
                            let back = if kx > rail_in + arc {
                                format!("H{}", num(rail_out - arc))
                            } else {
                                String::new()
                            };
                            let tail = if last {
                                format!("V{}{}", num(y + arc), self.turn_north_east)
                            } else {
                                String::new()
                            };
                            let piece = format!(
                                "{head}{}{middle}M{},{}{back}{}{tail}",
                                self.turn_south_east,
                                num(kx + kid.width),
                                num(ky),
                                self.turn_east_north
                            );
                            self.track.push_str(&piece);
                        }
                        boxes.push(kid.clone());
                        at.push(ky);
                        at.push(kx);
                    }
                    reverse(&mut boxes[from..], 0);
                    reverse(&mut at, 2 * from);
                }
                Kind::Bypass => {
                    let kid = box_n.kids[0].clone();
                    let kx = x + 2.0 * arc;
                    self.line(x, y, kx);
                    self.line(kx + kid.width, y, right);
                    self.track.push_str(&format!(
                        "M{},{}{}V{}{}H{}{}V{}{}",
                        num(x),
                        num(y),
                        self.turn_east_north,
                        num(y - box_n.reach + arc),
                        self.turn_north_east,
                        num(right - 2.0 * arc),
                        self.turn_east_south,
                        num(y - arc),
                        self.turn_south_east
                    ));
                    boxes.push(kid);
                    at.push(kx);
                    at.push(y);
                }
                Kind::Loop => {
                    let kid = box_n.kids[0].clone();
                    let kx = x + (box_n.width - kid.width) / 2.0;
                    let loop_y = y + box_n.reach;
                    let middle = x + box_n.width / 2.0;
                    self.line(x, y, kx);
                    self.line(kx + kid.width, y, right);
                    self.track.push_str(&format!(
                        "M{},{}{}V{}{}H{}{}V{}{}",
                        num(x + arc),
                        num(y),
                        self.loop_down,
                        num(loop_y - arc),
                        self.turn_south_east,
                        num(right - arc),
                        self.turn_east_north,
                        num(y + arc),
                        self.loop_up
                    ));
                    self.arrows.push_str(&format!(
                        "M{},{}l{},{}v{}z",
                        num(middle - ARROW / 2.0),
                        num(loop_y),
                        num(ARROW),
                        num(-ARROW / 2.0),
                        num(ARROW)
                    ));
                    if !box_n.text.is_empty() {
                        self.nodes.push_str(&format!(
                            "<text class=\"pelers-edge-label pelers-count\" x=\"{}\" y=\"{}\" font-size=\"{}\" fill=\"var(--_m)\">{}</text>",
                            num(middle),
                            num(loop_y + COUNT_SIZE + 3.0),
                            js_number_to_string(COUNT_SIZE),
                            esc(&box_n.text)
                        ));
                    }
                    boxes.push(kid);
                    at.push(kx);
                    at.push(y);
                }
                _ => {
                    let text = &box_n.text;
                    let space =
                        if text.starts_with(' ') || text.ends_with(' ') || text.contains("  ") {
                            " xml:space=\"preserve\""
                        } else {
                            ""
                        };
                    let shape = format!(
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
                        num(x),
                        num(y - self.box_top),
                        num(box_n.width),
                        num(self.m.height)
                    );
                    let label = format!(
                        "<text x=\"{}\" y=\"{}\"{space}",
                        num(x + box_n.width / 2.0),
                        num(y + self.text_shift)
                    );
                    match box_n.kind {
                        Kind::Terminal => self.nodes.push_str(&format!(
                            "<g class=\"pelers-node pelers-terminal\">{shape} rx=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{}/>{label} font-family=\"{FONT_MONO}\">{}</text></g>",
                            num(self.box_top),
                            self.stroke,
                            esc(text)
                        )),
                        Kind::NonTerminal => self.nodes.push_str(&format!(
                            "<g class=\"pelers-node pelers-nonterminal\" data-id=\"{}\">{shape} fill=\"var(--_bg)\" stroke=\"var(--_b)\"{}{ROUNDED}/>{label}>{}</text></g>",
                            esc_text(&box_n.id),
                            self.stroke,
                            esc(text)
                        )),
                        _ => {
                            let inner: String = text
                                .chars()
                                .skip(2)
                                .take(text.chars().count().saturating_sub(4))
                                .collect();
                            self.nodes.push_str(&format!(
                                "<g class=\"pelers-node pelers-special\">{shape} fill=\"var(--_bg)\" stroke=\"var(--_b)\" stroke-dasharray=\"4 3\"{}{ROUNDED}/>{label} font-style=\"italic\"><tspan fill=\"var(--_m)\">? </tspan>{}<tspan fill=\"var(--_m)\"> ?</tspan></text></g>",
                                self.stroke,
                                esc(&inner)
                            ));
                        }
                    }
                }
            }
        }
    }
}

pub fn render_railroad(model: &RailroadModel, config: &Config, options: &RenderOptions) -> Drawn {
    tighten(options, |tight| draw(model, config, options, tight))
}

fn draw(model: &RailroadModel, config: &Config, options: &RenderOptions, tight: bool) -> Drawn {
    let base = options.font_size.unwrap_or(16.0);
    let font_size = option(config, "fontSize", 0.0, 200.0);
    let size = if font_size != 0.0 {
        font_size
    } else {
        js_round(base * 0.875)
    };
    let head_size = if font_size != 0.0 { font_size } else { base };
    let padding_raw = option(config, "padding", -1.0, 200.0);
    let line_height = js_round(size * 1.5);
    let padding = if padding_raw < 0.0 { 12.0 } else { padding_raw };
    let m = Metrics {
        size,
        padding,
        height: even(
            line_height
                + if padding_raw < 0.0 {
                    9.0
                } else {
                    2.0 * padding_raw
                },
        ),
        gap: js_min(
            option(config, "horizontalSeparation", 16.0, 500.0),
            if tight { 12.0 } else { f64::INFINITY },
        ),
        sep: option(config, "verticalSeparation", 10.0, 500.0),
        arc: js_min(
            option(config, "arcRadius", 10.0, 200.0),
            if tight { 6.0 } else { f64::INFINITY },
        ),
    };
    let show_markers_off = matches!(
        config
            .get("railroad")
            .and_then(Value::as_object)
            .and_then(|s| s.get("showMarkers")),
        Some(Value::Bool(false))
    );
    let radius = if show_markers_off {
        0.0
    } else {
        option(config, "markerRadius", 4.0, 50.0)
    };
    let stroke_width = option(config, "strokeWidth", 0.0, 20.0);
    let stroke = if stroke_width > 0.0 {
        format!(" stroke-width=\"{}\"", num(stroke_width))
    } else {
        String::new()
    };
    let pad = options.padding.unwrap_or(8.0);
    let arc = m.arc;
    let a = num(arc);
    let turn = |sweep: u8, dx: &str, dy: &str| -> String {
        format!("a{a},{a} 0 0 {sweep} {dx}{a},{dy}{a}")
    };

    let title = layout_label(
        model.title.as_deref(),
        false,
        base,
        if tight { title_room(options) } else { 4000.0 },
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 16.0
    } else {
        0.0
    };
    let head_height = js_round(head_size * 1.5);
    let text_shift = size * 0.35;
    let box_top = m.height / 2.0;

    let mut placer = Placer {
        m,
        arc,
        box_top,
        text_shift,
        stroke: stroke.clone(),
        turn_east_south: turn(1, "", ""),
        turn_south_east: turn(0, "", ""),
        turn_east_north: turn(0, "", "-"),
        turn_north_east: turn(1, "", "-"),
        loop_down: turn(0, "-", ""),
        loop_up: turn(0, "-", "-"),
        track: String::new(),
        arrows: String::new(),
        nodes: String::new(),
    };

    let mut rules = String::new();
    let mut width = title.width;
    let mut top = pad + title_height;
    let mut bottom = pad + title.height;
    for rule in &model.rules {
        let room = if tight {
            js_max(
                80.0,
                options.max_width.unwrap_or(320.0) - 2.0 * pad - 4.0 * radius - 2.0 * LEAD - 1.0,
            )
        } else {
            f64::INFINITY
        };
        let box_n = measure_node(&rule.definition, &placer.m, room);
        let name = shown(&rule.name);
        let baseline = top + head_height + HEAD_GAP + box_n.up;
        let start = pad + 2.0 * radius;
        let left = start + LEAD;
        let end = left + box_n.width + LEAD;
        placer.track = String::new();
        placer.arrows = String::new();
        placer.nodes = String::new();
        placer.line(start, baseline, left);
        placer.line(left + box_n.width, baseline, end);
        placer.place(&box_n, left, baseline);
        let markers = if radius > 0.0 {
            format!(
                "<circle class=\"pelers-marker pelers-start\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_l)\"/><circle class=\"pelers-marker pelers-end\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_l)\"/>",
                num(pad + radius),
                num(baseline),
                num(radius),
                num(end + radius),
                num(baseline),
                num(radius)
            )
        } else {
            String::new()
        };
        rules.push_str(&format!(
            "<g class=\"pelers-rule\" data-id=\"{}\"><text class=\"pelers-rule-name\" x=\"{}\" y=\"{}\" font-size=\"{}\" font-weight=\"var(--_hw)\" text-anchor=\"start\">{}</text><path class=\"pelers-edge pelers-track\" d=\"{}\" fill=\"none\" stroke=\"var(--_l)\"{stroke}/>{}{markers}{}</g>",
            esc_text(&rule.name),
            num(pad),
            num(top + head_height / 2.0 + head_size * 0.35),
            num(head_size),
            esc(&name),
            placer.track,
            if placer.arrows.is_empty() {
                String::new()
            } else {
                format!("<path class=\"pelers-marker\" d=\"{}\" fill=\"var(--_l)\"/>", placer.arrows)
            },
            placer.nodes
        ));
        width = js_max(
            js_max(width, end + 2.0 * radius - pad),
            measure(&name, head_size, BOLD_STYLE),
        );
        bottom = baseline + box_n.down;
        top = bottom + placer.m.sep + RULE_GAP;
    }

    let total_width = (width + 2.0 * pad).ceil();
    let total_height = (bottom + pad).ceil();
    let mut content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    if !rules.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-rules\" transform=\"translate(0.5,0.5)\" font-size=\"{}\" text-anchor=\"middle\">{rules}</g>",
            num(size)
        ));
    }

    Drawn {
        svg: svg_document(
            "railroad",
            total_width,
            total_height,
            base,
            options,
            Accessible {
                acc_title: model.acc_title.as_deref(),
                acc_descr: model.acc_descr.as_deref(),
            },
            &content,
        ),
        width: total_width,
        height: total_height,
        links: Vec::new(),
    }
}
