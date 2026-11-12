use std::collections::HashMap;

use super::db::StateDb;
use super::graph::{StateEdge, StateNode};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{runs_across, turn_to_fit};
use crate::layout::compound::compound_layout;
use crate::layout::{CEdge, CNode, Dir, LayeredOptions, cnode, direction};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg_with, num};
use crate::svg::edges::{
    EdgePath, Fill, SideReach, casings_svg, edge_casing, edge_label_svg, marker, marker_trim,
    over_fills, route_path,
};
use crate::svg::ink::{ink_fill, paints};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::shapes::end_side;
use crate::svg::theme::{ResolvedStyle, class_names, resolve_style, sets_fill};
use crate::svg::title::with_title;
use crate::text::entities::decode_entities;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, ITALIC};
use crate::util::jsnum::{js_max, js_min, js_round};
use crate::util::url::{linked, safe_url};
use crate::util::value::Value;

const NODE_SEP: f64 = 40.0;
const EDGE_SEP: f64 = 16.0;
const RANK_SEP: f64 = 44.0;
const GROUP_PAD: f64 = 16.0;
const REGION_PAD: f64 = 10.0;
const PAD_X: f64 = 16.0;
const PAD_Y: f64 = 10.0;
const LOOP: f64 = 26.0;
const FOLD: f64 = 8.0;
const NOTE_WRAP: f64 = 320.0;
const CORNER: f64 = 10.0;
const BAR: f64 = 8.0;
const SURFACE: &str = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";
const RULE: &str = " fill=\"none\" stroke=\"var(--_b)\"";
const MERMAID_CLASSES: [&str; 4] = [
    "statediagram-state",
    "statediagram-cluster-alt",
    "statediagram-cluster",
    "statediagram-note",
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum K {
    State,
    Described,
    Start,
    End,
    Choice,
    Fork,
    Join,
    Note,
    Composite,
    Region,
    Hidden,
}

const CLASSES: [&str; 9] = [
    "pelers-node pelers-state",
    "pelers-node pelers-state",
    "pelers-node pelers-state-start",
    "pelers-node pelers-state-end",
    "pelers-node pelers-state-choice",
    "pelers-node pelers-state-fork",
    "pelers-node pelers-state-join",
    "pelers-note",
    "pelers-cluster pelers-state-composite",
];

fn kind_of(shape: Option<&str>) -> K {
    match shape {
        Some("rectWithTitle") => K::Described,
        Some("stateStart") => K::Start,
        Some("stateEnd") => K::End,
        Some("choice") => K::Choice,
        Some("fork") => K::Fork,
        Some("join") => K::Join,
        Some("note") => K::Note,
        Some("divider") => K::Hidden,
        _ => K::State,
    }
}

struct View<'a> {
    node: Option<&'a StateNode>,
    kind: K,
    style: Option<ResolvedStyle>,
    label: Label,
    lines: Vec<Label>,
    w: f64,
    h: f64,
    title_h: f64,
}

fn vertical(dir: Dir) -> bool {
    matches!(dir, Dir::TB | Dir::BT)
}

fn number_option(config: &Config, key: &str, fallback: f64) -> f64 {
    match config
        .get("state")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::Number(value)) if *value > 0.0 => js_min(*value, 2000.0),
        _ => fallback,
    }
}

fn curve_of(config: &Config) -> Option<String> {
    match config
        .get("flowchart")
        .and_then(Value::as_object)
        .and_then(|section| section.get("curve"))
    {
        Some(Value::String(curve)) => Some(curve.clone()),
        _ => None,
    }
}

fn unquote(text: &str) -> &str {
    text.trim_matches('"')
}

