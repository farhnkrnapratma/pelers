use std::collections::HashMap;

use super::{JourneyModel, JourneyTask, journey_actors};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit::{fit_label, plain_label};
use crate::diagrams::common::fit_width::turn_to_fit;
use crate::svg::builder::{esc_text, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ROUNDED, series_color};
use crate::text::label::Label;
use crate::text::measurer::BOLD_STYLE;
use crate::util::chars::trim;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};

const PAD_X: f64 = 10.0;
const PAD_Y: f64 = 8.0;
const MIN_TEXT: f64 = 124.0;
const MAX_TEXT: f64 = 200.0;
const GAP: f64 = 12.0;
const BAND_PAD: f64 = 6.0;
const GUTTER: f64 = 24.0;
const LEVEL: f64 = 26.0;
const FACE: f64 = 11.0;
const DOT: f64 = 5.0;
const DOT_STEP: f64 = 13.0;
const LEGEND_GAP: f64 = 20.0;

const EYES: &str =
    "<circle cx=\"-3.6\" cy=\"-2.8\" r=\"1.3\"/><circle cx=\"3.6\" cy=\"-2.8\" r=\"1.3\"/>";

fn mood(score: f64) -> &'static str {
    if score > 3.0 {
        "happy"
    } else if score < 3.0 {
        "sad"
    } else {
        "neutral"
    }
}

fn mouth(mood: &str) -> &'static str {
    match mood {
        "happy" => "M-5,2A6,6 0 0 0 5,2",
        "sad" => "M-5,6.5A6,6 0 0 1 5,6.5",
        _ => "M-4.5,4.5H4.5",
    }
}

fn face(x: f64, y: f64, mood: &str) -> String {
    format!(
        "<g class=\"pelers-face pelers-face-{mood}\" transform=\"translate({},{})\"><circle r=\"{}\" fill=\"var(--_bg)\" stroke=\"var(--_l)\"/><g fill=\"var(--_l)\">{EYES}</g><path d=\"{}\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\"/></g>",
        num(x),
        num(y),
        js_number_to_string(FACE),
        mouth(mood)
    )
}

pub fn render_journey(model: &JourneyModel, options: &RenderOptions) -> Drawn {
    turn_to_fit(options, |down| {
        if down {
            vertical(model, options)
        } else {
            across(model, options)
        }
    })
}

fn accessible(model: &JourneyModel) -> Accessible<'_> {
    Accessible {
        acc_title: model.acc_title.as_deref(),
        acc_descr: model.acc_descr.as_deref(),
    }
}

struct Legend {
    name: String,
    label: Label,
    x: f64,
    y: f64,
    h: f64,
}

struct Run {
    start: usize,
    count: usize,
    label: Option<Label>,
}

