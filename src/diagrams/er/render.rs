use std::collections::HashMap;

use super::graph::{GraphEdge, build_er_graph};
use super::markers::{MARKER_LENGTH, er_marker};
use super::model::{ErAttribute, ErDb};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{runs_across, turn_to_fit};
use crate::diagrams::common::generics::parse_generic_types;
use crate::layout::compound::{compound_layout, shift_layout};
use crate::layout::{CEdge, CNode, Dir, LayeredOptions, cnode, direction};
use crate::preprocess::Config;
use crate::svg::builder::{
    esc, esc_text, label_svg, label_svg_with, num, shared_style, span_style, text_style,
};
use crate::svg::cluster::{Crossings, cluster_title_x, mark_crossings};
use crate::svg::edges::{
    Fill, FillUnder, casing, casings_svg, edge_ink, edge_label_svg, fill_under, hollow_on,
    route_path,
};
use crate::svg::ink::{ON_DARK, ON_LIGHT, ink, ink_fill, paints};
use crate::svg::options::LinkInfo;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ROUNDED, ResolvedStyle, class_names, resolve_style, rounded, sets_fill};
use crate::svg::title::title_label;
use crate::text::entities::decode_entities;
use crate::text::icons::icon_markup;
use crate::text::label::{Label, has_math, label_line, layout_label, line_middle};
use crate::text::measurer::{BOLD_STYLE, ITALIC, MONO};
use crate::util::chars::is_js_space;
use crate::util::jsnum::{js_hypot, js_max, js_min, js_number_to_string, js_round};
use crate::util::url::{internal_link, rel_attr};
use crate::util::value::Value;

const NODE_SEP: f64 = 48.0;
const EDGE_SEP: f64 = 20.0;
const RANK_SEP: f64 = 64.0;
const GROUP_PAD: f64 = 20.0;
const LOOP: f64 = 28.0;
const LOOP_BAND: f64 = 38.0;
const MIN_WIDTH: f64 = 96.0;
const BOX_PAD_X: f64 = 18.0;
const BOX_PAD_Y: f64 = 12.0;
const HEAD_PAD_Y: f64 = 9.0;
const CELL_PAD_X: f64 = 12.0;
const CELL_GAP: f64 = 16.0;
const ROW_PAD: f64 = 9.0;
const COMMENT_WRAP: f64 = 260.0;
const NO_WRAP: f64 = 1e6;
const PORT_GAP: f64 = 18.0;
const PORT_MARGIN: f64 = 14.0;
const BOX_ATTRS: &str = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";

fn number_option(config: &Config, key: &str, fallback: f64) -> f64 {
    match config
        .get("er")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::Number(value)) if *value > 0.0 => js_min(*value, 2000.0),
        _ => fallback,
    }
}

type Cells = [Label; 4];

struct Row {
    cells: Cells,
    h: f64,
}

struct NodeView {
    style: ResolvedStyle,
    label: Label,
    w: f64,
    h: f64,
    head_h: f64,
    rows: Vec<Row>,
    cols: Vec<f64>,
    row_size: f64,
    key_size: f64,
    mono: u32,
    below: bool,
}

struct Loop {
    label: Label,
    band: f64,
    at: f64,
}

fn has_font_family(text: &str) -> bool {
    text.contains("\"font-family:") || text.contains(";font-family:")
}

fn strip_default(raw: &str) -> &str {
    match raw.strip_prefix("default") {
        Some(rest) => rest.strip_prefix(is_js_space).unwrap_or(rest),
        None => raw,
    }
}

