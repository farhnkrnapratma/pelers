use super::db::{IshikawaDb, IshikawaNode};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::{title_room, turn_to_fit};
use crate::diagrams::common::literal_label::break_label;
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, num};
use crate::svg::edges::{ARROW, marker};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::ROUNDED;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::{js_hypot, js_max, js_min, js_round};

const SLANT: f64 = 0.5;
const FIRST_ROW: f64 = 14.0;
const ROW_GAP: f64 = 6.0;
const BONE_END: f64 = 8.0;
const TWIG_STEP: f64 = 22.0;
const STUB: f64 = 18.0;
const TEXT_GAP: f64 = 5.0;
const PAIR_GAP: f64 = 26.0;
const HEAD_GAP: f64 = 34.0;
const TAIL: f64 = 28.0;
const BOX_PAD_X: f64 = 10.0;
const BOX_PAD_Y: f64 = 5.0;

struct Bone {
    label: Label,
    depth: usize,
    parent: isize,
    slot: usize,
    kids: usize,
    rise: f64,
    at: f64,
    text_end: f64,
}

struct Category<'a> {
    node: &'a IshikawaNode,
    label: Label,
    box_w: f64,
    box_h: f64,
    bones: Vec<Bone>,
    rise: f64,
    reach: f64,
}

struct Frame {
    node: usize,
    depth: usize,
    parent: isize,
    slot: usize,
    bone: isize,
}

fn arrange<'a>(
    nodes: &'a [IshikawaNode],
    node: usize,
    up: bool,
    size: f64,
    small: f64,
) -> Category<'a> {
    let root = &nodes[node];
    let label = layout_label(Some(&root.text), false, size, 180.0, 0);
    let line_height = js_round(small * 1.5);
    let mut bones: Vec<Bone> = Vec::new();
    let mut rows: Vec<usize> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let push = |stack: &mut Vec<Frame>, children: &[usize], depth: usize, parent: isize| {
        let n = children.len();
        for slot in (0..n).rev() {
            let index = if up { n - 1 - slot } else { slot };
            stack.push(Frame {
                node: children[index],
                depth,
                parent,
                slot,
                bone: -1,
            });
        }
    };
    push(&mut stack, &root.children, 2, -1);
    while let Some(mut frame) = stack.pop() {
        if frame.bone != -1 {
            rows.push(frame.bone as usize);
            continue;
        }
        let index = bones.len();
        let frame_node = &nodes[frame.node];
        bones.push(Bone {
            label: layout_label(Some(&frame_node.text), false, small, 200.0, 0),
            depth: frame.depth,
            parent: frame.parent,
            slot: frame.slot,
            kids: frame_node.children.len(),
            rise: 0.0,
            at: 0.0,
            text_end: 0.0,
        });
        let depth = frame.depth;
        if depth % 2 == 0 {
            rows.push(index);
        } else {
            frame.bone = index as isize;
            stack.push(frame);
        }
        push(&mut stack, &frame_node.children, depth + 1, index as isize);
    }

    let mut rise = FIRST_ROW;
    for &index in &rows {
        let height = js_max(bones[index].label.height, line_height) + ROW_GAP;
        bones[index].rise = rise + height / 2.0;
        rise += height;
    }
    let box_w = label.width + 2.0 * BOX_PAD_X;
    let mut reach = box_w / 2.0;
    for k in 0..bones.len() {
        let parent = usize::try_from(bones[k].parent).ok();
        if bones[k].depth % 2 == 0 {
            let at = parent.map_or(0.0, |p| bones[p].at);
            let kids = bones[k].kids;
            bones[k].at = at;
            bones[k].text_end =
                at - if kids > 0 {
                    (kids + 1) as f64 * TWIG_STEP
                } else {
                    STUB
                } - TEXT_GAP;
        } else {
            let p = parent.expect("a twig has a level bone");
            let at = bones[p].at - (bones[p].kids as f64 - bones[k].slot as f64) * TWIG_STEP;
            bones[k].at = at;
            bones[k].text_end = at - TEXT_GAP;
        }
        reach = js_max(reach, bones[k].label.width - bones[k].text_end);
    }
    Category {
        node: root,
        box_h: label.height + 2.0 * BOX_PAD_Y,
        label,
        box_w,
        bones,
        rise: rise + BONE_END,
        reach,
    }
}

pub fn render_ishikawa(model: &IshikawaDb, config: &Config, options: &RenderOptions) -> Drawn {
    turn_to_fit(options, |down| draw(model, config, options, down))
}

