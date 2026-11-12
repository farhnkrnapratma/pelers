use std::collections::HashMap;

use super::AgentflowModel;
use super::color_slots::{KIND_COUNT, kind_slot};
use super::db::AgentflowDb;
use super::graph::{AgentGraph, GraphEdge, build_agent_graph};
use super::types::LabelType;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{runs_across, turn_to_fit};
use crate::diagrams::flowchart::shapes::canonical_shape;
use crate::layout::compound::{compound_layout, inherit_crossed_directions, shift_layout};
use crate::layout::{CEdge, CNode, Dir, LayeredOptions, cnode, direction};
use crate::svg::builder::{esc, esc_text, label_svg, label_svg_with, num};
use crate::svg::cluster::{Crossings, cluster_title_x, mark_crossings};
use crate::svg::edges::{
    EdgePath, Fill, LoopEdge, LoopNode, casings_svg, edge_casing, edge_label_svg, loop_path,
    marker, marker_trim, over_fills, route_path,
};
use crate::svg::ink::{ink_fill, paints};
use crate::svg::options::LinkInfo;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::shapes::{PAD_X, PAD_Y, View, draw_shape, inset_route, shape_filled, shape_size};
use crate::svg::theme::{
    ResolvedStyle, class_names, resolve_style, rounded, series_color, sets_fill,
};
use crate::text::entities::decode_entities;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, ITALIC};
use crate::util::jsnum::{js_max, js_min, js_round};
use crate::util::url::{internal_link, linked, rel_attr};
use crate::util::value::{Object, Value};

pub const PALETTE: usize = 8;

const NODE_SEP: f64 = 40.0;
const EDGE_SEP: f64 = 16.0;
const RANK_SEP: f64 = 48.0;
const GROUP_PAD: f64 = 20.0;
const LOOP: f64 = 26.0;
const DOTS: f64 = 14.0;
const COLLAPSED: &str = "collapsed";

pub fn agent_graph_for(db: &mut AgentflowDb, curve: Option<&str>) -> AgentGraph {
    build_agent_graph(db, PALETTE, curve)
}

fn number_option(config: &Object, key: &str, fallback: f64) -> f64 {
    match config
        .get("agentflow")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::Number(value)) if *value > 0.0 => js_min(*value, 2000.0),
        _ => fallback,
    }
}

fn flow_end(dir: Dir) -> i32 {
    match dir {
        Dir::TB => 2,
        Dir::RL => 3,
        Dir::BT => 0,
        Dir::LR => 1,
    }
}

fn tint(slot: usize, wash: &str, style: &ResolvedStyle) -> String {
    let color = series_color(slot);
    let fill_opacity = if sets_fill(&style.shape) {
        String::new()
    } else {
        format!(" fill-opacity=\"{wash}\"")
    };
    format!(" fill=\"{color}\"{fill_opacity} stroke=\"{color}\"")
}

fn dot(x: f64, y: f64) -> String {
    format!(
        "M{},{}a1.75,1.75 0 1 0 3.5,0a1.75,1.75 0 1 0 -3.5,0Z",
        num(x - 1.75),
        num(y)
    )
}

fn strip_default(raw: &str) -> String {
    match raw.strip_prefix("default") {
        Some(rest) => match rest.chars().next() {
            Some(c) if c.is_whitespace() => rest[c.len_utf8()..].to_owned(),
            _ => rest.to_owned(),
        },
        None => raw.to_owned(),
    }
}

struct NodeView {
    shape: String,
    style: ResolvedStyle,
    label: Label,
    w: f64,
    h: f64,
    dy: f64,
}

struct Held {
    edge: GraphEdge,
    label: Label,
    src: usize,
    dst: usize,
}

fn depth_of(cnodes: &[CNode]) -> Vec<i64> {
    let n = cnodes.len();
    let mut depth = vec![-1i64; n];
    for i in 0..n {
        let mut chain: Vec<usize> = Vec::new();
        let mut j = i as i64;
        while j >= 0 && depth[j as usize] < 0 && chain.len() <= n {
            chain.push(j as usize);
            j = cnodes[j as usize].parent as i64;
        }
        let start = if j >= 0 { depth[j as usize] + 1 } else { 0 };
        for (offset, &node) in chain.iter().rev().enumerate() {
            depth[node] = start + offset as i64;
        }
    }
    depth
}