fn text_left(label: &Label, x: f64, y: f64, attrs: &str, icons: bool, mono: u32) -> String {
    let lines = &label.lines;
    if lines.is_empty() {
        return String::new();
    }
    if let [line] = lines.as_slice()
        && let [only] = line.as_slice()
        && only.icon.is_none()
        && only.math.is_none()
        && only.style == mono
        && !(mono != 0 && only.text.contains("  "))
    {
        return format!(
            "<text{} x=\"{}\" y=\"{}\">{}</text>",
            text_style(attrs, mono),
            num(x),
            num(y),
            esc(&only.text)
        );
    }
    let shared = shared_style(label);
    let shift = label.size * 0.35;
    let top = y - shift - label.line_height / 2.0;
    let mut body = String::new();
    let mut extra = String::new();
    for (i, line) in lines.iter().enumerate() {
        let middle = top + line_middle(label, i);
        if has_math(line) {
            extra.push_str(&label_svg_with(
                &label_line(label, i),
                x + label.widths[i] / 2.0,
                middle,
                attrs,
                icons,
            ));
            continue;
        }
        let y = middle + shift;
        let has_icon = line.iter().any(|span| span.icon.is_some());
        let mut sx = x;
        let mut placed = false;
        for span in line {
            if let Some(icon) = &span.icon {
                let side = label.size * 1.1;
                extra.push_str(&format!(
                    "<svg class=\"pelers-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{}</svg>",
                    esc_text(icon),
                    num(sx),
                    num(y - label.size * 0.35 - side / 2.0),
                    num(side),
                    num(side),
                    icon_markup(icon)
                ));
            } else if !span.text.is_empty() {
                let pos = if has_icon || !placed {
                    format!(" x=\"{}\" y=\"{}\"", num(sx), num(y))
                } else {
                    String::new()
                };
                body.push_str(&format!(
                    "<tspan{pos}{}>{}</tspan>",
                    span_style(span.style & !shared),
                    esc(&span.text)
                ));
                placed = true;
            }
            sx += span.width;
        }
    }
    format!(
        "<text{} xml:space=\"preserve\">{body}</text>{extra}",
        text_style(attrs, shared)
    )
}

fn cell_text(raw: &str) -> String {
    let converted = parse_generic_types(raw);
    if converted == raw {
        raw.to_owned()
    } else {
        converted.replace('<', "&lt;").replace('>', "&gt;")
    }
}

pub fn render_er(db: &ErDb, config: &Config, options: &RenderOptions) -> Drawn {
    if !runs_across(Some(&db.direction)) {
        return draw(db, config, options, false);
    }
    turn_to_fit(options, |down| draw(db, config, options, down))
}

