use super::db::{KanbanDb, KanbanNode, KanbanSection};
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::title_room;
use crate::diagrams::common::icon::icon_svg;
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, label_svg_with, num, text_style};
use crate::svg::options::LinkInfo;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{CRITICAL, ROUNDED, class_names};
use crate::svg::title::title_label;
use crate::text::icons::icon_markup;
use crate::text::label::{Label, Span, label_line, layout_label, line_middle, with_heights};
use crate::text::measurer::{BOLD_STYLE, SUB, SUP, UNDERLINE, width};
use crate::util::chars::trim;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::url::{link_url, rel_attr, safe_url};
use crate::util::value::Value;

const COLUMN_GAP: f64 = 12.0;
const COLUMN_PAD: f64 = 8.0;
const CARD_PAD: f64 = 10.0;
const CARD_GAP: f64 = 8.0;
const META_GAP: f64 = 4.0;
const ICON_GAP: f64 = 6.0;

fn priority_stroke(priority: &str) -> Option<String> {
    match priority {
        "very high" => Some(format!("stroke=\"{CRITICAL}\"")),
        "high" => Some(format!("stroke=\"{CRITICAL}\" opacity=\"0.5\"")),
        "low" => Some("stroke=\"var(--_m)\" opacity=\"0.5\"".to_owned()),
        "very low" => Some("stroke=\"var(--_m)\"".to_owned()),
        _ => None,
    }
}

fn fit_label(raw: Option<&str>, markdown: bool, size: f64, max: f64, base: u32) -> Label {
    let label = layout_label(raw, markdown, size, max, base);
    if label.width <= max {
        return label;
    }
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut widths: Vec<f64> = Vec::new();
    let mut widest = 0.0;
    let mut line: Vec<Span> = Vec::new();
    let mut at = 0.0;
    let commit = |lines: &mut Vec<Vec<Span>>,
                  widths: &mut Vec<f64>,
                  widest: &mut f64,
                  line: &mut Vec<Span>,
                  at: &mut f64| {
        lines.push(std::mem::take(line));
        widths.push(*at);
        *widest = js_max(*widest, *at);
        *at = 0.0;
    };
    for i in 0..label.lines.len() {
        if label.widths[i] <= max {
            line = label.lines[i].clone();
            at = label.widths[i];
            commit(&mut lines, &mut widths, &mut widest, &mut line, &mut at);
            continue;
        }
        for span in &label.lines[i] {
            if span.icon.is_some() || span.math.is_some() {
                if at > 0.0 && at + span.width > max {
                    commit(&mut lines, &mut widths, &mut widest, &mut line, &mut at);
                }
                line.push(span.clone());
                at += span.width;
                continue;
            }
            let mut text = String::new();
            let mut from = at;
            for ch in span.text.chars() {
                let w = width(&ch.to_string(), size, span.style);
                if at > 0.0 && at + w > max {
                    if !text.is_empty() {
                        line.push(Span::text(text.clone(), span.style, at - from));
                    }
                    commit(&mut lines, &mut widths, &mut widest, &mut line, &mut at);
                    text.clear();
                    from = 0.0;
                }
                text.push(ch);
                at += w;
            }
            if !text.is_empty() {
                line.push(Span::text(text, span.style, at - from));
            }
        }
        commit(&mut lines, &mut widths, &mut widest, &mut line, &mut at);
    }
    let fitted = Label {
        height: lines.len() as f64 * label.line_height,
        lines,
        widths,
        width: widest,
        size: label.size,
        line_height: label.line_height,
        heights: label.heights.clone(),
    };
    if label.heights.is_some() {
        with_heights(fitted)
    } else {
        fitted
    }
}

fn is_plain(span: &Span) -> bool {
    span.icon.is_none() && span.math.is_none() && span.style & (SUB | SUP | UNDERLINE) == 0
}

