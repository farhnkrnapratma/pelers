use std::collections::HashMap;

use super::model::{EntityKind, EventModelingModel};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::turn_to_fit;
use crate::diagrams::common::literal_label::{break_label, left_label_svg, literal_label};
use crate::svg::builder::{esc, esc_text, label_svg, num};
use crate::svg::edges::{marker, marker_trim};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ROUNDED, series_color};
use crate::text::entities::decode_entities;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, MONO, width as measure};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};

const FONT_MONO: &str = "var(--_fm)";
const BOX_PAD_X: f64 = 12.0;
const BOX_PAD_Y: f64 = 10.0;
const BOX_MIN_WIDTH: f64 = 96.0;
const BOX_GAP: f64 = 16.0;
const STAGGER: f64 = 80.0;
const LANE_PAD: f64 = 12.0;
const LANE_GAP: f64 = 8.0;
const DATA_COLUMNS: usize = 48;
const NOTE_COLUMNS: usize = 80;
const CARD_GAP: f64 = 8.0;
const RAIL_GAP: f64 = 8.0;
const CARD_MIN: f64 = 120.0;
const ARROW_ROOM: f64 = 20.0;

fn series_of(kind: EntityKind) -> Option<usize> {
    match kind {
        EntityKind::Command => Some(0),
        EntityKind::Event => Some(1),
        EntityKind::Readmodel => Some(2),
        EntityKind::Processor => Some(6),
        EntityKind::Ui => None,
    }
}