fn strip_mermaid_classes(raw: &str) -> String {
    raw.split(' ')
        .filter(|token| !MERMAID_CLASSES.contains(token))
        .collect::<Vec<_>>()
        .join(" ")
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn js_sign(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

fn js_or_one(value: f64) -> f64 {
    if value == 0.0 || value.is_nan() {
        1.0
    } else {
        value
    }
}

fn rect(x: f64, y: f64, w: f64, h: f64, rest: &str) -> String {
    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{rest}/>",
        num(x),
        num(y),
        num(w),
        num(h)
    )
}

fn group(dir: Dir, pad_x: f64, pad_top: f64, pad_bottom: f64) -> CNode {
    let mut c = cnode(0.0, 0.0, -1);
    c.is_group = true;
    c.dir = Some(dir);
    c.pad_x = pad_x;
    c.pad_top = pad_top;
    c.pad_bottom = pad_bottom;
    c
}

fn title_x(c: &CNode, width: f64, crossings: &HashMap<i64, Vec<f64>>) -> f64 {
    let Some(xs) = crossings.get(&(js_round(c.y - c.h / 2.0) as i64)) else {
        return c.x;
    };
    if width == 0.0 {
        return c.x;
    }
    let left = c.x - c.w / 2.0;
    let half = width / 2.0;
    let candidates = [c.x, left + GROUP_PAD + half, left + c.w - GROUP_PAD - half];
    let mut best = c.x;
    let mut best_gap = -1.0f64;
    for x in candidates {
        let mut gap = f64::INFINITY;
        for &cx in xs {
            if cx > left && cx < left + c.w {
                gap = js_min(gap, (cx - x).abs() - half);
            }
        }
        if gap >= 6.0 {
            return x;
        }
        if gap > best_gap + 0.5 {
            best_gap = gap;
            best = x;
        }
    }
    best
}

fn divide(
    items: &[usize],
    views: &[View],
    cnodes: &[CNode],
    dividers: &mut String,
    pad: f64,
    bounds: (f64, f64, f64, f64),
) {
    let (left, top, right, bottom) = bounds;
    let regions: Vec<usize> = items
        .iter()
        .copied()
        .filter(|&i| views[i].kind == K::Region)
        .collect();
    if regions.len() < 2 {
        return;
    }
    let lo = |i: usize, x: bool| {
        if x {
            cnodes[i].x - cnodes[i].w / 2.0
        } else {
            cnodes[i].y - cnodes[i].h / 2.0
        }
    };
    let hi = |i: usize, x: bool| {
        if x {
            cnodes[i].x + cnodes[i].w / 2.0
        } else {
            cnodes[i].y + cnodes[i].h / 2.0
        }
    };
    for x in [true, false] {
        let mut sorted = regions.clone();
        sorted.sort_by(|&a, &b| {
            let d = lo(a, x) - lo(b, x);
            if d != 0.0 && !d.is_nan() {
                d.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.cmp(&b)
            }
        });
        let apart = (1..sorted.len()).all(|k| hi(sorted[k - 1], x) <= lo(sorted[k], x) + 0.5);
        if !apart {
            continue;
        }
        for k in 1..sorted.len() {
            let at = num((hi(sorted[k - 1], x) + lo(sorted[k], x)) / 2.0 + pad);
            let d = if x {
                format!("M{at},{}V{}", num(top), num(bottom))
            } else {
                format!("M{},{at}H{}", num(left), num(right))
            };
            dividers.push_str(&format!("<path class=\"pelers-divider\" d=\"{d}\"/>"));
        }
        return;
    }
    for i in regions {
        let c = &cnodes[i];
        dividers.push_str(&rect(
            c.x - c.w / 2.0 + pad,
            c.y - c.h / 2.0 + pad,
            c.w,
            c.h,
            &format!(" class=\"pelers-divider\" rx=\"{}\"", num(CORNER)),
        ));
    }
}

fn attach(route: &mut [f64], at: usize, c: &CNode, view: &View, pad: f64) {
    if c.is_group {
        return;
    }
    let bar = matches!(view.kind, K::Fork | K::Join);
    let other = if at == 0 { 3 } else { at - 3 };
    for axis in [1usize, 0] {
        let extent = if axis == 1 { c.h } else { c.w };
        let drawn_extent = if axis == 1 { view.h } else { view.w };
        let breadth = if axis == 1 { view.w } else { view.h };
        let center = (if axis == 1 { c.y } else { c.x }) + pad;
        let cross = (if axis == 1 { c.x } else { c.y }) + pad;
        let delta = route[at + axis] - center;
        if ((delta.abs() - extent / 2.0).abs()) >= 0.5 {
            continue;
        }
        route[at + axis] -= (js_sign(delta) * (extent - drawn_extent)) / 2.0;
        if bar && breadth > drawn_extent {
            route[at + 1 - axis] = clamp(
                route[other + 1 - axis],
                cross - breadth / 2.0 + 6.0,
                cross + breadth / 2.0 - 6.0,
            );
        }
        return;
    }
}

pub fn render_state(db: &StateDb, config: &Config, options: &RenderOptions) -> Drawn {
    let dir = db.direction();
    if !runs_across(Some(dir.as_str())) {
        return draw(db, config, options, false);
    }
    turn_to_fit(options, |down| draw(db, config, options, down))
}

fn draw(db: &StateDb, config: &Config, options: &RenderOptions, turned: bool) -> Drawn {
    let graph = &db.graph;
    let curve = curve_of(config);
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let wrap_width = number_option(config, "wrappingWidth", 200.0);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();
    let dir = db.direction();
    let root_dir = if turned {
        Dir::TB
    } else {
        direction(Some(dir.as_str()))
    };
    let mut links: Vec<crate::svg::options::LinkInfo> = Vec::new();
    let no_label = layout_label(None, false, size, 0.0, 0);
    let text = |raw: Option<&str>, font_size: f64, wrap: f64, style: u32| {
        layout_label(raw, true, font_size, wrap, style)
    };

    let mut index: HashMap<&str, usize> = HashMap::new();
    let mut views: Vec<View> = Vec::new();
    let mut cnodes: Vec<CNode> = Vec::new();
    for node in &graph.nodes {
        if node.shape.as_deref() == Some("noteGroup") {
            continue;
        }
        let kind = if node.is_group {
            if node.shape.as_deref() == Some("divider") {
                K::Region
            } else {
                K::Composite
            }
        } else {
            kind_of(node.shape.as_deref())
        };
        index.insert(node.id.as_str(), views.len());
        views.push(View {
            node: Some(node),
            kind,
            style: None,
            label: no_label.clone(),
            lines: Vec::new(),
            w: 0.0,
            h: 0.0,
            title_h: 0.0,
        });
    }
    let states = views.len();

    let mut logical: Vec<i64> = vec![-1; states];
    for i in 0..states {
        let parent_id = views[i].node.and_then(|n| n.parent_id.as_deref());
        let parent = parent_id.and_then(|id| index.get(id).copied());
        logical[i] = match parent {
            Some(p) if p != i && views[p].node.is_some_and(|n| n.is_group) => p as i64,
            _ => -1,
        };
    }
    {
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); states];
        let mut queue: Vec<usize> = Vec::new();
        let mut placed = vec![false; states];
        for i in 0..states {
            if logical[i] == -1 {
                placed[i] = true;
                queue.push(i);
            } else {
                kids[logical[i] as usize].push(i);
            }
        }
        let mut done = 0;
        let settle = |queue: &mut Vec<usize>, placed: &mut Vec<bool>, done: &mut usize| {
            while *done < queue.len() {
                for &kid in &kids[queue[*done]] {
                    if placed[kid] {
                        continue;
                    }
                    placed[kid] = true;
                    queue.push(kid);
                }
                *done += 1;
            }
        };
        settle(&mut queue, &mut placed, &mut done);
        let mut seen = vec![-1i64; states];
        for i in 0..states {
            if placed[i] {
                continue;
            }
            let mut ring = i;
            while seen[ring] != i as i64 {
                seen[ring] = i as i64;
                ring = logical[ring] as usize;
            }
            logical[ring] = -1;
            placed[ring] = true;
            queue.push(ring);
            settle(&mut queue, &mut placed, &mut done);
        }
    }
    let mut owners = vec![-1i64; states];
    for i in 0..states {
        let owner = views[i]
            .node
            .and_then(|n| n.note_for.as_deref())
            .and_then(|id| index.get(id).copied());
        let Some(owner) = owner else {
            continue;
        };
        owners[i] = owner as i64;
        logical[i] = logical[owner];
    }

    let inner_dirs: Vec<Dir> = views
        .iter()
        .map(|view| direction(view.node.and_then(|n| n.dir.as_deref())))
        .collect();
    let across = |i: usize, logical: &[i64]| -> bool {
        vertical(if logical[i] == -1 {
            root_dir
        } else {
            inner_dirs[logical[i] as usize]
        })
    };

    let mut degree = vec![0i64; states];
    let mut noted = vec![false; states];
    let mut appear: Vec<f64> = vec![f64::INFINITY; states];
    let mut appeared = 0.0f64;
    let mut counts = vec![0i64; 2 * states];
    for edge in &graph.edges {
        let (Some(&src), Some(&dst)) =
            (index.get(edge.start.as_str()), index.get(edge.end.as_str()))
        else {
            continue;
        };
        if edge.dashed {
            let target = if views[src].kind == K::Note { dst } else { src };
            noted[target] = true;
            continue;
        }
        counts[2 * src] += 1;
        degree[src] = degree[src].max(counts[2 * src]);
        counts[2 * dst + 1] += 1;
        degree[dst] = degree[dst].max(counts[2 * dst + 1]);
        if appear[src] == f64::INFINITY {
            appear[src] = appeared;
            appeared += 1.0;
        }
        if appear[dst] == f64::INFINITY {
            appear[dst] = appeared;
            appeared += 1.0;
        }
    }

    for i in 0..states {
        let node = views[i].node.expect("state views have nodes");
        let style = resolve_style(&if node.css_styles.is_empty() {
            node.css_compiled_styles.clone()
        } else {
            [node.css_compiled_styles.clone(), node.css_styles.clone()].concat()
        });
        let text_style =
            (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let font_size = style.font_size.unwrap_or(size);
        let horizontal = across(i, &logical);
        let kind = views[i].kind;
        let mut c: Option<CNode> = None;
        let (mut w, mut h) = (0.0, 0.0);
        match kind {
            K::Composite => {
                let label = text(node.label.as_deref(), font_size, 4000.0, text_style);
                views[i].title_h = if label.height > 0.0 {
                    label.height + 12.0
                } else {
                    0.0
                };
                let mut composite = group(
                    direction(views[i].node.and_then(|n| n.dir.as_deref())),
                    GROUP_PAD,
                    views[i].title_h + GROUP_PAD,
                    GROUP_PAD,
                );
                composite.min_w = label.width + 2.0 * GROUP_PAD;
                views[i].label = label;
                c = Some(composite);
            }
            K::Region => {
                c = Some(group(
                    direction(views[i].node.and_then(|n| n.dir.as_deref())),
                    REGION_PAD,
                    REGION_PAD,
                    REGION_PAD,
                ));
            }
            K::Start => {
                w = 14.0;
                h = 14.0;
            }
            K::End => {
                w = 18.0;
                h = 18.0;
            }
            K::Choice => {
                w = 30.0;
                h = 30.0;
            }
            K::Fork | K::Join => {
                let length = clamp(40.0 * degree[i] as f64 + 16.0, 72.0, 320.0);
                w = if horizontal { length } else { BAR };
                h = if horizontal { BAR } else { length };
            }
            K::Hidden => {}
            K::Note => {
                let label = text(
                    node.label.as_deref(),
                    small,
                    js_max(wrap_width, NOTE_WRAP),
                    0,
                );
                w = label.width + 24.0;
                h = label.height + 16.0;
                views[i].label = label;
            }
            K::State | K::Described => {
                let label = text(node.label.as_deref(), font_size, wrap_width, text_style);
                let mut width = label.width;
                let mut height = label.height + 2.0 * PAD_Y;
                if kind == K::Described {
                    height += 14.0;
                    for line in &node.description {
                        let shown = if line.is_empty() { " " } else { line.as_str() };
                        let line_label = text(Some(shown), small, wrap_width, text_style);
                        if line_label.width > width {
                            width = line_label.width;
                        }
                        height += line_label.height;
                        views[i].lines.push(line_label);
                    }
                }
                w = js_max(width + 2.0 * PAD_X, 56.0);
                h = height;
                views[i].label = label;
            }
        }
        views[i].style = Some(style);
        views[i].w = w;
        views[i].h = h;
        let mut c = c.unwrap_or_else(|| cnode(w, h, -1));
        if matches!(kind, K::State | K::Described) {
            c.span = (if horizontal { w } else { h }) - 24.0;
        }
        cnodes.push(c);
    }

    let mut cedges: Vec<CEdge> = Vec::new();
    let mut edge_labels: Vec<Label> = Vec::new();
    let mut drawn: Vec<&StateEdge> = Vec::new();
    let mut loop_count = vec![0i64; states];
    let mut loop_width = vec![0.0f64; states];
    let mut loop_height = vec![0.0f64; states];
    let mut anchors: HashMap<usize, usize> = HashMap::new();
    let encloses = |composite: usize, item: usize, logical: &[i64], cnodes: &[CNode]| -> bool {
        if !cnodes[composite].is_group {
            return false;
        }
        let mut p = logical[item];
        while p != -1 {
            if p as usize == composite {
                return true;
            }
            p = logical[p as usize];
        }
        false
    };
    for edge in &graph.edges {
        let (Some(&start), Some(&end)) =
            (index.get(edge.start.as_str()), index.get(edge.end.as_str()))
        else {
            continue;
        };
        let mut src = start;
        let mut dst = end;
        let label = text(Some(edge.label.as_str()), small, wrap_width, 0);
        let label_w = if label.width > 0.0 {
            label.width + 12.0
        } else {
            0.0
        };
        if src == dst {
            loop_count[src] += 1;
            loop_width[src] = js_max(loop_width[src], label_w);
            loop_height[src] += label.height;
        } else if encloses(src, dst, &logical, &cnodes) {
            anchors.insert(cedges.len(), src);
            appear.push(-1.0);
            views.push(View {
                node: None,
                kind: K::Hidden,
                style: None,
                label: no_label.clone(),
                lines: Vec::new(),
                w: 0.0,
                h: 0.0,
                title_h: 0.0,
            });
            cnodes.push(cnode(0.0, 0.0, src as i32));
            src = views.len() - 1;
        } else if encloses(dst, src, &logical, &cnodes) {
            anchors.insert(cedges.len(), dst);
            appear.push(f64::INFINITY);
            views.push(View {
                node: None,
                kind: K::Hidden,
                style: None,
                label: no_label.clone(),
                lines: Vec::new(),
                w: 0.0,
                h: 0.0,
                title_h: 0.0,
            });
            cnodes.push(cnode(0.0, 0.0, dst as i32));
            dst = views.len() - 1;
        }
        drawn.push(edge);
        cedges.push(CEdge {
            src,
            dst,
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
        edge_labels.push(label);
    }
    for i in 0..states {
        if owners[i] != -1 {
            continue;
        }
        let horizontal = across(i, &logical);
        let mut room = 0.0;
        if loop_count[i] > 0 {
            let reach = LOOP + (loop_count[i] - 1) as f64 * 8.0;
            room = if horizontal {
                reach + loop_width[i]
            } else {
                reach * 0.75 + 4.0 + loop_height[i]
            };
            if !cnodes[i].is_group {
                if horizontal {
                    cnodes[i].w += 2.0 * room;
                    cnodes[i].h = js_max(cnodes[i].h, loop_height[i]);
                } else {
                    cnodes[i].h += 2.0 * room;
                    cnodes[i].w = js_max(cnodes[i].w, loop_width[i]);
                }
                room = 0.0;
            }
        }
        if !noted[i] && room == 0.0 {
            cnodes[i].parent = logical[i] as i32;
            continue;
        }
        let mut c = if horizontal {
            group(Dir::LR, room, 0.0, 0.0)
        } else {
            group(Dir::TB, 0.0, room, room)
        };
        c.parent = logical[i] as i32;
        appear.push(appear[i]);
        views.push(View {
            node: None,
            kind: K::Hidden,
            style: None,
            label: no_label.clone(),
            lines: Vec::new(),
            w: 0.0,
            h: 0.0,
            title_h: 0.0,
        });
        cnodes.push(c);
        cnodes[i].parent = (views.len() - 1) as i32;
    }
    for i in 0..states {
        if owners[i] != -1 {
            cnodes[i].parent = cnodes[owners[i] as usize].parent;
        }
    }

    {
        let mut members: Vec<Vec<usize>> = vec![Vec::new(); cnodes.len()];
        let mut top: Vec<usize> = Vec::new();
        for (i, c) in cnodes.iter().enumerate() {
            let p = c.parent;
            if p >= 0 {
                members[p as usize].push(i);
            } else {
                top.push(i);
            }
        }
        let initial = |i: usize| if views[i].kind == K::Start { 0 } else { 1 };
        let by_flow = |a: &usize, b: &usize| {
            initial(*a)
                .cmp(&initial(*b))
                .then_with(|| {
                    let (x, y) = (appear[*a], appear[*b]);
                    if x == y {
                        std::cmp::Ordering::Equal
                    } else if x < y {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                })
                .then(a.cmp(b))
        };
        let mut seq = 0.0;
        top.sort_by(by_flow);
        let mut pending: Vec<usize> = top.into_iter().rev().collect();
        while let Some(i) = pending.pop() {
            cnodes[i].seq = seq;
            seq += 1.0;
            let mut inside = std::mem::take(&mut members[i]);
            inside.sort_by(by_flow);
            for &k in inside.iter().rev() {
                pending.push(k);
            }
        }
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
            tracks: matches!(curve.as_deref(), Some("step" | "stepBefore" | "stepAfter")),
            straight: curve.as_deref() == Some("linear"),
            head_room: marker_trim("arrow_point"),
            ..LayeredOptions::default()
        },
    );

    let width = (layout.width + 2.0 * pad).ceil();
    let height = (layout.height + 2.0 * pad).ceil();

    let mut crossings: HashMap<i64, Vec<f64>> = HashMap::new();
    for e in &cedges {
        let route = &e.route;
        let mut k = 3;
        while k + 3 < route.len() {
            crossings
                .entry(js_round(route[k + 1]) as i64)
                .or_default()
                .push(route[k]);
            k += 3;
        }
    }

    let mut children: Vec<Vec<usize>> = vec![Vec::new(); cnodes.len()];
    let mut order: Vec<usize> = Vec::new();
    for (i, c) in cnodes.iter().enumerate() {
        let p = c.parent;
        if p >= 0 {
            children[p as usize].push(i);
        } else {
            order.push(i);
        }
    }
    let roots = order.clone();
    let mut q = 0;
    while q < order.len() {
        let parent = order[q];
        order.extend(children[parent].iter().copied());
        q += 1;
    }

    let mut clusters = String::new();
    let mut fills: Vec<Fill> = Vec::new();
    let mut dividers = String::new();
    let mut nodes_out = String::new();
    let mut notes_out = String::new();
    divide(
        &roots,
        &views,
        &cnodes,
        &mut dividers,
        pad,
        (pad / 2.0, pad / 2.0, width - pad / 2.0, height - pad / 2.0),
    );

    for &i in &order {
        let view = &views[i];
        let (Some(node), Some(style)) = (view.node, view.style.as_ref()) else {
            continue;
        };
        if view.kind >= K::Region {
            continue;
        }
        let c = cnodes[i].clone();
        let x = c.x + pad;
        let y = c.y + pad;
        let w = view.w;
        let h = view.h;
        let ink = ink_fill(&style.shape, &style.text, None);
        let label_attrs = format!(" class=\"pelers-label\"{ink}{}", style.text);
        let mut body = match view.kind {
            K::Composite => {
                let left = x - c.w / 2.0;
                let top = y - c.h / 2.0;
                if paints(&style.shape) {
                    fills.push(Fill {
                        box_: [left, top, left + c.w, top + c.h],
                        shape: style.shape.clone(),
                    });
                }
                let mut body = rect(
                    left,
                    top,
                    c.w,
                    c.h,
                    &format!(
                        " rx=\"{}\" fill=\"var(--_a)\"{} stroke=\"var(--_b)\"{}",
                        num(CORNER),
                        if sets_fill(&style.shape) {
                            ""
                        } else {
                            " fill-opacity=\"0.5\""
                        },
                        style.shape
                    ),
                );
                if view.title_h > 0.0 {
                    body.push_str(&format!(
                        "<path d=\"M{},{}H{}\"{RULE}{}/>",
                        num(left),
                        num(top + view.title_h),
                        num(left + c.w),
                        style.line
                    ));
                    body.push_str(&label_svg_with(
                        &view.label,
                        title_x(&c, view.label.width, &crossings) + pad,
                        top + view.title_h / 2.0,
                        &format!(" class=\"pelers-cluster-label\"{ink}{}", style.text),
                        icons,
                    ));
                }
                divide(
                    &children[i],
                    &views,
                    &cnodes,
                    &mut dividers,
                    pad,
                    (left, top + view.title_h, left + c.w, top + c.h),
                );
                body
            }
            K::Start => format!("<circle r=\"7\" fill=\"var(--_l)\"{}/>", style.shape),
            K::End => format!(
                "<circle r=\"8.5\" fill=\"var(--_bg)\" stroke=\"var(--_l)\"{}/><circle r=\"5\" fill=\"var(--_l)\"{}/>",
                style.line, style.shape
            ),
            K::Choice => format!(
                "<polygon points=\"0,{} {},0 0,{} {},0\"{SURFACE}{}/>",
                num(-h / 2.0),
                num(w / 2.0),
                num(h / 2.0),
                num(-w / 2.0),
                style.shape
            ),
            K::Fork | K::Join => rect(
                -w / 2.0,
                -h / 2.0,
                w,
                h,
                &format!(" rx=\"2\" fill=\"var(--_l)\"{}", style.shape),
            ),
            K::Note => {
                let r = num(w / 2.0);
                let b = num(h / 2.0);
                let fold = num(w / 2.0 - FOLD);
                let corner = num(FOLD - h / 2.0);
                let mut body = format!(
                    "<path d=\"M-{r},-{b}H{fold}L{r},{corner}V{b}H-{r}Z\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"/>\
<path d=\"M{fold},-{b}V{corner}H{r}\"{RULE}/>"
                );
                body.push_str(&label_svg_with(
                    &view.label,
                    0.0,
                    0.0,
                    &format!(
                        " class=\"pelers-label\" font-size=\"{}\" fill=\"var(--_m)\"",
                        num(small)
                    ),
                    icons,
                ));
                body
            }
            _ => {
                let mut body = rect(
                    -w / 2.0,
                    -h / 2.0,
                    w,
                    h,
                    &format!(
                        " rx=\"{}\"{SURFACE}{}",
                        num(CORNER.min(h / 2.0)),
                        style.shape
                    ),
                );
                if view.kind == K::State {
                    body.push_str(&label_svg_with(&view.label, 0.0, 0.0, &label_attrs, icons));
                } else {
                    let mut at = PAD_Y - h / 2.0 + view.label.height;
                    body.push_str(&format!(
                        "<path d=\"M{},{}H{}\"{RULE}{}/>",
                        num(-w / 2.0),
                        num(at + 6.0),
                        num(w / 2.0),
                        style.line
                    ));
                    body.push_str(&label_svg_with(
                        &view.label,
                        0.0,
                        at - view.label.height / 2.0,
                        &label_attrs,
                        icons,
                    ));
                    at += 14.0;
                    for line in &view.lines {
                        body.push_str(&label_svg_with(
                            line,
                            0.0,
                            at + line.height / 2.0,
                            &format!(
                                " class=\"pelers-state-description\" font-size=\"{}\"{ink}{}",
                                num(small),
                                style.text
                            ),
                            icons,
                        ));
                        at += line.height;
                    }
                }
                body
            }
        };

        if let Some(link) = db.links.get(&node.id) {
            let tooltip = decode_entities(unquote(&link.tooltip));
            if !tooltip.is_empty() {
                body = format!("<title>{}</title>{body}", esc(&tooltip));
            }
            body = linked(
                &body,
                &safe_url(unquote(&link.url)),
                &node.id,
                &mut links,
                options,
                " target=\"_blank\"",
            );
        }
        let open = format!(
            "<g class=\"{}{}\" data-id=\"{}\"",
            CLASSES[view.kind as usize],
            class_names(&strip_mermaid_classes(&node.css_classes)),
            esc_text(&node.id)
        );
        if view.kind == K::Composite {
            clusters.push_str(&format!("{open}>{body}</g>"));
        } else if view.kind == K::Note {
            notes_out.push_str(&format!(
                "{open} transform=\"translate({},{})\">{body}</g>",
                num(x),
                num(y)
            ));
        } else {
            nodes_out.push_str(&format!(
                "{open} transform=\"translate({},{})\">{body}</g>",
                num(x),
                num(y)
            ));
        }
    }

    let reach = {
        let mut reach = SideReach::default();
        for (i, ce) in cedges.iter().enumerate() {
            if ce.src == ce.dst || ce.route.len() < 6 || drawn[i].dashed {
                continue;
            }
            reach.add(
                ce.dst as i64,
                i64::from(end_side(&ce.route, ce.route.len() - 3, &cnodes[ce.dst])),
                "arrow_point",
            );
        }
        reach
    };

    let mut casings = String::new();
    let mut edges_out = String::new();
    let mut labels_out = String::new();
    let mut loop_seen = vec![0i64; states];
    let mut loop_text = vec![0.0f64; states];
    for i in 0..cedges.len() {
        let edge = drawn[i];
        let label = edge_labels[i].clone();
        let note = edge.dashed;
        let end_type = if note { "none" } else { "arrow_point" };
        let src = cedges[i].src;
        let dst = cedges[i].dst;
        let path: EdgePath = if src == dst {
            let a = cnodes[src].clone();
            let view_w = views[src].w;
            let view_h = views[src].h;
            let k = loop_seen[src];
            loop_seen[src] += 1;
            let side = across(src, &logical);
            let w = if a.is_group { a.w } else { view_w };
            let h = if a.is_group { a.h } else { view_h };
            let out = (if side { w } else { h }) / 2.0;
            let spread = js_max(
                js_min((if side { h } else { w }) / 2.0 - 4.0, 8.0 + k as f64 * 6.0),
                2.0,
            );
            let loop_reach = LOOP + k as f64 * 8.0;
            let trim = marker_trim(end_type) / js_or_one(loop_reach.hypot(spread));
            let px = |dx: f64, dy: f64| a.x + pad + if side { out + dx } else { -dy };
            let py = |dx: f64, dy: f64| a.y + pad + if side { dy } else { out + dx };
            let at = |dx: f64, dy: f64| format!("{},{}", num(px(dx, dy)), num(py(dx, dy)));
            let mut p = EdgePath {
                d: format!(
                    "M{}C{} {} {}",
                    at(0.0, -spread),
                    at(loop_reach, -2.0 * spread),
                    at(loop_reach, 2.0 * spread),
                    at(loop_reach * trim, spread + spread * trim)
                ),
                tokens: None,
                sx: 0.0,
                sy: 0.0,
                sdx: 0.0,
                sdy: 0.0,
                ex: px(0.0, spread),
                ey: py(0.0, spread),
                edx: px(-loop_reach, -spread) - px(0.0, 0.0),
                edy: py(-loop_reach, -spread) - py(0.0, 0.0),
            };
            let len = js_or_one(p.edx.hypot(p.edy));
            p.edx /= len;
            p.edy /= len;
            let furthest = LOOP + (loop_count[src] - 1) as f64 * 8.0;
            let below = loop_text[src] + label.height / 2.0;
            loop_text[src] += label.height;
            cedges[i].label_x = if side {
                a.x + out + furthest + 4.0 + cedges[i].label_w / 2.0
            } else {
                a.x
            };
            cedges[i].label_y = if side {
                a.y - loop_height[src] / 2.0 + below
            } else {
                a.y + out + furthest * 0.75 + 4.0 + below
            };
            p
        } else {
            let mut route = cedges[i].route.clone();
            let leaves = reach.of(src as i64, i64::from(end_side(&route, 0, &cnodes[src])));
            let arrives = reach.of(
                dst as i64,
                i64::from(end_side(&route, route.len() - 3, &cnodes[dst])),
            );
            let mut k = 0;
            while k < route.len() {
                route[k] += pad;
                route[k + 1] += pad;
                k += 3;
            }
            attach(&mut route, 0, &cnodes[src], &views[src], pad);
            let arrive_at = route.len() - 3;
            attach(&mut route, arrive_at, &cnodes[dst], &views[dst], pad);
            if let Some(&composite) = anchors.get(&i)
                && route.len() >= 6
            {
                let g = &cnodes[composite];
                let inward = views[src].kind == K::Hidden;
                let at = if inward { 0 } else { route.len() - 3 };
                let g_dir = g.dir.unwrap_or(Dir::TB);
                let near = (matches!(g_dir, Dir::TB | Dir::LR)) == inward;
                let point = if vertical(g_dir) {
                    [
                        route[at],
                        g.y + pad
                            + if near {
                                views[composite].title_h - g.h / 2.0
                            } else {
                                g.h / 2.0
                            },
                        0.0,
                    ]
                } else {
                    [
                        g.x + pad + if near { -g.w / 2.0 } else { g.w / 2.0 },
                        route[at + 1],
                        1.0,
                    ]
                };
                route = if inward {
                    [point.to_vec(), route].concat()
                } else {
                    [route, point.to_vec()].concat()
                };
            }
            route_path(
                &route,
                curve.as_deref(),
                0.0,
                marker_trim(end_type),
                Some(leaves),
                Some(arrives),
            )
        };
        let over = over_fills(
            &path.d,
            &[path.ex, path.ey],
            &fills,
            "",
            path.tokens.as_deref(),
        );
        if over.as_deref() == Some("") {
            casings.push_str(&edge_casing(&path, "none", end_type, "", false));
        }
        let over_color = over.clone().unwrap_or_default();
        let kind = if note {
            format!(
                "pelers-note-edge\" stroke=\"{}\" stroke-dasharray=\"3 4",
                if over_color.is_empty() {
                    "var(--_b)".to_owned()
                } else {
                    over_color.clone()
                }
            )
        } else if !over_color.is_empty() {
            format!("pelers-transition\" stroke=\"{over_color}")
        } else {
            "pelers-transition".to_owned()
        };
        let ink = if over_color.is_empty() {
            "var(--_l)".to_owned()
        } else {
            over_color
        };
        edges_out.push_str(&format!(
            "<g class=\"pelers-edge {kind}\" data-id=\"{}\"><path d=\"{}\"/>{}</g>",
            esc_text(&edge.id),
            path.d,
            marker(end_type, path.ex, path.ey, path.edx, path.edy, &ink, "none")
        ));
        if label.width > 0.0 {
            let x = cedges[i].label_x + pad;
            let y = cedges[i].label_y + pad;
            labels_out.push_str(&edge_label_svg(&edge.id, &label, x, y, "", icons));
        }
    }

    let titled = with_title(
        db.title.as_deref(),
        size,
        pad,
        width,
        height,
        &format!(
            "{}{}{}{}{}<g class=\"pelers-nodes\">{nodes_out}{notes_out}</g>",
            if clusters.is_empty() {
                String::new()
            } else {
                format!("<g class=\"pelers-clusters\">{clusters}</g>")
            },
            if dividers.is_empty() {
                String::new()
            } else {
                format!(
                    "<g class=\"pelers-dividers\" stroke-dasharray=\"4 4\"{RULE}>{dividers}</g>"
                )
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
                    num(small)
                )
            },
        ),
    );
    let svg = svg_document(
        "state",
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
        links: std::mem::take(&mut links),
    }
}
