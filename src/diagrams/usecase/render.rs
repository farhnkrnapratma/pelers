use std::collections::HashMap;

use super::graph::{GraphEdge, GraphNode, build_usecase_graph};
use super::types::{ActorType, BoundaryType, LabelType, UsecaseModel};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{runs_across, turn_to_fit};
use crate::layout::compound::{compound_layout, shift_layout};
use crate::layout::{CEdge, CNode, Dir, LayeredOptions, cnode, direction};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, label_svg_with, num};
use crate::svg::edges::{
    EdgePath, Fill, FillUnder, casing, casings_svg, edge_ink, edge_label_svg, fill_under,
    hollow_on, marker, marker_trim, route_path,
};
use crate::svg::ink::{ink_fill, paints};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ResolvedStyle, class_names, resolve_style, rounded, sets_fill};
use crate::svg::title::title_label;
use crate::text::label::{Label, label_line, layout_label, line_middle};
use crate::text::measurer::{BOLD_STYLE, ITALIC};
use crate::util::chars::is_js_space;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};

const NODE_SEP: f64 = 32.0;
const EDGE_SEP: f64 = 16.0;
const RANK_SEP: f64 = 72.0;
const GROUP_PAD: f64 = 20.0;
const LOOP: f64 = 26.0;
const FIGURE_W: f64 = 40.0;
const FIGURE_H: f64 = 60.0;
const FIGURE_GAP: f64 = 6.0;
const TAB_PAD: f64 = 6.0;
const CELL_PAD: f64 = 10.0;
const ROW_PAD: f64 = 8.0;
const FOLD: f64 = 9.0;
const EXTENSION: f64 = 12.0;
const PORT_GAP: f64 = 14.0;
const SHAPE_ATTRS: &str = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Actor,
    Ellipse,
    Box,
    Json,
    Boundary,
}

#[derive(Clone, Debug)]
struct View {
    node: GraphNode,
    kind: Kind,
    style: ResolvedStyle,
    label: Label,
    stereotype: Label,
    w: f64,
    h: f64,
    rows: Vec<(Label, Label)>,
    key_w: f64,
    head_h: f64,
    loops: usize,
}

fn number_option(config: &Config, key: &str, fallback: f64) -> f64 {
    match config
        .get("usecase")
        .and_then(crate::util::value::Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(crate::util::value::Value::Number(value)) if *value > 0.0 => js_min(*value, 2000.0),
        _ => fallback,
    }
}

fn literal(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' | '<' | '>' | '\\' => out.push_str(&format!("&#{};", c as u32)),
            other => out.push(other),
        }
    }
    out
}

fn text(raw: Option<&str>, markdown: bool, size: f64, wrap: f64, base: u32) -> Label {
    let shown = match raw {
        Some(raw) if !markdown => Some(literal(raw)),
        Some(raw) => Some(raw.to_owned()),
        None => None,
    };
    layout_label(shown.as_deref(), markdown, size, wrap, base)
}

fn anchor(view: &View, c: &CNode, side: i32, along: f64) -> (f64, f64) {
    let sign = if side == 1 || side == 2 { 1.0 } else { -1.0 };
    let upright = side == 1 || side == 3;
    let mut reach = (if upright { view.w } else { view.h }) / 2.0;
    let mut middle = 0.0;
    match view.kind {
        Kind::Actor => {
            if upright {
                reach = FIGURE_W / 2.0 + 3.0;
                middle = (FIGURE_H - view.h) / 2.0;
            } else {
                reach += 2.0;
            }
        }
        Kind::Ellipse => {
            let span = (if upright { view.h } else { view.w }) / 2.0;
            reach *= js_max(1.0 - (along / span).powi(2), 0.0).sqrt();
        }
        _ => {}
    }
    if upright {
        (c.x + sign * reach, c.y + middle + along)
    } else {
        (c.x + along, c.y + sign * reach)
    }
}

fn side_length(view: &View, side: i32) -> f64 {
    let upright = side == 1 || side == 3;
    if view.kind == Kind::Actor {
        return if upright {
            FIGURE_H * 0.5
        } else {
            FIGURE_W * 0.6
        };
    }
    let length = if upright { view.h } else { view.w };
    if view.kind == Kind::Ellipse {
        length * 0.6
    } else {
        length - 16.0
    }
}

#[derive(Clone, Copy)]
struct End {
    edge: usize,
    at: usize,
    toward: f64,
}

