use super::format::formatter;
use super::layout::{Box_, Insets, layout_treemap};
use super::model::{TreeNode, TreemapModel};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_text::fit_text;
use crate::diagrams::common::fit_width::{cap_width, title_room};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, num};
use crate::svg::ink::ink_fill;
use crate::svg::palette::series_ink_fill;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{class_names, resolve_style, rounded, series_color, sets_fill};
use crate::text::entities::decode_entities;
use crate::text::label::layout_label;
use crate::text::measurer::{BOLD_STYLE, ITALIC, width as measure};
use crate::util::jsnum::{js_max, js_min, js_round};
use crate::util::value::Value;

const HEADER: f64 = 28.0;
const SHORT_HEADER: f64 = 18.0;
const SIDE: f64 = 6.0;
const LINE: f64 = 1.25;
const BOLD: u32 = BOLD_STYLE;

fn title_part(raw: &str) -> String {
    esc(decode_entities(raw).trim()).trim().to_owned()
}

fn leaf_count(model: &TreemapModel) -> usize {
    let mut count = 0;
    let mut stack: Vec<usize> = model.roots.clone();
    while let Some(index) = stack.pop() {
        let node = &model.nodes[index];
        match &node.children {
            Some(children) => stack.extend(children.iter().copied()),
            None => {
                if node.value.is_some_and(|v| v.is_finite() && v > 0.0) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn corners(attrs: &str, b: &Box_) -> String {
    let cap = js_min(b.w, b.h) / 4.0;
    if cap < 4.0 {
        rounded(attrs).replace("rx:var(--_r)", &format!("rx:min(var(--_r),{}px)", num(cap)))
    } else {
        rounded(attrs)
    }
}

fn option(config: &Config, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match config
        .get("treemap")
        .and_then(Value::as_object)
        .and_then(|t| t.get(key))
    {
        Some(Value::Number(value)) if *value >= min => js_min(*value, max),
        _ => fallback,
    }
}

fn section_tint(shape: &str) -> &'static str {
    if shape.contains("\"fill:") || shape.contains(";fill:") {
        ""
    } else {
        " fill-opacity=\"0.12\""
    }
}

pub fn render_treemap(model: &TreemapModel, config: &Config, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let tiny = js_round(size * 0.75);
    let least = js_round(size * 0.5625);
    let treemap = config.get("treemap").and_then(Value::as_object);
    let pad = options
        .padding
        .unwrap_or_else(|| option(config, "diagramPadding", 8.0, 0.0, 2000.0));

    let width = js_round(cap_width(
        options,
        option(config, "nodeWidth", 64.0, 1.0, 400.0) * 10.0,
        200.0,
        2.0 * pad,
    ));
    let height = js_round(option(config, "nodeHeight", 40.0, 1.0, 400.0) * 10.0);
    let show_values = !matches!(
        treemap.and_then(|t| t.get("showValues")),
        Some(Value::Bool(false))
    );
    let value_format = match treemap.and_then(|t| t.get("valueFormat")) {
        Some(Value::String(spec)) => spec.clone(),
        _ => ",".to_owned(),
    };
    let format = formatter(&value_format);
    let crowd = js_min(
        1.0,
        js_max(
            0.0,
            ((width * height) / js_max(1.0, leaf_count(model) as f64) / 800.0).log10(),
        ),
    );
    let (boxes, placed) = layout_treemap(
        &model.nodes,
        &model.roots,
        width,
        height,
        Insets {
            gap: option(config, "padding", js_round(2.0 + 2.0 * crowd), 0.0, 200.0),
            header: HEADER,
            short: SHORT_HEADER,
            side: js_round(3.0 + 3.0 * crowd),
        },
    );

    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        title_room(options),
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + 8.0
    } else {
        0.0
    };
    let left = pad + js_max(0.0, (title.width - width) / 2.0);
    let top = pad + title_height;

    let mut sections = String::new();
    let mut leaves = String::new();
    for &index in &placed {
        let b = &boxes[index];
        let node: &TreeNode = &model.nodes[b.node];
        let style = resolve_style(node.css_styles.as_deref().unwrap_or(&[]));
        let color = series_color(b.series);
        let x = left + b.x;
        let y = top + b.y;
        let name = decode_entities(&node.name).trim().to_owned();
        let value = if show_values {
            format.format(b.value)
        } else {
            String::new()
        };
        let group = format!("data-id=\"{}\">", esc_text(&node.name));
        let own = title_part(&node.name);
        let mut path: Vec<String> = if own.is_empty() {
            Vec::new()
        } else {
            vec![own.clone()]
        };
        let mut p = b.parent;
        while let Some(pi) = p {
            if boxes[pi].headed {
                break;
            }
            let section = title_part(&model.nodes[boxes[pi].node].name);
            if !section.is_empty() {
                path.insert(0, section);
            }
            p = boxes[pi].parent;
        }
        let full = path.join(" › ");
        let tooltip = |drawn: Option<&str>| -> String {
            if full.is_empty() || (drawn == Some(name.as_str()) && full == own) {
                String::new()
            } else {
                format!("<title>{full}</title>")
            }
        };
        let rect = format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\"",
            num(x),
            num(y),
            num(b.w),
            num(b.h)
        );
        let fixed = style.font_size;
        let text_style =
            (if style.bold { BOLD } else { 0 }) | (if style.italic { ITALIC } else { 0 });
        let ink = if sets_fill(&style.shape) {
            ink_fill(&style.shape, &style.text, None)
        } else {
            series_ink_fill(b.series)
        };

        if node.children.is_some() {
            let mut text = String::new();
            let mut drawn: Option<String> = None;
            if b.headed {
                let short = b.header < HEADER;
                let label_size = fixed.unwrap_or(if short { tiny } else { small });
                let value_size = fixed.unwrap_or(tiny);
                let room = b.w - 2.0 * SIDE - 4.0;
                let value_width = if value.is_empty() {
                    0.0
                } else {
                    measure(&value, value_size, text_style)
                };
                let with_value =
                    !short && value_width > 0.0 && room - value_width - 12.0 >= 3.0 * label_size;
                let label = if label_size * LINE < b.header {
                    fit_text(
                        &name,
                        if with_value {
                            room - value_width - 12.0
                        } else {
                            room
                        },
                        label_size,
                        label_size,
                        text_style | BOLD,
                    )
                } else {
                    None
                };
                let baseline = y + b.header / 2.0 + 1.0;
                drawn = label.as_ref().map(|l| l.text.clone());
                if let Some(label) = &label {
                    text.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-weight=\"var(--_hw)\"{}{ink}{}>{}</text>",
                        num(x + SIDE + 2.0),
                        num(baseline + label_size * 0.35),
                        if short {
                            format!(" font-size=\"{}\"", num(label_size))
                        } else {
                            String::new()
                        },
                        style.text,
                        esc(&label.text)
                    ));
                }
                if with_value && value_size * LINE <= HEADER {
                    text.push_str(&format!(
                        "<text class=\"pelers-value\" x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"end\"{}{}>{}</text>",
                        num(x + b.w - SIDE - 2.0),
                        num(baseline + value_size * 0.35),
                        num(tiny),
                        if ink.is_empty() {
                            " fill=\"var(--_m)\"".to_owned()
                        } else {
                            ink.clone()
                        },
                        style.text,
                        esc(&value)
                    ));
                }
            }
            let shape = if b.headed {
                format!(
                    "{rect}{}/>",
                    corners(
                        &format!(
                            "{} stroke=\"{color}\"{}",
                            section_tint(&style.shape),
                            style.shape
                        ),
                        b
                    )
                )
            } else {
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\"/>",
                    num(x),
                    num(y),
                    num(b.w),
                    num(b.h)
                )
            };
            sections.push_str(&format!(
                "<g class=\"pelers-node pelers-section{}\" {group}{}{shape}{text}</g>",
                class_names(node.class_selector.as_deref().unwrap_or("")),
                tooltip(drawn.as_deref())
            ));
            continue;
        }

        let mut text = String::new();
        let room = b.w - 8.0;
        let smallest = fixed.unwrap_or(least);
        let tallest = match fixed {
            None => js_min(size, ((b.h - 4.0) / LINE).floor()),
            Some(f) => {
                if f * LINE <= b.h - 4.0 {
                    f
                } else {
                    0.0
                }
            }
        };
        let label = if tallest >= smallest {
            fit_text(&name, room, tallest, smallest, text_style)
        } else {
            None
        };
        if let Some(label) = &label {
            let cx = num(x + b.w / 2.0);
            let cy = y + b.h / 2.0;
            let value_size = fixed.unwrap_or(js_max(least, js_round(label.size * 0.8)));
            let lines = (label.size + value_size) * LINE;
            if !value.is_empty()
                && lines <= b.h - 4.0
                && measure(&value, value_size, text_style) <= room
            {
                let first = cy - lines / 2.0 + (label.size * LINE) / 2.0;
                let second = cy + lines / 2.0 - (value_size * LINE) / 2.0;
                text = format!(
                    "<text x=\"{cx}\" y=\"{}\" font-size=\"{}\"{ink}{}>{}</text><text class=\"pelers-value\" x=\"{cx}\" y=\"{}\" font-size=\"{}\"{ink}{}>{}</text>",
                    num(first + label.size * 0.35),
                    num(label.size),
                    style.text,
                    esc(&label.text),
                    num(second + value_size * 0.35),
                    num(value_size),
                    style.text,
                    esc(&value)
                );
            } else {
                text = format!(
                    "<text x=\"{cx}\" y=\"{}\" font-size=\"{}\"{ink}{}>{}</text>",
                    num(cy + label.size * 0.35),
                    num(label.size),
                    style.text,
                    esc(&label.text)
                );
            }
        }
        leaves.push_str(&format!(
            "<g class=\"pelers-node pelers-leaf{}\" {group}{}{rect}{}/>{text}</g>",
            class_names(node.class_selector.as_deref().unwrap_or("")),
            tooltip(label.as_ref().map(|l| l.text.as_str())),
            corners(&style.shape, b)
        ));
    }

    let total_width = (js_max(width, title.width) + 2.0 * pad).ceil();
    let total_height = (height + title_height + 2.0 * pad).ceil();
    let content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &if sections.is_empty() {
        String::new()
    } else {
        format!(
            "<g class=\"pelers-sections\" font-size=\"{}\">{sections}</g>",
            num(small)
        )
    } + &if leaves.is_empty() {
        String::new()
    } else {
        format!(
            "<g class=\"pelers-leaves\" text-anchor=\"middle\" fill=\"var(--_bg)\">{leaves}</g>"
        )
    };
    Drawn {
        svg: svg_document(
            "treemap",
            total_width,
            total_height,
            size,
            options,
            Accessible {
                acc_title: model.acc_title.as_deref(),
                acc_descr: model.acc_descr.as_deref(),
            },
            &content,
        ),
        width: total_width,
        height: total_height,
        links: Vec::new(),
    }
}
