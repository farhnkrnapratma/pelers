use super::{PacketBlock, PacketModel, packet_option, packet_section};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_text::fit_text;
use crate::diagrams::common::fit_width::{title_room, turn_to_fit};
use crate::diagrams::common::literal_label::literal_label;
use crate::preprocess::Config;
use crate::svg::builder::{esc, label_svg, num};
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::ROUNDED;
use crate::text::entities::decode_entities;
use crate::text::label::{Label, layout_label};
use crate::text::measurer::BOLD_STYLE;
use crate::util::chars::trim;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::Value;

pub fn render_packet(model: &PacketModel, config: &Config, options: &RenderOptions) -> Drawn {
    turn_to_fit(options, |down| draw(model, config, options, down))
}

fn is_piece(block: &PacketBlock) -> bool {
    block.bits == block.end - block.start
}

fn reflow(model: &PacketModel, bits_per_row: f64) -> Vec<Vec<PacketBlock>> {
    let mut whole: Vec<PacketBlock> = Vec::new();
    for row in &model.rows {
        for block in row {
            if let Some(prev) = whole.last_mut()
                && is_piece(prev)
                && is_piece(block)
                && prev.label == block.label
                && block.start == prev.end + 1.0
                && block.start % model.bits_per_row == 0.0
            {
                prev.end = block.end;
                prev.bits = prev.end - prev.start;
            } else {
                whole.push(block.clone());
            }
        }
    }
    let mut rows: Vec<Vec<PacketBlock>> = Vec::new();
    for block in &whole {
        let mut start = block.start;
        while start <= block.end {
            let end = js_min(
                block.end,
                ((start / bits_per_row).floor() + 1.0) * bits_per_row - 1.0,
            );
            let index = (start / bits_per_row).floor() as usize;
            while rows.len() <= index {
                rows.push(Vec::new());
            }
            rows[index].push(PacketBlock {
                start,
                end,
                ..block.clone()
            });
            start = end + 1.0;
        }
    }
    rows
}

