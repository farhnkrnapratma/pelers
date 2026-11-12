use std::collections::HashMap;

use super::db::RequirementModel;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{runs_across, turn_to_fit};
use crate::layout::compound::compound_layout;
use crate::layout::{CEdge, CNode, Dir, LayeredOptions, cnode, direction};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, num, shared_style, span_style, text_style};
use crate::svg::edges::{edge_label_svg, route_path};
use crate::svg::ink::ink_fill;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ResolvedStyle, class_names, resolve_style, rounded};
use crate::svg::title::title_label;
use crate::text::label::{Label, has_math, label_line, layout_label, line_middle};
use crate::text::measurer::{BOLD_STYLE, ITALIC, width as measure};
use crate::util::jsnum::{js_max, js_min, js_round};

const NODE_SEP: f64 = 40.0;
const EDGE_SEP: f64 = 16.0;
const RANK_SEP: f64 = 56.0;
const PAD_X: f64 = 14.0;
const PAD_Y: f64 = 10.0;
const ROW_GAP: f64 = 2.0;
const COLUMN_GAP: f64 = 12.0;
const MIN_WIDTH: f64 = 120.0;
const NAME_WRAP: f64 = 280.0;
const VALUE_WRAP: f64 = 240.0;
const LOOP: f64 = 26.0;
const CONTAINS_RADIUS: f64 = 6.0;
const ARROW_LENGTH: f64 = 9.0;
const ARROW_HALF: f64 = 4.5;
const END_STEP: f64 = 28.0;
const END_MARGIN: f64 = 14.0;

#[derive(Clone)]
struct GraphNode {
    id: String,
    requirement: Option<usize>,
    element: Option<usize>,
    css_styles: Vec<String>,
    css_classes: String,
}

#[derive(Clone)]
struct GraphEdge {
    id: String,
    start: Option<String>,
    end: Option<String>,
    type_: String,
    contains: bool,
}

fn build_graph(model: &RequirementModel) -> (Vec<GraphNode>, Vec<GraphEdge>) {
    let mut nodes: Vec<GraphNode> = Vec::new();
    for (i, requirement) in model.requirements.iter().enumerate() {
        nodes.push(GraphNode {
            id: requirement.name.clone(),
            requirement: Some(i),
            element: None,
            css_styles: requirement.css_styles.clone(),
            css_classes: requirement.classes.join(" "),
        });
    }
    for (i, element) in model.elements.iter().enumerate() {
        nodes.push(GraphNode {
            id: element.name.clone(),
            requirement: None,
            element: Some(i),
            css_styles: element.css_styles.clone(),
            css_classes: element.classes.join(" "),
        });
    }
    let has = |id: &str| -> Option<String> {
        let known = model.requirements.iter().any(|r| r.name == id)
            || model.elements.iter().any(|e| e.name == id);
        known.then(|| id.to_owned())
    };
    let edges = model
        .relations
        .iter()
        .enumerate()
        .map(|(index, relation)| GraphEdge {
            id: format!("{}-{}-{index}", relation.src, relation.dst),
            start: has(&relation.src),
            end: has(&relation.dst),
            type_: relation.type_.clone(),
            contains: relation.type_ == "contains",
        })
        .collect();
    (nodes, edges)
}

struct Row {
    key: Label,
    value: Label,
    height: f64,
}

struct NodeView {
    node: usize,
    style: ResolvedStyle,
    kind: Label,
    name: Label,
    rows: Vec<Row>,
    key_width: f64,
    head_height: f64,
    w: f64,
    h: f64,
}

fn restyle(label: Label, bits: u32) -> Label {
    if bits == 0 || label.lines.is_empty() {
        return label;
    }
    let mut width = 0.0f64;
    let mut widths: Vec<f64> = Vec::new();
    let mut lines = Vec::with_capacity(label.lines.len());
    for line in &label.lines {
        let mut total = 0.0f64;
        let mut spans = Vec::with_capacity(line.len());
        for span in line {
            let style = span.style | bits;
            let w = if span.icon.is_some() || span.math.is_some() {
                span.width
            } else {
                measure(&span.text, label.size, style)
            };
            total += w;
            let mut copy = span.clone();
            copy.style = style;
            copy.width = w;
            spans.push(copy);
        }
        widths.push(total);
        if total > width {
            width = total;
        }
        lines.push(spans);
    }
    Label {
        lines,
        widths,
        width,
        ..label
    }
}