fn contains(cnodes: &[CNode], depth: &[i64], group: usize, i: usize) -> bool {
    let mut j = i as i64;
    let mut d = depth[i];
    while d > depth[group] {
        if j < 0 {
            return false;
        }
        j = cnodes[j as usize].parent as i64;
        d -= 1;
    }
    j == group as i64 && i != group
}

fn flow_of(cnodes: &[CNode], i: usize, root: Dir) -> Dir {
    let mut dir: Option<Dir> = None;
    let mut p = cnodes[i].parent;
    let mut hops = 0;
    while p >= 0 && dir.is_none() && hops < 10000 {
        dir = cnodes[p as usize].dir;
        p = cnodes[p as usize].parent;
        hops += 1;
    }
    dir.unwrap_or(root)
}

fn flows_sideways(cnodes: &[CNode], i: usize, root: Dir) -> bool {
    matches!(flow_of(cnodes, i, root), Dir::LR | Dir::RL)
}

fn blocked(cnodes: &[CNode], i: usize, r: &[f64]) -> bool {
    let (x_lo, x_hi) = (js_min(r[0], r[3]), js_max(r[0], r[3]));
    let (y_lo, y_hi) = (js_min(r[1], r[4]), js_max(r[1], r[4]));
    cnodes.iter().enumerate().any(|(k, c)| {
        k != i
            && !c.is_group
            && c.x + c.w / 2.0 > x_lo - 1.0
            && c.x - c.w / 2.0 < x_hi + 1.0
            && c.y + c.h / 2.0 > y_lo - 1.0
            && c.y - c.h / 2.0 < y_hi + 1.0
    })
}

fn view_of(view: &NodeView) -> View<'_> {
    View {
        shape: view.shape.as_str(),
        w: view.w,
        h: view.h,
    }
}

#[derive(Default)]
struct Out {
    casings: String,
    edges: String,
    labels: String,
}

fn put(
    out: &mut Out,
    fills: &[Fill],
    edge: &GraphEdge,
    label: &Label,
    path: &EdgePath,
    at: (f64, f64),
    icons: bool,
) {
    let style = if !edge.style.is_empty() || !edge.css_compiled_styles.is_empty() {
        let mut declarations = edge.css_compiled_styles.clone();
        declarations.extend(edge.style.iter().cloned());
        Some(resolve_style(&declarations))
    } else {
        None
    };
    let line = style.as_ref().map_or("", |s| s.line.as_str());
    let end_type = edge.arrow_type_end;
    let over = over_fills(
        &path.d,
        &[path.ex, path.ey],
        fills,
        line,
        path.tokens.as_deref(),
    );
    let over_text = over.clone().unwrap_or_default();
    let color = if !over_text.is_empty() {
        over_text.clone()
    } else {
        match style.as_ref().and_then(|s| s.stroke.as_ref()) {
            Some(stroke) => esc(stroke).into_owned(),
            None => "var(--_l)".to_owned(),
        }
    };
    let kind = if edge.thickness.as_deref() == Some("dotted") {
        "reference"
    } else if end_type == "arrow_cross" {
        "failure"
    } else {
        "sequence"
    };
    let raw_classes = edge
        .classes
        .replace("edge-thickness-normal", "")
        .replace("edge-pattern-solid", "")
        .replace("flowchart-link", "");
    let classes = class_names(&raw_classes);
    if over.as_deref() == Some("") {
        out.casings
            .push_str(&edge_casing(path, "none", end_type, line, false));
    }
    let stroke_attr = if over_text.is_empty() {
        String::new()
    } else {
        format!(" stroke=\"{over_text}\"")
    };
    let dashed = if edge.thickness.as_deref() == Some("dotted") {
        " stroke-dasharray=\"3 4\""
    } else {
        ""
    };
    let style_line = style.as_ref().map_or(String::new(), |s| s.line.clone());
    out.edges.push_str(&format!(
        "<g class=\"pelers-edge pelers-edge-{kind}{classes}\" data-id=\"{}\"{stroke_attr}{style_line}><path d=\"{}\"{dashed}/>{}</g>",
        esc_text(&edge.id),
        path.d,
        marker(end_type, path.ex, path.ey, path.edx, path.edy, &color, "none")
    ));
    if label.width > 0.0 {
        let text_attrs = style.as_ref().map_or(String::new(), |s| s.text.clone());
        out.labels.push_str(&edge_label_svg(
            &edge.id,
            label,
            at.0,
            at.1,
            &text_attrs,
            icons,
        ));
    }
}

