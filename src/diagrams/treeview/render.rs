use super::icons::{BUILTIN_PACK, glyph, icon_config, node_icon};
use super::model::{NodeType, TreeNode, TreeViewModel};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{tighten, title_room};
use crate::diagrams::common::literal_label::{left_label_svg, literal_label};
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ROUNDED, class_names};
use crate::text::entities::decode_entities;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, ITALIC, MONO, width as measure};
use crate::util::chars::is_js_space;
use crate::util::jsnum::{js_max, js_min, js_round};
use crate::util::value::{Object, Value};

const ICON_GAP: f64 = 6.0;
const DESC_GAP: f64 = 16.0;
const GUIDE_INSET: f64 = 7.0;
const ELBOW: f64 = 4.0;
const HIGHLIGHT_OVERHANG: f64 = 4.0;
const FONT_MONO: &str = "var(--_fm)";

fn setting(config: &Object, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match config
        .get("treeView")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::Number(value)) if *value >= min && *value <= max => *value,
        _ => fallback,
    }
}

pub struct TreeSettings {
    pub padding_x: f64,
    pub padding_y: f64,
    pub row_indent: f64,
    pub line_thickness: f64,
}

pub fn tree_settings(config: &Object) -> TreeSettings {
    TreeSettings {
        padding_x: setting(config, "paddingX", 5.0, 0.0, 100.0),
        padding_y: setting(config, "paddingY", 2.0, 0.0, 100.0),
        row_indent: setting(config, "rowIndent", 15.0, 0.0, 400.0),
        line_thickness: setting(config, "lineThickness", 1.0, 0.25, 12.0),
    }
}

struct Row {
    node: usize,
    x: f64,
    cy: f64,
    icon: Option<String>,
    text: String,
    label_x: f64,
    highlighted: bool,
    label: Option<Label>,
    description: Option<Label>,
    top: f64,
    height: f64,
}

pub fn render_treeview(model: &TreeViewModel, config: &Config, options: &RenderOptions) -> Drawn {
    tighten(options, |tight| draw(model, config, options, tight))
}