fn draw(model: &PacketModel, config: &Config, options: &RenderOptions, down: bool) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let label_size = js_round(size * 0.875);
    let bit_size = js_round(size * 0.625);
    let pad = options.padding.unwrap_or(8.0);

    let row_height = packet_option(config, "rowHeight", 32.0, 1.0, 400.0);
    let mut bit_width = packet_option(config, "bitWidth", 32.0, 1.0, 400.0);
    let padding_x = packet_option(config, "paddingX", 5.0, 0.0, 400.0);
    let padding_y = packet_option(config, "paddingY", 5.0, 0.0, 400.0);
    let section = packet_section(config);
    let show_bits = section.and_then(|s| s.get("showBits")) != Some(&Value::Bool(false));
    let descending = matches!(
        section.and_then(|s| s.get("bitOrder")),
        Some(Value::String(order)) if order == "descending"
    );
    let mut bits_per_row = model.bits_per_row;
    let mut reflowed: Option<Vec<Vec<PacketBlock>>> = None;
    if down {
        let available = options.max_width.unwrap_or(320.0);
        let count = js_max(
            1.0,
            ((available - 2.0 * pad) / js_min(bit_width, 18.0)).floor(),
        );
        bits_per_row = js_min(bits_per_row, js_max(8.0, 2f64.powf(count.log2().floor())));
        let last = model
            .rows
            .last()
            .and_then(|row| row.last())
            .map_or(0.0, |block| block.end);
        if ((last + 1.0) / bits_per_row).ceil() > 10000.0 {
            bits_per_row = model.bits_per_row;
        }
        bit_width = js_min(
            bit_width,
            js_max(18.0, (available - 2.0 * pad) / bits_per_row),
        );
        reflowed = Some(reflow(model, bits_per_row));
    }
    let rows: &Vec<Vec<PacketBlock>> = reflowed.as_ref().unwrap_or(&model.rows);

    let mut labels: Vec<Vec<Label>> = Vec::new();
    if down {
        for row in rows {
            labels.push(
                row.iter()
                    .map(|block| {
                        let room = js_max(
                            1.0,
                            (block.end - block.start + 1.0) * bit_width
                                - js_min(padding_x, bit_width - 1.0)
                                - 6.0,
                        );
                        literal_label(trim(&decode_entities(&block.label)), label_size, room, 0)
                    })
                    .collect(),
            );
        }
    }
    let numbers = if show_bits { bit_size + 4.0 } else { 0.0 };
    let row_heights: Vec<f64> = (0..rows.len())
        .map(|index| {
            let mut height = row_height;
            if down {
                for label in &labels[index] {
                    height = js_max(height, label.height + 10.0);
                }
            }
            height
        })
        .collect();
    let mut tops: Vec<f64> = Vec::new();
    let mut rows_height = 0.0;
    for height in &row_heights {
        tops.push(rows_height);
        rows_height += numbers + height + padding_y;
    }
    if !rows.is_empty() {
        rows_height -= padding_y;
    }

    let gap = js_min(padding_x, bit_width - 1.0);
    let width = if rows.is_empty() {
        0.0
    } else {
        bits_per_row * bit_width - gap
    };
    let title = layout_label(
        model.title.as_deref(),
        false,
        size,
        if down { title_room(options) } else { 4000.0 },
        BOLD_STYLE,
    );
    let title_height = if title.height > 0.0 {
        title.height + if rows_height > 0.0 { 12.0 } else { 0.0 }
    } else {
        0.0
    };

    let left = pad + js_max(0.0, (title.width - width) / 2.0);
    let mut fields = String::new();
    let mut bits = String::new();
    for (index, row) in rows.iter().enumerate() {
        let y = pad + tops[index] + numbers;
        let height = row_heights[index];
        for (position, block) in row.iter().enumerate() {
            let count = block.end - block.start + 1.0;
            let first = block.start % bits_per_row;
            let x = left
                + (if descending {
                    bits_per_row - first - count
                } else {
                    first
                }) * bit_width;
            let w = count * bit_width - gap;
            fields.push_str(&format!(
                "<g class=\"pele-node\" data-id=\"{}-{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_s)\" stroke=\"var(--_b)\"{ROUNDED}/>",
                js_number_to_string(block.start),
                js_number_to_string(block.end),
                num(x),
                num(y),
                num(w),
                num(height)
            ));
            if down {
                fields.push_str(&label_svg(
                    &labels[index][position],
                    x + w / 2.0,
                    y + height / 2.0,
                    "",
                ));
            } else if let Some(text) = fit_text(
                trim(&decode_entities(&block.label)),
                w - 6.0,
                label_size,
                js_round(size * 0.5),
                0,
            ) && text.size <= height
            {
                let font = if text.size == label_size {
                    String::new()
                } else {
                    format!(" font-size=\"{}\"", num(text.size))
                };
                fields.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\"{font}>{}</text>",
                    num(x + w / 2.0),
                    num(y + height / 2.0 + text.size * 0.35),
                    esc(&text.text)
                ));
            }
            fields.push_str("</g>");
            if !show_bits {
                continue;
            }
            let (leading, trailing) = if descending {
                (block.end, block.start)
            } else {
                (block.start, block.end)
            };
            if count == 1.0 {
                bits.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                    num(x + w / 2.0),
                    num(y - 4.0),
                    js_number_to_string(leading)
                ));
            } else {
                bits.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\">{}</text><text x=\"{}\" y=\"{}\" text-anchor=\"end\">{}</text>",
                    num(x),
                    num(y - 4.0),
                    js_number_to_string(leading),
                    num(x + w),
                    num(y - 4.0),
                    js_number_to_string(trailing)
                ));
            }
        }
    }

    let total_width = (js_max(width, title.width) + 2.0 * pad).ceil();
    let total_height = (rows_height + title_height + 2.0 * pad).ceil();
    let mut content = String::new();
    if !fields.is_empty() {
        content.push_str(&format!(
            "<g class=\"pele-packet-fields\" font-size=\"{}\" text-anchor=\"middle\">{fields}</g>",
            js_number_to_string(label_size)
        ));
    }
    if !bits.is_empty() {
        content.push_str(&format!(
            "<g class=\"pele-packet-bits\" font-size=\"{}\" fill=\"var(--_m)\">{bits}</g>",
            js_number_to_string(bit_size)
        ));
    }
    content.push_str(&label_svg(
        &title,
        total_width / 2.0,
        total_height - pad - title.height / 2.0,
        " class=\"pele-title\" font-weight=\"var(--_tw)\"",
    ));

    let acc = Accessible {
        acc_title: model.acc_title.as_deref(),
        acc_descr: model.acc_descr.as_deref(),
    };
    Drawn {
        svg: svg_document(
            "packet",
            total_width,
            total_height,
            size,
            options,
            acc,
            &content,
        ),
        width: total_width,
        height: total_height,
    }
}
