use std::collections::{HashMap, HashSet};

use super::layout::{HEAD_LEFT, HEAD_TOP, Layout, layout_c4};
use super::model::{C4Db, Element};
use super::route::{Box, HEAD, LABEL_PAD, LANE, Lanes, Route, crosses, cyl_ry, queue_rx, route};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::title_room;
use crate::diagrams::common::literal_label::left_label_svg;
use crate::errors::{PelersError, PelersErrorCode};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, num};
use crate::svg::edges::{
    ARROW, EdgePath, Fill, casings_svg, edge_casing, marker, marker_trim, over_fills,
};
use crate::svg::ink::{ink_fill, paints};
use crate::svg::options::LinkInfo;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::shapes::draw_shape;
use crate::svg::theme::{ResolvedStyle, resolve_style, rounded};
use crate::text::label::layout_label;
use crate::text::measurer::{BOLD_STYLE, ITALIC};
use crate::util::jsnum::{js_max, js_min, js_round};
use crate::util::url::{linked, sanitize_url};

const PAIR_SHIFT: f64 = 7.0;
const NARROW_GAP: f64 = 36.0;
const WIDEN: f64 = 24.0;
const MAX_OFFSET: f64 = 2000.0;
const EDGE: f64 = 8.0;
const SLIDE: [f64; 7] = [0.5, 0.4, 0.6, 0.3, 0.7, 0.2, 0.8];
const CARE_BUDGET: f64 = 2000000.0;

fn user_style(fill: Option<&str>, stroke: Option<&str>, color: Option<&str>) -> ResolvedStyle {
    let mut declarations: Vec<String> = Vec::new();
    if let Some(fill) = fill.filter(|s| !s.is_empty()) {
        declarations.push(format!("fill:{fill}"));
    }
    if let Some(stroke) = stroke.filter(|s| !s.is_empty()) {
        declarations.push(format!("stroke:{stroke}"));
    }
    if let Some(color) = color.filter(|s| !s.is_empty()) {
        declarations.push(format!("color:{color}"));
    }
    resolve_style(&declarations)
}

fn restore_url(url: &str) -> String {
    if url.contains("\u{b6}\u{df}") {
        url.replace("\u{fb02}\u{b0}\u{b0}", "#")
            .replace("\u{fb02}\u{b0}", "#")
            .replace("\u{b6}\u{df}", ";")
    } else {
        url.to_owned()
    }
}

fn link(
    body: &str,
    url: Option<&str>,
    id: &str,
    links: &mut Vec<LinkInfo>,
    options: &RenderOptions,
) -> String {
    let Some(url) = url.filter(|u| !u.is_empty()) else {
        return body.to_owned();
    };
    linked(
        body,
        &sanitize_url(&restore_url(url)),
        id,
        links,
        options,
        "",
    )
}

fn draw_narrow(db: &C4Db, options: &RenderOptions) -> Result<Drawn, PelersError> {
    let (mut best, mut squeezed) = draw(db, options, true, NARROW_GAP)?;
    let mut gap = NARROW_GAP + WIDEN;
    while squeezed > 0 && gap <= NARROW_GAP + 3.0 * WIDEN {
        let (wider, left) = draw(db, options, true, gap)?;
        if left < squeezed {
            best = wider;
            squeezed = left;
        }
        gap += WIDEN;
    }
    Ok(best)
}

pub fn render_c4(
    db: &C4Db,
    _config: &Config,
    options: &RenderOptions,
) -> Result<Drawn, PelersError> {
    let (loose, _) = draw(db, options, false, NARROW_GAP)?;
    let Some(available) = options.max_width.filter(|a| *a > 0.0) else {
        return Ok(loose);
    };
    if loose.width <= available {
        return Ok(loose);
    }
    let narrow = draw_narrow(db, options)?;
    Ok(if narrow.width < loose.width {
        narrow
    } else {
        loose
    })
}

fn offset(value: Option<f64>) -> f64 {
    match value {
        Some(v) if v != 0.0 && !v.is_nan() => js_max(-MAX_OFFSET, js_min(MAX_OFFSET, v)),
        _ => 0.0,
    }
}

fn covered(taken: &[f64], x0: f64, y0: f64, x1: f64, y1: f64) -> f64 {
    let mut area = 0.0;
    for t in taken.as_chunks::<4>().0 {
        let w = js_min(x1, t[2]) - js_max(x0, t[0]);
        if w <= 0.0 {
            continue;
        }
        let h = js_min(y1, t[3]) - js_max(y0, t[1]);
        if h > 0.0 {
            area += w * h;
        }
    }
    area
}