fn start_text(label: &Label, x: f64, top: f64, attrs: &str) -> String {
    let shared = shared_style(label);
    let mut body = String::new();
    let mut math = String::new();
    for i in 0..label.lines.len() {
        let line = &label.lines[i];
        let middle = top + line_middle(label, i);
        if has_math(line) {
            math.push_str(&label_svg(
                &label_line(label, i),
                x + label.widths[i] / 2.0,
                middle,
                attrs,
            ));
            continue;
        }
        let y = middle + label.size * 0.35;
        let mut first = true;
        for span in line {
            if span.icon.is_some() || span.text.is_empty() {
                continue;
            }
            let position = if first {
                format!(" x=\"{}\" y=\"{}\"", num(x), num(y))
            } else {
                String::new()
            };
            body.push_str(&format!(
                "<tspan{position}{}>{}</tspan>",
                span_style(span.style & !shared),
                esc(&span.text)
            ));
            first = false;
        }
    }
    let text = if body.is_empty() {
        String::new()
    } else {
        format!(
            "<text{} text-anchor=\"start\" xml:space=\"preserve\">{body}</text>",
            text_style(attrs, shared)
        )
    };
    text + &math
}

fn contains_marker(x: f64, y: f64, dx: f64, dy: f64) -> String {
    let r = CONTAINS_RADIUS;
    let cx = x - dx * r;
    let cy = y - dy * r;
    format!(
        "<circle class=\"pelers-marker\" cx=\"{}\" cy=\"{}\" r=\"{}\"/><path class=\"pelers-marker\" d=\"M{},{}L{},{}M{},{}L{},{}\"/>",
        num(cx),
        num(cy),
        num(r),
        num(cx - dx * r),
        num(cy - dy * r),
        num(cx + dx * r),
        num(cy + dy * r),
        num(cx + dy * r),
        num(cy - dx * r),
        num(cx - dy * r),
        num(cy + dx * r)
    )
}

fn arrow_marker(x: f64, y: f64, dx: f64, dy: f64) -> String {
    let bx = x - dx * ARROW_LENGTH;
    let by = y - dy * ARROW_LENGTH;
    format!(
        "<path class=\"pelers-marker\" d=\"M{},{}L{},{}L{},{}\" stroke-linejoin=\"round\"/>",
        num(bx - dy * ARROW_HALF),
        num(by + dx * ARROW_HALF),
        num(x),
        num(y),
        num(bx + dy * ARROW_HALF),
        num(by - dx * ARROW_HALF)
    )
}

#[derive(Clone, Copy)]
struct End {
    edge: usize,
    at: usize,
    toward: f64,
}

