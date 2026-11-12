use std::collections::HashMap;

use super::db::ClassDb;
use super::graph::{GraphEdge, GraphNode, NodeShape, Pattern, build_class_graph};
use super::markers::{class_marker, class_marker_trim};
use super::members::ClassMember;
use super::types::ClassNode;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{runs_across, tighten, turn_to_fit};
use crate::diagrams::common::generics::parse_generic_types;
use crate::layout::components::wrap_components;
use crate::layout::compound::compound_layout;
use crate::layout::{CEdge, CNode, Dir, LayeredOptions, cnode, direction};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, label_svg_with, num};
use crate::svg::cluster::{
    Crossings, cluster_title_x, mark_crossings, struck_title, title_crossed,
};
use crate::svg::edges::{EdgePath, edge_label_svg, route_path};
use crate::svg::ink::ink_fill;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::shapes::end_side;
use crate::svg::theme::{ROUNDED, ResolvedStyle, class_names, resolve_style, rounded};
use crate::svg::title::with_title;
use crate::text::entities::decode_entities;
use crate::text::label::{Label, has_math, layout_label};
use crate::text::measurer::{BOLD_STYLE, ITALIC, MONO, width as measure};
use crate::util::chars::trim;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::url::linked;
use crate::util::value::Value;

const NODE_SEP: f64 = 40.0;
const EDGE_SEP: f64 = 16.0;
const RANK_SEP: f64 = 56.0;
const GROUP_PAD: f64 = 20.0;
const LOOP: f64 = 26.0;
const PAD_X: f64 = 12.0;
const TITLE_PAD: f64 = 8.0;
const SECTION_PAD: f64 = 6.0;
const EMPTY_SECTION: f64 = 10.0;
const MIN_WIDTH: f64 = 64.0;
const FOLD: f64 = 8.0;
const LINE: &str = "var(--_l)";
const FONT_MONO: &str = "var(--_fm)";
const TARGETS: [&str; 4] = ["_self", "_blank", "_parent", "_top"];

fn number_option(section: Option<&Value>, key: &str, fallback: f64) -> f64 {
    match section
        .and_then(Value::as_object)
        .and_then(|object| object.get(key))
    {
        Some(Value::Number(value)) if *value > 0.0 => js_min(*value, 2000.0),
        _ => fallback,
    }
}

fn class_section(config: &Config) -> Option<&Value> {
    config.get("class")
}

#[derive(Clone, Debug)]
struct Line {
    text: String,
    width: f64,
    classifier: String,
}

#[derive(Clone, Debug)]
enum Annotation {
    Text(String),
    Math(Label),
}

#[derive(Clone, Debug)]
struct View {
    node: GraphNode,
    style: ResolvedStyle,
    label: Label,
    w: f64,
    h: f64,
    loops: usize,
    annotations: Vec<Annotation>,
    attributes: Vec<Line>,
    methods: Vec<Line>,
    title_h: f64,
    attributes_h: f64,
    font_size: f64,
}

#[derive(Clone, Copy, Debug)]
struct End {
    edge: usize,
    at: usize,
    toward: f64,
    order: usize,
    room: f64,
    side: i32,
    after: bool,
}

#[derive(Clone, Debug)]
struct Card {
    text: String,
    math: Option<Label>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    parent: usize,
}

#[derive(Clone, Copy, Debug)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

fn member_line(member: &ClassMember, size: f64, mono: bool) -> Line {
    let (display, _) = member.display_details();
    let text = trim(&decode_entities(&display)).to_owned();
    let mut style = if member.classifier == "*" { ITALIC } else { 0 };
    if mono {
        style |= MONO;
    }
    Line {
        width: measure(&text, size, style),
        text,
        classifier: member.classifier.clone(),
    }
}

fn set_entry(list: &mut Vec<(String, String)>, key: &str, value: &str) {
    match list.iter_mut().find(|(k, _)| k == key) {
        Some(entry) => entry.1 = value.to_owned(),
        None => list.push((key.to_owned(), value.to_owned())),
    }
}

fn strip_default(raw: &str) -> String {
    match raw.strip_prefix("default") {
        Some(rest) => {
            let mut chars = rest.chars();
            match chars.next() {
                Some(c) if c.is_whitespace() => chars.as_str().to_owned(),
                _ => rest.to_owned(),
            }
        }
        None => raw.to_owned(),
    }
}

fn has_font_family_override(text: &str) -> bool {
    text.match_indices("font-family:")
        .any(|(at, _)| at > 0 && matches!(text.as_bytes()[at - 1], b'"' | b';'))
}

fn js_slice_last(text: &str) -> &str {
    let mut chars = text.chars();
    chars.next_back();
    chars.as_str()
}

pub fn render_class(db: &ClassDb, config: &Config, options: &RenderOptions) -> Drawn {
    let dir = db.direction.clone();
    let fitted = |down: bool| tighten(options, |tight| draw(db, config, options, down, tight));
    if runs_across(Some(dir.as_str())) {
        turn_to_fit(options, fitted)
    } else {
        fitted(false)
    }
}