fn to_edge_path(r: &Route) -> EdgePath {
    EdgePath {
        d: r.d.clone(),
        tokens: None,
        sx: r.sx,
        sy: r.sy,
        sdx: r.sdx,
        sdy: r.sdy,
        ex: r.ex,
        ey: r.ey,
        edx: r.edx,
        edy: r.edy,
    }
}

fn box_of(layout: &Layout, found: Option<(bool, usize)>) -> Option<Box> {
    let (is_shape, index) = found?;
    if is_shape {
        layout.shape_boxes[index].clone()
    } else {
        layout.boundary_boxes[index].clone()
    }
}

fn draw(
    db: &C4Db,
    options: &RenderOptions,
    tight: bool,
    gap: f64,
) -> Result<(Drawn, usize), PelersError> {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.8125);
    let pad = options.padding.unwrap_or(8.0);
    let mut links: Vec<LinkInfo> = Vec::new();
    let mut layout = layout_c4(
        db,
        size,
        small,
        if tight {
            Some(js_max(80.0, options.max_width.unwrap_or(320.0) - 2.0 * pad))
        } else {
            None
        },
        gap,
    );
    let root = layout.groups[0].boxed.clone();

    let mut min_x = 0.0f64;
    let mut min_y = 0.0f64;
    let mut max_x = root.w;
    let mut max_y = root.h;
    let mut grow = |x0: f64, y0: f64, x1: f64, y1: f64| {
        min_x = js_min(min_x, x0);
        min_y = js_min(min_y, y0);
        max_x = js_max(max_x, x1);
        max_y = js_max(max_y, y1);
    };

    let mut ends: Vec<Box> = Vec::new();
    let mut pairs: HashMap<usize, HashSet<usize>> = HashMap::new();
    for rel in &db.rels {
        let from = if rel.from_attr.is_some() {
            None
        } else {
            db.find_element(&rel.from)
        };
        let to = if rel.to_attr.is_some() {
            None
        } else {
            db.find_element(&rel.to)
        };
        let (Some(a), Some(b)) = (box_of(&layout, from), box_of(&layout, to)) else {
            let mut error = PelersError::new(
                format!(
                    "C4 rel \"{}\" -> \"{}\" references an unknown shape or boundary",
                    rel.from, rel.to
                ),
                PelersErrorCode::Semantic,
            );
            error.diagram_type = Some("c4".to_owned());
            return Err(error);
        };
        pairs.entry(a.id).or_default().insert(b.id);
        ends.push(a);
        ends.push(b);
    }

    let mut nodes: Vec<Box> = Vec::new();
    let mut taken: Vec<f64> = Vec::new();
    for &g in &layout.order {
        for &n in &layout.groups[g].nodes {
            let b = layout.nodes[n].boxed.clone();
            taken.extend_from_slice(&[b.x + EDGE, b.y + EDGE, b.x + b.w - EDGE, b.y + b.h - EDGE]);
            nodes.push(b);
        }
    }
    let careful = db.rels.len() as f64
        * (nodes.len() as f64 + layout.order.len() as f64 + 3.0 * db.rels.len() as f64)
        <= CARE_BUDGET;

    let mut casings = String::new();
    let mut edges_out = String::new();
    let trim = marker_trim("arrow_point");
    let mut fills: Vec<Fill> = Vec::new();
    for &g in &layout.order {
        if g == 0 {
            continue;
        }
        let group = &layout.groups[g];
        let boundary = &db.boundaries[group.boundary];
        let shape = user_style(boundary.bg_color.as_deref(), None, None).shape;
        if paints(&shape) {
            let b = &group.boxed;
            fills.push(Fill {
                box_: [b.x, b.y, b.x + b.w, b.y + b.h],
                shape,
            });
        }
    }
    let mut lanes = Lanes::default();
    for &g in &layout.order {
        if g != 0 {
            lanes.frame(&layout.groups[g].boxed);
        }
    }
    let mut routes: Vec<Route> = Vec::new();
    for (index, rel) in db.rels.iter().enumerate() {
        let a = ends[index * 2].clone();
        let b = ends[index * 2 + 1].clone();
        let style = user_style(None, rel.line_color.as_deref(), None);
        let start_arrow = rel.type_ == "birel" || rel.type_ == "rel_b";
        let end_arrow = rel.type_ != "rel_b";
        let reverse = a.id != b.id
            && pairs
                .get(&b.id)
                .is_some_and(|targets| targets.contains(&a.id));
        let r = route(
            &a,
            &b,
            if reverse { PAIR_SHIFT } else { 0.0 },
            if careful {
                Some(nodes.as_slice())
            } else {
                None
            },
            &mut lanes,
            if start_arrow { trim } else { 0.0 },
            if end_arrow { trim } else { 0.0 },
        );
        grow(
            js_min(r.x0, r.x1),
            js_min(r.y0, r.y1),
            js_max(r.x0, r.x1),
            js_max(r.y0, r.y1),
        );
        if start_arrow && careful {
            taken.extend_from_slice(&[r.sx - ARROW, r.sy - ARROW, r.sx + ARROW, r.sy + ARROW]);
        }
        if end_arrow && careful {
            taken.extend_from_slice(&[r.ex - ARROW, r.ey - ARROW, r.ex + ARROW, r.ey + ARROW]);
        }
        let over = over_fills(&r.d, &[r.sx, r.sy, r.ex, r.ey], &fills, &style.line, None);
        let over_text = over.clone().unwrap_or_default();
        let color = if !over_text.is_empty() {
            over_text.clone()
        } else if let Some(stroke) = style.stroke.as_deref().filter(|s| !s.is_empty()) {
            esc(stroke).into_owned()
        } else {
            "var(--_l)".to_owned()
        };
        if over_text.is_empty() && over.is_some() {
            casings.push_str(&edge_casing(
                &to_edge_path(&r),
                if start_arrow { "arrow_point" } else { "none" },
                if end_arrow { "arrow_point" } else { "none" },
                &style.line,
                false,
            ));
        }
        let mut markers = String::new();
        if start_arrow {
            markers.push_str(&marker(
                "arrow_point",
                r.sx,
                r.sy,
                r.sdx,
                r.sdy,
                &color,
                "none",
            ));
        }
        if end_arrow {
            markers.push_str(&marker(
                "arrow_point",
                r.ex,
                r.ey,
                r.edx,
                r.edy,
                &color,
                "none",
            ));
        }
        edges_out.push_str(&format!(
            "<g class=\"pelers-edge\" data-id=\"{}\"{}{}><path d=\"{}\"/>{markers}</g>",
            esc_text(&format!("{}-{}", rel.from, rel.to)),
            if over_text.is_empty() {
                String::new()
            } else {
                format!(" stroke=\"{over_text}\"")
            },
            style.line,
            r.d
        ));
        routes.push(r);
    }

    for &g in &layout.order {
        let group = &layout.groups[g];
        if !careful || group.head_w == 0.0 {
            continue;
        }
        let (gx, gy, gw) = (group.boxed.x, group.boxed.y, group.boxed.w);
        let head_w = group.head_w;
        let top = gy + HEAD_TOP;
        let bottom = gy + group.head - LANE;
        let mut fewest = f64::INFINITY;
        let candidates = [HEAD_LEFT, (gw - head_w) / 2.0, gw - HEAD_LEFT - head_w];
        let mut chosen = group.head_x;
        for x in candidates {
            let count = routes
                .iter()
                .filter(|r| crosses(&r.points, gx + x - 4.0, top, gx + x + head_w + 4.0, bottom))
                .count() as f64;
            if count < fewest {
                fewest = count;
                chosen = x;
            }
            if count == 0.0 {
                break;
            }
        }
        layout.groups[g].head_x = chosen;
        taken.extend_from_slice(&[gx + chosen - 4.0, top, gx + chosen + head_w + 4.0, bottom]);
    }

    let mut labels_out = String::new();
    for (index, rel) in db.rels.iter().enumerate() {
        let r = &routes[index];
        let id = format!("{}-{}", rel.from, rel.to);
        let style = user_style(None, None, rel.text_color.as_deref());
        let text = format!(
            "{}{}",
            if db.c4_type.as_deref() == Some("C4Dynamic") {
                format!("{}: ", index + 1)
            } else {
                String::new()
            },
            rel.label
        );
        let label = layout_label(Some(&text), false, small, r.room, 0);
        let techn_text = rel
            .techn
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| format!("[{t}]"));
        let techn = layout_label(techn_text.as_deref(), false, small, r.room, ITALIC);
        let lw = js_max(label.width, techn.width) + LABEL_PAD;
        let lh = label.height + techn.height;
        if lh == 0.0 {
            continue;
        }

        let beside = r.nx.abs() * (lw / 2.0) + r.ny.abs() * (lh / 2.0) + 2.0;
        let aside = if r.lean != 0.0 {
            js_max(0.0, js_min(r.lean, beside - 14.0))
        } else {
            beside
        };
        let mut dx = r.nx * aside + if tight { 0.0 } else { offset(rel.offset_x) };
        let mut dy = r.ny * aside + if tight { 0.0 } else { offset(rel.offset_y) };
        let mut at = 0.5;
        if careful {
            let mut least = f64::INFINITY;
            let moved = dx != r.nx * aside || dy != r.ny * aside;
            let passes = if moved { 2 } else { 1 };
            for pass in 0..passes {
                if least <= 0.0 {
                    break;
                }
                let px = if pass == 1 { r.nx * aside } else { dx };
                let py = if pass == 1 { r.ny * aside } else { dy };
                for t in SLIDE {
                    let cx = r.x0 + (r.x1 - r.x0) * t + px;
                    let cy = r.y0 + (r.y1 - r.y0) * t + py;
                    let area = covered(
                        &taken,
                        cx - lw / 2.0,
                        cy - lh / 2.0,
                        cx + lw / 2.0,
                        cy + lh / 2.0,
                    );
                    if area < least {
                        least = area;
                        at = t;
                        dx = px;
                        dy = py;
                    }
                    if area == 0.0 {
                        break;
                    }
                }
            }
        }
        let cx = r.x0 + (r.x1 - r.x0) * at + dx;
        let top = r.y0 + (r.y1 - r.y0) * at + dy - lh / 2.0;
        if careful {
            taken.extend_from_slice(&[cx - lw / 2.0, top, cx + lw / 2.0, top + lh]);
        }
        grow(cx - lw / 2.0, top, cx + lw / 2.0, top + lh);
        let body = format!(
            "{}{}",
            label_svg(&label, cx, top + label.height / 2.0, &style.text),
            label_svg(
                &techn,
                cx,
                top + label.height + techn.height / 2.0,
                &format!(
                    " class=\"pelers-c4-techn\" fill=\"var(--_m)\"{}",
                    style.text
                )
            )
        );
        labels_out.push_str(&format!(
            "<g class=\"pelers-edge-label\" data-id=\"{}\">{}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"var(--_bg)\"/>{}</g>",
            esc_text(&id),
            rel.descr
                .as_deref()
                .filter(|d| !d.is_empty())
                .map_or_else(String::new, |d| format!("<title>{}</title>", esc_text(d))),
            num(cx - lw / 2.0),
            num(top),
            num(lw),
            num(lh),
            link(&body, rel.link.as_deref(), &id, &mut links, options)
        ));
    }

    let mut clusters = String::new();
    let mut nodes_out = String::new();
    for &g in &layout.order {
        if g != 0 {
            let boundary = db.boundaries[layout.groups[g].boundary].clone();
            let group = &layout.groups[g];
            let style = user_style(
                boundary.bg_color.as_deref(),
                boundary.border_color.as_deref(),
                boundary.font_color.as_deref(),
            );
            let x = group.boxed.x + group.head_x;
            let mut y = group.boxed.y + HEAD_TOP;
            let color = format!(
                "{}{}",
                ink_fill(&style.shape, &style.text, None),
                style.text
            );
            let muted = format!(
                "{}{}",
                ink_fill(&style.shape, &style.text, Some("var(--_m)")),
                style.text
            );
            let mut text = left_label_svg(
                &group.labels[0],
                x,
                y,
                &format!(" class=\"pelers-cluster-label\" font-weight=\"var(--_hw)\"{color}"),
            );
            y += group.labels[0].height;
            text.push_str(&left_label_svg(
                &group.labels[1],
                x,
                y,
                &format!(" class=\"pelers-c4-type\" font-size=\"{small}\"{muted}"),
            ));
            y += group.labels[1].height;
            text.push_str(&left_label_svg(
                &group.labels[2],
                x,
                y,
                &format!(" class=\"pelers-c4-descr\" font-size=\"{small}\"{muted}"),
            ));
            let kind = if boundary.node_type.is_some() {
                "node"
            } else {
                "boundary"
            };
            let dashed = if boundary.node_type.is_some() {
                ""
            } else {
                " stroke-dasharray=\"6 4\""
            };
            clusters.push_str(&format!(
                "<g class=\"pelers-cluster pelers-c4-{kind}\" data-id=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"var(--_b)\"{dashed}{}/>{}</g>",
                esc_text(&boundary.alias),
                num(group.boxed.x),
                num(group.boxed.y),
                num(group.boxed.w),
                num(group.boxed.h),
                rounded(&style.shape),
                link(&text, boundary.link.as_deref(), &boundary.alias, &mut links, options)
            ));
        }
        for &n in &layout.groups[g].nodes {
            let node = &layout.nodes[n];
            let shape: &Element = &db.shapes[node.shape];
            let external = shape.kind.starts_with("external");
            let style = user_style(
                shape.bg_color.as_deref(),
                shape.border_color.as_deref(),
                shape.font_color.as_deref(),
            );
            let stroke = format!(
                " stroke=\"var(--_b)\"{}",
                if external {
                    " stroke-dasharray=\"4 3\""
                } else {
                    ""
                }
            );
            let b = &node.boxed;
            let (w, h, form) = (b.w, b.h, b.form);
            let labels = &node.labels;
            let dx = if form == "h-cyl" {
                -queue_rx(h) / 2.0
            } else {
                0.0
            };
            let dy = match form {
                "person" => HEAD / 2.0,
                "cyl" => cyl_ry(w) / 2.0,
                _ => 0.0,
            };
            let color = format!(
                "{}{}",
                ink_fill(&style.shape, &style.text, None),
                style.text
            );
            let muted = format!(
                "{}{}",
                ink_fill(&style.shape, &style.text, Some("var(--_m)")),
                style.text
            );
            let mut y = dy - node.text_h / 2.0;
            let mut text = label_svg(
                &labels[0],
                dx,
                y + labels[0].height / 2.0,
                &format!(
                    " class=\"pelers-c4-type\" font-size=\"{}\"{muted}",
                    small - 1.0
                ),
            );
            y += labels[0].height;
            text.push_str(&label_svg(
                &labels[1],
                dx,
                y + labels[1].height / 2.0,
                &format!(" class=\"pelers-label\" font-weight=\"var(--_hw)\"{color}"),
            ));
            y += labels[1].height;
            text.push_str(&label_svg(
                &labels[2],
                dx,
                y + labels[2].height / 2.0,
                &format!(" class=\"pelers-c4-techn\" font-size=\"{small}\"{muted}"),
            ));
            y += labels[2].height + 4.0;
            text.push_str(&label_svg(
                &labels[3],
                dx,
                y + labels[3].height / 2.0,
                &format!(" class=\"pelers-c4-descr\" font-size=\"{small}\"{color}"),
            ));
            let fill_var = if external { "--_bg" } else { "--_s" };
            let drawn_shape = draw_shape(
                form,
                w,
                h,
                &format!(" fill=\"var({fill_var})\"{stroke}{}", style.shape),
                &format!("{stroke}{}", style.line),
            );
            nodes_out.push_str(&format!(
                "<g class=\"pelers-node pelers-shape-{form} pelers-c4-{}{}\" data-id=\"{}\" transform=\"translate({},{})\">{}</g>",
                shape.kind,
                if external { " pelers-c4-external" } else { "" },
                esc_text(&shape.alias),
                num(b.x + w / 2.0),
                num(b.y + h / 2.0),
                link(
                    &format!("{drawn_shape}{text}"),
                    shape.link.as_deref(),
                    &shape.alias,
                    &mut links,
                    options
                )
            ));
        }
    }

    let title = layout_label(
        db.title.as_deref(),
        false,
        size,
        if tight { title_room(options) } else { 4000.0 },
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let inner = js_max(max_x - min_x, title.width);
    let width = (inner + 2.0 * pad).ceil();
    let height = (max_y - min_y + title_height + 2.0 * pad).ceil();
    let ox = pad - min_x + (inner - (max_x - min_x)) / 2.0;
    let oy = pad + title_height - min_y;

    let mut body = String::new();
    body.push_str(&format!(
        "<g transform=\"translate({},{})\">",
        num(ox),
        num(oy)
    ));
    if !clusters.is_empty() {
        body.push_str(&format!("<g class=\"pelers-clusters\">{clusters}</g>"));
    }
    body.push_str(&casings_svg(&casings));
    if !edges_out.is_empty() {
        body.push_str(&format!(
            "<g class=\"pelers-edges\" fill=\"none\" stroke=\"var(--_l)\" stroke-linecap=\"round\">{edges_out}</g>"
        ));
    }
    if !nodes_out.is_empty() {
        body.push_str(&format!("<g class=\"pelers-nodes\">{nodes_out}</g>"));
    }
    if !labels_out.is_empty() {
        body.push_str(&format!(
            "<g class=\"pelers-edge-labels\" font-size=\"{}\">{labels_out}</g>",
            num(small)
        ));
    }
    body.push_str("</g>");

    let content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &body;
    let svg = svg_document(
        "c4",
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

    Ok((
        Drawn {
            svg,
            width,
            height,
            links,
        },
        lanes.squeezed,
    ))
}
