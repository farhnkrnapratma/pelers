use std::collections::HashMap;
use std::fmt::Write as _;

use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{runs_across, tighten, title_room, turn_to_fit};
use crate::diagrams::flowchart::db::FlowDb;
use crate::diagrams::flowchart::graph::{FlowGraph, GraphEdge, GraphNode, build_flow_graph};
use crate::diagrams::flowchart::shapes::canonical_shape;
use crate::diagrams::flowchart::types::LabelType;
use crate::layout::compound::{compound_layout, inherit_crossed_directions, shift_layout};
use crate::layout::{CEdge, CNode, CompoundResult, Dir, LayeredOptions, cnode, direction};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, label_svg_with, num};
use crate::svg::cluster::{
    Crossings, cluster_title_x, mark_crossings, struck_title, title_crossed,
};
use crate::svg::edges::{
    EdgePath, Fill, LoopEdge, LoopNode, SideReach, casings_svg, edge_casing, edge_label_svg,
    loop_path, marker, marker_trim, over_fills, route_path,
};
use crate::svg::ink::{ink_fill, paints};
use crate::svg::options::LinkInfo;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::shapes::{
    View, draw_shape, end_side, inset_route, shape_filled, shape_has_label, shape_size, shape_span,
};
use crate::svg::theme::{ResolvedStyle, class_names, resolve_style, rounded, sets_fill};
use crate::svg::title::title_label;
use crate::text::entities::decode_entities;
use crate::text::icons::{icon_markup, with_icons};
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, ITALIC};
use crate::util::chars::is_js_space;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::url::{image_url, internal_link, linked, rel_attr, safe_url};
use crate::util::value::{Object, Value};

pub type FlowConfig = Config;

pub struct FlowVariant {
    pub kind: &'static str,
    pub prepare: fn(&mut FlowGraph),
    pub layout: fn(&mut [CNode], &mut [CEdge], Dir, &LayeredOptions, bool) -> CompoundResult,
    pub draw_group: fn(&CNode, &Label, &ResolvedStyle, &str, &str, Dir, bool) -> String,
    pub filled_box: fn(&CNode, Dir) -> [f64; 4],
}

const NODE_SEP: f64 = 40.0;
const EDGE_SEP: f64 = 16.0;
const RANK_SEP: f64 = 48.0;
const GROUP_PAD: f64 = 20.0;
const LOOP: f64 = 26.0;
const SHAPE_ATTRS: &str = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";
const LINE_ATTRS: &str = " stroke=\"var(--_b)\"";

struct NodeView<'a> {
    node: &'a GraphNode,
    style: ResolvedStyle,
    label: Label,
    shape: &'static str,
    w: f64,
    h: f64,
    dy: f64,
}

fn present(text: Option<&str>) -> bool {
    text.is_some_and(|t| !t.is_empty())
}

fn number_option(flow: Option<&Object>, key: &str, fallback: f64) -> f64 {
    match flow.and_then(|section| section.get(key)) {
        Some(Value::Number(value)) if *value > 0.0 => value.min(2000.0),
        _ => fallback,
    }
}

fn asset_size(value: Option<f64>, fallback: f64) -> f64 {
    match value {
        Some(value) if value > 0.0 && value <= 4000.0 => value,
        _ => fallback,
    }
}

fn strip_default(raw: &str) -> &str {
    match raw.strip_prefix("default") {
        Some(rest) => match rest.chars().next() {
            Some(c) if is_js_space(c) => &rest[c.len_utf8()..],
            _ => rest,
        },
        None => raw,
    }
}

fn has_internal_link_class(classes: &str) -> bool {
    let padded = format!("{classes} ");
    padded
        .match_indices(" internal-link")
        .any(|(at, found)| padded[at + found.len()..].starts_with(' '))
}

fn flows_sideways(nodes: &[CNode], i: usize, root: Dir) -> bool {
    let mut dir = None;
    let mut p = nodes[i].parent;
    let mut hops = 0;
    while p >= 0 && dir.is_none() && hops < 10000 {
        dir = nodes[p as usize].dir;
        p = nodes[p as usize].parent;
        hops += 1;
    }
    matches!(dir.unwrap_or(root), Dir::LR | Dir::RL)
}

