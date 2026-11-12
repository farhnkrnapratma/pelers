use std::collections::HashMap;

use super::layout::{Alignment, LayoutOptions, SankeyLayout, layout_sankey};
use super::parser::SankeyDb;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{fit_width, title_room};
use crate::errors::PelersError;
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{resolve_style, series_color};
use crate::text::entities::decode_entities;
use crate::text::label::layout_label;
use crate::text::measurer::{BOLD_STYLE, width as measure};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::{Object, Value};

const LABEL_GAP: f64 = 6.0;
const VALUE_GAP: f64 = 15.0;
const LABEL_ROOM: f64 = 1.8;
const CAREFUL: usize = 300;

fn content_id(text: &str) -> String {
    let mut a: u32 = 0x811c_9dc5;
    let mut b: u32 = 5381;
    for unit in text.encode_utf16() {
        let c = u32::from(unit);
        a = (a ^ c).wrapping_mul(0x0100_0193);
        b = b.wrapping_mul(33).wrapping_add(c);
    }
    format!("{}{}", radix36(a), radix36(b))
}

fn radix36(mut value: u32) -> String {
    const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_owned();
    }
    let mut out = Vec::new();
    while value > 0 {
        out.push(DIGITS[(value % 36) as usize]);
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn number_option(conf: &Object, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match conf.get(key) {
        Some(Value::Number(value)) if *value >= min => js_min(*value, max),
        _ => fallback,
    }
}

fn text_option(conf: &Object, key: &str) -> String {
    match conf.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(n)) if n.is_finite() => js_number_to_string(*n),
        _ => String::new(),
    }
}

fn rounded(value: f64) -> f64 {
    if value.abs() < 1e15 {
        js_round(value * 100.0) / 100.0
    } else {
        value
    }
}

fn fill_style(color: &Value) -> String {
    match color {
        Value::String(text) => resolve_style(&[format!("fill:{text}")]).shape,
        _ => String::new(),
    }
}

fn fill_of(style: &str) -> Option<&str> {
    let inner = style.strip_prefix(" style=\"fill:")?.strip_suffix(";\"")?;
    if inner.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        None
    } else {
        Some(inner)
    }
}

#[derive(Clone)]
struct LabelPos {
    value: String,
    width: f64,
    after: bool,
    left: f64,
    y: f64,
}

fn overlaps(
    bands: &HashMap<i64, Vec<usize>>,
    labels: &[LabelPos],
    line_height: f64,
    own: usize,
    left: f64,
    y: f64,
) -> bool {
    bands
        .get(&((y / line_height).floor() as i64))
        .is_some_and(|list| {
            list.iter().any(|&other| {
                other != own
                    && (labels[other].y - y).abs() < line_height
                    && left < labels[other].left + labels[other].width + LABEL_GAP
                    && left + labels[own].width > labels[other].left - LABEL_GAP
            })
        })
}

pub fn render_sankey(
    model: &SankeyDb,
    config: &Config,
    options: &RenderOptions,
) -> Result<Drawn, PelersError> {
    let conf = match config.get("sankey") {
        Some(Value::Object(object)) => object.clone(),
        _ => Object::default(),
    };
    let natural = number_option(&conf, "width", 600.0, 1.0, 10000.0);
    let mut failure = None;
    let drawn = fit_width(options, natural, 160.0, |chart_width, _| {
        match draw(model, &conf, options, chart_width, natural) {
            Ok(drawn) => drawn,
            Err(error) => {
                failure = Some(error);
                Drawn {
                    svg: String::new(),
                    width: 0.0,
                    height: 0.0,
                    links: Vec::new(),
                }
            }
        }
    });
    match failure {
        Some(error) => Err(error),
        None => Ok(drawn),
    }
}