fn text_block(label: &Label, x: f64, y: f64, attrs: &str, icons: bool, end: bool) -> String {
    let mut out = String::new();
    for i in 0..label.lines.len() {
        let line = &label.lines[i];
        let cy = y + line_middle(label, i);
        if line.len() == 1 && is_plain(&line[0]) {
            out.push_str(&format!(
                "<text{} x=\"{}\" y=\"{}\"{}>{}</text>",
                text_style(attrs, line[0].style),
                num(x),
                num(cy + label.size * 0.35),
                if end { " text-anchor=\"end\"" } else { "" },
                esc(&line[0].text)
            ));
        } else {
            let w = label.widths[i];
            out.push_str(&label_svg_with(
                &label_line(label, i),
                x + (if end { -w } else { w }) / 2.0,
                cy,
                attrs,
                icons,
            ));
        }
    }
    out
}

struct Card<'a> {
    item: &'a KanbanNode,
    label: Label,
    ticket: Label,
    assigned: Label,
    meta: f64,
    height: f64,
}

fn has(name: Option<&str>) -> bool {
    name.is_some_and(|n| !n.is_empty() && !icon_markup(n).is_empty())
}

fn holds(raw: Option<&str>, font: f64, reserved: f64, math_width: &mut f64) {
    let Some(raw) = raw.filter(|r| r.contains("$$")) else {
        return;
    };
    let label = layout_label(Some(raw), true, font, f64::INFINITY, 0);
    for spans in &label.lines {
        for span in spans {
            if let Some(math) = &span.math {
                *math_width = js_max(*math_width, math.width() + reserved);
            }
        }
    }
}