fn draw(model: &TreeViewModel, config: &Config, options: &RenderOptions, tight: bool) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let pad = options.padding.unwrap_or(8.0);
    let icons = icon_config(config);
    let mono = options
        .monospace
        .as_ref()
        .is_some_and(|list| list.iter().any(|name| name == "files"));
    let family = if mono {
        format!(" font-family=\"{FONT_MONO}\"")
    } else {
        String::new()
    };
    let max_width = options.max_width.unwrap_or(320.0);

    let settings = tree_settings(config);
    let (padding_x, padding_y) = (settings.padding_x, settings.padding_y);
    let mut max_depth = 0.0f64;
    let mut depths: Vec<(usize, f64)> = model
        .root()
        .children
        .iter()
        .map(|&node| (node, 0.0))
        .collect();
    let mut i = 0;
    while tight && i < depths.len() {
        let (node, depth) = depths[i];
        max_depth = js_max(max_depth, depth);
        for &child in &model.nodes[node].children {
            depths.push((child, depth + 1.0));
        }
        i += 1;
    }
    let step = if tight {
        js_min(
            settings.row_indent + padding_x,
            js_max(2.0, (max_width - 120.0) / js_max(1.0, max_depth)),
        )
    } else {
        settings.row_indent + padding_x
    };
    let row_height = js_round(size * 1.5) + 2.0 * padding_y;
    let icon_size = size;

    let title = layout_label(
        model.title.as_deref(),
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

    let has_icon = |list: &[usize]| -> bool {
        list.iter().any(|&child| {
            let node = &model.nodes[child];
            node_icon(&node.name, node.node_type, node.icon.as_deref(), &icons).is_some()
        })
    };
    let mut rows: Vec<Row> = Vec::new();
    let mut row_of: Vec<Option<usize>> = vec![None; model.nodes.len()];
    let mut pending: Vec<(usize, f64, bool)> = Vec::new();
    let push = |pending: &mut Vec<(usize, f64, bool)>, list: &[usize], depth: f64| {
        let slot = has_icon(list);
        for &node in list.iter().rev() {
            pending.push((node, depth, slot));
        }
    };
    push(&mut pending, &model.root().children, 0.0);
    let mut label_right = 0.0f64;
    let mut any_highlight = false;
    let mut rows_height = 0.0f64;
    while let Some((index, depth, slot)) = pending.pop() {
        let node: &TreeNode = &model.nodes[index];
        let x = depth * step;
        let text = decode_entities(&node.name);
        let label_x = x + if slot { icon_size + ICON_GAP } else { 0.0 };
        let room = js_max(
            40.0,
            max_width - 2.0 * pad - 2.0 * HIGHLIGHT_OVERHANG - label_x,
        );
        let directory = node.node_type == NodeType::Directory;
        let style = (if directory { BOLD_STYLE } else { 0 }) | (if mono { MONO } else { 0 });
        let label = tight.then(|| literal_label(&text, size, room, style));
        let description = match (&node.description, tight) {
            (Some(description), true) => Some(literal_label(
                &decode_entities(description),
                size,
                room,
                ITALIC,
            )),
            _ => None,
        };
        let width = match &label {
            Some(label) => label.width,
            None => measure(&text, size, style),
        };
        if label_x + width > label_right {
            label_right = label_x + width;
        }
        let highlighted = node
            .css_class
            .as_deref()
            .is_some_and(|classes| classes.split(is_js_space).any(|c| c == "highlight"));
        if highlighted {
            any_highlight = true;
        }
        let label_height = label.as_ref().map_or(0.0, |l| l.height);
        let description_height = description.as_ref().map_or(0.0, |d| d.height);
        let height = if tight {
            js_max(
                row_height,
                label_height + description_height + 2.0 * padding_y,
            )
        } else {
            row_height
        };
        let cy = rows_height
            + if tight {
                padding_y + js_max(icon_size, label_height) / 2.0
            } else {
                height / 2.0
            };
        row_of[index] = Some(rows.len());
        rows.push(Row {
            node: index,
            x,
            cy,
            icon: node_icon(&node.name, node.node_type, node.icon.as_deref(), &icons),
            text,
            label_x,
            highlighted,
            label,
            description,
            top: rows_height,
            height,
        });
        rows_height += height;
        push(&mut pending, &node.children, depth + 1.0);
    }

    let desc_x = label_right + DESC_GAP;
    let mut width = label_right;
    for row in &rows {
        let Some(description) = model.nodes[row.node].description.as_deref() else {
            continue;
        };
        let w = match &row.description {
            Some(label) => label.width,
            None => measure(&decode_entities(description), size, ITALIC),
        };
        width = js_max(width, if tight { row.label_x } else { desc_x } + w);
    }
    width = js_max(width, title.width);
    let any_highlight_inset = if any_highlight {
        HIGHLIGHT_OVERHANG
    } else {
        0.0
    };
    let inset = pad + any_highlight_inset;
    let top = pad + title_height;
    let total_width = (width + 2.0 * inset).ceil();
    let total_height = (top + rows_height + pad).ceil();

    let mut guides = String::new();
    for row in &rows {
        let children = &model.nodes[row.node].children;
        if children.is_empty() {
            continue;
        }
        let gx = inset + row.x + GUIDE_INSET;
        let tick = js_max(gx, inset + row.x + step - padding_x);
        let row_cy = |child: usize| rows[row_of[child].unwrap_or(0)].cy;
        let last = top + row_cy(children[children.len() - 1]);
        let r = js_min(ELBOW, tick - gx);
        let mut d = format!(
            "M{},{}V{}",
            num(gx),
            num(top + row.top + row.height - 2.0),
            num(last - r)
        );
        if r > 0.0 {
            d.push_str(&format!(
                "Q{},{} {},{}H{}",
                num(gx),
                num(last),
                num(gx + r),
                num(last),
                num(tick)
            ));
        }
        let mut i = 0;
        while i < children.len() - 1 && tick > gx {
            d.push_str(&format!(
                "M{},{}H{}",
                num(gx),
                num(top + row_cy(children[i])),
                num(tick)
            ));
            i += 1;
        }
        guides.push_str(&format!("<path d=\"{d}\"/>"));
    }

    let baseline = size * 0.35;
    let mut body = String::new();
    for row in &rows {
        let node = &model.nodes[row.node];
        let cy = top + row.cy;
        let directory = node.node_type == NodeType::Directory;
        let mut inner = String::new();
        if row.highlighted {
            let x = inset + row.x - HIGHLIGHT_OVERHANG;
            inner.push_str(&format!(
                "<rect class=\"pelers-tree-highlight\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_c)\" fill-opacity=\"0.14\"{ROUNDED}/>",
                num(x),
                num(top + row.top + 1.0),
                num(total_width - inset + HIGHLIGHT_OVERHANG - x),
                num(row.height - 2.0)
            ));
        }
        if let Some(icon) = &row.icon {
            let name = icon.strip_prefix(&format!("{BUILTIN_PACK}:")).unwrap_or("");
            let fallback = if directory { "folder" } else { "file" };
            let path = glyph(name).or_else(|| glyph(fallback)).unwrap_or("");
            let drawn = options
                .icons
                .as_ref()
                .and_then(|hook| (hook.0)(icon))
                .unwrap_or_else(|| {
                    format!("<path d=\"{path}\" stroke=\"var(--_m)\" stroke-width=\"1.5\"/>")
                });
            inner.push_str(&format!(
                "<svg class=\"pelers-icon\" data-icon=\"{}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\">{drawn}</svg>",
                esc_text(icon),
                num(inset + row.x),
                num(cy - icon_size / 2.0),
                num(icon_size),
                num(icon_size)
            ));
        }
        let bold_attr = if directory {
            " font-weight=\"var(--_hw)\""
        } else {
            ""
        };
        if let Some(label) = &row.label {
            inner.push_str(&left_label_svg(
                label,
                inset + row.label_x,
                top + row.top + padding_y,
                &format!(" class=\"pelers-label\"{bold_attr}{family}"),
            ));
        } else if !row.text.is_empty() {
            inner.push_str(&format!(
                "<text class=\"pelers-label\" x=\"{}\" y=\"{}\"{bold_attr}{family} xml:space=\"preserve\">{}</text>",
                num(inset + row.label_x),
                num(cy + baseline),
                esc(&row.text)
            ));
        }
        if let Some(description) = &row.description {
            inner.push_str(&left_label_svg(
                description,
                inset + row.label_x,
                top + row.top + padding_y + row.label.as_ref().map_or(0.0, |l| l.height),
                " class=\"pelers-tree-description\" font-style=\"italic\" fill=\"var(--_m)\"",
            ));
        } else if let Some(text) = &node.description {
            inner.push_str(&format!(
                "<text class=\"pelers-tree-description\" x=\"{}\" y=\"{}\" font-style=\"italic\" fill=\"var(--_m)\" xml:space=\"preserve\">{}</text>",
                num(inset + desc_x),
                num(cy + baseline),
                esc(&decode_entities(text))
            ));
        }
        body.push_str(&format!(
            "<g class=\"pelers-node pelers-tree-{}{}\" data-id=\"{}\">{inner}</g>",
            if directory { "dir" } else { "file" },
            class_names(node.css_class.as_deref().unwrap_or("")),
            esc_text(&node.name)
        ));
    }

    let mut content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    if !guides.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-tree-guides\" fill=\"none\" stroke=\"var(--_b)\" stroke-width=\"{}\">{guides}</g>",
            num(settings.line_thickness)
        ));
    }
    content.push_str(&body);

    Drawn {
        svg: svg_document(
            "treeView",
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