fn draw(
    db: &ClassDb,
    config: &Config,
    options: &RenderOptions,
    turned: bool,
    tight: bool,
) -> Drawn {
    let conf = class_section(config);
    let graph = build_class_graph(db, class_bool(conf, "hierarchicalNamespaces", true));
    let hide_empty = class_bool(conf, "hideEmptyMembersBox", false);
    let monospaced = options
        .monospace
        .as_ref()
        .is_some_and(|list| list.iter().any(|item| item == "members"));
    let mono = |style: &ResolvedStyle| monospaced && !has_font_family_override(&style.text);
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let tiny = js_round(size * 0.8);
    let tiny_height = js_round(tiny * 1.4);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();
    let root_dir = if turned {
        Dir::TB
    } else {
        direction(Some(db.direction.as_str()))
    };

    let math_text = |raw: &str, font_size: f64| -> Option<Label> {
        if !raw.contains("$$") {
            return None;
        }
        let label = layout_label(Some(raw), false, font_size, 4000.0, 0);
        label
            .lines
            .iter()
            .any(|line| has_math(line))
            .then_some(label)
    };
    let card_width = |raw: &str| -> f64 {
        if raw.is_empty() {
            return 0.0;
        }
        match math_text(raw, tiny) {
            Some(label) => label.width,
            None => measure(&decode_entities(raw), tiny, 0),
        }
    };

    let mut kept: Vec<(String, String)> = Vec::new();
    let mut effect: HashMap<String, String> = HashMap::new();
    let mut from_defs: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let class_style = |cls: &ClassNode,
                       kept: &mut Vec<(String, String)>,
                       effect: &mut HashMap<String, String>,
                       from_defs: &mut HashMap<String, Vec<(String, String)>>|
     -> ResolvedStyle {
        kept.clear();
        let mut keep = |decl: &str, kept: &mut Vec<(String, String)>| {
            let prop = match effect.get(decl) {
                Some(prop) => prop.clone(),
                None => {
                    let one = resolve_style(&[decl.to_owned()]);
                    let prop = if one.shape.is_empty() && one.text.is_empty() {
                        String::new()
                    } else {
                        let cut = decl.find(':').unwrap_or(decl.len().saturating_sub(1));
                        decl[..cut].trim().to_lowercase()
                    };
                    effect.insert(decl.to_owned(), prop.clone());
                    prop
                }
            };
            if !prop.is_empty() {
                set_entry(kept, &prop, decl);
            }
        };
        match from_defs.get(&cls.css_classes).cloned() {
            None => {
                for name in cls.css_classes.split(' ') {
                    if let Some(def) = db.style_classes.get(name) {
                        for s in &def.styles {
                            for part in s.split(',') {
                                keep(part, kept);
                            }
                        }
                    }
                }
                from_defs.insert(cls.css_classes.clone(), kept.clone());
            }
            Some(base) => {
                for (prop, decl) in base {
                    set_entry(kept, &prop, &decl);
                }
            }
        }
        for s in &cls.styles {
            keep(s, kept);
        }
        if kept.is_empty() {
            resolve_style(&[])
        } else {
            let values: Vec<String> = kept.iter().map(|(_, decl)| decl.clone()).collect();
            resolve_style(&values)
        }
    };

    let mut children = vec![0usize; graph.nodes.len()];
    for node in &graph.nodes {
        if node.parent >= 0 {
            children[node.parent as usize] += 1;
        }
    }

    let mut crossed = vec![0u8; graph.nodes.len()];
    for edge in &graph.edges {
        if edge.source < 0 || edge.target < 0 {
            continue;
        }
        let a = graph.nodes[edge.source as usize].parent;
        let b = graph.nodes[edge.target as usize].parent;
        if a == b {
            continue;
        }
        for start in [a, b] {
            let mut g = start;
            while g >= 0 && crossed[g as usize] == 0 {
                crossed[g as usize] = 1;
                g = graph.nodes[g as usize].parent;
            }
        }
    }

    let class_view = |node: &GraphNode,
                      cls: &ClassNode,
                      kept: &mut Vec<(String, String)>,
                      effect: &mut HashMap<String, String>,
                      from_defs: &mut HashMap<String, Vec<(String, String)>>|
     -> View {
        let style = class_style(cls, kept, effect, from_defs);
        let font_size = style.font_size.unwrap_or(size);
        let member_size = js_round(font_size * 0.875);
        let line_height = js_round(member_size * 1.5);
        let generic = if cls.type_.is_empty() {
            String::new()
        } else {
            format!(
                "&lt;{}&gt;",
                parse_generic_types(&cls.type_)
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
            )
        };
        let label = layout_label(
            Some(&format!("{}{generic}", cls.label)),
            false,
            font_size,
            4000.0,
            BOLD_STYLE | if style.italic { ITALIC } else { 0 },
        );
        let annotations: Vec<Annotation> = cls
            .annotations
            .iter()
            .map(|a| match math_text(&format!("«{}»", trim(a)), tiny) {
                Some(label) => Annotation::Math(label),
                None => Annotation::Text(format!("«{}»", trim(&decode_entities(a)))),
            })
            .collect();
        let mono_on = mono(&style);
        let attributes: Vec<Line> = cls
            .members
            .iter()
            .map(|m| member_line(m, member_size, mono_on))
            .collect();
        let methods: Vec<Line> = cls
            .methods
            .iter()
            .map(|m| member_line(m, member_size, mono_on))
            .collect();
        let mut inner = label.width;
        let mut annotations_h = 0.0;
        for annotation in &annotations {
            match annotation {
                Annotation::Text(text) => {
                    inner = js_max(inner, measure(text.as_str(), tiny, 0));
                    annotations_h += tiny_height;
                }
                Annotation::Math(label) => {
                    inner = js_max(inner, label.width);
                    annotations_h += js_max(tiny_height, label.height);
                }
            }
        }
        for line in attributes.iter().chain(methods.iter()) {
            inner = js_max(inner, line.width);
        }
        let sections = !(hide_empty && attributes.is_empty() && methods.is_empty());
        let section = |count: usize| -> f64 {
            if !sections {
                0.0
            } else if count > 0 {
                2.0 * SECTION_PAD + count as f64 * line_height
            } else {
                EMPTY_SECTION
            }
        };
        let title_h =
            2.0 * TITLE_PAD + annotations_h + js_max(label.height, js_round(font_size * 1.5));
        let attributes_h = section(attributes.len());
        View {
            node: node.clone(),
            w: js_max(inner + 2.0 * PAD_X, MIN_WIDTH),
            h: title_h + attributes_h + section(methods.len()),
            style,
            label,
            loops: 0,
            annotations,
            attributes,
            methods,
            title_h,
            attributes_h,
            font_size,
        }
    };

    let base = |node: &GraphNode, label: Label, w: f64, h: f64, font_size: f64| -> View {
        View {
            node: node.clone(),
            style: resolve_style(&[]),
            label,
            w,
            h,
            loops: 0,
            annotations: Vec::new(),
            attributes: Vec::new(),
            methods: Vec::new(),
            title_h: 0.0,
            attributes_h: 0.0,
            font_size,
        }
    };

    let mut views: Vec<View> = Vec::new();
    let mut cnodes: Vec<CNode> = Vec::new();
    for (i, node) in graph.nodes.iter().enumerate() {
        let (view, mut c) = if node.is_group {
            let label = layout_label(Some(&node.label), false, size, 4000.0, 0);
            let mut view = base(node, label.clone(), 0.0, 0.0, size);
            let c = if children[i] > 0 {
                let mut c = cnode(0.0, 0.0, -1);
                c.is_group = true;
                c.pad_x = GROUP_PAD;
                c.pad_top = if label.height > 0.0 {
                    label.height + 16.0
                } else {
                    GROUP_PAD
                };
                c.pad_bottom = GROUP_PAD;
                let widen = if crossed[i] == 1 && !tight {
                    js_min(label.width + 24.0, 240.0)
                } else {
                    0.0
                };
                c.min_w = label.width + 2.0 * GROUP_PAD - 12.0 + widen;
                c
            } else {
                view.w = js_max(label.width + 2.0 * GROUP_PAD, 80.0);
                view.h = label.height + 2.0 * GROUP_PAD;
                cnode(view.w, view.h, -1)
            };
            (view, c)
        } else if let Some(cls) = node.class_id.as_deref().and_then(|id| db.classes.get(id)) {
            let view = class_view(node, cls, &mut kept, &mut effect, &mut from_defs);
            let c = cnode(view.w, view.h, -1);
            (view, c)
        } else if node.shape == NodeShape::Note {
            let label = layout_label(Some(&node.label), true, small, 260.0, 0);
            let view = base(
                node,
                label.clone(),
                js_max(label.width + 2.0 * PAD_X, 40.0),
                label.height + 2.0 * TITLE_PAD,
                small,
            );
            let c = cnode(view.w, view.h, -1);
            (view, c)
        } else {
            let label = layout_label(Some(&node.label), false, small, 200.0, 0);
            let view = base(
                node,
                label.clone(),
                js_max(label.width, 12.0) + 4.0,
                js_max(label.height, 12.0),
                small,
            );
            let c = cnode(view.w, view.h, -1);
            (view, c)
        };
        c.parent = node.parent as i32;
        c.seq = i as f64;
        views.push(view);
        cnodes.push(c);
    }

    let mut cedges: Vec<CEdge> = Vec::new();
    let mut drawn: Vec<GraphEdge> = Vec::new();
    let mut edge_labels: Vec<Label> = Vec::new();
    let mut start_room: Vec<f64> = Vec::new();
    let mut end_room: Vec<f64> = Vec::new();
    let mut crowd = vec![0.0f64; views.len() * 4];
    let meet = |crowd: &mut Vec<f64>, item: usize, slot: usize, room: f64| {
        crowd[item * 4 + slot] += 1.0;
        if room > crowd[item * 4 + slot + 1] {
            crowd[item * 4 + slot + 1] = room;
        }
    };
    for edge in &graph.edges {
        if edge.source < 0 || edge.target < 0 {
            continue;
        }
        let (source, target) = (edge.source as usize, edge.target as usize);
        let label = layout_label(edge.label.as_deref(), true, small, 200.0, 0);
        let start_w = card_width(&edge.start_label_right);
        let end_w = card_width(&edge.end_label_left);
        let label_w = if label.width > 0.0 {
            label.width + 12.0
        } else {
            0.0
        };
        if source == target {
            views[source].loops += 1;
            cnodes[source].w += 2.0 * (LOOP + label_w);
        } else {
            meet(&mut crowd, source, 0, start_w);
            meet(&mut crowd, target, 2, end_w);
        }
        start_room.push(start_w);
        end_room.push(end_w);
        drawn.push(edge.clone());
        edge_labels.push(label.clone());
        cedges.push(CEdge {
            src: source,
            dst: target,
            minlen: 1.0,
            label_w,
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

    if matches!(root_dir, Dir::TB | Dir::BT) {
        for i in 0..views.len() {
            if views[i].node.class_id.is_none() {
                continue;
            }
            let mut need = 0.0f64;
            for slot in [0usize, 2] {
                let count = crowd[i * 4 + slot];
                let room = crowd[i * 4 + slot + 1];
                if count > 1.0 {
                    need = js_max(
                        need,
                        (count - 1.0) * (if room > 0.0 { room + 24.0 } else { 14.0 }) + 24.0,
                    );
                }
            }
            need = js_min(need, 480.0);
            if need > views[i].w {
                cnodes[i].w += need - views[i].w;
                views[i].w = need;
            }
        }
    }

    let lettered = (0..drawn.len()).any(|i| start_room[i] > 0.0 || end_room[i] > 0.0);
    let mut tall_cards = 0.0f64;
    if matches!(root_dir, Dir::TB | Dir::BT) {
        let card_height = |raw: &str| -> f64 {
            if raw.is_empty() {
                return 0.0;
            }
            js_max(
                tiny_height,
                math_text(raw, tiny).map_or(0.0, |label| label.height),
            )
        };
        for edge in &drawn {
            tall_cards = js_max(
                tall_cards,
                card_height(&edge.start_label_right) + card_height(&edge.end_label_left)
                    - 2.0 * tiny_height,
            );
        }
    }
    let rank_base = number_option(
        conf,
        "rankSpacing",
        if lettered { RANK_SEP + 16.0 } else { RANK_SEP },
    );
    let layout = compound_layout(
        &mut cnodes,
        &mut cedges,
        root_dir,
        &LayeredOptions {
            node_sep: number_option(conf, "nodeSpacing", NODE_SEP),
            edge_sep: EDGE_SEP,
            rank_sep: rank_base + tall_cards,
            port_sep: 20.0,
            apart: true,
            ..LayeredOptions::default()
        },
    );

    let mut sides: HashMap<usize, Vec<End>> = HashMap::new();
    let mut side_order: Vec<usize> = Vec::new();
    let attach = |cedges: &mut [CEdge],
                  views: &[View],
                  cnodes: &[CNode],
                  sides: &mut HashMap<usize, Vec<End>>,
                  side_order: &mut Vec<usize>,
                  edge: usize,
                  at: usize,
                  item: usize,
                  room: f64,
                  order: usize|
     -> Option<End> {
        let route = &mut cedges[edge].route;
        let c = &cnodes[item];
        let side = end_side(route, at, c);
        if side < 0 {
            return None;
        }
        let slack = (c.w - views[item].w) / 2.0;
        if side == 1 {
            route[at] -= slack;
        } else if side == 3 {
            route[at] += slack;
        }
        let across = (side & 1) as usize;
        let toward = route[(if at == 0 { 3 } else { at - 3 }) + across];
        let bend = toward - route[at + across];
        let after = if across == 1 { bend > 0.0 } else { bend >= 0.0 };
        let end = End {
            edge,
            at,
            toward,
            order,
            room,
            side,
            after,
        };
        let key = item * 4 + side as usize;
        match sides.get_mut(&key) {
            Some(list) => list.push(end),
            None => {
                sides.insert(key, vec![end]);
                side_order.push(key);
            }
        }
        Some(end)
    };
    let mut starts: Vec<Option<End>> = Vec::new();
    let mut ends: Vec<Option<End>> = Vec::new();
    for i in 0..cedges.len() {
        let (src, dst, len) = (cedges[i].src, cedges[i].dst, cedges[i].route.len());
        let routed = src != dst && len >= 6;
        starts.push(if routed {
            attach(
                &mut cedges,
                &views,
                &cnodes,
                &mut sides,
                &mut side_order,
                i,
                0,
                src,
                start_room[i],
                i,
            )
        } else {
            None
        });
        ends.push(if routed {
            attach(
                &mut cedges,
                &views,
                &cnodes,
                &mut sides,
                &mut side_order,
                i,
                len - 3,
                dst,
                end_room[i],
                i,
            )
        } else {
            None
        });
    }

    let on_border = |cnodes: &[CNode], item: usize, x: f64, y: f64| -> bool {
        let mut g = cnodes[item].parent;
        let mut hops = 0;
        while g >= 0 && hops < 10000 {
            let c = &cnodes[g as usize];
            let dx = (x - c.x).abs();
            let dy = (y - c.y).abs();
            if ((dy - c.h / 2.0).abs() < 0.5 && dx <= c.w / 2.0 + 0.5)
                || ((dx - c.w / 2.0).abs() < 0.5 && dy <= c.h / 2.0 + 0.5)
            {
                return true;
            }
            g = cnodes[g as usize].parent;
            hops += 1;
        }
        false
    };
    let mut gaps: Vec<f64> = Vec::new();
    for key in side_order.clone() {
        let mut list = sides.get(&key).cloned().unwrap_or_default();
        if list.len() < 2 {
            continue;
        }
        let item = key >> 2;
        let across = key & 1;
        let length = if across == 1 {
            views[item].h
        } else {
            views[item].w
        };
        list.sort_by(|a, b| {
            let d = a.toward - b.toward;
            if d != 0.0 && !d.is_nan() {
                d.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.order.cmp(&b.order)
            }
        });
        let mut total = 0.0;
        for j in 0..list.len() - 1 {
            let (a, b) = (list[j], list[j + 1]);
            let mut gap = if across == 1 { 20.0 } else { 24.0 };
            if a.room > 0.0 && a.after {
                gap += if across == 1 {
                    tiny_height + 6.0
                } else {
                    a.room
                };
            }
            if b.room > 0.0 && !b.after {
                gap += if across == 1 {
                    tiny_height + 6.0
                } else {
                    b.room
                };
            }
            if gaps.len() <= j {
                gaps.resize(j + 1, 0.0);
            }
            gaps[j] = gap;
            total += gap;
        }
        let scale = js_min(1.0, (length - 24.0) / total);
        if scale <= 0.0 {
            sides.insert(key, list);
            continue;
        }
        let mut at = (if across == 1 {
            cnodes[item].y
        } else {
            cnodes[item].x
        }) - (total * scale) / 2.0;
        for (j, end) in list.iter().enumerate() {
            let route = &mut cedges[end.edge].route;
            let step: isize = if end.at == 0 { 3 } else { -3 };
            let first = (end.at as isize + step) as usize;
            let mut border: isize = -1;
            let mut k = first as isize;
            while k >= 0
                && (k as usize) < route.len()
                && (route[k as usize + across] - route[first + across]).abs() < 0.5
            {
                if on_border(&cnodes, item, route[k as usize], route[k as usize + 1]) {
                    border = k;
                    break;
                }
                k += step;
            }
            if border >= 0 && (route[first + across] - at).abs() <= 24.0 {
                let mut k = first as isize;
                while k != border + step {
                    route[k as usize + across] = at;
                    k += step;
                }
            }
            route[end.at + across] = at;
            at += gaps.get(j).copied().unwrap_or(0.0) * scale;
        }
        sides.insert(key, list);
    }

    let straighten = |route: &mut Vec<f64>, end: Option<End>, room: f64, first: bool| {
        let Some(end) = end else {
            return;
        };
        if room <= 0.0 {
            return;
        }
        let across = (end.side & 1) as usize;
        let at = if first { 0 } else { route.len() - 3 };
        let along = at + 1 - across;
        let reach_from = if first { 3 } else { at - 3 };
        let reach = (route[reach_from + 1 - across] - route[along]).abs();
        let length = js_min(
            if across == 1 {
                room + 10.0
            } else {
                tiny_height + 6.0
            },
            reach * 0.35,
        );
        if length < 4.0 {
            return;
        }
        let mut point = [route[at], route[at + 1], route[at + 2]];
        point[1 - across] += if end.side == 0 || end.side == 3 {
            -1.0
        } else {
            1.0
        } * length;
        let insert_at = if first { 3 } else { at };
        route.splice(insert_at..insert_at, point);
    };
    for i in 0..cedges.len() {
        straighten(&mut cedges[i].route, starts[i], start_room[i], true);
        straighten(&mut cedges[i].route, ends[i], end_room[i], false);
    }

    let mut bounds = Bounds {
        min_x: 0.0,
        min_y: 0.0,
        max_x: layout.width,
        max_y: layout.height,
    };
    let mut cards: Vec<Card> = Vec::new();
    let add_card = |bounds: &mut Bounds,
                    cards: &mut Vec<Card>,
                    parent: usize,
                    raw: &str,
                    width: f64,
                    x: f64,
                    y: f64,
                    side: i32,
                    after: bool| {
        if raw.is_empty() {
            return;
        }
        let math = math_text(raw, tiny);
        let h = match &math {
            Some(label) => js_max(tiny_height, label.height),
            None => tiny_height,
        };
        let (mut cx, mut cy) = (x, y);
        if side & 1 == 1 {
            cx += if side == 3 { -1.0 } else { 1.0 } * (5.0 + width / 2.0);
            cy += if after { 1.0 } else { -1.0 } * (7.0 + h / 2.0);
        } else {
            cx += if after { 1.0 } else { -1.0 } * (9.0 + width / 2.0);
            cy += if side == 0 { -1.0 } else { 1.0 } * (3.0 + h / 2.0);
        }
        cards.push(Card {
            text: decode_entities(raw),
            math,
            x: cx,
            y: cy,
            w: width,
            h,
            parent,
        });
        bounds.min_x = js_min(bounds.min_x, cx - width / 2.0);
        bounds.max_x = js_max(bounds.max_x, cx + width / 2.0);
        bounds.min_y = js_min(bounds.min_y, cy - h / 2.0);
        bounds.max_y = js_max(bounds.max_y, cy + h / 2.0);
    };
    let mut loop_seen: HashMap<usize, usize> = HashMap::new();
    let mut loop_at = vec![0usize; cedges.len()];
    for i in 0..cedges.len() {
        let (src, dst, route_len) = (cedges[i].src, cedges[i].dst, cedges[i].route.len());
        let edge = drawn[i].clone();
        if src == dst {
            let k = loop_seen.get(&src).copied().unwrap_or(0);
            loop_seen.insert(src, k + 1);
            loop_at[i] = k;
            let c = cnodes[src].clone();
            let view_w = views[src].w;
            let view_h = views[src].h;
            let x = c.x + view_w / 2.0;
            let spread = js_min(view_h / 2.0 - 4.0, 8.0 + k as f64 * 6.0);
            add_card(
                &mut bounds,
                &mut cards,
                src,
                &edge.start_label_right,
                start_room[i],
                x,
                c.y - spread,
                1,
                false,
            );
            add_card(
                &mut bounds,
                &mut cards,
                dst,
                &edge.end_label_left,
                end_room[i],
                x,
                c.y + spread,
                1,
                true,
            );
            cedges[i].label_x =
                c.x + view_w / 2.0 + LOOP + k as f64 * 8.0 + 4.0 + cedges[i].label_w / 2.0;
            cedges[i].label_y = c.y;
        } else if route_len >= 6 {
            let last = route_len - 3;
            let route = cedges[i].route.clone();
            let from = starts[i];
            let to = ends[i];
            add_card(
                &mut bounds,
                &mut cards,
                src,
                &edge.start_label_right,
                start_room[i],
                route[0],
                route[1],
                from.map_or(2, |end| end.side),
                from.is_none_or(|end| end.after),
            );
            add_card(
                &mut bounds,
                &mut cards,
                dst,
                &edge.end_label_left,
                end_room[i],
                route[last],
                route[last + 1],
                to.map_or(0, |end| end.side),
                to.is_none_or(|end| end.after),
            );
        }
    }

    if let Some(available) = options.max_width.filter(|a| *a > 0.0)
        && bounds.max_x - bounds.min_x + 2.0 * pad > available
    {
        let n = cnodes.len();
        let card_nodes: Vec<CNode> = cards
            .iter()
            .map(|item| {
                let mut c = cnode(item.w, item.h, item.parent as i32);
                c.x = item.x;
                c.y = item.y;
                c
            })
            .collect();
        let mut all = cnodes.clone();
        all.extend(card_nodes);
        let wrapped = wrap_components(
            &mut all,
            &mut cedges,
            js_max(1.0, available - 2.0 * pad),
            number_option(conf, "nodeSpacing", NODE_SEP),
        );
        cnodes = all[..n].to_vec();
        for (i, item) in cards.iter_mut().enumerate() {
            item.x = all[n + i].x;
            item.y = all[n + i].y;
        }
        if let Some(wrapped) = wrapped {
            bounds.min_x = 0.0;
            bounds.min_y = 0.0;
            bounds.max_x = wrapped.width;
            bounds.max_y = wrapped.height;
        }
    }

    let ox = pad - bounds.min_x;
    let oy = pad - bounds.min_y;
    let width = (bounds.max_x - bounds.min_x + 2.0 * pad).ceil();
    let height = (bounds.max_y - bounds.min_y + 2.0 * pad).ceil();
    let mut painter = Painter {
        options,
        links: Vec::new(),
        size,
        icons,
        tiny,
        tiny_height,
    };

    let mut crossings: Crossings = Crossings::new();
    if graph.nodes.first().is_some_and(|node| node.is_group) {
        for e in &cedges {
            mark_crossings(&mut crossings, &e.route);
        }
    }

    let mut clusters = String::new();
    let mut struck_titles = String::new();
    let mut nodes_out = String::new();
    for i in 0..views.len() {
        let view = views[i].clone();
        let c = cnodes[i].clone();
        let node = view.node.clone();
        let x = c.x + ox;
        let y = c.y + oy;
        let id = esc_text(&node.id);
        let w = if c.is_group { c.w } else { view.w };
        let h = if c.is_group { c.h } else { view.h };
        if node.is_group {
            let title_x = if c.is_group {
                cluster_title_x(&c, view.label.width, &crossings, GROUP_PAD)
            } else {
                c.x
            };
            let title_y = y - h / 2.0 + 8.0 + view.label.height / 2.0;
            let title_attrs = " class=\"pelers-cluster-label\" fill=\"var(--_m)\"";
            let struck = c.is_group && title_crossed(&c, view.label.width, title_x, &crossings);
            if struck {
                struck_titles.push_str(&struck_title(
                    &view.label,
                    title_x + ox,
                    title_y,
                    title_attrs,
                    &id,
                    &cnodes,
                    i,
                ));
            }
            clusters.push_str(&format!(
                "<g class=\"pelers-cluster pelers-namespace\" data-id=\"{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\" fill-opacity=\"0.5\" stroke=\"var(--_b)\"{ROUNDED}/>{}</g>",
                num(x - w / 2.0),
                num(y - h / 2.0),
                num(w),
                num(h),
                if struck {
                    String::new()
                } else {
                    label_svg(&view.label, title_x + ox, title_y, title_attrs)
                }
            ));
            continue;
        }
        let at = format!(" transform=\"translate({},{})\"", num(x), num(y));
        if let Some(cls) = node.class_id.as_deref().and_then(|cid| db.classes.get(cid)) {
            nodes_out.push_str(&class_svg(&view, cls, &id, &at, &mut painter, &mono));
        } else if node.shape == NodeShape::Note {
            let r = w / 2.0;
            let t = -h / 2.0;
            nodes_out.push_str(&format!(
                "<g class=\"pelers-node pelers-note\" data-id=\"{id}\"{at}><path d=\"M{},{}H{}L{},{}V{}H{}Z\" fill=\"var(--_a)\" stroke=\"var(--_b)\" stroke-linejoin=\"round\"/><path d=\"M{},{}V{}H{}\" fill=\"none\" stroke=\"var(--_b)\" stroke-linejoin=\"round\"/>{}</g>",
                num(-r),
                num(t),
                num(r - FOLD),
                num(r),
                num(t + FOLD),
                num(-t),
                num(-r),
                num(r - FOLD),
                num(t),
                num(t + FOLD),
                num(r),
                label_svg_with(&view.label, 0.0, 0.0, &format!(" class=\"pelers-label\" font-size=\"{}\"", num(small)), icons)
            ));
        } else {
            nodes_out.push_str(&format!(
                "<g class=\"pelers-node pelers-interface\" data-id=\"{id}\"{at}>{}</g>",
                label_svg_with(
                    &view.label,
                    0.0,
                    0.0,
                    &format!(" class=\"pelers-label\" font-size=\"{}\"", num(small)),
                    icons
                )
            ));
        }
    }

    let mut edges_out = String::new();
    let mut labels_out = String::new();
    for i in 0..cedges.len() {
        let edge = drawn[i].clone();
        let label = edge_labels[i].clone();
        let start_type = edge.arrow_type_start;
        let end_type = edge.arrow_type_end;
        let start_trim = class_marker_trim(start_type);
        let end_trim = class_marker_trim(end_type);
        let path: EdgePath = if cedges[i].src == cedges[i].dst {
            let c = cnodes[cedges[i].src].clone();
            let view = &views[cedges[i].src];
            let k = loop_at[i] as f64;
            let x = c.x + ox + view.w / 2.0;
            let y = c.y + oy;
            let spread = js_min(view.h / 2.0 - 4.0, 8.0 + k * 6.0);
            let reach = LOOP + k * 8.0;
            EdgePath {
                d: format!(
                    "M{},{}C{},{} {},{} {},{}",
                    num(x + start_trim),
                    num(y - spread),
                    num(x + reach + start_trim),
                    num(y - spread),
                    num(x + reach + end_trim),
                    num(y + spread),
                    num(x + end_trim),
                    num(y + spread)
                ),
                tokens: None,
                sx: x,
                sy: y - spread,
                sdx: -1.0,
                sdy: 0.0,
                ex: x,
                ey: y + spread,
                edx: -1.0,
                edy: 0.0,
            }
        } else {
            for k in (0..cedges[i].route.len()).step_by(3) {
                cedges[i].route[k] += ox;
                cedges[i].route[k + 1] += oy;
            }
            route_path(&cedges[i].route, None, start_trim, end_trim, None, None)
        };
        let attrs = match edge.pattern {
            Pattern::Dashed => " stroke-dasharray=\"6 4\"",
            Pattern::Dotted => " stroke-dasharray=\"2 4\"",
            Pattern::Solid => "",
        };
        let kind = if edge.pattern == Pattern::Dotted {
            " pelers-note-edge"
        } else {
            " pelers-relation"
        };
        edges_out.push_str(&format!(
            "<g class=\"pelers-edge{kind}\" data-id=\"{}\"><path d=\"{}\"{attrs}/>{}{}</g>",
            esc_text(&edge.id),
            path.d,
            class_marker(start_type, path.sx, path.sy, path.sdx, path.sdy, LINE),
            class_marker(end_type, path.ex, path.ey, path.edx, path.edy, LINE)
        ));
        if label.width > 0.0 {
            labels_out.push_str(&edge_label_svg(
                &edge.id,
                &label,
                cedges[i].label_x + ox,
                cedges[i].label_y + oy,
                "",
                icons,
            ));
        }
    }

    let mut cards_out = String::new();
    for item in &cards {
        cards_out.push_str(&match &item.math {
            Some(label) => label_svg_with(
                label,
                item.x + ox,
                item.y + oy,
                " class=\"pelers-cardinality\"",
                icons,
            ),
            None => format!(
                "<text class=\"pelers-cardinality\" x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                num(item.x + ox),
                num(item.y + oy + tiny * 0.35),
                esc(&item.text)
            ),
        });
    }

    let titled = with_title(
        db.title.as_deref(),
        size,
        pad,
        width,
        height,
        &format!(
            "{}{}{}{}{}<g class=\"pelers-nodes\">{nodes_out}</g>",
            if clusters.is_empty() {
                String::new()
            } else {
                format!("<g class=\"pelers-clusters\">{clusters}</g>")
            },
            if edges_out.is_empty() {
                String::new()
            } else {
                format!(
                    "<g class=\"pelers-edges\" fill=\"none\" stroke=\"{LINE}\" stroke-linecap=\"round\">{edges_out}</g>"
                )
            },
            if struck_titles.is_empty() {
                String::new()
            } else {
                format!("<g class=\"pelers-cluster-titles\">{struck_titles}</g>")
            },
            if labels_out.is_empty() {
                String::new()
            } else {
                format!(
                    "<g class=\"pelers-edge-labels\" font-size=\"{}\">{labels_out}</g>",
                    num(small)
                )
            },
            if cards_out.is_empty() {
                String::new()
            } else {
                format!(
                    "<g class=\"pelers-cardinalities\" font-size=\"{}\" fill=\"var(--_m)\">{cards_out}</g>",
                    num(tiny)
                )
            },
        ),
    );
    let svg = svg_document(
        "class",
        titled.width,
        titled.height,
        size,
        options,
        Accessible {
            acc_title: db.acc_title.as_deref(),
            acc_descr: db.acc_descr.as_deref(),
        },
        &titled.content,
    );
    Drawn {
        svg,
        width: titled.width,
        height: titled.height,
        links: painter.links,
    }
}

fn class_bool(section: Option<&Value>, key: &str, fallback: bool) -> bool {
    match section
        .and_then(Value::as_object)
        .and_then(|object| object.get(key))
    {
        Some(Value::Bool(value)) => *value,
        _ => fallback,
    }
}

struct Painter<'a> {
    options: &'a RenderOptions,
    links: Vec<crate::svg::options::LinkInfo>,
    size: f64,
    icons: bool,
    tiny: f64,
    tiny_height: f64,
}

fn class_svg(
    view: &View,
    cls: &ClassNode,
    id: &str,
    at: &str,
    painter: &mut Painter,
    mono: &dyn Fn(&ResolvedStyle) -> bool,
) -> String {
    let (size, icons, tiny, tiny_height) = (
        painter.size,
        painter.icons,
        painter.tiny,
        painter.tiny_height,
    );
    let w = view.w;
    let h = view.h;
    let style = &view.style;
    let left = -w / 2.0;
    let top = -h / 2.0;
    let member_size = js_round(view.font_size * 0.875);
    let line_height = js_round(member_size * 1.5);
    let mut body = format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{}/>",
        num(left),
        num(top),
        num(w),
        num(h),
        rounded(&style.shape)
    );
    let ink = ink_fill(&style.shape, &style.text, None);
    let first = top + view.title_h;
    let second = first + view.attributes_h;
    if h > view.title_h {
        let stroke = match &style.stroke {
            Some(stroke) => format!(" style=\"stroke:{}\"", esc(stroke)),
            None => String::new(),
        };
        body.push_str(&format!(
            "<path class=\"pelers-divider\" d=\"M{},{}H{}M{},{}H{}\" fill=\"none\" stroke=\"var(--_b)\"{stroke}/>",
            num(left),
            num(first),
            num(-left),
            num(left),
            num(second),
            num(-left)
        ));
    }
    let mut y = top + TITLE_PAD;
    let fill_attr = if ink.is_empty() {
        " fill=\"var(--_m)\"".to_owned()
    } else {
        ink.clone()
    };
    for annotation in &view.annotations {
        match annotation {
            Annotation::Text(text) => {
                body.push_str(&format!(
                    "<text class=\"pelers-annotation\" x=\"0\" y=\"{}\" text-anchor=\"middle\" font-size=\"{}\"{fill_attr}>{}</text>",
                    num(y + tiny_height / 2.0 + tiny * 0.35),
                    js_number_to_string(tiny),
                    esc(text)
                ));
                y += tiny_height;
            }
            Annotation::Math(label) => {
                let h = js_max(tiny_height, label.height);
                body.push_str(&label_svg_with(
                    label,
                    0.0,
                    y + h / 2.0,
                    &format!(
                        " class=\"pelers-annotation\" font-size=\"{}\"{fill_attr}",
                        js_number_to_string(tiny)
                    ),
                    icons,
                ));
                y += h;
            }
        }
    }
    let title_size = if view.font_size == size {
        String::new()
    } else {
        format!(" font-size=\"{}\"", num(view.font_size))
    };
    body.push_str(&label_svg_with(
        &view.label,
        0.0,
        (y + first - TITLE_PAD) / 2.0,
        &format!(
            " class=\"pelers-label\"{title_size} font-weight=\"var(--_hw)\"{ink}{}",
            style.text
        ),
        icons,
    ));
    let lines = |list: &[Line], kind: &str, from: f64| -> String {
        let mut out = String::new();
        let mut baseline = from + SECTION_PAD + line_height / 2.0 + member_size * 0.35;
        for line in list {
            let mark = match line.classifier.as_str() {
                "$" => {
                    let tail = if style.text.is_empty() {
                        " style=\"text-decoration:underline\"".to_owned()
                    } else {
                        format!("{}text-decoration:underline\"", js_slice_last(&style.text))
                    };
                    format!(" pelers-static\"{tail}")
                }
                "*" => format!(" pelers-abstract\" font-style=\"italic\"{}", style.text),
                _ => format!("\"{}", style.text),
            };
            out.push_str(&format!(
                "<text class=\"pelers-member pelers-{kind}{mark} x=\"{}\" y=\"{}\">{}</text>",
                num(left + PAD_X),
                num(baseline),
                esc(&line.text)
            ));
            baseline += line_height;
        }
        out
    };
    if !view.attributes.is_empty() || !view.methods.is_empty() {
        let font_family = if mono(style) {
            format!(" font-family=\"{FONT_MONO}\"")
        } else {
            String::new()
        };
        body.push_str(&format!(
            "<g class=\"pelers-members\" font-size=\"{}\"{ink}{font_family}>{}{}</g>",
            js_number_to_string(member_size),
            lines(&view.attributes, "attribute", first),
            lines(&view.methods, "method", second)
        ));
    }
    if let Some(tooltip) = &cls.tooltip {
        body = format!("<title>{}</title>{body}", esc_text(tooltip));
    }
    if let Some(link) = &cls.link {
        let target = match &cls.link_target {
            Some(target) if TARGETS.contains(&target.as_str()) => format!(" target=\"{target}\""),
            _ => String::new(),
        };
        body = linked(
            &body,
            link,
            &cls.id,
            &mut painter.links,
            painter.options,
            &target,
        );
    }
    format!(
        "<g class=\"pelers-node pelers-class{}\" data-id=\"{id}\"{at}>{body}</g>",
        class_names(&strip_default(&cls.css_classes))
    )
}