pub fn render_kanban(model: &KanbanDb, config: &Config, options: &RenderOptions) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.875);
    let line = js_round(size * 1.5);
    let small_line = js_round(small * 1.5);
    let pad = options.padding.unwrap_or(8.0);
    let icons = options.icons.is_some();
    let conf = config.get("kanban").and_then(Value::as_object);
    let section_width = match conf.and_then(|c| c.get("sectionWidth")) {
        Some(Value::Number(n)) => Some(*n),
        _ => None,
    };
    let glyph = small * 1.25;
    let mut math_width = 0.0;
    for section in &model.sections {
        let count = width(&section.items.len().to_string(), small, 0);
        holds(
            Some(&section.node.label),
            size,
            2.0 * COLUMN_PAD
                + 8.0
                + count
                + 12.0
                + if has(section.node.icon.as_deref()) {
                    glyph + ICON_GAP
                } else {
                    0.0
                },
            &mut math_width,
        );
        for item in &section.items {
            let reserved = 2.0 * (COLUMN_PAD + CARD_PAD);
            holds(
                Some(&item.label),
                small,
                reserved
                    + if has(item.icon.as_deref()) {
                        glyph + ICON_GAP
                    } else {
                        0.0
                    },
                &mut math_width,
            );
            holds(item.ticket.as_deref(), small, reserved, &mut math_width);
            holds(item.assigned.as_deref(), small, reserved, &mut math_width);
        }
    }
    let own_width = js_max(
        match section_width {
            Some(w) if w > 0.0 => js_min(js_max(w, 80.0), 2000.0),
            _ => 200.0,
        },
        math_width.ceil(),
    );
    let room = match options.max_width {
        Some(max) if max > 0.0 => max - 2.0 * pad,
        _ => f64::INFINITY,
    };
    let count = model.sections.len() as f64;
    let fits = count * (own_width + COLUMN_GAP) - COLUMN_GAP <= room;
    let per_row = if fits {
        js_max(count, 1.0)
    } else {
        js_max(
            1.0,
            ((room + COLUMN_GAP) / (own_width + COLUMN_GAP)).floor(),
        )
    };
    let column_width = if fits {
        own_width
    } else {
        js_max(
            js_max(
                js_min(own_width, room),
                js_min(
                    ((room + COLUMN_GAP) / per_row).floor() - COLUMN_GAP,
                    own_width * 2.0,
                ),
            ),
            math_width.ceil(),
        )
    };
    let base_url = match conf.and_then(|c| c.get("ticketBaseUrl")) {
        Some(Value::String(url)) => url.as_str(),
        _ => "",
    };
    let card_width = column_width - 2.0 * COLUMN_PAD;
    let text_width = card_width - 2.0 * CARD_PAD;
    let mut links: Vec<LinkInfo> = Vec::new();

    let sections: &[KanbanSection] = &model.sections;
    let mut titles: Vec<Label> = Vec::new();
    let mut count_widths: Vec<f64> = Vec::new();
    let mut cards: Vec<Vec<Card>> = Vec::new();
    let mut head_height = line;
    let mut body_heights: Vec<f64> = Vec::new();
    for section in sections {
        let count_width = width(&section.items.len().to_string(), small, 0);
        let reserved = count_width
            + 12.0
            + if has(section.node.icon.as_deref()) {
                glyph + ICON_GAP
            } else {
                0.0
            };
        let title = fit_label(
            Some(&section.node.label),
            true,
            size,
            card_width - 8.0 - reserved,
            BOLD_STYLE,
        );
        head_height = js_max(head_height, title.height);
        count_widths.push(count_width);
        titles.push(title);

        let mut list: Vec<Card> = Vec::new();
        let mut height = 0.0;
        for item in &section.items {
            let label = fit_label(
                Some(&item.label),
                true,
                small,
                text_width
                    - if has(item.icon.as_deref()) {
                        glyph + ICON_GAP
                    } else {
                        0.0
                    },
                0,
            );
            let ticket = fit_label(item.ticket.as_deref(), false, small, text_width, 0);
            let assigned = fit_label(item.assigned.as_deref(), false, small, text_width, 0);
            let meta = if ticket.width + assigned.width + 8.0 > text_width {
                ticket.height + assigned.height
            } else {
                js_max(ticket.height, assigned.height)
            };
            let card_height = 2.0 * CARD_PAD
                + js_max(label.height, small_line)
                + if meta != 0.0 { META_GAP + meta } else { 0.0 };
            height += card_height + CARD_GAP;
            list.push(Card {
                item,
                label,
                ticket,
                assigned,
                meta,
                height: card_height,
            });
        }
        cards.push(list);
        body_heights.push(height);
    }
    head_height += 2.0 * COLUMN_PAD;
    let mut row_heights: Vec<f64> = Vec::new();
    let mut row_tops: Vec<f64> = Vec::new();
    let mut y = 0.0;
    let mut first = 0;
    while first < sections.len() {
        let mut tallest = CARD_GAP;
        for &body in
            &body_heights[first..js_min(first as f64 + per_row, sections.len() as f64) as usize]
        {
            tallest = js_max(tallest, body);
        }
        row_tops.push(y);
        row_heights.push(head_height + tallest);
        y += head_height + tallest + COLUMN_GAP;
        first += per_row as usize;
    }
    let board_height = match (row_tops.last(), row_heights.last()) {
        (Some(top), Some(height)) => top + height,
        _ => 0.0,
    };

    let title = title_label(model.title.as_deref(), size, title_room(options));
    let title_height = if title.height > 0.0 {
        title.height + 16.0
    } else {
        0.0
    };
    let board_width = js_max(
        js_min(sections.len() as f64, per_row) * (column_width + COLUMN_GAP) - COLUMN_GAP,
        0.0,
    );
    let width_total = js_max(board_width, title.width);
    let left = pad + (width_total - board_width) / 2.0;
    let edge = column_width - COLUMN_PAD - 4.0;

    let mut out = String::new();
    for (index, section) in sections.iter().enumerate() {
        let row = (index as f64 / per_row).floor() as usize;
        let mut body = format!(
            "<rect width=\"{}\" height=\"{}\" fill=\"var(--_s)\"{ROUNDED}/>",
            js_number_to_string(column_width),
            num(row_heights[row])
        );
        body.push_str(&text_block(
            &titles[index],
            COLUMN_PAD + 4.0,
            COLUMN_PAD,
            " class=\"pelers-cluster-label\" font-weight=\"var(--_hw)\"",
            icons,
            false,
        ));
        body.push_str(&format!(
            "<text class=\"pelers-count\" x=\"{}\" y=\"{}\" text-anchor=\"end\" font-size=\"{}\" fill=\"var(--_m)\">{}</text>",
            js_number_to_string(edge),
            num(COLUMN_PAD + line / 2.0 + small * 0.35),
            js_number_to_string(small),
            section.items.len()
        ));
        if has(section.node.icon.as_deref()) {
            body.push_str(&icon_svg(
                section.node.icon.as_deref().unwrap_or(""),
                edge - count_widths[index] - ICON_GAP - glyph,
                COLUMN_PAD + (line - glyph) / 2.0,
                glyph,
            ));
        }

        let mut y = head_height;
        for card in &cards[index] {
            let item = card.item;
            let priority = trim(item.priority.as_deref().unwrap_or("")).to_lowercase();
            let color = priority_stroke(&priority);
            let mut inner = format!(
                "<rect width=\"{}\" height=\"{}\" fill=\"var(--_bg)\" stroke=\"var(--_b)\"{ROUNDED}/>",
                js_number_to_string(card_width),
                num(card.height)
            );
            if let Some(color) = &color {
                inner.push_str(&format!(
                    "<path class=\"pelers-priority\" d=\"M2.5,5V{}\" fill=\"none\" {color} stroke-width=\"3\" stroke-linecap=\"round\"/>",
                    num(card.height - 5.0)
                ));
            }
            inner.push_str(&text_block(
                &card.label,
                CARD_PAD,
                CARD_PAD,
                " class=\"pelers-label\"",
                icons,
                false,
            ));
            if has(item.icon.as_deref()) {
                inner.push_str(&icon_svg(
                    item.icon.as_deref().unwrap_or(""),
                    card_width - CARD_PAD - glyph,
                    CARD_PAD + (small_line - glyph) / 2.0,
                    glyph,
                ));
            }
            let bottom = card.height - CARD_PAD;
            if let Some(ticket) = item.ticket.as_deref().filter(|t| !t.is_empty()) {
                let top = bottom - card.meta;
                let href = if base_url.is_empty() {
                    None
                } else {
                    link_url(
                        &safe_url(&base_url.replacen("#TICKET#", ticket, 1)),
                        options,
                    )
                };
                match href {
                    None => inner.push_str(&text_block(
                        &card.ticket,
                        CARD_PAD,
                        top,
                        " class=\"pelers-ticket\" fill=\"var(--_m)\"",
                        icons,
                        false,
                    )),
                    Some(href) => {
                        inner.push_str(&format!(
                            "<a href=\"{}\" target=\"_blank\"{}>",
                            esc(&href),
                            rel_attr(options)
                        ));
                        inner.push_str(&text_block(
                            &card.ticket,
                            CARD_PAD,
                            top,
                            " class=\"pelers-ticket\" fill=\"var(--_c)\"",
                            icons,
                            false,
                        ));
                        inner.push_str("</a>");
                        links.push(LinkInfo {
                            id: item.id.clone(),
                            href,
                            internal: false,
                        });
                    }
                }
            }
            if item.assigned.as_deref().is_some_and(|a| !a.is_empty()) {
                inner.push_str(&text_block(
                    &card.assigned,
                    card_width - CARD_PAD,
                    bottom - card.assigned.height,
                    " class=\"pelers-assigned\" fill=\"var(--_m)\"",
                    icons,
                    true,
                ));
            }
            let priority_class = match color {
                Some(_) => format!(" pelers-priority-{}", priority.replacen(' ', "-", 1)),
                None => String::new(),
            };
            body.push_str(&format!(
                "<g class=\"pelers-node pelers-card{priority_class}{}\" data-id=\"{}\" transform=\"translate({},{})\" font-size=\"{}\">{inner}</g>",
                class_names(item.css_classes.as_deref().unwrap_or("")),
                esc_text(&item.id),
                js_number_to_string(COLUMN_PAD),
                num(y),
                js_number_to_string(small)
            ));
            y += card.height + CARD_GAP;
        }

        out.push_str(&format!(
            "<g class=\"pelers-cluster pelers-column{}\" data-id=\"{}\" transform=\"translate({},{})\">{body}</g>",
            class_names(section.node.css_classes.as_deref().unwrap_or("")),
            esc_text(&section.node.id),
            num(left + (index as f64 % per_row) * (column_width + COLUMN_GAP)),
            num(pad + title_height + row_tops[row])
        ));
    }

    let total_width = (width_total + 2.0 * pad).ceil();
    let total_height = (board_height + title_height + 2.0 * pad).ceil();
    let content = label_svg(
        &title,
        total_width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    ) + &out;
    Drawn {
        svg: svg_document(
            "kanban",
            total_width,
            total_height,
            size,
            options,
            Accessible::default(),
            &content,
        ),
        width: total_width,
        height: total_height,
        links,
    }
}