fn across(model: &JourneyModel, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let tiny = js_round(size * 0.75);
    let pad = options.padding.unwrap_or(8.0);
    let tasks: &[JourneyTask] = &model.tasks;
    let n = tasks.len();

    let names: Vec<String> = journey_actors(tasks)
        .into_iter()
        .filter(|name| !name.is_empty())
        .collect();
    let actor: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();

    let mut text_w = MIN_TEXT;
    let mut labels: Vec<Label> = tasks
        .iter()
        .map(|task| plain_label(Some(&task.task), small, text_w, 0))
        .collect();
    let mut widest = 0.0;
    for label in &labels {
        widest = js_max(widest, label.width);
    }
    if widest > text_w {
        text_w = js_min(widest.ceil(), MAX_TEXT);
        labels = tasks
            .iter()
            .map(|task| fit_label(Some(&task.task), small, text_w, 0))
            .collect();
    }

    let card_w = text_w + 2.0 * PAD_X;
    let pitch = card_w + GAP;
    let plot_w = if n > 0 { n as f64 * pitch - GAP } else { 0.0 };

    let line_h = js_round(small * 1.5);
    let limit = js_max(plot_w, 240.0);
    let mut legend: Vec<Legend> = Vec::new();
    let mut legend_w = 0.0;
    let mut legend_h = 0.0;
    let mut lx = 0.0;
    for name in &names {
        let label = fit_label(Some(name), small, limit - 2.0 * DOT - 6.0, 0);
        let w = 2.0 * DOT + 6.0 + label.width;
        if lx > 0.0 && lx + w > limit {
            lx = 0.0;
            legend_h += 4.0;
        }
        let y = if lx > 0.0 {
            legend.last().map_or(legend_h, |item| item.y)
        } else {
            legend_h
        };
        let h = js_max(label.height, line_h);
        legend.push(Legend {
            name: name.clone(),
            label,
            x: lx,
            y,
            h,
        });
        lx += w;
        legend_w = js_max(legend_w, lx);
        legend_h = js_max(legend_h, y + h);
        lx += LEGEND_GAP;
    }

    let body_w = if n > 0 { GUTTER + plot_w } else { 0.0 };
    let title = fit_label(
        model.title.as_deref(),
        size,
        js_max(body_w, 320.0),
        BOLD_STYLE,
    );
    let title_h = if title.height > 0.0 {
        title.height + 16.0
    } else {
        0.0
    };
    let width = js_max(
        js_max(body_w, title.width),
        if n > 0 { GUTTER + legend_w } else { 0.0 },
    );
    let left = pad + (width - body_w) / 2.0 + GUTTER;
    let top = pad + title_h;

    let mut runs: Vec<Run> = Vec::new();
    let mut band_h = 0.0;
    let mut i = 0;
    while i < n {
        let mut end = i + 1;
        while end < n && tasks[end].section == tasks[i].section {
            end += 1;
        }
        let label = if tasks[i].section.is_empty() {
            None
        } else {
            let label = fit_label(
                Some(&tasks[i].section),
                small,
                (end - i) as f64 * pitch - GAP - 2.0 * PAD_X,
                BOLD_STYLE,
            );
            band_h = js_max(band_h, js_max(label.height, line_h) + 2.0 * BAND_PAD);
            Some(label)
        };
        runs.push(Run {
            start: i,
            count: end - i,
            label,
        });
        i = end;
    }

    let mut text_h = line_h;
    for label in &labels {
        text_h = js_max(text_h, label.height);
    }
    let dots_h = if names.is_empty() {
        0.0
    } else {
        2.0 * DOT + 6.0
    };
    let card_h = text_h + dots_h + 2.0 * PAD_Y;
    let cards_y = top + if band_h > 0.0 { band_h + BAND_PAD } else { 0.0 };
    let chart_top = cards_y + card_h + FACE + 16.0;
    let chart_bottom = chart_top + 4.0 * LEVEL;
    let right = left + plot_w;

    let mut body = String::new();
    let mut bottom = top;
    if n > 0 {
        let mut grid = String::new();
        let mut scale = String::new();
        for score in (1..=5).rev() {
            let y = chart_top + (5 - score) as f64 * LEVEL;
            grid += &format!("M{},{}H{}", num(left - 6.0), num(y), num(right));
            scale += &format!(
                "<text x=\"{}\" y=\"{}\">{score}</text>",
                num(left - 12.0),
                num(y + tiny * 0.35)
            );
        }
        body += &format!(
            "<path class=\"pelers-grid\" d=\"{grid}\" fill=\"none\" stroke=\"var(--_a)\"/><g class=\"pelers-scale\" font-size=\"{}\" text-anchor=\"end\" fill=\"var(--_m)\">{scale}</g>",
            js_number_to_string(tiny)
        );

        let mut line = String::new();
        let mut stems = String::new();
        let mut faces = String::new();
        let mut cards: Vec<String> = Vec::with_capacity(n);
        for (i, task) in tasks.iter().enumerate() {
            let x = left + i as f64 * pitch;
            let cx = x + card_w / 2.0;
            let score = task.score;
            let level = if score.is_nan() {
                3.0
            } else {
                js_min(5.0, js_max(1.0, score))
            };
            let y = chart_top + (5.0 - level) * LEVEL;
            line += &format!("{}{},{}", if i == 0 { "M" } else { "L" }, num(cx), num(y));
            stems += &format!("M{},{}V{}", num(cx), num(cards_y + card_h), num(y - FACE));
            faces += &face(cx, y, mood(score));

            let people: Vec<&str> = task
                .people
                .iter()
                .filter(|name| !name.is_empty())
                .map(String::as_str)
                .collect();
            let step = js_min(
                DOT_STEP,
                (card_w - 2.0 * PAD_X - 2.0 * DOT) / js_max(people.len() as f64 - 1.0, 1.0),
            );
            let mut dots = String::new();
            for (k, name) in people.iter().enumerate() {
                let dot_x = cx + (k as f64 - (people.len() as f64 - 1.0) / 2.0) * step;
                dots += &format!(
                    "<circle class=\"pelers-actor\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"var(--_s)\"><title>{}</title></circle>",
                    num(dot_x),
                    num(cards_y + card_h - PAD_Y - DOT),
                    js_number_to_string(DOT),
                    series_color(actor.get(name).copied().unwrap_or(0)),
                    esc_text(name)
                );
            }
            cards.push(format!(
                "<g class=\"pelers-node pelers-task\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{ROUNDED}/>{}{dots}</g>",
                esc_text(trim(&task.task)),
                num(x),
                num(cards_y),
                num(card_w),
                num(card_h),
                label_svg(
                    &labels[i],
                    cx,
                    cards_y + PAD_Y + text_h / 2.0,
                    &format!(" font-size=\"{}\"", js_number_to_string(small))
                )
            ));
        }
        body += &format!(
            "<path class=\"pelers-stems\" d=\"{stems}\" fill=\"none\" stroke=\"var(--_b)\" stroke-dasharray=\"2 3\"/>"
        );
        if n > 1 {
            body += &format!(
                "<path class=\"pelers-edge\" d=\"{line}\" fill=\"none\" stroke=\"var(--_l)\" stroke-linejoin=\"round\"/>"
            );
        }
        body += &faces;

        for run in &runs {
            let members = cards[run.start..run.start + run.count].concat();
            match &run.label {
                None => body += &members,
                Some(label) => {
                    let x = left + run.start as f64 * pitch;
                    let w = run.count as f64 * pitch - GAP;
                    body += &format!(
                        "<g class=\"pelers-cluster pelers-section\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\"{ROUNDED}/>{}{members}</g>",
                        esc_text(trim(&tasks[run.start].section)),
                        num(x),
                        num(top),
                        num(w),
                        num(band_h),
                        label_svg(
                            label,
                            x + w / 2.0,
                            top + band_h / 2.0,
                            &format!(
                                " font-size=\"{}\" font-weight=\"var(--_hw)\"",
                                js_number_to_string(small)
                            )
                        )
                    );
                }
            }
        }
        bottom = chart_bottom + FACE;

        if !legend.is_empty() {
            let x0 = left + (plot_w - legend_w) / 2.0;
            let y0 = bottom + 16.0;
            let mut items = String::new();
            for item in &legend {
                let cy = y0 + item.y + item.h / 2.0;
                items += &format!(
                    "<g class=\"pelers-legend-item\" data-id=\"{}\"><circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>{}</g>",
                    esc_text(&item.name),
                    num(x0 + item.x + DOT),
                    num(cy),
                    js_number_to_string(DOT),
                    series_color(actor.get(item.name.as_str()).copied().unwrap_or(0)),
                    label_svg(
                        &item.label,
                        x0 + item.x + 2.0 * DOT + 6.0 + item.label.width / 2.0,
                        cy,
                        ""
                    )
                );
            }
            body += &format!(
                "<g class=\"pelers-legend\" font-size=\"{}\">{items}</g>",
                js_number_to_string(small)
            );
            bottom = y0 + legend_h;
        }
    }

    let total_width = (width + 2.0 * pad).ceil();
    let total_height = (js_max(bottom, pad) + pad).ceil();
    let content = label_svg(
        &title,
        pad + width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &body;
    Drawn {
        svg: svg_document(
            "journey",
            total_width,
            total_height,
            size,
            options,
            accessible(model),
            &content,
        ),
        width: total_width,
        height: total_height,
        links: Vec::new(),
    }
}

fn vertical(model: &JourneyModel, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);
    let room = js_max(140.0, options.max_width.unwrap_or(320.0) - 2.0 * pad);
    let title = fit_label(model.title.as_deref(), size, room, BOLD_STYLE);
    let tasks: &[JourneyTask] = &model.tasks;
    let names: Vec<String> = journey_actors(tasks)
        .into_iter()
        .filter(|name| !name.is_empty())
        .collect();
    let actors: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, name)| (name.as_str(), i))
        .collect();

    let mut y = pad
        + if title.height != 0.0 {
            title.height + 16.0
        } else {
            0.0
        };
    let mut section: Option<&str> = None;
    let mut body = String::new();
    let mut width = title.width;
    for (index, task) in tasks.iter().enumerate() {
        if section != Some(task.section.as_str()) {
            section = Some(&task.section);
            let head = fit_label(Some(&task.section), size, room - 16.0, BOLD_STYLE);
            if head.height != 0.0 {
                body += &label_svg(
                    &head,
                    pad + 8.0 + head.width / 2.0,
                    y + head.height / 2.0,
                    " class=\"pelers-cluster-label\" font-weight=\"var(--_hw)\"",
                );
                width = js_max(width, head.width + 16.0);
                y += head.height + 8.0;
            }
        }
        let label = fit_label(Some(&task.task), small, room - 72.0, 0);
        let people: Vec<(&str, Label)> = task
            .people
            .iter()
            .filter(|name| !name.is_empty())
            .map(|name| (name.as_str(), fit_label(Some(name), small, room - 44.0, 0)))
            .collect();
        let text_h = js_max(label.height, 42.0);
        let mut h = 2.0 * PAD_Y + text_h;
        for (_, person) in &people {
            h += js_max(16.0, person.height) + 4.0;
        }
        let w = js_max(room, label.width + 72.0);
        body += &format!(
            "<g class=\"pelers-node pelers-journey-step\" data-id=\"{index}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{ROUNDED}/>",
            num(pad),
            num(y),
            num(w),
            num(h)
        );
        body += &label_svg(
            &label,
            pad + PAD_X + label.width / 2.0,
            y + PAD_Y + label.height / 2.0,
            " class=\"pelers-label\"",
        );
        let cx = pad + w - 28.0;
        body += &face(cx, y + PAD_Y + FACE, mood(task.score));
        if !task.score.is_nan() {
            body += &format!(
                "<text class=\"pelers-journey-score\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"middle\">{}</text>",
                num(cx),
                num(y + PAD_Y + 2.0 * FACE + small),
                js_number_to_string(small),
                num(js_min(5.0, js_max(1.0, task.score)))
            );
        }
        let mut py = y + PAD_Y + text_h + 4.0;
        for (name, person) in &people {
            let ph = js_max(16.0, person.height);
            body += &format!(
                "<circle class=\"pelers-actor\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
                num(pad + 16.0),
                num(py + ph / 2.0),
                js_number_to_string(DOT),
                series_color(actors.get(name).copied().unwrap_or(0))
            );
            body += &label_svg(
                person,
                pad + 28.0 + person.width / 2.0,
                py + ph / 2.0,
                &format!(
                    " class=\"pelers-journey-actor\" font-size=\"{}\"",
                    js_number_to_string(small)
                ),
            );
            py += ph + 4.0;
        }
        body += "</g>";
        y += h;
        if index + 1 < tasks.len() {
            body += &format!(
                "<path class=\"pelers-edge\" d=\"M{},{}v16\" fill=\"none\" stroke=\"var(--_l)\"/>",
                num(pad + w / 2.0),
                num(y)
            );
        }
        y += 16.0;
        width = js_max(width, w);
    }
    let total_width = (width + 2.0 * pad).ceil();
    let height = (js_max(
        pad + title.height,
        y - if tasks.is_empty() { 0.0 } else { 16.0 },
    ) + pad)
        .ceil();
    let content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &body;
    Drawn {
        svg: svg_document(
            "journey",
            total_width,
            height,
            size,
            options,
            accessible(model),
            &content,
        ),
        width: total_width,
        height,
        links: Vec::new(),
    }
}