fn curve_of(edge: &GraphEdge) -> Option<&str> {
    edge.curve.as_ref().and_then(Value::as_str)
}

pub fn render_flowchart(db: &FlowDb, config: &FlowConfig, options: &RenderOptions) -> Drawn {
    render_variant(db, config, options, None)
}

pub fn render_variant(
    db: &FlowDb,
    config: &FlowConfig,
    options: &RenderOptions,
    variant: Option<&FlowVariant>,
) -> Drawn {
    let fitted = |down: bool| {
        tighten(options, |tight| {
            draw(db, config, options, variant, down, tight)
        })
    };
    if runs_across(db.direction.as_deref()) {
        turn_to_fit(options, fitted)
    } else {
        fitted(false)
    }
}

fn draw(
    db: &FlowDb,
    config: &FlowConfig,
    options: &RenderOptions,
    variant: Option<&FlowVariant>,
    turned: bool,
    tight: bool,
) -> Drawn {
    with_icons(options.icons.clone(), || {
        draw_scene(db, config, options, variant, turned, tight)
    })
}

fn draw_scene(
    db: &FlowDb,
    config: &FlowConfig,
    options: &RenderOptions,
    variant: Option<&FlowVariant>,
    turned: bool,
    tight: bool,
) -> Drawn {
    let group_pad = if tight { 12.0 } else { GROUP_PAD };
    let own = variant
        .and_then(|v| config.get(v.kind))
        .and_then(Value::as_object);
    let flow = own.or_else(|| config.get("flowchart").and_then(Value::as_object));
    let curve = flow
        .and_then(|section| section.get("curve"))
        .and_then(Value::as_str);
    let mut graph = build_flow_graph(db, curve);
    if let Some(v) = variant {
        (v.prepare)(&mut graph);
    }
    let group_label_width = if variant.is_some() && tight {
        js_max(80.0, title_room(options) / 2.0)
    } else {
        4000.0
    };
    let size = options.font_size.unwrap_or(16.0);
    let edge_size = js_round(size * 0.875);
    let mut wrap_width = number_option(flow, "wrappingWidth", 200.0);
    if variant.is_some() && tight {
        wrap_width = js_min(wrap_width, 100.0);
    }
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();

    let index: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();
    let mut views: Vec<NodeView> = Vec::with_capacity(graph.nodes.len());
    let mut cnodes: Vec<CNode> = Vec::with_capacity(graph.nodes.len());

    for node in &graph.nodes {
        let css = if node.css_styles.is_empty() {
            node.css_compiled_styles.clone()
        } else {
            [
                node.css_compiled_styles.as_slice(),
                node.css_styles.as_slice(),
            ]
            .concat()
        };
        let style = resolve_style(&css);
        let text_style =
            (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let font_size = style.font_size.unwrap_or(size);
        let markdown = matches!(node.label_type, LabelType::Markdown);
        if node.is_group {
            let label = layout_label(
                node.label.as_deref(),
                markdown,
                font_size,
                group_label_width,
                text_style,
            );
            let mut c = cnode(0.0, 0.0, -1);
            c.is_group = true;
            c.dir = node
                .dir
                .as_deref()
                .filter(|d| !d.is_empty())
                .map(|d| direction(Some(d)));
            c.pad_x = group_pad;
            c.pad_top = if label.height > 0.0 {
                label.height + 16.0
            } else {
                GROUP_PAD
            };
            c.pad_bottom = group_pad;
            c.min_w = label.width + 2.0 * group_pad - 12.0;
            c.seq = views.len() as f64;
            cnodes.push(c);
            views.push(NodeView {
                node,
                style,
                label,
                shape: "group",
                w: 0.0,
                h: 0.0,
                dy: 0.0,
            });
        } else {
            let shape = canonical_shape(Some(node.shape.as_str()));
            let has_asset = present(node.img.as_deref()) || present(node.icon.as_deref());
            let text = if has_asset || shape_has_label(shape) {
                node.label.as_deref()
            } else {
                None
            };
            let label = layout_label(text, markdown, font_size, wrap_width, text_style);
            let (w, h, dy) = if has_asset {
                let box_size = if present(node.img.as_deref()) {
                    80.0
                } else {
                    48.0
                };
                let aw = asset_size(node.asset_width, box_size);
                let ah = asset_size(node.asset_height, box_size);
                let w = js_max(aw, label.width + 8.0);
                let h = ah
                    + if label.height > 0.0 {
                        label.height + 6.0
                    } else {
                        0.0
                    };
                let dy = if node.pos.as_deref() == Some("t") {
                    -ah / 2.0
                } else {
                    ah / 2.0
                };
                (w, h, if label.height == 0.0 { 0.0 } else { dy })
            } else {
                let s = shape_size(shape, label.width, label.height);
                (s.w, s.h, s.dy)
            };
            let mut c = cnode(w, h, -1);
            c.seq = views.len() as f64;
            cnodes.push(c);
            views.push(NodeView {
                node,
                style,
                label,
                shape,
                w,
                h,
                dy,
            });
        }
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        cnodes[i].parent = node
            .parent_id
            .as_ref()
            .and_then(|p| index.get(p.as_str()))
            .map_or(-1, |&j| j as i32);
    }
    let ends: Vec<(i32, i32)> = graph
        .edges
        .iter()
        .map(|edge| {
            (
                index.get(edge.start.as_str()).map_or(-1, |&j| j as i32),
                index.get(edge.end.as_str()).map_or(-1, |&j| j as i32),
            )
        })
        .collect();
    inherit_crossed_directions(&mut cnodes, &ends);

    let root_dir = if turned {
        Dir::TB
    } else {
        direction(db.direction.as_deref())
    };
    for (i, view) in views.iter().enumerate() {
        if view.node.is_group {
            cnodes[i].span = f64::INFINITY;
        } else if !present(view.node.img.as_deref()) && !present(view.node.icon.as_deref()) {
            cnodes[i].span = shape_span(
                view.shape,
                view.w,
                view.h,
                flows_sideways(&cnodes, i, root_dir),
            );
        }
    }

    let mut cedges: Vec<CEdge> = Vec::with_capacity(graph.edges.len());
    let mut edge_labels: Vec<Label> = Vec::with_capacity(graph.edges.len());
    let mut drawn: Vec<&GraphEdge> = Vec::with_capacity(graph.edges.len());
    for edge in &graph.edges {
        let (Some(&src), Some(&dst)) =
            (index.get(edge.start.as_str()), index.get(edge.end.as_str()))
        else {
            continue;
        };
        let label = layout_label(
            Some(edge.label.as_str()),
            matches!(edge.label_type, LabelType::Markdown),
            edge_size,
            wrap_width,
            0,
        );
        if src == dst {
            if flows_sideways(&cnodes, src, root_dir) {
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
        drawn.push(edge);
        edge_labels.push(label.clone());
        cedges.push(CEdge {
            src,
            dst,
            minlen: edge.minlen.unwrap_or(1.0),
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

    let layout_options = LayeredOptions {
        node_sep: number_option(flow, "nodeSpacing", NODE_SEP) * if tight { 0.5 } else { 1.0 },
        edge_sep: EDGE_SEP,
        rank_sep: number_option(flow, "rankSpacing", RANK_SEP) * if tight { 0.75 } else { 1.0 },
        port_sep: 20.0,
        tracks: curve.is_some_and(|c| c.starts_with("step")),
        straight: curve == Some("linear"),
        head_room: marker_trim("arrow_point"),
        ..LayeredOptions::default()
    };
    let layout = match variant {
        Some(v) => (v.layout)(&mut cnodes, &mut cedges, root_dir, &layout_options, tight),
        None => compound_layout(&mut cnodes, &mut cedges, root_dir, &layout_options),
    };

    let title = title_label(
        db.title.as_deref(),
        size,
        if tight { title_room(options) } else { 4000.0 },
    );
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
    let mut links: Vec<LinkInfo> = Vec::new();

    let mut crossings: Crossings = Crossings::new();
    if cnodes.iter().any(|c| c.is_group) {
        for e in &cedges {
            mark_crossings(&mut crossings, &e.route);
        }
    }

    let mut clusters = String::new();
    let mut struck_titles = String::new();
    let mut nodes_out = String::new();
    let mut fills: Vec<Fill> = Vec::new();
    for (i, view) in views.iter().enumerate() {
        let c = &cnodes[i];
        let node = view.node;
        let x = c.x;
        let y = c.y;
        let classes = class_names(strip_default(&node.css_classes));
        let id = esc_text(&node.id);
        if let Some(v) = variant.filter(|_| c.is_group && c.parent < 0) {
            if paints(&view.style.shape) {
                fills.push(Fill {
                    box_: (v.filled_box)(c, root_dir),
                    shape: view.style.shape.clone(),
                });
            }
            clusters.push_str(&(v.draw_group)(
                c,
                &view.label,
                &view.style,
                &classes,
                &id,
                root_dir,
                icons,
            ));
            continue;
        }
        if node.is_group {
            if paints(&view.style.shape) {
                fills.push(Fill {
                    box_: [x - c.w / 2.0, y - c.h / 2.0, x + c.w / 2.0, y + c.h / 2.0],
                    shape: view.style.shape.clone(),
                });
            }
            let title_x = cluster_title_x(c, view.label.width, &crossings, group_pad);
            let title_y = y - c.h / 2.0 + 8.0 + view.label.height / 2.0;
            let struck = title_crossed(c, view.label.width, title_x, &crossings);
            let title_attrs = format!(
                " class=\"pelers-cluster-label\"{}{}",
                if struck {
                    " fill=\"var(--_m)\"".to_owned()
                } else {
                    ink_fill(&view.style.shape, &view.style.text, Some("var(--_m)"))
                },
                view.style.text
            );
            if struck {
                struck_titles.push_str(&struck_title(
                    &view.label,
                    title_x,
                    title_y,
                    &title_attrs,
                    &id,
                    &cnodes,
                    i,
                ));
            }
            let _ = write!(
                clusters,
                "<g class=\"pelers-cluster{classes}\" data-id=\"{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\"{} stroke=\"var(--_b)\"{}/>{}</g>",
                num(x - c.w / 2.0),
                num(y - c.h / 2.0),
                num(c.w),
                num(c.h),
                if sets_fill(&view.style.shape) {
                    ""
                } else {
                    " fill-opacity=\"0.5\""
                },
                rounded(&view.style.shape),
                if struck {
                    String::new()
                } else {
                    label_svg_with(&view.label, title_x, title_y, &title_attrs, icons)
                }
            );
            continue;
        }

        let w = view.w;
        let h = view.h;
        let mut inner = String::new();
        let mut ink = String::new();
        if present(node.img.as_deref()) {
            let aw = asset_size(node.asset_width, 80.0);
            let ah = asset_size(node.asset_height, 80.0);
            let top = if view.label.height > 0.0 {
                if node.pos.as_deref() == Some("t") {
                    h / 2.0 - ah
                } else {
                    -h / 2.0
                }
            } else {
                -ah / 2.0
            };
            if let Some(src) = image_url(&safe_url(node.img.as_deref().unwrap_or("")), options) {
                inner = format!(
                    "<image href=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"{}\"/>",
                    esc(&src),
                    num(-aw / 2.0),
                    num(top),
                    num(aw),
                    num(ah),
                    if node.constraint.as_deref() == Some("on") {
                        "xMidYMid meet"
                    } else {
                        "none"
                    }
                );
            }
        } else if let Some(icon) = node.icon.as_deref().filter(|s| !s.is_empty()) {
            let box_size = asset_size(node.asset_height, 48.0);
            let top = if view.label.height > 0.0 {
                if node.pos.as_deref() == Some("t") {
                    h / 2.0 - box_size
                } else {
                    -h / 2.0
                }
            } else {
                -box_size / 2.0
            };
            let frame = if node.shape == "icon" {
                String::new()
            } else if node.shape == "iconCircle" {
                format!(
                    "<circle cy=\"{}\" r=\"{}\"{SHAPE_ATTRS}{}/>",
                    num(top + box_size / 2.0),
                    num(box_size / 2.0),
                    view.style.shape
                )
            } else {
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\"{SHAPE_ATTRS}{}/>",
                    num(-box_size / 2.0),
                    num(top),
                    num(box_size),
                    num(box_size),
                    if node.shape == "iconRounded" { 8 } else { 0 },
                    view.style.shape
                )
            };
            let glyph = box_size * 0.6;
            inner = frame
                + &format!(
                    "<svg class=\"pelers-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg>",
                    esc_text(icon),
                    num(-glyph / 2.0),
                    num(top + (box_size - glyph) / 2.0),
                    num(glyph),
                    num(glyph),
                    icon_markup(icon)
                );
        } else {
            inner = draw_shape(
                view.shape,
                w,
                h,
                &format!("{SHAPE_ATTRS}{}", view.style.shape),
                &format!("{LINE_ATTRS}{}", view.style.line),
            );
            if shape_filled(view.shape) {
                ink = ink_fill(&view.style.shape, &view.style.text, None);
            }
        }

        let mut text = label_svg_with(
            &view.label,
            0.0,
            view.dy,
            &format!(" class=\"pelers-label\"{ink}{}", view.style.text),
            icons,
        );
        if has_internal_link_class(&classes)
            && present(node.label.as_deref())
            && options.links != Some(false)
        {
            let name = decode_entities(node.label.as_deref().unwrap_or(""));
            let safe = internal_link(&name, options);
            let target = esc(&name);
            text = format!(
                "<a class=\"internal-link\"{} data-href=\"{target}\">{text}</a>",
                if safe {
                    format!(" href=\"{target}\"{}", rel_attr(options))
                } else {
                    String::new()
                }
            );
            links.push(LinkInfo {
                id: node.id.clone(),
                href: name,
                internal: true,
            });
        }
        let mut body = inner + &text;
        if present(node.tooltip.as_deref()) {
            body = format!(
                "<title>{}</title>{body}",
                esc_text(node.tooltip.as_deref().unwrap_or(""))
            );
        }
        if present(node.link.as_deref()) {
            let link_target = if present(node.link_target.as_deref()) {
                format!(
                    " target=\"{}\"",
                    esc(node.link_target.as_deref().unwrap_or(""))
                )
            } else {
                String::new()
            };
            body = linked(
                &body,
                node.link.as_deref().unwrap_or(""),
                &node.id,
                &mut links,
                options,
                &link_target,
            );
        }
        let _ = write!(
            nodes_out,
            "<g class=\"pelers-node pelers-shape-{}{classes}\" data-id=\"{id}\" transform=\"translate({},{})\">{body}</g>",
            view.shape,
            num(x),
            num(y)
        );
    }

    let mut reach = SideReach::default();
    for (i, ce) in cedges.iter().enumerate() {
        if ce.src == ce.dst
            || drawn[i].thickness.as_deref() == Some("invisible")
            || ce.route.len() < 6
        {
            continue;
        }
        let last = ce.route.len() - 3;
        reach.add(
            ce.src as i64,
            end_side(&ce.route, 0, &cnodes[ce.src]) as i64,
            &drawn[i].arrow_type_start,
        );
        reach.add(
            ce.dst as i64,
            end_side(&ce.route, last, &cnodes[ce.dst]) as i64,
            &drawn[i].arrow_type_end,
        );
    }

    let mut casings = String::new();
    let mut edges_out = String::new();
    let mut labels_out = String::new();
    let mut loop_seen: HashMap<usize, usize> = HashMap::new();
    for i in 0..cedges.len() {
        let (src, dst) = (cedges[i].src, cedges[i].dst);
        let edge = drawn[i];
        let label = &edge_labels[i];
        let style: Option<ResolvedStyle> =
            if !edge.style.is_empty() || !edge.css_compiled_styles.is_empty() {
                Some(resolve_style(
                    &[edge.css_compiled_styles.as_slice(), edge.style.as_slice()].concat(),
                ))
            } else {
                None
            };
        let style_line = style.as_ref().map_or("", |s| s.line.as_str());
        let style_text = style.as_ref().map_or("", |s| s.text.as_str());
        if edge.thickness.as_deref() != Some("invisible") {
            let start_type = edge.arrow_type_start.as_str();
            let end_type = edge.arrow_type_end.as_str();
            let path: EdgePath = if src == dst {
                let view = &views[src];
                let k = loop_seen.get(&src).copied().unwrap_or(0);
                loop_seen.insert(src, k + 1);
                let mut loop_edge = LoopEdge {
                    label_w: cedges[i].label_w,
                    label_h: cedges[i].label_h,
                    label_x: cedges[i].label_x,
                    label_y: cedges[i].label_y,
                };
                let path = loop_path(
                    &LoopNode {
                        x: cnodes[src].x,
                        y: cnodes[src].y,
                    },
                    (view.w, view.h),
                    k as f64,
                    flows_sideways(&cnodes, src, root_dir),
                    LOOP,
                    marker_trim(end_type),
                    &mut loop_edge,
                );
                cedges[i].label_x = loop_edge.label_x;
                cedges[i].label_y = loop_edge.label_y;
                path
            } else {
                let mut route = cedges[i].route.clone();
                let last = route.len().saturating_sub(3);
                let leaves = reach.of(src as i64, end_side(&route, 0, &cnodes[src]) as i64);
                let arrives = reach.of(dst as i64, end_side(&route, last, &cnodes[dst]) as i64);
                inset_route(
                    &mut route,
                    0,
                    &View {
                        shape: views[src].shape,
                        w: views[src].w,
                        h: views[src].h,
                    },
                    &cnodes[src],
                );
                inset_route(
                    &mut route,
                    last,
                    &View {
                        shape: views[dst].shape,
                        w: views[dst].w,
                        h: views[dst].h,
                    },
                    &cnodes[dst],
                );
                route_path(
                    &route,
                    curve_of(edge),
                    marker_trim(start_type),
                    marker_trim(end_type),
                    Some(leaves),
                    Some(arrives),
                )
            };
            let over = over_fills(
                &path.d,
                &[path.sx, path.sy, path.ex, path.ey],
                &fills,
                style_line,
                path.tokens.as_deref(),
            );
            let over_text = over.as_deref().unwrap_or("");
            let stroke = style
                .as_ref()
                .and_then(|s| s.stroke.as_deref())
                .filter(|s| !s.is_empty());
            let color = if !over_text.is_empty() {
                over_text.to_owned()
            } else if let Some(stroke) = stroke {
                esc(stroke).into_owned()
            } else {
                "var(--_l)".to_owned()
            };
            let thick = edge.thickness.as_deref() == Some("thick");
            let mut attrs = String::new();
            if thick {
                attrs.push_str(" stroke-width=\"2.5\"");
            } else if edge.thickness.as_deref() == Some("dotted") {
                attrs.push_str(" stroke-dasharray=\"3 4\"");
            }
            let classes = class_names(
                &edge
                    .classes
                    .replace("edge-thickness-normal", "")
                    .replace("edge-pattern-solid", "")
                    .replace("flowchart-link", ""),
            );
            if over.as_deref() == Some("") {
                casings.push_str(&edge_casing(&path, start_type, end_type, style_line, thick));
            }
            let _ = write!(
                edges_out,
                "<g class=\"pelers-edge{classes}\" data-id=\"{}\"{}{style_line}>",
                esc_text(&edge.id),
                if over_text.is_empty() {
                    String::new()
                } else {
                    format!(" stroke=\"{over_text}\"")
                }
            );
            let _ = write!(edges_out, "<path d=\"{}\"{attrs}/>", path.d);
            edges_out.push_str(&marker(
                start_type, path.sx, path.sy, path.sdx, path.sdy, &color, "none",
            ));
            edges_out.push_str(&marker(
                end_type, path.ex, path.ey, path.edx, path.edy, &color, "none",
            ));
            edges_out.push_str("</g>");
        }
        if label.width > 0.0 {
            labels_out.push_str(&edge_label_svg(
                &edge.id,
                label,
                cedges[i].label_x,
                cedges[i].label_y,
                style_text,
                icons,
            ));
        }
    }

    let content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &if clusters.is_empty() {
        String::new()
    } else {
        format!("<g class=\"pelers-clusters\">{clusters}</g>")
    } + &casings_svg(&casings)
        + &if edges_out.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{edges_out}</g>"
            )
        }
        + &if struck_titles.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pelers-cluster-titles\">{struck_titles}</g>")
        }
        + &if labels_out.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-edge-labels\" font-size=\"{}\">{labels_out}</g>",
                js_number_to_string(edge_size)
            )
        }
        + &format!("<g class=\"pelers-nodes\">{nodes_out}</g>");

    let svg = svg_document(
        variant.map_or("flowchart", |v| v.kind),
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
        links,
    }
}