fn draw(
    model: &SankeyDb,
    conf: &Object,
    options: &RenderOptions,
    chart_width: f64,
    natural: f64,
) -> Result<Drawn, PelersError> {
    let size = options.font_size.unwrap_or(16.0);
    let least = js_round(size * 0.625);
    let mut small = js_max(
        least,
        js_round(size * 0.875 * js_min(1.0, chart_width / natural)),
    );
    let pad = options.padding.unwrap_or(8.0);

    let chart_height = number_option(conf, "height", 400.0, 1.0, 10000.0);
    let node_width = js_min(
        number_option(conf, "nodeWidth", 10.0, 1.0, 200.0),
        chart_width,
    );
    let show_values = !matches!(conf.get("showValues"), Some(Value::Bool(false)));
    let prefix = decode_entities(&text_option(conf, "prefix"));
    let suffix = decode_entities(&text_option(conf, "suffix"));
    let outlined = matches!(conf.get("labelStyle"), Some(Value::String(s)) if s == "outlined");
    let alignment = match conf.get("nodeAlignment") {
        Some(Value::String(s)) if s == "left" => Alignment::Left,
        Some(Value::String(s)) if s == "right" => Alignment::Right,
        Some(Value::String(s)) if s == "center" => Alignment::Center,
        _ => Alignment::Justify,
    };
    let link_color = match conf.get("linkColor") {
        Some(Value::String(text)) => text.clone(),
        _ => "gradient".to_owned(),
    };
    let node_padding = number_option(conf, "nodePadding", 12.0, 0.0, 200.0)
        + if show_values { VALUE_GAP } else { 0.0 };

    let lay = |label_size: f64| -> Result<SankeyLayout, PelersError> {
        layout_sankey(
            model,
            LayoutOptions {
                width: chart_width,
                height: chart_height,
                node_width,
                node_padding,
                label_height: label_size * 1.15,
                alignment,
            },
        )
    };
    let mut layout = lay(small)?;
    let mut rows: HashMap<usize, usize> = HashMap::new();
    let mut fullest = 1usize;
    for node in &layout.nodes {
        let count = rows.get(&node.layer).copied().unwrap_or(0) + 1;
        rows.insert(node.layer, count);
        if count > fullest {
            fullest = count;
        }
    }
    let crowded = js_max(least, js_round(chart_height / fullest as f64 / LABEL_ROOM));
    if crowded < small {
        small = crowded;
        layout = lay(small)?;
    }
    let SankeyLayout { nodes, links } = layout;
    let line_height = small * 1.5;

    let mut custom: HashMap<&str, String> = HashMap::new();
    if let Some(Value::Object(colors)) = conf.get("nodeColors") {
        for (id, color) in colors.iter() {
            let style = fill_style(color);
            if !style.is_empty() {
                custom.insert(id, style);
            }
        }
    }
    let node_style = |i: usize| -> String {
        custom
            .get(model.nodes[i].id.as_str())
            .cloned()
            .unwrap_or_default()
    };
    let node_color = |i: usize| -> String {
        let style = node_style(i);
        match fill_of(&style) {
            Some(color) => color.to_owned(),
            None => series_color(i),
        }
    };

    let mut central = 0usize;
    let mut most = 0.0f64;
    for node in &nodes {
        if node.value > most {
            most = node.value;
            central = node.layer;
        }
    }
    let mut min_x = 0.0f64;
    let mut max_x = chart_width;
    let mut min_y = 0.0f64;
    let mut max_y = chart_height;
    let mut labels: Vec<LabelPos> = Vec::with_capacity(nodes.len());
    for (i, node) in nodes.iter().enumerate() {
        let name = decode_entities(&model.nodes[i].id);
        let value = if show_values {
            format!(
                "{prefix}{}{suffix}",
                js_number_to_string(rounded(node.value))
            )
        } else {
            String::new()
        };
        let width = measure(
            &if value.is_empty() {
                name.clone()
            } else {
                format!("{name} {value}")
            },
            small,
            0,
        );
        let after = if outlined {
            node.layer >= central
        } else {
            node.x < chart_width / 2.0
        };
        let x = if after {
            node.x + node_width + LABEL_GAP
        } else {
            node.x - LABEL_GAP
        };
        let y = node.y + node.height / 2.0;
        min_x = js_min(min_x, if after { x } else { x - width });
        max_x = js_max(max_x, if after { x + width } else { x });
        min_y = js_min(min_y, y - small * 0.75);
        max_y = js_max(max_y, y + small * 0.75);
        labels.push(LabelPos {
            value,
            width,
            after,
            left: if after { x } else { x - width },
            y,
        });
    }

    let mut bands: HashMap<i64, Vec<usize>> = HashMap::new();
    if nodes.len() <= CAREFUL {
        for (index, label) in labels.iter().enumerate() {
            let band = (label.y / line_height).floor() as i64;
            for key in [band - 2, band - 1, band, band + 1, band + 2] {
                bands.entry(key).or_default().push(index);
            }
        }
    }

    let mut labels_out = String::new();
    let mut nodes_out = String::new();
    for (i, node) in nodes.iter().enumerate() {
        let id = model.nodes[i].id.as_str();
        let name = decode_entities(id);
        let height = js_max(node.height, 1.0);
        let style = node_style(i);
        nodes_out.push_str(&format!(
            "<rect class=\"pelers-sankey-node\" data-id=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"{}\"{style}/>",
            esc_text(id),
            num(node.x),
            num(node.y),
            num(node_width),
            num(height),
            num(js_min(2.0, height / 2.0)),
            series_color(i)
        ));

        if nodes.len() <= CAREFUL
            && overlaps(&bands, &labels, line_height, i, labels[i].left, labels[i].y)
        {
            'place: for dy in [0.0, -line_height, line_height] {
                let after_options = [labels[i].after, !labels[i].after];
                for after in after_options {
                    let left = if after {
                        node.x + node_width + LABEL_GAP
                    } else {
                        node.x - LABEL_GAP - labels[i].width
                    };
                    let y = labels[i].y + dy;
                    if overlaps(&bands, &labels, line_height, i, left, y)
                        || left < min_x
                        || left + labels[i].width > max_x
                        || y - small * 0.75 < min_y
                        || y + small * 0.75 > max_y
                    {
                        continue;
                    }
                    let blocked = nodes.iter().enumerate().any(|(j, bar)| {
                        j != i
                            && left < bar.x + node_width + LABEL_GAP
                            && left + labels[i].width > bar.x - LABEL_GAP
                            && y - small * 0.75 < bar.y + bar.height
                            && y + small * 0.75 > bar.y
                    });
                    if blocked {
                        continue;
                    }
                    let own = js_max(js_max(node.y - y, y - node.y - node.height), 0.0);
                    let nearer = nodes.iter().enumerate().any(|(j, bar)| {
                        j != i
                            && bar.x == node.x
                            && js_max(js_max(bar.y - y, y - bar.y - bar.height), 0.0) <= own
                    });
                    if nearer {
                        continue;
                    }
                    labels[i].left = left;
                    labels[i].y = y;
                    labels[i].after = after;
                    break 'place;
                }
            }
        }

        let label = &labels[i];
        let x = if label.after {
            node.x + node_width + LABEL_GAP
        } else {
            node.x - LABEL_GAP
        };
        let position = format!(
            " x=\"{}\" y=\"{}\" text-anchor=\"{}\"",
            num(x),
            num(label.y + small * 0.35),
            if label.after { "start" } else { "end" }
        );
        let text = esc(&name).into_owned()
            + &if label.value.is_empty() {
                String::new()
            } else {
                format!(
                    " <tspan class=\"pelers-sankey-value\" fill=\"var(--_m)\">{}</tspan>",
                    esc(&label.value)
                )
            };
        let outline = if outlined {
            format!(
                "<text{position} fill=\"none\" stroke=\"var(--_bg)\" stroke-width=\"3\" stroke-linejoin=\"round\">{}{}</text>",
                esc(&name),
                if label.value.is_empty() {
                    String::new()
                } else {
                    format!(" {}", esc(&label.value))
                }
            )
        } else {
            String::new()
        };
        labels_out.push_str(&format!(
            "<g class=\"pelers-label\" data-id=\"{}\">{outline}<text{position}>{text}</text></g>",
            esc_text(id)
        ));
    }

    let fixed = if matches!(link_color.as_str(), "gradient" | "source" | "target") {
        String::new()
    } else {
        fill_style(&Value::String(link_color.clone()))
    };
    let id_prefix = options.id_prefix.clone().unwrap_or_default();
    let gradient = link_color == "gradient"
        && !id_prefix.is_empty()
        && id_prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    let mut gradients: Vec<(String, String)> = Vec::new();
    let mut links_out = String::new();
    for link in &links {
        let end = if link_color == "target" {
            link.target
        } else {
            link.source
        };
        let half = js_max(link.thickness, 1.0) / 2.0;
        let x0 = nodes[link.source].x + node_width;
        let x1 = nodes[link.target].x;
        let mut paint = format!(
            "fill=\"{}\"{}",
            series_color(end),
            if fixed.is_empty() {
                node_style(end)
            } else {
                fixed.clone()
            }
        );
        if gradient {
            let stops = format!(
                "x1=\"{}\" x2=\"{}\"><stop offset=\"0\" stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/>",
                num(x0),
                num(x1),
                node_color(link.source),
                node_color(link.target)
            );
            let id = format!("{id_prefix}-sankey-{}", content_id(&stops));
            if !gradients.iter().any(|(existing, _)| *existing == id) {
                gradients.push((
                    id.clone(),
                    format!(
                        "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" {stops}</linearGradient>"
                    ),
                ));
            }
            paint = format!("fill=\"url(#{id}) {}\"", node_color(link.source));
        }
        let mid = num((x0 + x1) / 2.0);
        let from = esc_text(&model.nodes[link.source].id);
        let to = esc_text(&model.nodes[link.target].id);
        links_out.push_str(&format!(
            "<path class=\"pelers-sankey-link\" data-id=\"{from}-&gt;{to}\" d=\"M{},{}C{mid},{} {mid},{} {},{}L{},{}C{mid},{} {mid},{} {},{}Z\" {paint}><title>{from} → {to}: {}{}{}</title></path>",
            num(x0),
            num(link.source_y - half),
            num(link.source_y - half),
            num(link.target_y - half),
            num(x1),
            num(link.target_y - half),
            num(x1),
            num(link.target_y + half),
            num(link.target_y + half),
            num(link.source_y + half),
            num(x0),
            num(link.source_y + half),
            esc(&prefix),
            js_number_to_string(rounded(link.value)),
            esc(&suffix),
        ));
    }

    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        title_room(options),
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };
    let inner = js_max(max_x - min_x, title.width);
    let ox = pad - min_x + (inner - (max_x - min_x)) / 2.0;
    let oy = pad + title_height - min_y;
    let width = (inner + 2.0 * pad).ceil();
    let height = (max_y - min_y + title_height + 2.0 * pad).ceil();

    let defs = if gradients.is_empty() {
        String::new()
    } else {
        format!(
            "<defs>{}</defs>",
            gradients
                .iter()
                .map(|(_, markup)| markup.as_str())
                .collect::<String>()
        )
    };
    let content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &format!(
        "<g transform=\"translate({},{})\">{defs}<g class=\"pelers-sankey-links\" fill-opacity=\"0.4\">{links_out}</g><g class=\"pelers-sankey-nodes\">{nodes_out}</g><g class=\"pelers-labels\" font-size=\"{}\">{labels_out}</g></g>",
        num(ox),
        num(oy),
        js_number_to_string(small)
    );

    Ok(Drawn {
        svg: svg_document(
            "sankey",
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
    })
}
