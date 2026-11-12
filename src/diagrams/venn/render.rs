use super::db::{VennDb, normalize_text};
use super::layout::{Circle, Spot, find_spot, layout_circles, region_path};
use crate::diagrams::common::Drawn;
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, num};
use crate::svg::ink::ink;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ResolvedStyle, resolve_style, series_color};
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, ITALIC};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};

const RADIUS: f64 = 100.0;
const MAX_GROWTH: f64 = 1.6;

struct Line {
    id: Option<String>,
    raw: String,
    size: f64,
    base: u32,
    label: Label,
}

struct Region {
    key: String,
    sets: Vec<String>,
    inside: Vec<usize>,
    spot: Spot,
    lines: Vec<Line>,
    style: ResolvedStyle,
    across: bool,
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn key_of(ids: &[String]) -> String {
    if ids.len() == 1 {
        ids[0].clone()
    } else {
        format!(
            "[{}]",
            ids.iter()
                .map(|id| json_string(id))
                .collect::<Vec<_>>()
                .join(",")
        )
    }
}

fn dedup(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        if !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

pub fn render_venn(model: &VennDb, _config: &Config, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let tiny = js_round(size * 0.8125);
    let pad = options.padding.unwrap_or(8.0);
    let no_style = resolve_style(&[]);

    let circles: Vec<Circle> = layout_circles(&model.subsets);
    let index = |id: &str| circles.iter().position(|c| c.id == id);

    let mut styles: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for decl in &model.styles {
        let key = key_of(&decl.targets);
        let at = match styles.iter().position(|(k, _)| *k == key) {
            Some(at) => at,
            None => {
                styles.push((key.clone(), Vec::new()));
                styles.len() - 1
            }
        };
        for (property, value) in &decl.styles {
            let merged = &mut styles[at].1;
            match merged.iter_mut().find(|(p, _)| p == property) {
                Some(entry) => entry.1 = value.clone(),
                None => merged.push((property.clone(), value.clone())),
            }
        }
    }
    let style_of = |key: &str| -> ResolvedStyle {
        match styles.iter().find(|(k, _)| k == key) {
            None => no_style.clone(),
            Some((_, declared)) => {
                let list: Vec<String> = declared
                    .iter()
                    .map(|(property, value)| format!("{property}:{value}"))
                    .collect();
                resolve_style(&list)
            }
        }
    };

    let mut neighbors: Vec<Vec<usize>> = vec![Vec::new(); circles.len()];
    for a in 0..circles.len() {
        for b in a + 1..circles.len() {
            let d = crate::util::jsnum::js_hypot(
                circles[a].x - circles[b].x,
                circles[a].y - circles[b].y,
            );
            if d < circles[a].r + circles[b].r {
                neighbors[a].push(b);
                neighbors[b].push(a);
            }
        }
    }

    let mut stated: Vec<(String, usize)> = Vec::new();
    for (at, subset) in model.subsets.iter().enumerate() {
        let key = key_of(&subset.sets);
        match stated.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = at,
            None => stated.push((key, at)),
        }
    }
    let mut regions: Vec<Region> = Vec::new();
    for (key, at) in &stated {
        let subset = &model.subsets[*at];
        let sets = dedup(&subset.sets);
        let inside: Vec<usize> = sets.iter().map(|id| index(id).unwrap()).collect();
        let mut smallest = inside[0];
        for &i in &inside {
            if circles[i].r < circles[smallest].r {
                smallest = i;
            }
        }
        let style = style_of(key);
        let text = match &subset.label {
            Some(label) => Some(label.clone()),
            None if subset.sets.len() == 1 => Some(subset.sets[0].clone()),
            None => None,
        };
        let base =
            (if style.bold { BOLD_STYLE } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let others: Vec<usize> = neighbors[smallest]
            .iter()
            .copied()
            .filter(|i| !inside.contains(i))
            .collect();
        let mut spot = find_spot(&circles, &inside, &others);
        let across = !others.is_empty() && spot.margin < circles[smallest].r * 0.2;
        if across {
            spot = find_spot(&circles, &inside, &[]);
        }
        let mut lines = Vec::new();
        if let Some(text) = text.filter(|t| !t.is_empty()) {
            let font = if sets.len() == 1 { size } else { small };
            lines.push(Line {
                id: None,
                label: layout_label(Some(&text), false, font, 4000.0, base),
                raw: text,
                size: font,
                base,
            });
        }
        regions.push(Region {
            key: key.clone(),
            sets,
            inside,
            spot,
            lines,
            style,
            across,
        });
    }
    for node in &model.text_nodes {
        let raw = node.label.clone().unwrap_or_else(|| node.id.clone());
        let key = key_of(&node.sets);
        if let Some(region) = regions.iter_mut().find(|r| r.key == key) {
            region.lines.push(Line {
                id: Some(node.id.clone()),
                label: layout_label(Some(&raw), false, tiny, 4000.0, 0),
                raw,
                size: tiny,
                base: 0,
            });
        }
    }

    let mut largest = 0.0f64;
    for c in &circles {
        if c.r > largest {
            largest = c.r;
        }
    }
    let mut scale = if largest > 0.0 { RADIUS / largest } else { 1.0 };
    let mut growth = 1.0f64;
    for region in &regions {
        if region.spot.margin <= 0.0 {
            continue;
        }
        let mut width = 0.0f64;
        let mut height = 0.0f64;
        for line in &region.lines {
            width = js_max(width, line.label.width);
            height += line.label.height;
        }
        let need = crate::util::jsnum::js_hypot(width, height) / 2.0 + 4.0;
        growth = js_max(
            growth,
            js_min(need / (region.spot.margin * scale), MAX_GROWTH),
        );
    }
    scale *= growth;

    for region in &mut regions {
        let room = js_max(region.spot.margin * scale * 1.8, 72.0);
        for line in &mut region.lines {
            if line.label.width > room {
                line.label = layout_label(Some(&line.raw), false, line.size, room, line.base);
            }
        }
    }

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for c in &circles {
        min_x = js_min(min_x, (c.x - c.r) * scale);
        min_y = js_min(min_y, (c.y - c.r) * scale);
        max_x = js_max(max_x, (c.x + c.r) * scale);
        max_y = js_max(max_y, (c.y + c.r) * scale);
    }
    for region in &regions {
        let mut wide = 0.0f64;
        let mut tall = 0.0f64;
        for line in &region.lines {
            wide = js_max(wide, line.label.width);
            tall += line.label.height;
        }
        min_x = js_min(min_x, region.spot.x * scale - wide / 2.0);
        max_x = js_max(max_x, region.spot.x * scale + wide / 2.0);
        min_y = js_min(min_y, region.spot.y * scale - tall / 2.0);
        max_y = js_max(max_y, region.spot.y * scale + tall / 2.0);
    }
    if circles.is_empty() {
        min_x = 0.0;
        min_y = 0.0;
        max_x = 0.0;
        max_y = 0.0;
    }

    let title_text = model.title.as_deref().map(normalize_text);
    let title = layout_label(title_text.as_deref(), false, size, 4000.0, BOLD_STYLE);
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let inner = js_max(max_x - min_x, title.width);
    let ox = pad + (inner - (max_x - min_x)) / 2.0 - min_x;
    let oy = pad + title_height - min_y;
    let width = (inner + 2.0 * pad).ceil();
    let height = (max_y - min_y + title_height + 2.0 * pad).ceil();
    let placed: Vec<Circle> = circles
        .iter()
        .map(|c| Circle {
            id: c.id.clone(),
            x: c.x * scale + ox,
            y: c.y * scale + oy,
            r: c.r * scale,
        })
        .collect();

    let mut sets = String::new();
    for (i, c) in placed.iter().enumerate() {
        let color = series_color(i);
        let style = regions
            .iter()
            .find(|r| r.key == c.id)
            .map_or_else(|| no_style.clone(), |r| r.style.clone());
        sets.push_str(&format!(
            "<circle class=\"pelers-venn-set\" data-id=\"{}\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{color}\" fill-opacity=\"0.2\" stroke=\"{color}\"{}/>",
            esc_text(&c.id),
            num(c.x),
            num(c.y),
            num(c.r),
            style.shape
        ));
    }

    let painted: Vec<&Region> = regions
        .iter()
        .filter(|r| r.sets.len() > 1 && !r.style.shape.is_empty())
        .collect();
    let under = |region: &Region| -> String {
        if region.across {
            return String::new();
        }
        let mut fill = if region.sets.len() == 1
            && (region.style.shape.contains("\"fill-opacity:")
                || region.style.shape.contains(";fill-opacity:"))
        {
            region.style.shape.clone()
        } else {
            String::new()
        };
        for other in &painted {
            if other.sets.iter().all(|id| region.sets.contains(id)) {
                fill = other.style.shape.clone();
            }
        }
        fill
    };

    let mut overlaps = String::new();
    let mut labels = String::new();
    for region in &regions {
        let id = esc_text(&region.sets.join("|"));
        if region.sets.len() > 1 && !region.style.shape.is_empty() {
            let members: Vec<Circle> = region.inside.iter().map(|&i| placed[i].clone()).collect();
            let d = region_path(&members, &|v| num(v).to_string());
            if !d.is_empty() {
                overlaps.push_str(&format!(
                    "<path class=\"pelers-venn-overlap\" data-id=\"{id}\" d=\"{d}\" fill=\"none\"{}/>",
                    region.style.shape
                ));
            }
        }
        let total: f64 = region.lines.iter().map(|l| l.label.height).sum();
        if total == 0.0 {
            continue;
        }
        let x = region.spot.x * scale + ox;
        let mut y = region.spot.y * scale + oy - total / 2.0;
        let fill = under(region);
        let mut content = String::new();
        for line in &region.lines {
            let own = match &line.id {
                None => region.style.text.clone(),
                Some(id) => style_of(id).text,
            };
            let color = ink(&fill, &own).unwrap_or("");
            let attrs = match &line.id {
                None => format!(
                    " class=\"pelers-label\"{}{}{own}",
                    if line.size == size {
                        String::new()
                    } else {
                        format!(" font-size=\"{}\"", js_number_to_string(line.size))
                    },
                    if color.is_empty() {
                        String::new()
                    } else {
                        format!(" fill=\"{color}\"")
                    }
                ),
                Some(id) => format!(
                    " class=\"pelers-venn-text\" data-id=\"{}\" font-size=\"{}\" fill=\"{}\"{own}",
                    esc_text(id),
                    js_number_to_string(tiny),
                    if color.is_empty() { "var(--_m)" } else { color }
                ),
            };
            content.push_str(&label_svg(
                &line.label,
                x,
                y + line.label.height / 2.0,
                &attrs,
            ));
            y += line.label.height;
        }
        labels.push_str(&format!(
            "<g class=\"pelers-venn-region\" data-id=\"{id}\">{content}</g>"
        ));
    }

    let content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &format!("<g class=\"pelers-venn-sets\">{sets}</g>")
        + &if overlaps.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pelers-venn-overlaps\">{overlaps}</g>")
        }
        + &if labels.is_empty() {
            String::new()
        } else {
            format!("<g class=\"pelers-venn-labels\">{labels}</g>")
        };

    Drawn {
        svg: svg_document(
            "venn",
            width,
            height,
            size,
            options,
            Accessible::default(),
            &content,
        ),
        width,
        height,
        links: Vec::new(),
    }
}