fn draw(db: &ErDb, config: &Config, options: &RenderOptions, turned: bool) -> Drawn {
    let graph = build_er_graph(db);
    let monospaced = options
        .monospace
        .as_ref()
        .is_some_and(|list| list.iter().any(|name| name == "attributes"));
    let size = options.font_size.unwrap_or(16.0);
    let small_size = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();

    let index: HashMap<&str, usize> = graph
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    let mut views: Vec<NodeView> = Vec::new();
    let mut cnodes: Vec<CNode> = Vec::new();
    let mut cedges: Vec<CEdge> = Vec::new();
    let mut edge_labels: Vec<Label> = Vec::new();
    let mut drawn: Vec<&GraphEdge> = Vec::new();
    let mut loop_arena: Vec<Loop> = Vec::new();
    let mut loop_of: Vec<Option<usize>> = Vec::new();
    let mut loops: Vec<(usize, Vec<usize>)> = Vec::new();
    for edge in &graph.edges {
        let (Some(&src), Some(&dst)) =
            (index.get(edge.start.as_str()), index.get(edge.end.as_str()))
        else {
            continue;
        };
        let label = layout_label(Some(&edge.label), true, small_size, 200.0, 0);
        let mut loop_index = None;
        if src == dst {
            loop_arena.push(Loop {
                label: label.clone(),
                band: 0.0,
                at: 0.0,
            });
            let id = loop_arena.len() - 1;
            loop_index = Some(id);
            match loops.iter_mut().find(|(node, _)| *node == src) {
                Some((_, list)) => list.push(id),
                None => loops.push((src, vec![id])),
            }
        }
        loop_of.push(loop_index);
        drawn.push(edge);
        cedges.push(CEdge {
            src,
            dst,
            minlen: 1.0,
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
        edge_labels.push(label);
    }

    let horizontal = |i: usize| -> bool {
        let mut at = i;
        for _ in 0..graph.nodes.len() {
            let Some(up) = graph.nodes[at]
                .parent_id
                .as_deref()
                .and_then(|parent| index.get(parent))
                .copied()
            else {
                break;
            };
            if let Some(dir) = graph.nodes[up].dir.as_deref().filter(|dir| !dir.is_empty()) {
                return dir == "LR" || dir == "RL";
            }
            at = up;
        }
        db.direction == "LR" || db.direction == "RL"
    };

    for (i, node) in graph.nodes.iter().enumerate() {
        let decls: Vec<String> = if node.css_styles.is_empty() {
            node.css_compiled_styles.clone()
        } else {
            node.css_compiled_styles
                .iter()
                .chain(node.css_styles.iter())
                .cloned()
                .collect()
        };
        let style = resolve_style(&decls);
        let base =
            (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let mono = if monospaced && !has_font_family(&style.text) {
            MONO
        } else {
            0
        };
        let name_size = style.font_size.unwrap_or(size);
        let row_size = style.font_size.unwrap_or(small_size);
        let key_size = style.font_size.unwrap_or_else(|| js_round(size * 0.75));
        let label = if node.is_group {
            layout_label(Some(&node.label), true, name_size, 4000.0, base)
        } else {
            let text = if node.alias.is_empty() {
                &node.label
            } else {
                &node.alias
            };
            layout_label(Some(text), true, name_size, 240.0, base)
        };
        let own: Vec<usize> = loops
            .iter()
            .find(|(node, _)| *node == i)
            .map_or_else(Vec::new, |(_, list)| list.clone());
        let below = !own.is_empty() && !node.is_group && horizontal(i);
        let mut view = NodeView {
            style,
            label,
            w: 0.0,
            h: 0.0,
            head_h: 0.0,
            rows: Vec::new(),
            cols: Vec::new(),
            row_size,
            key_size,
            mono,
            below,
        };
        let mut reach = 0.0f64;
        let mut bands = 0.0f64;
        for &l in &own {
            let lp = &mut loop_arena[l];
            lp.band = js_max(
                LOOP_BAND,
                if below {
                    lp.label.width + 12.0
                } else {
                    lp.label.height + 6.0
                },
            );
            bands += lp.band;
            reach = js_max(
                reach,
                if below {
                    lp.label.height
                } else {
                    lp.label.width
                },
            );
        }
        let min_w = js_max(MIN_WIDTH, if below { bands + 8.0 } else { 0.0 });
        let c = if node.is_group {
            let mut c = cnode(0.0, 0.0, -1);
            c.is_group = true;
            c.dir = node
                .dir
                .as_deref()
                .filter(|dir| !dir.is_empty())
                .map(|dir| direction(Some(dir)));
            c.pad_x = GROUP_PAD;
            c.pad_top = if view.label.height > 0.0 {
                view.label.height + 16.0
            } else {
                GROUP_PAD
            };
            c.pad_bottom = GROUP_PAD;
            c.min_w = js_max(view.label.width + 2.0 * GROUP_PAD - 12.0, MIN_WIDTH);
            c
        } else {
            let name_h = js_max(view.label.height, js_round(name_size * 1.5));
            if node.attributes.is_empty() {
                view.w = js_max(view.label.width + 2.0 * BOX_PAD_X, min_w);
                view.h = js_max(
                    name_h + 2.0 * BOX_PAD_Y,
                    if below { 0.0 } else { bands + 8.0 },
                );
            } else {
                let row_line = js_round(row_size * 1.5);
                let mut widths = [0.0f64; 4];
                for attribute in &node.attributes {
                    let cells = attribute_cells(attribute, row_size, key_size, base | mono, base);
                    let mut h = row_line;
                    for k in 0..4 {
                        if cells[k].width > widths[k] {
                            widths[k] = cells[k].width;
                        }
                        if cells[k].height > h {
                            h = cells[k].height;
                        }
                    }
                    view.rows.push(Row {
                        cells,
                        h: h + ROW_PAD,
                    });
                    view.h += h + ROW_PAD;
                }
                let mut columns = 0usize;
                let mut inner = 0.0f64;
                for &w in &widths {
                    if w == 0.0 {
                        continue;
                    }
                    inner += w + if columns > 0 { CELL_GAP } else { 0.0 };
                    columns += 1;
                }
                view.w = js_max(
                    js_max(inner + 2.0 * CELL_PAD_X, view.label.width + 2.0 * BOX_PAD_X),
                    min_w,
                );
                let spare = if columns > 0 {
                    (view.w - inner - 2.0 * CELL_PAD_X) / columns as f64
                } else {
                    0.0
                };
                let mut x = CELL_PAD_X;
                for &w in &widths {
                    view.cols.push(x);
                    if w > 0.0 {
                        x += w + spare + CELL_GAP;
                    }
                }
                view.head_h = name_h + 2.0 * HEAD_PAD_Y;
                view.h += view.head_h;
            }
            let room = if own.is_empty() {
                0.0
            } else {
                2.0 * (MARKER_LENGTH + LOOP * 0.75 + 10.0 + reach)
            };
            if below {
                cnode(view.w, view.h + room, -1)
            } else {
                cnode(view.w + room, view.h, -1)
            }
        };
        let mut c = c;
        c.seq = i as f64;
        views.push(view);
        cnodes.push(c);
    }
    for (i, node) in graph.nodes.iter().enumerate() {
        cnodes[i].parent = node
            .parent_id
            .as_deref()
            .map_or(-1, |parent| index.get(parent).map_or(-1, |&at| at as i32));
    }

    let root_dir = if turned {
        Dir::TB
    } else {
        direction(Some(&db.direction))
    };
    let layout = compound_layout(
        &mut cnodes,
        &mut cedges,
        root_dir,
        &LayeredOptions {
            node_sep: number_option(config, "nodeSpacing", NODE_SEP),
            edge_sep: EDGE_SEP,
            rank_sep: number_option(config, "rankSpacing", RANK_SEP),
            port_sep: 24.0,
            ..LayeredOptions::default()
        },
    );

    spread_ends(&mut cedges, &cnodes, &views);

    let mut layout_width = layout.width;
    for (i, own) in &loops {
        let c = &cnodes[*i];
        if !c.is_group {
            continue;
        }
        let mut reach = 0.0f64;
        for &l in own {
            reach = js_max(reach, loop_arena[l].label.width);
        }
        layout_width = js_max(
            layout_width,
            c.x + c.w / 2.0 + MARKER_LENGTH + LOOP * 0.75 + 10.0 + reach,
        );
    }

    let title = title_label(db.title.as_deref(), size, 4000.0);
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let inner = js_max(layout_width, title.width);
    let ox = pad + (inner - layout_width) / 2.0;
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
    let mut fills: Vec<Fill> = Vec::new();
    let mut nodes_out = String::new();
    for (i, view) in views.iter_mut().enumerate() {
        let c = &cnodes[i];
        let node = &graph.nodes[i];
        let classes = class_names(strip_default(&node.css_classes));
        let shape = view.style.shape.clone();
        let text = view.style.text.clone();
        if c.is_group {
            view.w = c.w;
            view.h = c.h;
            let x = c.x - c.w / 2.0;
            let y = c.y - c.h / 2.0;
            let title_x = cluster_title_x(c, view.label.width, &crossings, GROUP_PAD);
            if paints(&shape) {
                fills.push(Fill {
                    box_: [x, y, x + c.w, y + c.h],
                    shape: shape.clone(),
                });
            }
            clusters.push_str(&format!(
                "<g class=\"pelers-cluster{classes}\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\"{} stroke=\"var(--_b)\"{}/>{}</g>",
                esc_text(&node.id),
                num(x),
                num(y),
                num(c.w),
                num(c.h),
                if sets_fill(&shape) { "" } else { " fill-opacity=\"0.5\"" },
                rounded(&shape),
                label_svg_with(
                    &view.label,
                    title_x,
                    y + 8.0 + view.label.height / 2.0,
                    &format!(
                        " class=\"pelers-cluster-label\"{}{text}",
                        ink_fill(&shape, &text, Some("var(--_m)"))
                    ),
                    icons
                )
            ));
            continue;
        }

        let w = view.w;
        let h = view.h;
        let x0 = -w / 2.0;
        let y0 = -h / 2.0;
        let color = ink_fill(&shape, &text, None);
        let muted = if color.is_empty() {
            " fill=\"var(--_m)\"".to_owned()
        } else {
            color.clone()
        };
        let shape_attrs = format!(
            "x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"",
            num(x0),
            num(y0),
            num(w),
            num(h)
        );
        let mut name = label_svg_with(
            &view.label,
            0.0,
            if view.rows.is_empty() {
                0.0
            } else {
                y0 + view.head_h / 2.0
            },
            &format!(" class=\"pelers-label\"{color}{text}"),
            icons,
        );
        let display = decode_entities(if node.alias.is_empty() {
            &node.label
        } else {
            &node.alias
        });
        if format!("{classes} ").contains(" internal-link ")
            && !name.is_empty()
            && options.links != Some(false)
        {
            let safe = internal_link(&display, options);
            let target = esc(&display);
            name = format!(
                "<a class=\"internal-link\"{} data-href=\"{target}\">{name}</a>",
                if safe {
                    format!(" href=\"{target}\"{}", rel_attr(options))
                } else {
                    String::new()
                }
            );
            links.push(LinkInfo {
                id: node.label.clone(),
                href: display.clone(),
                internal: true,
            });
        }

        let body = if view.rows.is_empty() {
            format!(
                "<rect {shape_attrs}{}/>",
                rounded(&format!("{BOX_ATTRS}{shape}"))
            ) + &name
        } else {
            let row_line = js_round(view.row_size * 1.5);
            let line_color = match view.style.stroke.as_deref().filter(|s| !s.is_empty()) {
                Some(stroke) => esc(stroke).into_owned(),
                None => "var(--_b)".to_owned(),
            };
            let mut stripes = String::new();
            let mut cells = String::new();
            let mut y = y0 + view.head_h;
            for (r, row) in view.rows.iter().enumerate() {
                if r % 2 == 0 {
                    let span = format!("x=\"{}\" width=\"{}\"", num(x0), num(w));
                    if r == view.rows.len() - 1 {
                        stripes.push_str(&format!(
                            "<rect {span} y=\"{}\" height=\"{}\"{ROUNDED}/><rect {span} y=\"{}\" height=\"{}\"/>",
                            num(y),
                            num(row.h),
                            num(y),
                            num(row.h / 2.0)
                        ));
                    } else {
                        stripes.push_str(&format!(
                            "<rect {span} y=\"{}\" height=\"{}\"/>",
                            num(y),
                            num(row.h)
                        ));
                    }
                }
                let baseline = y + ROW_PAD / 2.0 + row_line / 2.0 + view.row_size * 0.35;
                let [kind, attribute, keys, comment] = &row.cells;
                cells.push_str(&text_left(
                    kind,
                    x0 + view.cols[0],
                    baseline,
                    &format!(" class=\"pelers-er-type\"{muted}{text}"),
                    icons,
                    view.mono,
                ));
                cells.push_str(&text_left(
                    attribute,
                    x0 + view.cols[1],
                    baseline,
                    &format!(" class=\"pelers-er-name\"{color}{text}"),
                    icons,
                    view.mono,
                ));
                cells.push_str(&text_left(
                    keys,
                    x0 + view.cols[2],
                    baseline,
                    &format!(
                        " class=\"pelers-er-keys\" font-size=\"{}\"{muted}{text}",
                        js_number_to_string(view.key_size)
                    ),
                    icons,
                    view.mono,
                ));
                cells.push_str(&text_left(
                    comment,
                    x0 + view.cols[3],
                    baseline,
                    &format!(" class=\"pelers-er-comment\"{muted}{text}"),
                    icons,
                    0,
                ));
                y += row.h;
            }
            let divider_y = num(y0 + view.head_h);
            let shade = match ink(&shape, "") {
                Some(ON_LIGHT) => ON_DARK,
                Some(ON_DARK) => ON_LIGHT,
                _ => "var(--_bg)",
            };
            format!(
                "<rect {shape_attrs}{}/><g class=\"pelers-er-stripes\" fill=\"{shade}\"{}>{stripes}</g><path class=\"pelers-er-divider\" d=\"M{},{divider_y}H{}\" stroke=\"{line_color}\"/><rect {shape_attrs} fill=\"none\" stroke=\"var(--_b)\"{}/>{name}<g class=\"pelers-er-attributes\" font-size=\"{}\">{cells}</g>",
                rounded(&format!("{BOX_ATTRS}{shape}")),
                if shape.contains("fill:") {
                    " opacity=\"0.6\""
                } else {
                    ""
                },
                num(x0),
                num(x0 + w),
                rounded(&view.style.line),
                js_number_to_string(view.row_size)
            )
        };
        nodes_out.push_str(&format!(
            "<g class=\"pelers-node pelers-entity{classes}\" data-id=\"{}\" transform=\"translate({},{})\">{body}</g>",
            esc_text(&node.label),
            num(c.x),
            num(c.y)
        ));
    }

    for (i, own) in &loops {
        let view = &views[*i];
        let total = own.iter().fold(0.0f64, |sum, &l| sum + loop_arena[l].band);
        let scale = js_min(
            1.0,
            js_max((if view.below { view.w } else { view.h }) - 8.0, 0.0) / total,
        );
        let mut at = (-total * scale) / 2.0;
        for &l in own {
            let lp = &mut loop_arena[l];
            lp.band *= scale;
            lp.at = at + lp.band / 2.0;
            at += lp.band;
        }
    }

    let mut edges_out = String::new();
    let mut casings = String::new();
    let mut labels_out = String::new();
    for i in 0..cedges.len() {
        let edge = drawn[i];
        let label = &edge_labels[i];
        let src = cedges[i].src;
        let dst = cedges[i].dst;
        let d: String;
        let mut start: String;
        let mut end: String;
        if let Some(l) = loop_of[i] {
            let lp = &loop_arena[l];
            let c = &cnodes[src];
            let view = &views[src];
            let spread = js_max(js_min(lp.band / 2.0 - 6.0, 13.0), 1.0);
            let far = MARKER_LENGTH + LOOP * 0.75 + 6.0;
            if view.below {
                let x = c.x + lp.at;
                let y = c.y + view.h / 2.0;
                let turn = num(y + MARKER_LENGTH);
                let out = num(y + MARKER_LENGTH + LOOP);
                d = format!(
                    "M{},{}V{turn}C{},{out} {},{out} {},{turn}V{}",
                    num(x - spread),
                    num(y),
                    num(x - spread),
                    num(x + spread),
                    num(x + spread),
                    num(y)
                );
                start = er_marker(edge.arrow_type_start, x - spread, y, 0.0, -1.0);
                end = er_marker(edge.arrow_type_end, x + spread, y, 0.0, -1.0);
                cedges[i].label_x = x;
                cedges[i].label_y = y + far + label.height / 2.0;
            } else {
                let x = c.x + view.w / 2.0;
                let y = c.y + lp.at;
                let turn = num(x + MARKER_LENGTH);
                let out = num(x + MARKER_LENGTH + LOOP);
                d = format!(
                    "M{},{}H{turn}C{out},{} {out},{} {turn},{}H{}",
                    num(x),
                    num(y - spread),
                    num(y - spread),
                    num(y + spread),
                    num(y + spread),
                    num(x)
                );
                start = er_marker(edge.arrow_type_start, x, y - spread, -1.0, 0.0);
                end = er_marker(edge.arrow_type_end, x, y + spread, -1.0, 0.0);
                cedges[i].label_x = x + far + label.width / 2.0;
                cedges[i].label_y = y;
            }
        } else {
            let mut route = straighten(&cedges[i].route);
            if route.len() < 6 {
                continue;
            }
            inset(&mut route, 0, &views[src], &cnodes[src]);
            let last = route.len() - 3;
            inset(&mut route, last, &views[dst], &cnodes[dst]);
            let dx = (route[last] - route[0]).abs();
            let dy = (route[last + 1] - route[1]).abs();
            let direct = route.len() == 6
                && (dx < 0.01 || dy < 0.01 || js_hypot(dx, dy) <= 3.0 * MARKER_LENGTH);
            let stub = if direct { 0.0 } else { MARKER_LENGTH };
            let path = route_path(&route, None, stub, stub, None, None);
            d = if stub > 0.0 {
                format!(
                    "M{},{}L{}L{},{}",
                    num(path.sx),
                    num(path.sy),
                    &path.d[1..],
                    num(path.ex),
                    num(path.ey)
                )
            } else {
                path.d.clone()
            };
            start = er_marker(edge.arrow_type_start, path.sx, path.sy, path.sdx, path.sdy);
            end = er_marker(edge.arrow_type_end, path.ex, path.ey, path.edx, path.edy);
        }
        let under = fill_under(&d, &[], &fills, None);
        let over: Option<String> = match &under {
            FillUnder::Off => None,
            FillUnder::Crosses => Some(String::new()),
            FillUnder::Holder(k) => Some(edge_ink(&fills[*k], "")),
        };
        if over.as_deref() == Some("") {
            casings.push_str(&casing(&d, &format!("{start}{end}"), "", false));
        }
        if let (FillUnder::Holder(k), Some(ink)) = (&under, &over)
            && !ink.is_empty()
        {
            start = hollow_on(&start, &fills[*k]);
            end = hollow_on(&end, &fills[*k]);
        }
        let dashed = edge.dashed;
        let over_attr = over
            .as_deref()
            .filter(|ink| !ink.is_empty())
            .map_or_else(String::new, |ink| format!(" stroke=\"{ink}\""));
        edges_out.push_str(&format!(
            "<g class=\"pelers-edge pelers-er-{}\" data-id=\"{}\"{over_attr}><path d=\"{d}\"{}/>{start}{end}</g>",
            if dashed { "non-identifying" } else { "identifying" },
            esc_text(&edge.id),
            if dashed { " stroke-dasharray=\"4 3\"" } else { "" }
        ));
        if label.width > 0.0 {
            labels_out.push_str(&edge_label_svg(
                &edge.id,
                label,
                cedges[i].label_x,
                cedges[i].label_y,
                "",
                icons,
            ));
        }
    }

    let mut content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    if !clusters.is_empty() {
        content.push_str(&format!("<g class=\"pelers-clusters\">{clusters}</g>"));
    }
    content.push_str(&casings_svg(&casings));
    if !edges_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{edges_out}</g>"
        ));
    }
    if !labels_out.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-edge-labels\" font-size=\"{}\">{labels_out}</g>",
            js_number_to_string(small_size)
        ));
    }
    content.push_str(&format!("<g class=\"pelers-nodes\">{nodes_out}</g>"));

    let svg = svg_document(
        "er",
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

fn attribute_cells(
    attribute: &ErAttribute,
    row_size: f64,
    key_size: f64,
    base_mono: u32,
    base: u32,
) -> Cells {
    [
        layout_label(
            Some(&cell_text(&attribute.attr_type)),
            true,
            row_size,
            NO_WRAP,
            base_mono,
        ),
        layout_label(
            Some(&cell_text(&attribute.name)),
            true,
            row_size,
            NO_WRAP,
            base_mono,
        ),
        layout_label(
            Some(&attribute.keys.join(", ")),
            false,
            key_size,
            NO_WRAP,
            base_mono,
        ),
        layout_label(
            Some(&cell_text(&attribute.comment)),
            true,
            row_size,
            COMMENT_WRAP,
            base,
        ),
    ]
}

fn sign(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        x
    }
}