fn spread_ends(cnodes: &mut [CNode], cedges: &mut [CEdge], views: &[NodeView]) {
    let mut sides: Vec<(usize, Vec<End>)> = Vec::new();
    let mut index: HashMap<usize, usize> = HashMap::new();
    for (edge_index, edge) in cedges.iter().enumerate() {
        let route_len = edge.route.len();
        if edge.src == edge.dst || route_len < 6 {
            continue;
        }
        for at in [0, route_len - 3] {
            let node = if at == 0 { edge.src } else { edge.dst };
            let c = &cnodes[node];
            let dx = edge.route[at] - c.x;
            let dy = edge.route[at + 1] - c.y;
            let next = if at == 0 { 3 } else { at - 3 };
            let side = if (dy.abs() - c.h / 2.0).abs() < 0.5 {
                if dy < 0.0 { 0 } else { 2 }
            } else if (dx.abs() - c.w / 2.0).abs() < 0.5 {
                if dx < 0.0 { 3 } else { 1 }
            } else {
                continue;
            };
            let key = node * 4 + side;
            let toward = edge.route[if side % 2 == 0 { next } else { next + 1 }];
            let end = End {
                edge: edge_index,
                at,
                toward,
            };
            match index.get(&key) {
                Some(&slot) => sides[slot].1.push(end),
                None => {
                    index.insert(key, sides.len());
                    sides.push((key, vec![end]));
                }
            }
        }
    }
    for (key, mut list) in sides {
        if list.len() < 2 {
            continue;
        }
        let node = key >> 2;
        let vertical = key % 2 == 1;
        let span = (if vertical {
            views[node].h
        } else {
            views[node].w
        }) - 2.0 * END_MARGIN;
        let step = js_max(0.0, js_min(END_STEP, span / (list.len() as f64 - 1.0)));
        list.sort_by(|a, b| {
            a.toward
                .partial_cmp(&b.toward)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for (j, end) in list.iter().enumerate() {
            let offset = (j as f64 - (list.len() as f64 - 1.0) / 2.0) * step;
            if vertical {
                cedges[end.edge].route[end.at + 1] = cnodes[node].y + offset;
            } else {
                cedges[end.edge].route[end.at] = cnodes[node].x + offset;
            }
        }
    }
}

fn strip_default(raw: &str) -> &str {
    match raw.strip_prefix("default") {
        Some(rest) => match rest.chars().next() {
            Some(c) if crate::util::chars::is_js_space(c) => &rest[c.len_utf8()..],
            _ => rest,
        },
        None => raw,
    }
}

pub fn render_requirement(
    model: &RequirementModel,
    config: &Config,
    options: &RenderOptions,
) -> Drawn {
    if !runs_across(Some(&model.direction)) {
        return draw(model, config, options, false);
    }
    turn_to_fit(options, |down| draw(model, config, options, down))
}

fn draw(
    model: &RequirementModel,
    _config: &Config,
    options: &RenderOptions,
    turned: bool,
) -> Drawn {
    let (graph_nodes, graph_edges) = build_graph(model);
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);

    let mut index: HashMap<String, usize> = HashMap::new();
    let mut views: Vec<NodeView> = Vec::new();
    let mut cnodes: Vec<CNode> = Vec::new();
    for node in &graph_nodes {
        let style = resolve_style(&node.css_styles);
        let base =
            (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let name_size = style.font_size.unwrap_or(size);
        let body_size = style.font_size.unwrap_or(small);
        let requirement = node.requirement.map(|i| &model.requirements[i]);
        let element = node.element.map(|i| &model.elements[i]);
        let kind_text = format!("«{}»", requirement.map_or("Element", |r| r.type_.as_str()));
        let kind = restyle(
            layout_label(Some(&kind_text), false, body_size, 4000.0, 0),
            base,
        );
        let name = restyle(
            layout_label(Some(&node.id), true, name_size, NAME_WRAP, 0),
            base | BOLD_STYLE,
        );
        let fields: Vec<(&str, String)> = match requirement {
            Some(r) => vec![
                ("ID", r.requirement_id.clone()),
                ("Text", r.text.clone()),
                ("Risk", r.risk.clone()),
                ("Verification", r.verify_method.clone()),
            ],
            None => {
                let e = element.expect("a node is a requirement or an element");
                vec![("Type", e.type_.clone()), ("Doc Ref", e.doc_ref.clone())]
            }
        };
        let mut rows: Vec<Row> = Vec::new();
        let mut key_width = 0.0f64;
        let mut value_width = 0.0f64;
        let mut body_height = 0.0f64;
        for (label, text) in fields {
            if text.is_empty() {
                continue;
            }
            let key = restyle(layout_label(Some(label), false, body_size, 4000.0, 0), base);
            let value = restyle(
                layout_label(Some(&text), true, body_size, VALUE_WRAP, 0),
                base,
            );
            let height = js_max(key.height, value.height);
            key_width = js_max(key_width, key.width);
            value_width = js_max(value_width, value.width);
            body_height += height + ROW_GAP;
            rows.push(Row { key, value, height });
        }
        if !rows.is_empty() {
            body_height += 2.0 * PAD_Y - ROW_GAP;
        }
        let head_height = 2.0 * PAD_Y + kind.height + name.height;
        let w = js_max(
            js_max(MIN_WIDTH, js_max(kind.width, name.width) + 2.0 * PAD_X),
            if rows.is_empty() {
                0.0
            } else {
                key_width + COLUMN_GAP + value_width + 2.0 * PAD_X
            },
        );
        index.entry(node.id.clone()).or_insert(views.len());
        let h = head_height + body_height;
        let mut c = cnode(w, h, -1);
        c.seq = views.len() as f64;
        cnodes.push(c);
        views.push(NodeView {
            node: views.len(),
            style,
            kind,
            name,
            rows,
            key_width,
            head_height,
            w,
            h,
        });
    }

    let root_dir = if turned {
        Dir::TB
    } else {
        direction(Some(&model.direction))
    };
    let sideways = matches!(root_dir, Dir::LR | Dir::RL);
    let mut cedges: Vec<CEdge> = Vec::new();
    let mut edge_labels: Vec<Label> = Vec::new();
    let mut drawn: Vec<&GraphEdge> = Vec::new();
    for edge in &graph_edges {
        let src = edge.start.as_ref().and_then(|s| index.get(s).copied());
        let dst = edge.end.as_ref().and_then(|s| index.get(s).copied());
        let (Some(src), Some(dst)) = (src, dst) else {
            continue;
        };
        let label = layout_label(Some(&format!("«{}»", edge.type_)), false, small, 4000.0, 0);
        if src == dst {
            if sideways {
                cnodes[src].h += 2.0 * (LOOP + label.height + 8.0);
            } else {
                cnodes[src].w += 2.0 * (LOOP + label.width + 12.0);
            }
        }
        drawn.push(edge);
        cedges.push(CEdge {
            src,
            dst,
            minlen: 1.0,
            label_w: label.width + 12.0,
            label_h: label.height + 4.0,
            route: Vec::new(),
            label_x: 0.0,
            label_y: 0.0,
        });
        edge_labels.push(label);
    }

    let layout = compound_layout(
        &mut cnodes,
        &mut cedges,
        root_dir,
        &LayeredOptions {
            node_sep: NODE_SEP,
            edge_sep: EDGE_SEP,
            rank_sep: RANK_SEP,
            port_sep: 20.0,
            ..LayeredOptions::default()
        },
    );

    spread_ends(&mut cnodes, &mut cedges, &views);

    let title = title_label(model.title.as_deref(), size, 4000.0);
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let inner = js_max(layout.width, title.width);
    let ox = pad + (inner - layout.width) / 2.0;
    let oy = pad + title_height;
    let width = (inner + 2.0 * pad).ceil();
    let height = (layout.height + title_height + 2.0 * pad).ceil();

    let mut nodes_out = String::new();
    for (i, view) in views.iter().enumerate() {
        let node = &graph_nodes[view.node];
        let style = &view.style;
        let w = view.w;
        let h = view.h;
        let color = ink_fill(&style.shape, &style.text, None);
        let muted = if color.is_empty() {
            " fill=\"var(--_m)\"".to_owned()
        } else {
            color.clone()
        };
        let left = -w / 2.0;
        let top = -h / 2.0;
        let mut body = format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{}/>",
            num(left),
            num(top),
            num(w),
            num(h),
            rounded(&style.shape)
        );
        body.push_str(&label_svg(
            &view.kind,
            0.0,
            top + PAD_Y + view.kind.height / 2.0,
            &format!(
                " class=\"pelers-stereotype\" font-size=\"{}\"{muted}{}",
                crate::util::jsnum::js_number_to_string(view.kind.size),
                style.text
            ),
        ));
        body.push_str(&label_svg(
            &view.name,
            0.0,
            top + PAD_Y + view.kind.height + view.name.height / 2.0,
            &format!(
                " class=\"pelers-label\" font-size=\"{}\" font-weight=\"var(--_hw)\"{color}{}",
                crate::util::jsnum::js_number_to_string(view.name.size),
                style.text
            ),
        ));
        if !view.rows.is_empty() {
            let divider = top + view.head_height;
            body.push_str(&format!(
                "<path class=\"pelers-divider\" d=\"M{},{}H{}\" stroke=\"var(--_b)\"{}/>",
                num(left),
                num(divider),
                num(-left),
                style.line
            ));
            let mut y = divider + PAD_Y;
            let mut rows_out = String::new();
            for row in &view.rows {
                rows_out.push_str(&start_text(
                    &row.key,
                    left + PAD_X,
                    y,
                    &format!(" class=\"pelers-field\"{muted}{}", style.text),
                ));
                rows_out.push_str(&start_text(
                    &row.value,
                    left + PAD_X + view.key_width + COLUMN_GAP,
                    y,
                    &format!(" class=\"pelers-value\"{color}{}", style.text),
                ));
                y += row.height + ROW_GAP;
            }
            body.push_str(&format!(
                "<g class=\"pelers-fields\" font-size=\"{}\">{rows_out}</g>",
                crate::util::jsnum::js_number_to_string(view.rows[0].key.size)
            ));
        }
        let class_kind = if node.requirement.is_some() {
            "pelers-requirement"
        } else {
            "pelers-element"
        };
        nodes_out.push_str(&format!(
            "<g class=\"pelers-node {class_kind}{}\" data-id=\"{}\" transform=\"translate({},{})\">{body}</g>",
            class_names(strip_default(&node.css_classes)),
            esc_text(&node.id),
            num(cnodes[i].x + ox),
            num(cnodes[i].y + oy)
        ));
    }

    let mut edges_out = String::new();
    let mut labels_out = String::new();
    let mut loop_seen: HashMap<usize, usize> = HashMap::new();
    for i in 0..cedges.len() {
        let ce = cedges[i].clone();
        let edge = drawn[i];
        let label = &edge_labels[i];
        let start_trim = if edge.contains {
            2.0 * CONTAINS_RADIUS
        } else {
            0.0
        };
        let mut label_x = ce.label_x + ox;
        let mut label_y = ce.label_y + oy;
        let path = if ce.src == ce.dst {
            let view = &views[ce.src];
            let cx = cnodes[ce.src].x + ox;
            let cy = cnodes[ce.src].y + oy;
            let k = *loop_seen.get(&ce.src).unwrap_or(&0);
            loop_seen.insert(ce.src, k + 1);
            let half = (if sideways { view.w } else { view.h }) / 2.0;
            let spread = js_min(half - 4.0, 12.0 + k as f64 * 6.0);
            let reach = LOOP + k as f64 * 8.0;
            let at = |out: f64, along: f64| -> String {
                if sideways {
                    format!("{},{}", num(cx + along), num(cy + view.h / 2.0 + out))
                } else {
                    format!("{},{}", num(cx + view.w / 2.0 + out), num(cy + along))
                }
            };
            let pull = reach / 0.75;
            let d = format!(
                "M{}C{} {} {}",
                at(start_trim, -spread),
                at(pull, -spread),
                at(pull, spread),
                at(0.0, spread)
            );
            if sideways {
                label_x = cx;
                label_y = cy + view.h / 2.0 + reach + 4.0 + ce.label_h / 2.0;
            } else {
                label_x = cx + view.w / 2.0 + reach + 4.0 + ce.label_w / 2.0;
                label_y = cy;
            }
            EdgePathOut {
                d,
                sx: if sideways {
                    cx - spread
                } else {
                    cx + view.w / 2.0
                },
                sy: if sideways {
                    cy + view.h / 2.0
                } else {
                    cy - spread
                },
                sdx: if sideways { 0.0 } else { -1.0 },
                sdy: if sideways { -1.0 } else { 0.0 },
                ex: if sideways {
                    cx + spread
                } else {
                    cx + view.w / 2.0
                },
                ey: if sideways {
                    cy + view.h / 2.0
                } else {
                    cy + spread
                },
                edx: if sideways { 0.0 } else { -1.0 },
                edy: if sideways { -1.0 } else { 0.0 },
            }
        } else {
            let mut route = cedges[i].route.clone();
            let mut k = 0;
            while k < route.len() {
                route[k] += ox;
                if k + 1 < route.len() {
                    route[k + 1] += oy;
                }
                k += 3;
            }
            let routed = route_path(&route, None, start_trim, 0.0, None, None);
            EdgePathOut {
                d: routed.d,
                sx: routed.sx,
                sy: routed.sy,
                sdx: routed.sdx,
                sdy: routed.sdy,
                ex: routed.ex,
                ey: routed.ey,
                edx: routed.edx,
                edy: routed.edy,
            }
        };
        edges_out.push_str(&format!(
            "<g class=\"pelers-edge pelers-relationship-{}\" data-id=\"{}\"><path d=\"{}\"{}/>{}</g>",
            if edge.contains { "contains" } else { "arrow" },
            esc_text(&edge.id),
            path.d,
            if edge.contains {
                ""
            } else {
                " stroke-dasharray=\"5 4\""
            },
            if edge.contains {
                contains_marker(path.sx, path.sy, path.sdx, path.sdy)
            } else {
                arrow_marker(path.ex, path.ey, path.edx, path.edy)
            }
        ));
        labels_out.push_str(&edge_label_svg(
            &edge.id, label, label_x, label_y, "", false,
        ));
    }

    let mut content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    if !edges_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{edges_out}</g>"
        ));
    }
    if !labels_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-edge-labels\" font-size=\"{}\" fill=\"var(--_m)\">{labels_out}</g>",
            crate::util::jsnum::js_number_to_string(small)
        ));
    }
    content.push_str(&format!("<g class=\"pelers-nodes\">{nodes_out}</g>"));

    Drawn {
        svg: svg_document(
            "requirement",
            width,
            height,
            size,
            options,
            Accessible {
                acc_title: model.acc_title.as_deref(),
                acc_descr: model.acc_descr.as_deref(),
            },
            &content,
        ),
        width,
        height,
        links: Vec::new(),
    }
}

struct EdgePathOut {
    d: String,
    sx: f64,
    sy: f64,
    sdx: f64,
    sdy: f64,
    ex: f64,
    ey: f64,
    edx: f64,
    edy: f64,
}