fn attach_ends(cedges: &mut [CEdge], cnodes: &[CNode], views: &[View]) {
    let mut sides: Vec<(usize, Vec<End>)> = Vec::new();
    let add = |sides: &mut Vec<(usize, Vec<End>)>,
               cedges: &[CEdge],
               index: usize,
               edge: usize,
               at: usize,
               next: usize| {
        let c = &cnodes[index];
        if c.is_group {
            return;
        }
        let route = &cedges[edge].route;
        let dx = route[at] - c.x;
        let dy = route[at + 1] - c.y;
        let side = if (dy.abs() - c.h / 2.0).abs() < 0.5 {
            if dy < 0.0 { 0 } else { 2 }
        } else if (dx.abs() - c.w / 2.0).abs() < 0.5 {
            if dx < 0.0 { 3 } else { 1 }
        } else {
            return;
        };
        let key = index * 4 + side as usize;
        let toward = route[next + if side & 1 == 1 { 1 } else { 0 }];
        let end = End { edge, at, toward };
        match sides.iter_mut().find(|(k, _)| *k == key) {
            Some((_, list)) => list.push(end),
            None => sides.push((key, vec![end])),
        }
    };
    for e in 0..cedges.len() {
        let last = cedges[e].route.len() as isize - 3;
        if cedges[e].src == cedges[e].dst || last < 3 {
            continue;
        }
        let (src, dst) = (cedges[e].src, cedges[e].dst);
        add(&mut sides, cedges, src, e, 0, 3);
        add(
            &mut sides,
            cedges,
            dst,
            e,
            last as usize,
            (last - 3) as usize,
        );
    }
    for (key, mut ends) in sides {
        let index = key >> 2;
        let side = (key & 3) as i32;
        let view = &views[index];
        let step = if ends.len() > 1 {
            js_min(PORT_GAP, side_length(view, side) / (ends.len() - 1) as f64)
        } else {
            0.0
        };
        ends.sort_by(|a, b| {
            a.toward
                .partial_cmp(&b.toward)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (i, end) in ends.iter().enumerate() {
            let along = (i as f64 - (ends.len() as f64 - 1.0) / 2.0) * step;
            let (x, y) = anchor(view, &cnodes[index], side, along);
            cedges[end.edge].route[end.at] = x;
            cedges[end.edge].route[end.at + 1] = y;
        }
    }
}

fn extension(x: f64, y: f64, dx: f64, dy: f64, color: &str) -> String {
    let bx = x - dx * EXTENSION;
    let by = y - dy * EXTENSION;
    let half = EXTENSION * 0.5;
    format!(
        "<path class=\"pelers-marker\" d=\"M{},{}L{},{}L{},{}Z\" fill=\"var(--_bg)\" stroke=\"{color}\" stroke-linejoin=\"round\"/>",
        num(x),
        num(y),
        num(bx - dy * half),
        num(by + dx * half),
        num(bx + dy * half),
        num(by - dx * half)
    )
}

fn trim_for(kind: &str) -> f64 {
    if kind == "extension" {
        EXTENSION
    } else {
        marker_trim(kind)
    }
}

fn mark(kind: &str, x: f64, y: f64, dx: f64, dy: f64, color: &str) -> String {
    if kind == "extension" {
        extension(x, y, dx, dy, color)
    } else {
        marker(kind, x, y, dx, dy, color, "none")
    }
}

fn figure(view: &View, icons: bool) -> String {
    let node = &view.node;
    let shape = view.style.shape.as_str();
    let line = format!(" fill=\"none\" stroke=\"var(--_b)\"{}", view.style.line);
    let mut head = -21.0;
    let mut radius = 9.0;
    let mut out = match node.actor_type {
        Some(ActorType::Hollow) => {
            radius = 8.0;
            format!(
                "<circle cy=\"-21\" r=\"8\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"{shape}/><path d=\"M-20,-9H20V0H5.5L20,15.5L12,25.5L0,12L-12,25.5L-20,15.5L-5.5,0H-20Z\" fill=\"var(--_bg)\" stroke=\"var(--_b)\" stroke-linejoin=\"round\"{shape}/>"
            )
        }
        Some(ActorType::Awesome) => format!(
            "<path d=\"M0,-30A11,11 0 1 1 0,-8A11,11 0 1 1 0,-30ZM-20,25C-20,9 -12,-3 0,-3C12,-3 20,9 20,25C20,28 18,30 15,30H-15C-18,30 -20,28 -20,25Z\" fill=\"var(--_b)\"{shape}/>"
        ),
        Some(ActorType::Icon) => {
            head = -14.0;
            radius = 12.0;
            let name = node.icon.clone().unwrap_or_default();
            let markup = if icons {
                crate::text::icons::icon_markup(&name)
            } else {
                String::new()
            };
            format!(
                "<rect x=\"-22\" y=\"-26\" width=\"44\" height=\"44\"{}/><svg class=\"pelers-icon\" data-icon=\"{}\" x=\"-14\" y=\"-18\" width=\"28\" height=\"28\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{markup}</svg>",
                rounded(&format!("{SHAPE_ATTRS}{shape}")),
                esc_text(&name)
            )
        }
        _ => format!(
            "<circle cy=\"-21\" r=\"9\"{SHAPE_ATTRS}{shape}/><path d=\"M0,-12V10M-14,-3H14M0,10L-12,30M0,10L12,30\"{line}/>"
        ),
    };
    if node.business == Some(true) {
        out.push_str(&format!(
            "<path class=\"pelers-business\" d=\"M{},{}L{},{}\"{line}/>",
            num(0.12 * radius),
            num(head + 0.99 * radius),
            num(0.92 * radius),
            num(head - 0.39 * radius)
        ));
    }
    out
}

pub fn render_usecase(model: &UsecaseModel, config: &Config, options: &RenderOptions) -> Drawn {
    if !runs_across(Some(model.direction.as_str())) {
        return draw(model, config, options, false);
    }
    turn_to_fit(options, |down| draw(model, config, options, down))
}

fn extension_trim(kind: &str) -> f64 {
    trim_for(kind)
}

fn draw(model: &UsecaseModel, config: &Config, options: &RenderOptions, turned: bool) -> Drawn {
    let graph = build_usecase_graph(model);
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let tiny = js_round(size * 0.8125);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();
    let root_dir = if turned {
        Dir::TB
    } else {
        direction(Some(model.direction.as_str()))
    };
    let sideways = root_dir == Dir::LR || root_dir == Dir::RL;

    let mut index: HashMap<String, usize> = HashMap::new();
    let mut views: Vec<View> = Vec::new();
    let mut cnodes: Vec<CNode> = Vec::new();
    for (i, node) in graph.nodes.iter().enumerate() {
        index.insert(node.id.clone(), i);
    }
    for node in &graph.nodes {
        let mut compiled = node.css_compiled_styles.clone();
        if !node.css_styles.is_empty() {
            compiled.extend(node.css_styles.iter().cloned());
        }
        let style = resolve_style(&compiled);
        let base =
            (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let font_size = style.font_size.unwrap_or(size);
        let markdown = node.label_type == LabelType::Markdown;
        let stereotype_raw = node.stereotype.as_ref().map(|s| format!("«{s}»"));
        let stereotype = text(stereotype_raw.as_deref(), false, tiny, 200.0, base);
        let mut view = View {
            node: node.clone(),
            kind: Kind::Box,
            style,
            label: stereotype.clone(),
            stereotype: stereotype.clone(),
            w: 0.0,
            h: 0.0,
            rows: Vec::new(),
            key_w: 0.0,
            head_h: 0.0,
            loops: 0,
        };
        let mut c = if node.is_group {
            view.kind = Kind::Boundary;
            view.label = text(Some(&node.label), markdown, font_size, 4000.0, base);
            let title_h = view.label.height;
            let mut group = cnode(0.0, 0.0, -1);
            group.is_group = true;
            group.pad_x = GROUP_PAD;
            group.pad_bottom = GROUP_PAD;
            group.pad_top = if node.boundary_type == Some(BoundaryType::Package) {
                title_h + 2.0 * TAB_PAD + 16.0
            } else if title_h > 0.0 {
                title_h + 16.0
            } else {
                GROUP_PAD
            };
            group.min_w = view.label.width + 2.0 * GROUP_PAD;
            group
        } else {
            if let Some(rows) = &node.json_rows {
                view.kind = Kind::Json;
                view.label = text(
                    Some(&node.label),
                    false,
                    font_size,
                    4000.0,
                    base | BOLD_STYLE,
                );
                let mut value_w = 0.0f64;
                for row in rows {
                    let key = text(Some(&row.key), false, small, 4000.0, base);
                    let value = text(Some(&row.value), false, small, 260.0, base);
                    view.key_w = js_max(view.key_w, key.width);
                    value_w = js_max(value_w, value.width);
                    view.h +=
                        js_max(js_max(key.height, value.height), js_round(small * 1.5)) + ROW_PAD;
                    view.rows.push((key, value));
                }
                if view.key_w > 0.0 {
                    view.key_w += 2.0 * CELL_PAD;
                }
                view.head_h = js_max(view.label.height, js_round(font_size * 1.5)) + ROW_PAD;
                view.w = js_max(
                    js_max(
                        view.label.width + 2.0 * CELL_PAD,
                        if view.rows.is_empty() {
                            0.0
                        } else {
                            view.key_w + value_w + 2.0 * CELL_PAD
                        },
                    ),
                    72.0,
                );
                view.h += view.head_h;
            } else if node.actor_type.is_some() {
                view.kind = Kind::Actor;
                view.label = text(Some(&node.label), markdown, font_size, 140.0, base);
                view.w = js_max(js_max(FIGURE_W, view.label.width), view.stereotype.width);
                view.h = FIGURE_H + FIGURE_GAP + view.stereotype.height + view.label.height;
            } else {
                let note_size = if node.shape == "note" {
                    small
                } else {
                    font_size
                };
                view.label = text(Some(&node.label), markdown, note_size, 180.0, base);
                let tw = js_max(view.label.width, view.stereotype.width);
                let th = view.label.height + view.stereotype.height;
                if node.shape == "usecaseEllipse" || node.shape == "usecaseBusiness" {
                    view.kind = Kind::Ellipse;
                    view.w = js_max(
                        tw + 2.0 * node.padding
                            + if node.business == Some(true) {
                                16.0
                            } else {
                                4.0
                            },
                        88.0,
                    );
                    view.h = js_max(th + 28.0, 48.0);
                } else if node.shape == "note" {
                    view.w = tw + 2.0 * node.padding + 6.0;
                    view.h = th + 18.0;
                } else {
                    view.w = js_max(tw + 2.0 * node.padding + 12.0, 80.0);
                    view.h = js_max(th + 24.0, 44.0);
                }
            }
            cnode(view.w, view.h, -1)
        };
        c.seq = views.len() as f64;
        views.push(view);
        cnodes.push(c);
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        cnodes[i].parent = match &node.parent_id {
            Some(parent) => index.get(parent).map_or(-1, |&p| p as i32),
            None => -1,
        };
    }

    let mut cedges: Vec<CEdge> = Vec::new();
    let mut edge_labels: Vec<Label> = Vec::new();
    let mut drawn: Vec<GraphEdge> = Vec::new();
    for edge in &graph.edges {
        let (Some(&src), Some(&dst)) = (index.get(&edge.start), index.get(&edge.end)) else {
            continue;
        };
        let semantic = edge.relationship_type == "include" || edge.relationship_type == "extend";
        let raw = if semantic {
            format!("«{}»", edge.label.clone().unwrap_or_default())
        } else {
            edge.label.clone().unwrap_or_default()
        };
        let label = text(
            if raw.is_empty() && !semantic && edge.label.is_none() {
                None
            } else {
                Some(raw.as_str())
            },
            edge.label_type == Some(LabelType::Markdown),
            small,
            200.0,
            0,
        );
        if src == dst {
            views[src].loops += 1;
            if sideways {
                cnodes[src].h += 2.0
                    * (LOOP
                        + if label.height > 0.0 {
                            label.height + 8.0
                        } else {
                            0.0
                        });
            } else {
                cnodes[src].w += 2.0
                    * (LOOP
                        + if label.width > 0.0 {
                            label.width + 12.0
                        } else {
                            0.0
                        });
            }
        }
        drawn.push(edge.clone());
        edge_labels.push(label.clone());
        cedges.push(CEdge {
            src,
            dst,
            minlen: edge.minlen,
            label_w: if label.width > 0.0 {
                label.width + 12.0
            } else {
                0.0
            },
            label_h: if label.height > 0.0 {
                label.height + 4.0
            } else {
                0.0
            },
            route: Vec::new(),
            label_x: 0.0,
            label_y: 0.0,
        });
    }

    let layout = compound_layout(
        &mut cnodes,
        &mut cedges,
        root_dir,
        &LayeredOptions {
            node_sep: number_option(config, "nodeSpacing", NODE_SEP),
            edge_sep: EDGE_SEP,
            rank_sep: number_option(config, "rankSpacing", RANK_SEP),
            port_sep: 20.0,
            ..LayeredOptions::default()
        },
    );

    let title = title_label(model.title.as_deref(), size, 4000.0);
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let inner = js_max(layout.width, title.width);
    let ox = pad + (inner - layout.width) / 2.0;
    let oy = pad + title_height;
    shift_layout(&mut cnodes, &mut cedges, ox, oy);
    let width = (inner + 2.0 * pad).ceil();
    let height = (layout.height + title_height + 2.0 * pad).ceil();

    let mut crossings: HashMap<i64, Vec<f64>> = HashMap::new();
    for e in &cedges {
        let mut k = 0;
        while k < e.route.len() {
            crossings
                .entry(js_round(e.route[k + 1]) as i64)
                .or_default()
                .push(e.route[k]);
            k += 3;
        }
    }
    attach_ends(&mut cedges, &cnodes, &views);

    let mut clusters = String::new();
    let mut fills: Vec<Fill> = Vec::new();
    let mut nodes_out = String::new();
    for (i, view) in views.iter().enumerate() {
        let c = &cnodes[i];
        let node = &view.node;
        let classes = class_names(&strip_default(&node.css_classes));
        let id = esc_text(&node.id);
        let text_attrs = view.style.text.clone();
        let shape = view.style.shape.clone();
        let color = ink_fill(&shape, &view.style.text, None);
        let muted = if color.is_empty() {
            " fill=\"var(--_m)\"".to_owned()
        } else {
            color.clone()
        };
        let w = view.w;
        let h = view.h;

        if view.kind == Kind::Boundary {
            let x = c.x - c.w / 2.0;
            let y = c.y - c.h / 2.0;
            if paints(&shape) {
                fills.push(Fill {
                    box_: [x, y, x + c.w, y + c.h],
                    shape: shape.clone(),
                });
            }
            let box_attrs = format!(
                " fill=\"var(--_a)\"{} stroke=\"var(--_b)\"{shape}",
                if sets_fill(&shape) {
                    ""
                } else {
                    " fill-opacity=\"0.5\""
                }
            );
            let title_attrs = format!(" class=\"pelers-cluster-label\"{muted}{text_attrs}");
            let body = if node.boundary_type == Some(BoundaryType::Package) {
                let tab_w = js_min(view.label.width + 2.0 * TAB_PAD + 8.0, c.w);
                let tab_h = view.label.height + 2.0 * TAB_PAD;
                format!(
                    "<rect class=\"pelers-boundary-tab\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{box_attrs}/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{box_attrs}/>{}",
                    num(x),
                    num(y),
                    num(tab_w),
                    num(tab_h),
                    num(x),
                    num(y + tab_h),
                    num(c.w),
                    num(c.h - tab_h),
                    label_svg_with(
                        &view.label,
                        x + tab_w / 2.0,
                        y + tab_h / 2.0,
                        &title_attrs,
                        icons
                    )
                )
            } else {
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>{}",
                    num(x),
                    num(y),
                    num(c.w),
                    num(c.h),
                    rounded(&box_attrs),
                    label_svg_with(
                        &view.label,
                        title_x(c, view.label.width, &crossings),
                        y + 8.0 + view.label.height / 2.0,
                        &title_attrs,
                        icons
                    )
                )
            };
            clusters.push_str(&format!(
                "<g class=\"pelers-cluster pelers-boundary{classes}\" data-id=\"{id}\">{body}</g>"
            ));
            continue;
        }

        let (kind, body) = match view.kind {
            Kind::Actor => {
                let top = -h / 2.0;
                let stereotype_y = top + FIGURE_H + FIGURE_GAP + view.stereotype.height / 2.0;
                let body = format!(
                    "<g class=\"pelers-actor-figure\" transform=\"translate(0,{})\">{}</g>{}{}",
                    num(top + FIGURE_H / 2.0),
                    figure(view, icons),
                    label_svg_with(
                        &view.stereotype,
                        0.0,
                        stereotype_y,
                        &format!(
                            " class=\"pelers-stereotype\" font-size=\"{}\" fill=\"var(--_m)\"",
                            js_number_to_string(tiny)
                        ),
                        icons
                    ),
                    label_svg_with(
                        &view.label,
                        0.0,
                        h / 2.0 - view.label.height / 2.0,
                        &format!(" class=\"pelers-label\"{text_attrs}"),
                        icons
                    )
                );
                ("pelers-actor", body)
            }
            Kind::Json => {
                let x0 = -w / 2.0;
                let mut y = -h / 2.0 + view.head_h;
                let mut cells = String::new();
                let mut lines = format!("M{},{}H{}", num(x0), num(y), num(x0 + w));
                if view.key_w > 0.0 {
                    lines.push_str(&format!(
                        "M{},{}V{}",
                        num(x0 + view.key_w),
                        num(y),
                        num(h / 2.0)
                    ));
                }
                for (key, value) in &view.rows {
                    let row_h =
                        js_max(js_max(key.height, value.height), js_round(small * 1.5)) + ROW_PAD;
                    cells.push_str(&label_svg_with(
                        key,
                        x0 + CELL_PAD + key.width / 2.0,
                        y + row_h / 2.0,
                        &format!(" class=\"pelers-json-key\"{muted}"),
                        icons,
                    ));
                    cells.push_str(&left_label(
                        value,
                        x0 + view.key_w + CELL_PAD,
                        y + row_h / 2.0,
                        &format!(" class=\"pelers-json-value\"{color}{text_attrs}"),
                        icons,
                    ));
                    y += row_h;
                }
                let body = format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/><path class=\"pelers-json-lines\" d=\"{lines}\" fill=\"none\" stroke=\"var(--_b)\"{}/>{}<g font-size=\"{}\">{cells}</g>",
                    num(x0),
                    num(-h / 2.0),
                    num(w),
                    num(h),
                    rounded(&format!("{SHAPE_ATTRS}{shape}")),
                    view.style.line,
                    label_svg_with(
                        &view.label,
                        0.0,
                        -h / 2.0 + view.head_h / 2.0,
                        &format!(
                            " class=\"pelers-label\" font-weight=\"var(--_hw)\"{color}{text_attrs}"
                        ),
                        icons
                    ),
                    js_number_to_string(small)
                );
                ("pelers-json", body)
            }
            _ => {
                let label_y = view.stereotype.height / 2.0;
                let (kind, outline) = match view.kind {
                    Kind::Ellipse => {
                        let mut outline = format!(
                            "<ellipse rx=\"{}\" ry=\"{}\"{SHAPE_ATTRS}{shape}/>",
                            num(w / 2.0),
                            num(h / 2.0)
                        );
                        if node.business == Some(true) {
                            let from = w / 2.0 - 15.0;
                            let to = w / 2.0 - 5.0;
                            let arc = |at: f64| {
                                (h / 2.0) * js_max(1.0 - (at / (w / 2.0)).powi(2), 0.0).sqrt()
                            };
                            outline.push_str(&format!(
                                "<path class=\"pelers-business\" d=\"M{},{}L{},{}\" fill=\"none\" stroke=\"var(--_b)\"{}/>",
                                num(from),
                                num(arc(from)),
                                num(to),
                                num(-arc(to)),
                                view.style.line
                            ));
                        }
                        ("pelers-usecase", outline)
                    }
                    _ if node.shape == "note" => {
                        let r = w / 2.0;
                        let b = h / 2.0;
                        let outline = format!(
                            "<path d=\"M{},{}H{}L{},{}V{}H{}Z\" fill=\"var(--_a)\" stroke=\"var(--_b)\" stroke-linejoin=\"round\"{shape}/><path d=\"M{},{}V{}H{}\" fill=\"none\" stroke=\"var(--_b)\" stroke-linejoin=\"round\"{}/>",
                            num(-r),
                            num(-b),
                            num(r - FOLD),
                            num(r),
                            num(FOLD - b),
                            num(b),
                            num(-r),
                            num(r - FOLD),
                            num(-b),
                            num(FOLD - b),
                            num(r),
                            view.style.line
                        );
                        ("pelers-note", outline)
                    }
                    _ => (
                        "pelers-usecase",
                        format!(
                            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>",
                            num(-w / 2.0),
                            num(-h / 2.0),
                            num(w),
                            num(h),
                            rounded(&format!("{SHAPE_ATTRS}{shape}"))
                        ),
                    ),
                };
                let note_font = if node.shape == "note" {
                    format!(" font-size=\"{}\"", js_number_to_string(small))
                } else {
                    String::new()
                };
                let body = format!(
                    "{}{}{}",
                    outline,
                    label_svg_with(
                        &view.stereotype,
                        0.0,
                        -view.label.height / 2.0,
                        &format!(
                            " class=\"pelers-stereotype\" font-size=\"{}\"{muted}",
                            js_number_to_string(tiny)
                        ),
                        icons
                    ),
                    label_svg_with(
                        &view.label,
                        0.0,
                        label_y,
                        &format!(" class=\"pelers-label\"{note_font}{color}{text_attrs}"),
                        icons
                    )
                );
                (kind, body)
            }
        };
        nodes_out.push_str(&format!(
            "<g class=\"pelers-node {kind}{classes}\" data-id=\"{id}\" transform=\"translate({},{})\">{body}</g>",
            num(c.x),
            num(c.y)
        ));
    }

    let mut casings = String::new();
    let mut edges_out = String::new();
    let mut labels_out = String::new();
    let mut loop_seen: HashMap<usize, usize> = HashMap::new();
    for i in 0..cedges.len() {
        let edge = drawn[i].clone();
        let label = edge_labels[i].clone();
        let style = if !edge.style.is_empty() || !edge.css_compiled_styles.is_empty() {
            let mut declarations = edge.css_compiled_styles.clone();
            declarations.extend(edge.style.iter().cloned());
            Some(resolve_style(&declarations))
        } else {
            None
        };
        let start_type = edge.arrow_type_start;
        let end_type = edge.arrow_type_end;
        let src = cedges[i].src;
        let dst = cedges[i].dst;
        let path: EdgePath;
        if src == dst {
            let c = cnodes[src].clone();
            let view = views[src].clone();
            let k = *loop_seen.get(&src).unwrap_or(&0);
            loop_seen.insert(src, k + 1);
            let side = if sideways { 2 } else { 1 };
            let spread = js_min(side_length(&view, side) / 2.0, 8.0 + k as f64 * 6.0);
            let reach = LOOP + k as f64 * 8.0;
            let (ax, ay) = anchor(&view, &c, side, -spread);
            let (bx, by) = anchor(&view, &c, side, spread);
            let len = {
                let v = reach.hypot(spread);
                if v == 0.0 || v.is_nan() { 1.0 } else { v }
            };
            let ux = if sideways { spread / len } else { reach / len };
            let uy = if sideways { reach / len } else { spread / len };
            let st = extension_trim(start_type);
            let et = extension_trim(end_type);
            let out = |x: f64, y: f64, sign: f64, far: f64| -> String {
                if sideways {
                    let f = if far != 0.0 { 1.0 } else { 0.0 };
                    format!("{},{}", num(x + sign * spread * f), num(y + far))
                } else {
                    let f = if far != 0.0 { 1.0 } else { 0.0 };
                    format!("{},{}", num(x + far), num(y + sign * spread * f))
                }
            };
            let sx = if sideways { ax - ux * st } else { ax + ux * st };
            let sy = if sideways { ay + uy * st } else { ay - uy * st };
            let ex = bx + ux * et;
            let ey = by + uy * et;
            path = EdgePath {
                d: format!(
                    "M{},{}C{} {} {},{}",
                    num(sx),
                    num(sy),
                    out(ax, ay, -1.0, reach),
                    out(bx, by, 1.0, reach),
                    num(ex),
                    num(ey)
                ),
                tokens: None,
                sx: ax,
                sy: ay,
                sdx: if sideways { ux } else { -ux },
                sdy: if sideways { -uy } else { uy },
                ex: bx,
                ey: by,
                edx: -ux,
                edy: -uy,
            };
            if sideways {
                cedges[i].label_x = c.x;
                cedges[i].label_y = js_max(ay, by) + reach * 0.75 + 6.0 + label.height / 2.0;
            } else {
                cedges[i].label_x = js_max(ax, bx) + reach * 0.75 + 8.0 + label.width / 2.0;
                cedges[i].label_y = c.y;
            }
        } else {
            if cedges[i].route.len() < 6 {
                continue;
            }
            path = route_path(
                &cedges[i].route,
                None,
                extension_trim(start_type),
                extension_trim(end_type),
                None,
                None,
            );
        }

        let tips = [path.sx, path.sy, path.ex, path.ey];
        let line = style.as_ref().map(|s| s.line.clone()).unwrap_or_default();
        let fill_result = fill_under(&path.d, &tips, &fills, path.tokens.as_deref());
        let (over, holder): (Option<String>, Option<usize>) = match fill_result {
            FillUnder::Off => (None, None),
            FillUnder::Crosses => (Some(String::new()), None),
            FillUnder::Holder(idx) => (Some(edge_ink(&fills[idx], &line)), Some(idx)),
        };
        let over_text = over.clone().unwrap_or_default();
        let color = if !over_text.is_empty() {
            over_text.clone()
        } else {
            match style.as_ref().and_then(|s| s.stroke.as_ref()) {
                Some(stroke) => esc(stroke).into_owned(),
                None => "var(--_l)".to_owned(),
            }
        };
        if over.as_deref() == Some("") {
            let bg = "var(--_bg)";
            let markers = format!(
                "{}{}",
                mark(start_type, path.sx, path.sy, path.sdx, path.sdy, bg),
                mark(end_type, path.ex, path.ey, path.edx, path.edy, bg)
            );
            casings.push_str(&casing(&path.d, &markers, &line, false));
        }
        let dashes = if edge.internal {
            "2 3"
        } else if edge.pattern == "dotted" {
            "5 4"
        } else if edge.animate {
            "9 5"
        } else {
            ""
        };
        let marks = format!(
            "{}{}",
            mark(start_type, path.sx, path.sy, path.sdx, path.sdy, &color),
            mark(end_type, path.ex, path.ey, path.edx, path.edy, &color)
        );
        let marks_out = match (holder, over.as_deref()) {
            (Some(idx), Some(over_value)) if !over_value.is_empty() => {
                hollow_on(&marks, &fills[idx])
            }
            _ => marks,
        };
        let stroke_attr = if over_text.is_empty() {
            String::new()
        } else {
            format!(" stroke=\"{over_text}\"")
        };
        let style_line = style.as_ref().map(|s| s.line.clone()).unwrap_or_default();
        let class_extra = class_names(&strip_relationship_defaults(&edge.classes));
        edges_out.push_str(&format!(
            "<g class=\"pelers-edge pelers-{}{class_extra}\" data-id=\"{}\"{stroke_attr}{style_line}><path d=\"{}\"{}/>{marks_out}</g>",
            edge.relationship_type,
            esc_text(&edge.id),
            path.d,
            if dashes.is_empty() {
                String::new()
            } else {
                format!(" stroke-dasharray=\"{dashes}\"")
            }
        ));
        if label.width > 0.0 {
            let semantic =
                edge.relationship_type == "include" || edge.relationship_type == "extend";
            let extra = format!(
                "{}{}",
                if semantic { " fill=\"var(--_m)\"" } else { "" },
                style.as_ref().map(|s| s.text.clone()).unwrap_or_default()
            );
            labels_out.push_str(&edge_label_svg(
                &edge.id,
                &label,
                cedges[i].label_x,
                cedges[i].label_y,
                &extra,
                icons,
            ));
        }
    }

    let content = format!(
        "{}{}{}{}{}<g class=\"pelers-nodes\">{nodes_out}</g>",
        label_svg(
            &title,
            width / 2.0,
            pad + title.height / 2.0,
            " class=\"pelers-title\" font-weight=\"var(--_tw)\""
        ),
        if clusters.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pelers-clusters\">{clusters}</g>")
        },
        casings_svg(&casings),
        if edges_out.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{edges_out}</g>"
            )
        },
        if labels_out.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-edge-labels\" font-size=\"{}\">{labels_out}</g>",
                js_number_to_string(small)
            )
        },
    );
    let svg = svg_document(
        "usecase",
        width,
        height,
        size,
        options,
        Accessible {
            acc_title: Some(model.acc_title.as_str()).filter(|t| !t.is_empty()),
            acc_descr: Some(model.acc_descr.as_str()).filter(|t| !t.is_empty()),
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

fn strip_default(raw: &str) -> String {
    match raw.strip_prefix("default") {
        Some(rest) => {
            let mut chars = rest.chars();
            match chars.next() {
                Some(c) if is_js_space(c) => chars.as_str().to_owned(),
                _ => rest.to_owned(),
            }
        }
        None => raw.to_owned(),
    }
}

fn strip_relationship_defaults(raw: &str) -> String {
    const PREFIX: &str = "default relationship relationship-";
    match raw.strip_prefix(PREFIX) {
        Some(rest) => {
            let after = rest.trim_start_matches(|c: char| !is_js_space(c));
            let mut out = after.to_owned();
            if let Some(first) = out.chars().next()
                && is_js_space(first)
            {
                out.remove(0);
            }
            out
        }
        None => raw.to_owned(),
    }
}

fn title_x(c: &CNode, width: f64, crossings: &HashMap<i64, Vec<f64>>) -> f64 {
    let Some(xs) = crossings.get(&(js_round(c.y - c.h / 2.0) as i64)) else {
        return c.x;
    };
    if width == 0.0 {
        return c.x;
    }
    let left = c.x - c.w / 2.0 + 6.0;
    let right = c.x + c.w / 2.0 - 6.0;
    let mut cuts: Vec<f64> = xs
        .iter()
        .copied()
        .filter(|&x| x > left && x < right)
        .collect();
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cuts.push(right);
    let need = width / 2.0 + 6.0;
    let mut best = c.x;
    let mut best_off = f64::INFINITY;
    let mut widest = 0.0f64;
    let mut from = left;
    for cut in cuts {
        if cut - from >= 2.0 * need {
            let x = js_min(js_max(c.x, from + need), cut - need);
            if (x - c.x).abs() < best_off {
                best_off = (x - c.x).abs();
                best = x;
            }
        } else if best_off == f64::INFINITY && cut - from > widest {
            widest = cut - from;
            best = (from + cut) / 2.0;
        }
        from = cut;
    }
    best
}

fn left_label(label: &Label, x: f64, cy: f64, attrs: &str, icons: bool) -> String {
    if label.lines.len() < 2 {
        return label_svg_with(label, x + label.width / 2.0, cy, attrs, icons);
    }
    let mut out = String::new();
    for i in 0..label.lines.len() {
        out.push_str(&label_svg_with(
            &label_line(label, i),
            x + label.widths[i] / 2.0,
            cy - label.height / 2.0 + line_middle(label, i),
            attrs,
            icons,
        ));
    }
    out
}