fn straighten(route: &[f64]) -> Vec<f64> {
    let mut out: Vec<f64> = route.iter().take(3).copied().collect();
    let mut k = 3;
    while k < route.len() {
        let prev = out.len() - 3;
        let flat = |axis: usize| {
            (out[prev + axis] - route[k + axis]).abs() < 0.01
                && (route[k + axis] - route[k + 3 + axis]).abs() < 0.01
        };
        if k + 3 < route.len() && (flat(0) || flat(1)) {
            k += 3;
            continue;
        }
        out.extend_from_slice(&route[k..k + 3]);
        k += 3;
    }
    out
}

fn spread_ends(cedges: &mut [CEdge], cnodes: &[CNode], views: &[NodeView]) {
    let mut sides: HashMap<usize, Vec<(usize, usize, f64)>> = HashMap::new();
    let mut add = |node: usize, edge: usize, route: &[f64], at: usize, next: usize| {
        let c = &cnodes[node];
        let dx = route[at] - c.x;
        let dy = route[at + 1] - c.y;
        let side = if (dy.abs() - c.h / 2.0).abs() < 0.5 {
            if dy < 0.0 { 0 } else { 2 }
        } else if (dx.abs() - c.w / 2.0).abs() < 0.5 {
            if dx < 0.0 { 3 } else { 1 }
        } else {
            return;
        };
        let toward = route[next + usize::from(side & 1 == 1)];
        sides
            .entry(node * 4 + side)
            .or_default()
            .push((edge, at, toward));
    };
    for (ei, e) in cedges.iter().enumerate() {
        let last = e.route.len() as i64 - 3;
        if e.src == e.dst || last < 3 {
            continue;
        }
        let last = last as usize;
        add(e.src, ei, &e.route, 0, 3);
        add(e.dst, ei, &e.route, last, last - 3);
    }
    for (key, mut ends) in sides {
        if ends.len() < 2 {
            continue;
        }
        let node = key >> 2;
        let vertical = (key & 1) == 1;
        let length = if cnodes[node].is_group {
            if vertical {
                cnodes[node].h
            } else {
                cnodes[node].w
            }
        } else if vertical {
            views[node].h
        } else {
            views[node].w
        };
        let step = js_max(
            js_min(
                PORT_GAP,
                (length - 2.0 * PORT_MARGIN) / (ends.len() - 1) as f64,
            ),
            0.0,
        );
        let center = if vertical {
            cnodes[node].y
        } else {
            cnodes[node].x
        };
        ends.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        for (i, &(edge, at, _)) in ends.iter().enumerate() {
            cedges[edge].route[at + usize::from(vertical)] =
                center + (i as f64 - (ends.len() - 1) as f64 / 2.0) * step;
        }
    }
}

fn inset(route: &mut [f64], at: usize, view: &NodeView, c: &CNode) {
    if c.is_group {
        return;
    }
    let dx = route[at] - c.x;
    let dy = route[at + 1] - c.y;
    if c.w != view.w && (dx.abs() - c.w / 2.0).abs() < 0.5 {
        route[at] -= (sign(dx) * (c.w - view.w)) / 2.0;
    }
    if c.h != view.h && (dy.abs() - c.h / 2.0).abs() < 0.5 {
        route[at + 1] -= (sign(dy) * (c.h - view.h)) / 2.0;
    }
}