pub fn render_agentflow(model: &AgentflowModel, config: &Object, options: &RenderOptions) -> Drawn {
    if !runs_across(model.db.direction.as_deref()) {
        return draw(model, config, options, false);
    }
    turn_to_fit(options, |down| draw(model, config, options, down))
}

fn draw(model: &AgentflowModel, config: &Object, options: &RenderOptions, turned: bool) -> Drawn {
    let db = &model.db;
    let graph = &model.graph;
    let size = options.font_size.unwrap_or(16.0);
    let edge_size = js_round(size * 0.875);
    let wrap_width = number_option(config, "wrappingWidth", 120.0);
    let min_width = number_option(config, "minNodeWidth", 120.0);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();

    let index: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut views: Vec<NodeView> = Vec::new();
    let mut cnodes: Vec<CNode> = Vec::new();
    for node in &graph.nodes {
        let styles = if node.css_styles.is_empty() {
            node.css_compiled_styles.clone()
        } else {
            let mut all = node.css_compiled_styles.clone();
            all.extend(node.css_styles.iter().cloned());
            all
        };
        let style = resolve_style(&styles);
        let base =
            (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let markdown = node.label_type == LabelType::Markdown;
        let font_size = style.font_size.unwrap_or(size);
        let label_text = node.label.as_deref();
        let (view, c) = if node.is_group {
            let label = layout_label(label_text, markdown, font_size, 4000.0, base);
            let mut group = cnode(0.0, 0.0, -1);
            group.is_group = true;
            group.dir = node
                .dir
                .as_deref()
                .filter(|d| !d.is_empty())
                .map(|d| direction(Some(d)));
            group.pad_x = GROUP_PAD;
            group.pad_top = if label.height > 0.0 {
                label.height + 16.0
            } else {
                GROUP_PAD
            };
            group.pad_bottom = GROUP_PAD;
            group.min_w = label.width + 2.0 * GROUP_PAD - 12.0;
            let view = NodeView {
                shape: "group".to_owned(),
                style,
                label,
                w: 0.0,
                h: 0.0,
                dy: 0.0,
            };
            (view, group)
        } else if node.shape == "collapsedGroup" {
            let label = layout_label(label_text, markdown, font_size, wrap_width, base);
            let w = js_max(label.width, min_width) + 2.0 * PAD_X;
            let h = label.height + 2.0 * PAD_Y + DOTS;
            let view = NodeView {
                shape: COLLAPSED.to_owned(),
                style,
                label,
                w,
                h,
                dy: -DOTS / 2.0,
            };
            (view, cnode(w, h, -1))
        } else {
            let shape = canonical_shape(Some(node.shape.as_str()));
            let label = layout_label(label_text, markdown, font_size, wrap_width, base);
            let tw = if shape == "diam" {
                label.width
            } else {
                js_max(label.width, min_width)
            };
            let s = shape_size(shape, tw, label.height);
            let view = NodeView {
                shape: shape.to_owned(),
                style,
                label,
                w: s.w,
                h: s.h,
                dy: s.dy,
            };
            (view, cnode(s.w, s.h, -1))
        };
        let mut c = c;
        c.seq = views.len() as f64;
        views.push(view);
        cnodes.push(c);
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        cnodes[i].parent = match node.parent_id.as_deref() {
            Some(parent) => index.get(parent).map_or(-1, |&p| p as i32),
            None => -1,
        };
    }
    let ends: Vec<(i32, i32)> = graph
        .edges
        .iter()
        .map(|edge| {
            (
                index.get(edge.start.as_str()).map_or(-1, |&p| p as i32),
                index.get(edge.end.as_str()).map_or(-1, |&p| p as i32),
            )
        })
        .collect();
    inherit_crossed_directions(&mut cnodes, &ends);

    let root_dir = if turned {
        Dir::TB
    } else {
        direction(db.direction.as_deref())
    };
    let depth = depth_of(&cnodes);

    let mut cedges: Vec<CEdge> = Vec::new();
    let mut edge_labels: Vec<Label> = Vec::new();
    let mut drawn: Vec<GraphEdge> = Vec::new();
    let mut held: Vec<Held> = Vec::new();
    for edge in &graph.edges {
        let (Some(&src), Some(&dst)) =
            (index.get(edge.start.as_str()), index.get(edge.end.as_str()))
        else {
            continue;
        };
        let label = layout_label(
            Some(edge.label.as_str()).filter(|t| !t.is_empty()),
            edge.label_type == LabelType::Markdown,
            edge_size,
            wrap_width,
            0,
        );
        if contains(&cnodes, &depth, src, dst) || contains(&cnodes, &depth, dst, src) {
            held.push(Held {
                edge: edge.clone(),
                label,
                src,
                dst,
            });
            continue;
        }
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
        cedges.push(CEdge {
            src,
            dst,
            minlen: edge.minlen.unwrap_or(1) as f64,
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
        drawn.push(edge.clone());
        edge_labels.push(label);
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

    let title = layout_label(db.title.as_deref(), false, size, 4000.0, BOLD_STYLE);
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

    let mut held_paths: Vec<EdgePath> = Vec::new();
    let mut held_labels: Vec<f64> = Vec::new();
    let mut held_routes: Vec<Vec<f64>> = Vec::new();
    let careful = held.len() * cnodes.len() < 2_000_000;
    for entry in &held {
        let out_edge = contains(&cnodes, &depth, entry.dst, entry.src);
        let outer = if out_edge { entry.dst } else { entry.src };
        let i = if out_edge { entry.src } else { entry.dst };
        let g = cnodes[outer].clone();
        let n = cnodes[i].clone();
        let end = flow_end(g.dir.unwrap_or_else(|| flow_of(&cnodes, outer, root_dir)));
        let first = if out_edge { end } else { (end + 2) & 3 };
        let toward = |side: i32| -> Vec<f64> {
            let upright = side & 1 == 0;
            let sign = if side == 0 || side == 3 { -1.0 } else { 1.0 };
            let nx = if upright { n.x } else { n.x + sign * n.w / 2.0 };
            let ny = if upright { n.y + sign * n.h / 2.0 } else { n.y };
            let bx = if upright { n.x } else { g.x + sign * g.w / 2.0 };
            let by = if upright { g.y + sign * g.h / 2.0 } else { n.y };
            let axis = if upright { 0.0 } else { 1.0 };
            if out_edge {
                vec![nx, ny, axis, bx, by, axis]
            } else {
                vec![bx, by, axis, nx, ny, axis]
            }
        };
        let mut route = toward(first);
        if careful && blocked(&cnodes, i, &route) {
            for side in [(first + 1) & 3, (first + 3) & 3, (first + 2) & 3] {
                let other = toward(side);
                if !blocked(&cnodes, i, &other) {
                    route = other;
                    break;
                }
            }
        }
        held_routes.push(route.clone());
        held_labels.push((route[0] + route[3]) / 2.0);
        held_labels.push((route[1] + route[4]) / 2.0);
        let mut drawn_route = route.clone();
        let view = view_of(&views[i]);
        inset_route(&mut drawn_route, if out_edge { 0 } else { 3 }, &view, &n);
        held_paths.push(route_path(
            &drawn_route,
            Some("linear"),
            0.0,
            marker_trim(entry.edge.arrow_type_end),
            None,
            None,
        ));
    }

    let mut crossings: Crossings = Crossings::new();
    if cnodes.iter().any(|c| c.is_group) {
        for e in &cedges {
            mark_crossings(&mut crossings, &e.route);
        }
        for route in &held_routes {
            mark_crossings(&mut crossings, route);
        }
    }

    let mut clusters = String::new();
    let mut nodes_out = String::new();
    let mut fills: Vec<Fill> = Vec::new();
    for (i, view) in views.iter().enumerate() {
        let c = cnodes[i].clone();
        let node = &graph.nodes[i];
        let x = c.x;
        let y = c.y;
        let classes = class_names(&strip_default(&node.css_classes));
        let id = esc_text(&node.id);
        if c.is_group {
            if paints(&view.style.shape) {
                fills.push(Fill {
                    box_: [x - c.w / 2.0, y - c.h / 2.0, x + c.w / 2.0, y + c.h / 2.0],
                    shape: view.style.shape.clone(),
                });
            }
            let title_x = cluster_title_x(&c, view.label.width, &crossings, GROUP_PAD);
            let slot = node.color_index.unwrap_or(KIND_COUNT);
            clusters.push_str(&format!(
                "<g class=\"pelers-cluster pelers-flow{classes}\" data-id=\"{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/>{}</g>",
                num(x - c.w / 2.0),
                num(y - c.h / 2.0),
                num(c.w),
                num(c.h),
                rounded(&format!("{}{}", tint(slot, "0.06", &view.style), view.style.shape)),
                label_svg_with(
                    &view.label,
                    title_x,
                    y - c.h / 2.0 + 8.0 + view.label.height / 2.0,
                    &format!(
                        " class=\"pelers-cluster-label\"{}{}",
                        ink_fill(&view.style.shape, &view.style.text, Some("var(--_m)")),
                        view.style.text
                    ),
                    icons
                )
            ));
            continue;
        }

        let w = view.w;
        let h = view.h;
        let mut kind = String::new();
        let shape_svg: String;
        if view.shape == COLLAPSED {
            let slot = node.color_index.unwrap_or(KIND_COUNT);
            let color = series_color(slot);
            let line = h / 2.0 - DOTS - 2.0;
            let dots = line + (DOTS + 2.0) / 2.0;
            shape_svg = format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{}/><path d=\"M{},{}H{}\" fill=\"none\" stroke=\"{color}\" stroke-dasharray=\"3 3\"{}/><path class=\"pelers-collapsed-mark\" d=\"{}{}{}\" fill=\"{color}\" stroke=\"none\"/>",
                num(-w / 2.0),
                num(-h / 2.0),
                num(w),
                num(h),
                rounded(&format!(
                    "{}{}",
                    tint(slot, "0.06", &view.style),
                    view.style.shape
                )),
                num(-w / 2.0 + 10.0),
                num(line),
                num(w / 2.0 - 10.0),
                view.style.line,
                dot(-9.0, dots),
                dot(0.0, dots),
                dot(9.0, dots)
            );
        } else {
            let kind_name = node.kind.unwrap_or("task");
            let slot = kind_slot(kind_name).unwrap_or(1);
            kind = format!(" pelers-kind-{kind_name}");
            shape_svg = draw_shape(
                &view.shape,
                w,
                h,
                &format!("{}{}", tint(slot, "0.14", &view.style), view.style.shape),
                &format!(" stroke=\"{}\"{}", series_color(slot), view.style.line),
            );
        }

        let ink = if view.shape == COLLAPSED || shape_filled(&view.shape) {
            ink_fill(&view.style.shape, &view.style.text, None)
        } else {
            String::new()
        };
        let mut text = label_svg_with(
            &view.label,
            0.0,
            view.dy,
            &format!(" class=\"pelers-label\"{ink}{}", view.style.text),
            icons,
        );
        let padded = format!("{classes} ");
        let internal = padded.match_indices(" internal-link").any(|(at, _)| {
            let after = at + " internal-link".len();
            after == padded.len() || padded.as_bytes()[after] == b' '
        });
        if internal
            && !node.label.as_deref().unwrap_or("").is_empty()
            && options.links != Some(false)
        {
            let name = decode_entities(node.label.as_deref().unwrap_or(""));
            let safe = internal_link(&name, options);
            let target = esc(&name);
            let href = if safe {
                format!(" href=\"{target}\"{}", rel_attr(options))
            } else {
                String::new()
            };
            text = format!("<a class=\"internal-link\"{href} data-href=\"{target}\">{text}</a>");
            links.push(LinkInfo {
                id: node.id.clone(),
                href: name,
                internal: true,
            });
        }
        let mut body = shape_svg + &text;
        if let Some(tooltip) = node.tooltip.as_deref().filter(|t| !t.is_empty()) {
            body = format!("<title>{}</title>{body}", esc_text(tooltip));
        }
        if let Some(link) = node.link.as_deref().filter(|l| !l.is_empty()) {
            let attrs = match node.link_target.as_deref().filter(|t| !t.is_empty()) {
                Some(target) => format!(" target=\"{}\"", esc(target)),
                None => String::new(),
            };
            body = linked(&body, link, &node.id, &mut links, options, &attrs);
        }
        nodes_out.push_str(&format!(
            "<g class=\"pelers-node pelers-shape-{}{kind}{classes}\" data-id=\"{id}\" transform=\"translate({},{})\">{body}</g>",
            view.shape,
            num(x),
            num(y)
        ));
    }

    let mut out = Out::default();
    let mut loop_seen: HashMap<usize, usize> = HashMap::new();
    for i in 0..cedges.len() {
        let src = cedges[i].src;
        let dst = cedges[i].dst;
        let edge = drawn[i].clone();
        let label = edge_labels[i].clone();
        let end_type = edge.arrow_type_end;
        let path = if src == dst {
            let k = *loop_seen.get(&src).unwrap_or(&0);
            loop_seen.insert(src, k + 1);
            let c = cnodes[src].clone();
            let view = &views[src];
            let mut le = LoopEdge {
                label_w: cedges[i].label_w,
                label_h: cedges[i].label_h,
                label_x: cedges[i].label_x,
                label_y: cedges[i].label_y,
            };
            let path = loop_path(
                &LoopNode { x: c.x, y: c.y },
                (view.w, view.h),
                k as f64,
                flows_sideways(&cnodes, src, root_dir),
                LOOP,
                marker_trim(end_type),
                &mut le,
            );
            cedges[i].label_x = le.label_x;
            cedges[i].label_y = le.label_y;
            path
        } else {
            let mut route = cedges[i].route.clone();
            let src_view = view_of(&views[src]);
            inset_route(&mut route, 0, &src_view, &cnodes[src]);
            let at = route.len().saturating_sub(3);
            let dst_view = view_of(&views[dst]);
            inset_route(&mut route, at, &dst_view, &cnodes[dst]);
            route_path(
                &route,
                edge.curve.as_deref(),
                0.0,
                marker_trim(end_type),
                None,
                None,
            )
        };
        put(
            &mut out,
            &fills,
            &edge,
            &label,
            &path,
            (cedges[i].label_x, cedges[i].label_y),
            icons,
        );
    }
    for (i, entry) in held.iter().enumerate() {
        put(
            &mut out,
            &fills,
            &entry.edge,
            &entry.label,
            &held_paths[i],
            (held_labels[2 * i], held_labels[2 * i + 1]),
            icons,
        );
    }

    let acc = Accessible {
        acc_title: db.acc_title.as_deref().filter(|t| !t.is_empty()),
        acc_descr: db.acc_descr.as_deref().filter(|t| !t.is_empty()),
    };
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
        casings_svg(&out.casings),
        if out.edges.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{}</g>",
                out.edges
            )
        },
        if out.labels.is_empty() {
            String::new()
        } else {
            format!(
                "<g class=\"pelers-edge-labels\" font-size=\"{}\">{}</g>",
                crate::util::jsnum::js_number_to_string(edge_size),
                out.labels
            )
        },
    );
    let svg = svg_document("agentflow", width, height, size, options, acc, &content);
    Drawn {
        svg,
        width,
        height,
        links,
    }
}