fn draw(model: &IshikawaDb, _config: &Config, options: &RenderOptions, down: bool) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);
    if down {
        return vertical_causes(model, options);
    }
    let title = layout_label(model.title.as_deref(), false, size, 4000.0, BOLD_STYLE);
    let title_height = if title.height > 0.0 {
        title.height + 12.0
    } else {
        0.0
    };

    let mut body = String::new();
    let mut min_x = 0.0f64;
    let mut max_x = 0.0f64;
    let mut min_y = 0.0f64;
    let mut max_y = 0.0f64;
    if let Some(root) = model.root {
        let root_node = &model.nodes[root];
        let head = layout_label(Some(&root_node.text), false, size, 200.0, BOLD_STYLE);
        let head_h = js_max(head.height + 24.0, 48.0);
        let head_w = head.width + 24.0 + head_h / 2.0;
        let mut categories: Vec<Category> = root_node
            .children
            .iter()
            .enumerate()
            .map(|(i, &child)| arrange(&model.nodes, child, i % 2 == 0, size, small))
            .collect();
        for category in &mut categories {
            category.rise = js_max(category.rise, head_h / 2.0 + 14.0);
        }

        let mut xs: Vec<f64> = Vec::new();
        let mut x = -HEAD_GAP;
        let mut i = 0;
        while i < categories.len() {
            if i > 0 {
                let mut room = 0.0f64;
                for side in 0..2 {
                    let before = categories.get(i + side - 2);
                    let next = categories.get(i + side);
                    if let Some(before) = before {
                        room = js_max(
                            room,
                            before.reach + next.map_or(0.0, |next| next.box_w / 2.0),
                        );
                    }
                }
                x -= room + PAIR_GAP;
            }
            xs.push(x);
            xs.push(x);
            i += 2;
        }
        let tail = if categories.is_empty() {
            -HEAD_GAP
        } else {
            x - TAIL
        };

        max_x = head_w;
        min_x = tail;
        min_y = -head_h / 2.0;
        max_y = head_h / 2.0;
        let mut groups = String::new();
        for (i, category) in categories.iter_mut().enumerate() {
            let sign: f64 = if i % 2 == 0 { -1.0 } else { 1.0 };
            let ax = xs[i];
            let px = |at: f64, rise: f64| ax + at - rise * SLANT;
            let end_x = px(0.0, category.rise);
            let end_y = sign * category.rise;
            let trim = ARROW - 1.0;
            let mut d = format!(
                "M{},{}L{},{}",
                num(ax - (SLANT / js_hypot(SLANT, 1.0)) * trim),
                num((sign * trim) / js_hypot(SLANT, 1.0)),
                num(end_x),
                num(end_y)
            );
            let mut texts = String::new();
            for bone in &category.bones {
                let y = sign * bone.rise;
                if bone.depth % 2 == 0 {
                    d.push_str(&format!(
                        "M{},{}H{}",
                        num(px(bone.at, bone.rise)),
                        num(y),
                        num(px(bone.text_end + TEXT_GAP, bone.rise))
                    ));
                } else {
                    let from = category.bones[bone.parent as usize].rise;
                    d.push_str(&format!(
                        "M{},{}L{},{}",
                        num(px(bone.at, from)),
                        num(sign * from),
                        num(px(bone.at, bone.rise)),
                        num(y)
                    ));
                }
                let right = px(bone.text_end, bone.rise);
                texts.push_str(&label_svg(
                    &bone.label,
                    right - bone.label.width / 2.0,
                    y,
                    if bone.depth > 2 {
                        " fill=\"var(--_m)\""
                    } else {
                        ""
                    },
                ));
                min_x = js_min(min_x, right - bone.label.width);
            }
            let box_y = end_y + sign * (category.box_h / 2.0);
            min_x = js_min(min_x, end_x - category.box_w / 2.0);
            max_x = js_max(max_x, end_x + category.box_w / 2.0);
            min_y = js_min(min_y, box_y - category.box_h / 2.0);
            max_y = js_max(max_y, box_y + category.box_h / 2.0);
            groups.push_str(&format!(
                "<g class=\"pelers-ishikawa-category\" data-id=\"{}\"><path class=\"pelers-ishikawa-bones\" d=\"{d}\" fill=\"none\" stroke=\"var(--_l)\"/>{}<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{ROUNDED}/>{}{}</g>",
                esc_text(&category.node.text),
                marker("arrow_point", ax, 0.0, SLANT / js_hypot(SLANT, 1.0), -sign / js_hypot(SLANT, 1.0), "var(--_l)", "none"),
                num(end_x - category.box_w / 2.0),
                num(box_y - category.box_h / 2.0),
                num(category.box_w),
                num(category.box_h),
                label_svg(&category.label, end_x, box_y, " class=\"pelers-label\""),
                if texts.is_empty() {
                    String::new()
                } else {
                    format!("<g class=\"pelers-ishikawa-causes\" font-size=\"{}\">{texts}</g>", crate::util::jsnum::js_number_to_string(small))
                }
            ));
        }

        let r = head_h / 2.0;
        body = format!(
            "<path class=\"pelers-ishikawa-spine\" d=\"M{},0H{}\" fill=\"none\" stroke=\"var(--_l)\"/>{}{groups}<g class=\"pelers-ishikawa-head\" data-id=\"{}\"><path d=\"M0,{}H{}A{},{} 0 0 1 {},{}H0Z\" fill=\"var(--_s)\" stroke=\"var(--_b)\" stroke-linejoin=\"round\"/>{}</g>",
            num(tail),
            num(-(ARROW - 1.0)),
            marker("arrow_point", 0.0, 0.0, 1.0, 0.0, "var(--_l)", "none"),
            esc_text(&root_node.text),
            num(-r),
            num(head_w - r),
            num(r),
            num(r),
            num(head_w - r),
            num(r),
            label_svg(
                &head,
                12.0 + head.width / 2.0,
                0.0,
                " class=\"pelers-label\" font-weight=\"var(--_hw)\""
            ),
        );
    }

    let inner = js_max(max_x - min_x, title.width);
    let width = (inner + 2.0 * pad).ceil();
    let height = (max_y - min_y + title_height + 2.0 * pad).ceil();
    let ox = pad + (inner - (max_x - min_x)) / 2.0 - min_x;
    let oy = pad + title_height - min_y;
    let content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &if body.is_empty() {
        String::new()
    } else {
        format!(
            "<g transform=\"translate({},{})\">{body}</g>",
            num(ox),
            num(oy)
        )
    };
    Drawn {
        svg: svg_document(
            "ishikawa",
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

struct Entry {
    node: usize,
    depth: usize,
    parent: isize,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    label: Label,
}

fn vertical_causes(model: &IshikawaDb, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let pad = options.padding.unwrap_or(8.0);
    let room = js_max(140.0, options.max_width.unwrap_or(320.0) - 2.0 * pad);
    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        title_room(options),
        BOLD_STYLE,
    );
    let mut order: Vec<Entry> = Vec::new();
    let mut pending: Vec<(usize, usize, isize)> =
        model.root.map(|r| (r, 0, -1)).into_iter().collect();
    let mut depth = 0usize;
    while let Some((node, level, parent)) = pending.pop() {
        let index = order.len();
        order.push(Entry {
            node,
            depth: level,
            parent,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
            label: Label {
                lines: Vec::new(),
                widths: Vec::new(),
                width: 0.0,
                height: 0.0,
                size: 0.0,
                line_height: 0.0,
                heights: None,
            },
        });
        depth = depth.max(level);
        for &child in model.nodes[node].children.iter().rev() {
            pending.push((child, level + 1, index as isize));
        }
    }
    let indent = js_min(
        20.0,
        js_max(2.0, (room - 120.0) / js_max(1.0, depth as f64)),
    );
    let mut y = pad
        + if title.height != 0.0 {
            title.height + 12.0
        } else {
            0.0
        };
    let mut width = title.width;
    let mut nodes = String::new();
    let mut edges = String::new();
    for k in 0..order.len() {
        let (entry_depth, entry_node, entry_parent) = {
            let e = &order[k];
            (e.depth, e.node, e.parent)
        };
        let x = pad + entry_depth as f64 * indent;
        let label_room = js_max(60.0, room - entry_depth as f64 * indent - 20.0);
        let text = &model.nodes[entry_node].text;
        let raw = layout_label(
            Some(text),
            false,
            if entry_depth < 2 { size } else { small },
            label_room,
            if entry_depth < 2 { BOLD_STYLE } else { 0 },
        );
        let label = break_label(&raw, label_room);
        let w = label.width + 20.0;
        let h = label.height + 12.0;
        let entry_y = y;
        y += h + 12.0;
        width = js_max(width, x - pad + w);
        let framed = entry_depth < 2;
        let class = match entry_depth {
            0 => "pelers-ishikawa-head",
            1 => "pelers-ishikawa-category",
            _ => "pelers-ishikawa-cause",
        };
        nodes.push_str(&format!(
            "<g class=\"pelers-node {class}\" data-id=\"{}\">{}{}</g>",
            esc_text(text),
            if framed {
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"/>",
                    num(x),
                    num(entry_y),
                    num(w),
                    num(h),
                    if entry_depth == 0 { 12 } else { 4 }
                )
            } else {
                String::new()
            },
            label_svg(
                &label,
                x + w / 2.0,
                entry_y + h / 2.0,
                &format!(
                    " class=\"pelers-label\"{}{}",
                    if entry_depth < 2 { " font-weight=\"var(--_hw)\"" } else { "" },
                    if entry_depth > 1 {
                        format!(" font-size=\"{}\"", crate::util::jsnum::js_number_to_string(small))
                    } else {
                        String::new()
                    }
                ),
            )
        ));
        if entry_parent >= 0 {
            let parent = &order[entry_parent as usize];
            let gx = parent.x + indent / 2.0;
            edges.push_str(&format!(
                "<path class=\"pelers-edge\" d=\"M{},{}V{}H{}\" fill=\"none\" stroke=\"var(--_b)\"/>",
                num(gx),
                num(parent.y + parent.h),
                num(entry_y + h / 2.0),
                num(x)
            ));
        }
        let entry = &mut order[k];
        entry.x = x;
        entry.y = entry_y;
        entry.w = w;
        entry.h = h;
        entry.label = label;
    }
    let total_width = (width + 2.0 * pad).ceil();
    let height = (js_max(pad + title.height, y - 12.0) + pad).ceil();
    let content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &edges
        + &nodes;
    Drawn {
        svg: svg_document(
            "ishikawa",
            total_width,
            height,
            size,
            options,
            Accessible::default(),
            &content,
        ),
        width: total_width,
        height,
        links: Vec::new(),
    }
}