fn hard_wrap(text: &str, columns: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in decode_entities(text).split('\n') {
        let units = line.encode_utf16().count();
        if units <= columns {
            out.push(line.to_owned());
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        if chars.len() <= columns {
            out.push(line.to_owned());
            continue;
        }
        for chunk in chars.chunks(columns) {
            out.push(chunk.iter().collect());
        }
    }
    out
}

fn lines(rows: &[String], x: f64, y: f64, step: f64, attrs: &str) -> String {
    let mut body = String::new();
    for (i, row) in rows.iter().enumerate() {
        if !row.is_empty() {
            body.push_str(&format!(
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                num(x),
                num(y + i as f64 * step),
                esc(row)
            ));
        }
    }
    if body.is_empty() {
        String::new()
    } else {
        format!("<text{attrs} xml:space=\"preserve\">{body}</text>")
    }
}

fn widest(rows: &[String], size: f64, style: u32) -> f64 {
    let mut width = 0.0f64;
    for row in rows {
        width = js_max(width, measure(row, size, style));
    }
    width
}

pub fn render_eventmodel(model: &EventModelingModel, options: &RenderOptions) -> Drawn {
    turn_to_fit(options, |down| {
        if down {
            vertical(model, options)
        } else {
            horizontal(model, options)
        }
    })
}

struct Placed {
    box_index: usize,
    name: Label,
    data: Vec<String>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct Band {
    lane: usize,
    label: Label,
    right: f64,
    y: f64,
    h: f64,
}

fn horizontal(model: &EventModelingModel, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let name_size = js_round(size * 0.875);
    let small = js_round(size * 0.8125);
    let mono = js_round(size * 0.75);
    let mono_step = js_round(mono * 1.4);
    let small_step = js_round(small * 1.5);
    let pad = options.padding.unwrap_or(8.0);

    let mut bands: Vec<Band> = Vec::with_capacity(model.lanes.len());
    let mut label_column = 0.0f64;
    for lane in &model.lanes {
        let label = layout_label(Some(&lane.label), false, small, 4000.0, BOLD_STYLE);
        label_column = js_max(label_column, label.width);
        bands.push(Band {
            lane: lane.id,
            label,
            right: f64::NEG_INFINITY,
            y: 0.0,
            h: 0.0,
        });
    }
    label_column += 2.0 * LANE_PAD;

    let mut placed: Vec<Placed> = Vec::with_capacity(model.boxes.len());
    let mut previous: Option<usize> = None;
    for (index, entity) in model.boxes.iter().enumerate() {
        let band_index = bands
            .iter()
            .position(|b| b.lane == entity.lane)
            .unwrap_or(0);
        let name = layout_label(Some(&entity.name), false, name_size, 4000.0, BOLD_STYLE);
        let data = entity
            .data
            .as_deref()
            .filter(|d| !d.is_empty())
            .map(|d| hard_wrap(d, DATA_COLUMNS))
            .unwrap_or_default();
        let w = js_max(
            BOX_MIN_WIDTH,
            js_max(name.width, widest(&data, mono, MONO)) + 2.0 * BOX_PAD_X,
        );
        let h = 2.0 * BOX_PAD_Y
            + name.height
            + if data.is_empty() {
                0.0
            } else {
                6.0 + data.len() as f64 * mono_step
            };
        let mut x = 0.0;
        if let Some(prev) = previous {
            let before: &Placed = &placed[prev];
            let stagger = before.x + js_max(before.w / 2.0, before.w - STAGGER);
            let same_lane = model.boxes[placed[prev].box_index].lane == entity.lane;
            x = if same_lane {
                bands[band_index].right + BOX_GAP
            } else {
                js_max(stagger, bands[band_index].right + BOX_GAP)
            };
        }
        bands[band_index].right = x + w;
        bands[band_index].h = js_max(bands[band_index].h, h);
        placed.push(Placed {
            box_index: index,
            name,
            data,
            x,
            y: 0.0,
            w,
            h,
        });
        previous = Some(placed.len() - 1);
    }

    let mut content_right = 0.0f64;
    for item in &placed {
        content_right = js_max(content_right, item.x + item.w);
    }
    let lane_width = label_column + content_right + LANE_PAD;

    let title = layout_label(model.title.as_deref(), false, size, 4000.0, BOLD_STYLE);
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let left = pad;
    let top = pad + title_height;

    let mut y = top;
    let mut lanes = String::new();
    for band in bands.iter_mut() {
        band.y = y;
        band.h += 2.0 * LANE_PAD;
        let label = model
            .lanes
            .iter()
            .find(|lane| lane.id == band.lane)
            .map_or("", |lane| lane.label.as_str());
        lanes.push_str(&format!(
            "<g class=\"pelers-cluster pelers-em-lane\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\"{ROUNDED}/>{}</g>",
            esc_text(label),
            num(left),
            num(y),
            num(lane_width),
            num(band.h),
            label_svg(
                &band.label,
                left + LANE_PAD + band.label.width / 2.0,
                y + band.h / 2.0,
                &format!(
                    " class=\"pelers-cluster-label\" font-size=\"{}\" font-weight=\"var(--_hw)\" fill=\"var(--_m)\"",
                    js_number_to_string(small)
                ),
            )
        ));
        y += band.h + LANE_GAP;
    }
    let mut bottom = if bands.is_empty() { top } else { y - LANE_GAP };

    let mut nodes = String::new();
    for item in placed.iter_mut() {
        let entity = &model.boxes[item.box_index];
        let band = bands
            .iter()
            .find(|b| b.lane == entity.lane)
            .expect("every box has a band");
        item.x += left + label_column;
        item.y = band.y + (band.h - item.h) / 2.0;
        let series = series_of(entity.kind);
        let frame = format!(
            "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
            num(item.x),
            num(item.y),
            num(item.w),
            num(item.h)
        );
        let shape = match series {
            None => format!("<rect {frame} fill=\"var(--_bg)\" stroke=\"var(--_b)\"{ROUNDED}/>"),
            Some(slot) => format!(
                "<rect {frame} fill=\"var(--_bg)\"{ROUNDED}/><rect {frame} fill=\"{}\" fill-opacity=\"0.22\" stroke=\"{}\"{ROUNDED}/>",
                series_color(slot),
                series_color(slot)
            ),
        };
        let name_y = item.y + BOX_PAD_Y + item.name.height / 2.0;
        let text = label_svg(
            &item.name,
            item.x + item.w / 2.0,
            name_y,
            &format!(
                " class=\"pelers-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
                js_number_to_string(name_size)
            ),
        ) + &lines(
            &item.data,
            item.x + BOX_PAD_X,
            item.y + BOX_PAD_Y + item.name.height + 6.0 + mono_step / 2.0 + mono * 0.35,
            mono_step,
            &format!(
                " class=\"pelers-em-data\" font-family=\"{FONT_MONO}\" font-size=\"{}\"",
                js_number_to_string(mono)
            ),
        );
        nodes.push_str(&format!(
            "<g class=\"pelers-node pelers-em-{}{}\" data-id=\"{}\">{shape}{text}</g>",
            entity.kind.as_str(),
            if entity.reset { " pelers-em-reset" } else { "" },
            esc_text(&entity.id)
        ));
    }

    let mut edges = String::new();
    let trim = marker_trim("arrow_point");
    for relation in &model.relations {
        if relation.source == relation.target {
            continue;
        }
        let s = placed.iter().find(|p| p.box_index == relation.source);
        let t = placed.iter().find(|p| p.box_index == relation.target);
        let (Some(s), Some(t)) = (s, t) else { continue };
        let (d, head) = if model.boxes[s.box_index].lane == model.boxes[t.box_index].lane {
            let forward = t.x >= s.x;
            let sx = if forward { s.x + s.w } else { s.x };
            let tx = if forward { t.x } else { t.x + t.w };
            let sy = s.y + s.h / 2.0;
            let ty = t.y + t.h / 2.0;
            let dir = if tx >= sx { 1.0 } else { -1.0 };
            (
                format!(
                    "M{},{}L{},{}",
                    num(sx),
                    num(sy),
                    num(tx - dir * trim),
                    num(ty)
                ),
                marker("arrow_point", tx, ty, dir, 0.0, "var(--_l)", "none"),
            )
        } else {
            let down = t.y > s.y;
            let from = js_max(s.x, t.x);
            let to = js_min(s.x + s.w, t.x + t.w);
            let inset = |w: f64| js_min(24.0, w / 4.0);
            let overlap = to - from >= 16.0;
            let sx = if overlap {
                (from + to) / 2.0
            } else if t.x > s.x {
                s.x + s.w - inset(s.w)
            } else {
                s.x + inset(s.w)
            };
            let tx = if overlap {
                sx
            } else if t.x > s.x {
                t.x + inset(t.w)
            } else {
                t.x + t.w - inset(t.w)
            };
            let sy = if down { s.y + s.h } else { s.y };
            let ty = if down { t.y } else { t.y + t.h };
            let dir = if down { 1.0 } else { -1.0 };
            let end = ty - dir * trim;
            let mid = (sy + end) / 2.0;
            let d = if sx == tx {
                format!("M{},{}V{}", num(sx), num(sy), num(end))
            } else {
                format!(
                    "M{},{}C{},{} {},{} {},{}",
                    num(sx),
                    num(sy),
                    num(sx),
                    num(mid),
                    num(tx),
                    num(mid),
                    num(tx),
                    num(end)
                )
            };
            (
                d,
                marker("arrow_point", tx, ty, 0.0, dir, "var(--_l)", "none"),
            )
        };
        edges.push_str(&format!(
            "<g class=\"pelers-edge\" data-id=\"{}-{}\"><path d=\"{d}\" fill=\"none\" stroke=\"var(--_l)\"/>{head}</g>",
            esc_text(&model.boxes[relation.source].id),
            esc_text(&model.boxes[relation.target].id)
        ));
    }

    let names: HashMap<&str, &str> = model
        .boxes
        .iter()
        .map(|b| (b.id.as_str(), b.name.as_str()))
        .collect();
    let mut cards = String::new();
    let mut cards_right = 0.0f64;
    let card = |kind: &str,
                heading: &str,
                frame: &str,
                rows: &[String],
                bottom: &mut f64,
                cards: &mut String,
                cards_right: &mut f64| {
        let head_text = format!(
            "{heading} · {frame}{}",
            names.get(frame).map_or(String::new(), |n| format!(" {n}"))
        );
        let head = layout_label(Some(&head_text), false, small, 4000.0, BOLD_STYLE);
        let w = js_max(head.width, widest(rows, small, 0)) + 2.0 * BOX_PAD_X;
        let h = 2.0 * BOX_PAD_Y + head.height + rows.len() as f64 * small_step;
        let cy = *bottom
            + if cards.is_empty() && model.lanes.is_empty() {
                0.0
            } else {
                CARD_GAP
            };
        cards.push_str(&format!(
            "<g class=\"pelers-em-{kind}\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"{ROUNDED}/>{}{}</g>",
            esc_text(frame),
            num(left),
            num(cy),
            num(w),
            num(h),
            label_svg(
                &head,
                left + BOX_PAD_X + head.width / 2.0,
                cy + BOX_PAD_Y + head.height / 2.0,
                &format!(
                    " font-size=\"{}\" font-weight=\"var(--_hw)\"",
                    js_number_to_string(small)
                ),
            ),
            lines(
                rows,
                left + BOX_PAD_X,
                cy + BOX_PAD_Y + head.height + small_step / 2.0 + small * 0.35,
                small_step,
                &format!(" font-size=\"{}\"", js_number_to_string(small)),
            )
        ));
        *bottom = cy + h;
        *cards_right = js_max(*cards_right, w);
    };
    for note in &model.notes {
        let rows = hard_wrap(&note.text, NOTE_COLUMNS);
        card(
            "note",
            "Note",
            &note.frame,
            &rows,
            &mut bottom,
            &mut cards,
            &mut cards_right,
        );
    }
    for spec in &model.specifications {
        let mut rows = vec![format!("Given  {}", spec.given.join(", "))];
        if !spec.when.is_empty() {
            rows.push(format!("When  {}", spec.when.join(", ")));
        }
        rows.push(format!("Then  {}", spec.then.join(", ")));
        let wrapped: Vec<String> = rows
            .iter()
            .flat_map(|row| hard_wrap(row, NOTE_COLUMNS))
            .collect();
        card(
            "specification",
            "Given / When / Then",
            &spec.frame,
            &wrapped,
            &mut bottom,
            &mut cards,
            &mut cards_right,
        );
    }

    let width = js_max(
        js_max(if bands.is_empty() { 0.0 } else { lane_width }, cards_right),
        title.width,
    );
    let total_width = (width + 2.0 * pad).ceil();
    let total_height = (bottom + pad).ceil();
    let mut content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    content.push_str(&lanes);
    content.push_str(&edges);
    content.push_str(&nodes);
    content.push_str(&cards);

    Drawn {
        svg: svg_document(
            "eventmodeling",
            total_width,
            total_height,
            size,
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

fn vertical(model: &EventModelingModel, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let mono = js_round(size * 0.75);
    let pad = options.padding.unwrap_or(8.0);
    let room = js_max(160.0, options.max_width.unwrap_or(320.0) - 2.0 * pad);

    let mut runs: Vec<(usize, usize, usize)> = Vec::new();
    let mut passed: Vec<usize> = Vec::new();
    for relation in &model.relations {
        let (s, t) = (relation.source, relation.target);
        if s == t {
            continue;
        }
        if (s as i64 - t as i64).abs() < 2 {
            passed.push(s.max(t));
            continue;
        }
        if let Some(run) = runs.iter_mut().find(|r| r.0 == t) {
            run.1 = run.1.min(s);
            run.2 = run.2.max(s);
        } else {
            runs.push((t, s.min(t), s.max(t)));
        }
    }
    let mut ordered = runs.clone();
    ordered.sort_by_key(|r| r.1);
    let mut rail_ends: Vec<usize> = Vec::new();
    let mut rail_of: HashMap<usize, usize> = HashMap::new();
    for (target, from, to) in ordered {
        let rail = match rail_ends.iter().position(|&end| end < from) {
            Some(rail) => rail,
            None => rail_ends.len(),
        };
        if rail == rail_ends.len() {
            rail_ends.push(to);
        } else {
            rail_ends[rail] = to;
        }
        rail_of.insert(target, rail);
    }
    let gutter = 24.0 + RAIL_GAP * js_max(0.0, rail_ends.len() as f64 - 1.0);
    let card_width = js_max(CARD_MIN, room - gutter);
    let text_room = card_width - 2.0 * BOX_PAD_X;
    let title = layout_label(model.title.as_deref(), false, size, room, BOLD_STYLE);
    let mut y = pad
        + if title.height != 0.0 {
            title.height + 12.0
        } else {
            0.0
        };
    let mut bottom = pad + title.height;
    let mut width = js_max(title.width, card_width + gutter);
    let mut nodes = String::new();
    let mut edges = String::new();
    let mut cards = String::new();
    let mut placed: HashMap<usize, (f64, f64, f64, usize, f64)> = HashMap::new();
    let text = |raw: &str, font_size: f64, style: u32| -> Label {
        literal_label(&decode_entities(raw), font_size, text_room, style)
    };

    for (index, entity) in model.boxes.iter().enumerate() {
        let lane = model.lane(entity.lane);
        let lane_room = if passed.contains(&index) {
            text_room - ARROW_ROOM
        } else {
            text_room
        };
        let lane_label = break_label(
            &layout_label(Some(&lane.label), false, small, lane_room, BOLD_STYLE),
            lane_room,
        );
        let name = layout_label(
            Some(&entity.name),
            false,
            small,
            text_room - 36.0,
            BOLD_STYLE,
        );
        let data = entity
            .data
            .as_deref()
            .filter(|d| !d.is_empty())
            .map(|d| text(d, mono, MONO));
        let w = js_max(
            js_max(card_width, lane_label.width + 2.0 * BOX_PAD_X),
            js_max(
                name.width + 2.0 * BOX_PAD_X + 36.0,
                data.as_ref().map_or(0.0, |d| d.width) + 2.0 * BOX_PAD_X,
            ),
        );
        let head_h = lane_label.height + 12.0;
        let h = 2.0 * BOX_PAD_Y + name.height + data.as_ref().map_or(0.0, |d| d.height + 6.0);
        let series = series_of(entity.kind);
        let cy = y + head_h;
        let frame = format!(
            "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
            num(pad),
            num(cy),
            num(w),
            num(h)
        );
        let reset = if entity.reset { " pelers-em-reset" } else { "" };
        let series_rect = match series {
            None => String::new(),
            Some(slot) => format!(
                "<rect {frame} fill=\"{}\" fill-opacity=\"0.22\" stroke=\"{}\"{ROUNDED}/>",
                series_color(slot),
                series_color(slot)
            ),
        };
        let data_markup = match &data {
            None => String::new(),
            Some(data) => lines(
                &data
                    .lines
                    .iter()
                    .map(|line| {
                        line.iter()
                            .map(|span| span.text.as_str())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>(),
                pad + BOX_PAD_X,
                cy + BOX_PAD_Y + name.height + 6.0 + data.line_height / 2.0 + mono * 0.35,
                data.line_height,
                &format!(
                    " class=\"pelers-em-data\" font-family=\"{FONT_MONO}\" font-size=\"{}\"",
                    js_number_to_string(mono)
                ),
            ),
        };
        nodes.push_str(&format!(
            "<g class=\"pelers-cluster pelers-em-lane\" data-id=\"{}\">{}<g class=\"pelers-node pelers-em-{}{reset}\" data-id=\"{}\"><rect {frame} fill=\"var(--_bg)\" stroke=\"var(--_b)\"{ROUNDED}/>{series_rect}{}<text class=\"pelers-em-frame\" x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"{}\" fill=\"var(--_m)\">{}</text>{data_markup}</g></g>",
            esc_text(&lane.label),
            left_label_svg(
                &lane_label,
                pad + BOX_PAD_X,
                y,
                &format!(
                    " class=\"pelers-cluster-label\" font-size=\"{}\" font-weight=\"var(--_hw)\" fill=\"var(--_m)\"",
                    js_number_to_string(small)
                ),
            ),
            entity.kind.as_str(),
            esc_text(&entity.id),
            label_svg(
                &name,
                pad + BOX_PAD_X + name.width / 2.0,
                cy + BOX_PAD_Y + name.height / 2.0,
                &format!(
                    " class=\"pelers-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"",
                    js_number_to_string(small)
                ),
            ),
            num(pad + w - BOX_PAD_X),
            num(cy + BOX_PAD_Y + small),
            js_number_to_string(mono),
            esc(&entity.id)
        ));
        placed.insert(index, (cy, h, w, index, lane_label.width));
        width = js_max(width, w + gutter);
        bottom = cy + h;
        y = bottom + 28.0;
    }

    for relation in &model.relations {
        let (Some(s), Some(t)) = (placed.get(&relation.source), placed.get(&relation.target))
        else {
            continue;
        };
        if relation.source == relation.target {
            continue;
        }
        let (s_y, s_h, s_w, s_index, s_lane) = *s;
        let (t_y, t_h, t_w, t_index, t_lane) = *t;
        let down = t_y > s_y;
        let dir = if down { 1.0 } else { -1.0 };
        let sy = if down { s_y + s_h } else { s_y };
        let ty = if down { t_y } else { t_y + t_h };
        let (d, head) = if (s_index as i64 - t_index as i64).abs() == 1 {
            let lane_here = if down { t_lane } else { s_lane };
            let cx = pad + js_max(js_min(s_w, t_w) / 2.0, BOX_PAD_X + lane_here + 16.0);
            (
                format!("M{},{}V{}", num(cx), num(sy), num(ty - dir * trim_arrow())),
                marker("arrow_point", cx, ty, 0.0, dir, "var(--_l)", "none"),
            )
        } else {
            let rail = pad + width - gutter
                + 12.0
                + RAIL_GAP * *rail_of.get(&relation.target).unwrap_or(&0) as f64;
            let from = s_y + s_h / 2.0 - 5.0;
            let to = t_y + t_h / 2.0 + 5.0;
            (
                format!(
                    "M{},{}H{}V{}H{}",
                    num(pad + s_w),
                    num(from),
                    num(rail),
                    num(to),
                    num(pad + t_w + trim_arrow())
                ),
                marker("arrow_point", pad + t_w, to, -1.0, 0.0, "var(--_l)", "none"),
            )
        };
        edges.push_str(&format!(
            "<g class=\"pelers-edge\" data-id=\"{}-{}\"><path d=\"{d}\" fill=\"none\" stroke=\"var(--_l)\"/>{head}</g>",
            esc_text(&model.boxes[relation.source].id),
            esc_text(&model.boxes[relation.target].id)
        ));
    }

    let names: HashMap<&str, &str> = model
        .boxes
        .iter()
        .map(|b| (b.id.as_str(), b.name.as_str()))
        .collect();
    let card = |kind: &str,
                heading: &str,
                frame: &str,
                raw: &str,
                y: &mut f64,
                bottom: &mut f64,
                cards: &mut String,
                width: &mut f64| {
        let head_text = format!(
            "{heading} · {frame}{}",
            names.get(frame).map_or(String::new(), |n| format!(" {n}"))
        );
        let head = layout_label(Some(&head_text), false, small, text_room, BOLD_STYLE);
        let body = text(raw, small, 0);
        let w = js_max(
            card_width,
            js_max(head.width + 2.0 * BOX_PAD_X, body.width + 2.0 * BOX_PAD_X),
        );
        let h = 2.0 * BOX_PAD_Y + head.height + body.height;
        let body_lines: Vec<String> = body
            .lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|span| span.text.as_str())
                    .collect::<String>()
            })
            .collect();
        cards.push_str(&format!(
            "<g class=\"pelers-em-{kind}\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"{ROUNDED}/>{}{}</g>",
            esc_text(frame),
            num(pad),
            num(*y),
            num(w),
            num(h),
            label_svg(
                &head,
                pad + BOX_PAD_X + head.width / 2.0,
                *y + BOX_PAD_Y + head.height / 2.0,
                &format!(" font-size=\"{}\" font-weight=\"var(--_hw)\"", js_number_to_string(small)),
            ),
            lines(
                &body_lines,
                pad + BOX_PAD_X,
                *y + BOX_PAD_Y + head.height + body.line_height / 2.0 + small * 0.35,
                body.line_height,
                &format!(" font-size=\"{}\"", js_number_to_string(small)),
            )
        ));
        *bottom = *y + h;
        *y = *bottom + CARD_GAP;
        *width = js_max(*width, w);
    };
    for note in &model.notes {
        card(
            "note",
            "Note",
            &note.frame,
            &note.text,
            &mut y,
            &mut bottom,
            &mut cards,
            &mut width,
        );
    }
    for spec in &model.specifications {
        let mut rows = vec![format!("Given  {}", spec.given.join(", "))];
        if !spec.when.is_empty() {
            rows.push(format!("When  {}", spec.when.join(", ")));
        }
        rows.push(format!("Then  {}", spec.then.join(", ")));
        card(
            "specification",
            "Given / When / Then",
            &spec.frame,
            &rows.join("\n"),
            &mut y,
            &mut bottom,
            &mut cards,
            &mut width,
        );
    }

    let total_width = (width + 2.0 * pad).ceil();
    let total_height = (bottom + pad).ceil();
    let mut content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    content.push_str(&edges);
    content.push_str(&nodes);
    content.push_str(&cards);

    Drawn {
        svg: svg_document(
            "eventmodeling",
            total_width,
            total_height,
            size,
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

fn trim_arrow() -> f64 {
    marker_trim("arrow_point")
}
