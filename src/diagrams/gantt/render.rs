use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use super::axis::{auto_ticks, interval_ticks, time_format};
use super::dates::add;
use super::db::{GanttDb, Task};
use super::jsdate::JsDate;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit_width::cap_width;
use crate::errors::PelersError;
use crate::preprocess::Config;
use crate::svg::builder::{esc, esc_text, label_svg, label_svg_with, num};
use crate::svg::options::LinkInfo;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{CRITICAL, ROUNDED, resolve_style};
use crate::text::label::{Label, layout_label};
use crate::text::measurer::{BOLD_STYLE, width as measure};
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::url::linked;
use crate::util::value::Value;

const WIDTH: f64 = 800.0;
const MIN_PLOT: f64 = 240.0;
const LABEL_GAP: f64 = 8.0;
const WS: &str = "\\t\\n\\x0b\\x0c\\r \\x{a0}\\x{1680}\\x{2000}-\\x{200a}\\x{2028}\\x{2029}\\x{202f}\\x{205f}\\x{3000}\\x{feff}";

fn break_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!(r"(?i)<br[{WS}]*/?>")).expect("pattern"))
}

fn setting(config: &Config, key: &str, min: f64, max: f64) -> Option<f64> {
    match config
        .get("gantt")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::Number(value)) if value.is_finite() => Some(js_min(max, js_max(min, *value))),
        _ => None,
    }
}

fn text_setting(config: &Config, key: &str) -> Option<String> {
    match config
        .get("gantt")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn flag(config: &Config, key: &str) -> bool {
    matches!(
        config
            .get("gantt")
            .and_then(Value::as_object)
            .and_then(|section| section.get(key)),
        Some(Value::Bool(true))
    )
}

fn one_line(raw: &str, size: f64) -> Label {
    let flat = break_re().replace_all(raw, " ");
    let label = layout_label(Some(&flat), false, size, 4000.0, 0);
    if label.lines.len() <= 1 {
        return label;
    }
    Label {
        lines: vec![label.lines[0].clone()],
        widths: vec![label.widths[0]],
        width: label.widths[0],
        height: label.line_height,
        ..label
    }
}

fn scheduled(task: &Task) -> bool {
    matches!((task.start_time, task.end_time), (Some(s), Some(e)) if s.time().is_finite() && e.time().is_finite())
}

fn start_of(task: &Task) -> f64 {
    task.start_time.map_or(f64::NAN, |s| s.time())
}

fn end_of(task: &Task) -> f64 {
    task.end_time.map_or(f64::NAN, |e| e.time())
}

fn pack_rows(
    tasks: &[usize],
    all: &[Task],
    offset: usize,
    row_of: &mut HashMap<usize, usize>,
) -> usize {
    let mut sorted: Vec<usize> = tasks
        .iter()
        .copied()
        .filter(|&t| scheduled(&all[t]))
        .collect();
    sorted.sort_by(|&a, &b| {
        let by_start = start_of(&all[a])
            .partial_cmp(&start_of(&all[b]))
            .unwrap_or(std::cmp::Ordering::Equal);
        if by_start == std::cmp::Ordering::Equal {
            all[a].order.cmp(&all[b].order)
        } else {
            by_start
        }
    });
    let mut free: Vec<f64> = Vec::new();
    for &task in &sorted {
        let start = start_of(&all[task]);
        let mut row = 0;
        while row < free.len() && start < free[row] {
            row += 1;
        }
        if row == free.len() {
            free.push(end_of(&all[task]));
        } else {
            free[row] = end_of(&all[task]);
        }
        row_of.insert(task, offset + row);
    }
    let mut rows = free.len();
    for &task in tasks {
        if let std::collections::hash_map::Entry::Vacant(slot) = row_of.entry(task) {
            slot.insert(offset + rows);
            rows += 1;
        }
    }
    js_max(rows as f64, 1.0) as usize
}

struct Bar {
    task: usize,
    label: Label,
    row: usize,
    x0: f64,
    x1: f64,
    label_x: f64,
    inside: bool,
}

fn section_index(categories: &[(String, usize)], section: &str) -> usize {
    categories
        .iter()
        .find(|(name, _)| name == section)
        .map_or(0, |(_, index)| *index)
}

pub fn render_gantt(
    db: &mut GanttDb,
    config: &Config,
    options: &RenderOptions,
) -> Result<Drawn, PelersError> {
    let size = options.font_size.unwrap_or(16.0);
    let pad = options.padding.unwrap_or(8.0);
    let now = match options.now {
        Some(given) if given.is_finite() => given,
        _ => std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_millis() as f64),
    };

    let label_size = setting(config, "fontSize", 4.0, 72.0).unwrap_or(js_round(size * 0.8125));
    let section_size = setting(config, "sectionFontSize", 4.0, 72.0).unwrap_or(label_size);
    let tick_size = js_round(size * 0.75);
    let bar_height =
        setting(config, "barHeight", 4.0, 400.0).unwrap_or(js_round(label_size * 1.85));
    let bar_gap = setting(config, "barGap", 0.0, 400.0).unwrap_or(js_round(bar_height / 4.0));
    let row_height = bar_height + bar_gap;
    let section_styles =
        js_round(setting(config, "numberSectionStyles", 1.0, 1000.0).unwrap_or(4.0));
    let compact = db.display_mode == "compact";
    let top_axis = db.top_axis || flag(config, "topAxis");

    db.get_tasks(Some(now))?;
    let all: Vec<Task> = db.tasks.clone();
    let row_tasks: Vec<usize> = (0..all.len()).filter(|&t| !all[t].vert).collect();
    let verts: Vec<usize> = (0..all.len())
        .filter(|&t| all[t].vert && scheduled(&all[t]))
        .collect();

    let mut categories: Vec<(String, usize)> = Vec::new();
    for &task in &row_tasks {
        if !categories
            .iter()
            .any(|(name, _)| *name == all[task].section)
        {
            let index = categories.len();
            categories.push((all[task].section.clone(), index));
        }
    }

    let mut row_of: HashMap<usize, usize> = HashMap::new();
    let mut row_section: Vec<String> = Vec::new();
    if compact {
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for &task in &row_tasks {
            match groups
                .iter_mut()
                .find(|(section, _)| *section == all[task].section)
            {
                Some((_, list)) => list.push(task),
                None => groups.push((all[task].section.clone(), vec![task])),
            }
        }
        for (section, group) in groups {
            let rows = pack_rows(&group, &all, row_section.len(), &mut row_of);
            for _ in 0..rows {
                row_section.push(section.clone());
            }
        }
    } else {
        for &task in &row_tasks {
            row_of.insert(task, row_section.len());
            row_section.push(all[task].section.clone());
        }
    }
    let row_count = row_section.len();

    let mut start = f64::INFINITY;
    let mut stop = f64::NEG_INFINITY;
    for task in &all {
        if !scheduled(task) {
            continue;
        }
        start = js_min(start, start_of(task));
        stop = js_max(stop, end_of(task));
    }
    let has_plot = start <= stop;

    let title = layout_label(db.title.as_deref(), false, size, 4000.0, BOLD_STYLE);
    let title_height = if title.height > 0.0 {
        title.height + 16.0
    } else {
        0.0
    };

    if !has_plot {
        let width = js_max(title.width, 1.0) + 2.0 * pad;
        let width = width.ceil();
        let height = (js_max(title.height, 1.0) + 2.0 * pad).ceil();
        let content = label_svg(
            &title,
            width / 2.0,
            pad + title.height / 2.0,
            " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
        );
        let svg = svg_document(
            "gantt",
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
        return Ok(Drawn {
            svg,
            width,
            height,
            links: Vec::new(),
        });
    }

    let mut section_labels: Vec<Label> = Vec::new();
    let mut section_width: f64 = 0.0;
    for (name, _) in &categories {
        let label = layout_label(Some(name), false, section_size, 180.0, 0);
        section_width = js_max(section_width, label.width);
        section_labels.push(label);
    }

    let axis_format = if !db.axis_format.is_empty() {
        db.axis_format.clone()
    } else if db.date_format == "D" {
        "%d".to_owned()
    } else {
        text_setting(config, "axisFormat").unwrap_or_else(|| "%Y-%m-%d".to_owned())
    };
    let tick_label = |t: f64| -> String { time_format(&axis_format, &JsDate::from_ms(t)) };
    let sample_width = js_max(
        measure(&tick_label(start), tick_size, 0),
        measure(&tick_label(stop), tick_size, 0),
    );

    let left = pad
        + setting(config, "leftPadding", 0.0, 2000.0).unwrap_or_else(|| {
            js_max(
                if section_width > 0.0 {
                    section_width + 24.0
                } else {
                    0.0
                },
                sample_width / 2.0,
            )
        });
    let right_padding = setting(config, "rightPadding", 0.0, 2000.0)
        .unwrap_or_else(|| js_max(16.0, sample_width / 2.0));
    let target = cap_width(
        options,
        setting(config, "useWidth", 100.0, 20000.0).unwrap_or(WIDTH),
        280.0,
        0.0,
    );
    let plot_width = js_max(MIN_PLOT, target - left - right_padding - pad);
    let right = left + plot_width;
    let scale = if stop > start {
        plot_width / (stop - start)
    } else {
        0.0
    };
    let x = |t: f64| -> f64 {
        if scale == 0.0 {
            left + plot_width / 2.0
        } else {
            left + (t - start) * scale
        }
    };

    let tick_height = js_round(tick_size * 1.75);
    let top_padding = setting(config, "topPadding", 0.0, 2000.0);
    let grid_start = setting(config, "gridLineStartPadding", 0.0, 2000.0);
    let head_height = pad + title_height + if top_axis { tick_height } else { 0.0 };
    let plot_top = js_max(head_height, top_padding.unwrap_or(0.0));
    let grid_lead = match (top_padding, grid_start) {
        (Some(top), Some(grid)) => js_max(0.0, js_min(top - grid, plot_top - head_height)),
        _ => 0.0,
    };
    let plot_height = js_max(row_count as f64, 1.0) * row_height;
    let plot_bottom = plot_top + plot_height;

    let mut bars: Vec<Bar> = Vec::new();
    let mut extent = right + right_padding;
    for &task in &row_tasks {
        let label = one_line(&all[task].task, label_size);
        let row = row_of[&task];
        if !scheduled(&all[task]) {
            let label_x = left + LABEL_GAP + label.width / 2.0;
            extent = js_max(extent, label_x + label.width / 2.0);
            bars.push(Bar {
                task,
                label,
                row,
                x0: f64::NAN,
                x1: f64::NAN,
                label_x,
                inside: false,
            });
            continue;
        }
        let mut x0 = x(start_of(&all[task]));
        let render_end = all[task]
            .render_end_time
            .map_or(end_of(&all[task]), |r| r.time());
        let mut x1 = js_max(
            x(render_end),
            x0 + if all[task].milestone { 0.0 } else { 2.0 },
        );
        if all[task].milestone {
            let mid = x0 + (x(end_of(&all[task])) - x0) / 2.0;
            x0 = mid - bar_height / 2.0;
            x1 = mid + bar_height / 2.0;
        }
        let inside = !all[task].milestone && label.width + 2.0 * LABEL_GAP <= x1 - x0;
        let mut label_x = (x0 + x1) / 2.0;
        if !inside {
            let fits_right = x1 + LABEL_GAP + label.width <= right + right_padding;
            let fits_left = x0 - LABEL_GAP - label.width >= left;
            label_x = if fits_right || !fits_left {
                x1 + LABEL_GAP + label.width / 2.0
            } else {
                x0 - LABEL_GAP - label.width / 2.0
            };
            extent = js_max(extent, label_x + label.width / 2.0);
        }
        bars.push(Bar {
            task,
            label,
            row,
            x0,
            x1,
            label_x,
            inside,
        });
    }

    let interval = if !db.tick_interval.clone().unwrap_or_default().is_empty() {
        db.tick_interval.clone().unwrap_or_default()
    } else {
        text_setting(config, "tickInterval").unwrap_or_default()
    };
    let weekday = db
        .weekday
        .clone()
        .or_else(|| text_setting(config, "weekday"))
        .unwrap_or_else(|| "sunday".to_owned());
    let count = js_max(
        2.0,
        js_min(10.0, (plot_width / (sample_width + 24.0)).floor()),
    );
    let ticks = match interval_ticks(
        &interval,
        &weekday,
        start,
        stop,
        (plot_width / 3.0).ceil() as usize,
    ) {
        Some(list) => list,
        None => auto_ticks(start, stop, count),
    };

    let mut grid = String::new();
    let mut axis_bottom = String::new();
    let mut axis_top = String::new();
    let mut last_right = f64::NEG_INFINITY;
    let mut last_label = String::new();
    let bottom_y = plot_bottom + tick_height / 2.0 + 2.0;
    let top_y = plot_top - grid_lead - tick_height / 2.0 - 2.0;
    for &t in &ticks {
        let tx = x(t);
        grid.push_str(&format!(
            "M{},{}V{}",
            num(tx),
            num(plot_top - grid_lead),
            num(plot_bottom)
        ));
        let label = tick_label(t);
        let width = measure(&label, tick_size, 0);
        if tx - width / 2.0 < last_right + 8.0 || label == last_label {
            continue;
        }
        last_right = tx + width / 2.0;
        last_label = label.clone();
        extent = js_max(extent, last_right);
        let body = format!("text-anchor=\"middle\">{}</text>", esc(&label));
        axis_bottom.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" {body}",
            num(tx),
            num(bottom_y + tick_size * 0.35)
        ));
        if top_axis {
            axis_top.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" {body}",
                num(tx),
                num(top_y + tick_size * 0.35)
            ));
        }
    }

    let mut vert_lines = String::new();
    let mut vert_labels = String::new();
    let mut vert_ends: Vec<f64> = Vec::new();
    let vert_top = plot_bottom + tick_height + 4.0;
    let vert_line = js_round(label_size * 1.5);
    let mut vert_order = verts.clone();
    vert_order.sort_by(|&a, &b| {
        start_of(&all[a])
            .partial_cmp(&start_of(&all[b]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for &task in &vert_order {
        let vx = x(start_of(&all[task]));
        let label = one_line(&all[task].task, label_size);
        let cx = js_max(pad + label.width / 2.0, vx);
        let mut line = 0;
        while line < vert_ends.len() && cx - label.width / 2.0 < vert_ends[line] + 8.0 {
            line += 1;
        }
        let end = cx + label.width / 2.0;
        if line == vert_ends.len() {
            vert_ends.push(end);
        } else {
            vert_ends[line] = end;
        }
        extent = js_max(extent, vert_ends[line]);
        vert_lines.push_str(&format!(
            "<path class=\"pelers-vert\" data-id=\"{}\" d=\"M{},{}V{}\"/>",
            esc_text(&all[task].id),
            num(vx),
            num(plot_top - grid_lead),
            num(plot_bottom)
        ));
        vert_labels.push_str(&label_svg_with(
            &label,
            cx,
            vert_top + line as f64 * vert_line + vert_line / 2.0,
            &format!(
                " class=\"pelers-vert-label\" data-id=\"{}\"",
                esc_text(&all[task].id)
            ),
            false,
        ));
    }

    let width = (js_max(extent, pad + title.width) + pad).ceil();
    let height = (plot_bottom
        + tick_height
        + if vert_ends.is_empty() {
            0.0
        } else {
            4.0 + vert_ends.len() as f64 * vert_line
        }
        + pad)
        .ceil();

    let mut bands = String::new();
    let mut section_out = String::new();
    let mut row = 0usize;
    while row < row_count {
        let section = row_section[row].clone();
        let mut end = row + 1;
        while end < row_count && row_section[end] == section {
            end += 1;
        }
        let index = section_index(&categories, &section) as f64 % section_styles;
        let y = plot_top + row as f64 * row_height;
        let h = (end - row) as f64 * row_height;
        if categories.len() > 1 || !section.is_empty() {
            bands.push_str(&format!(
                "<rect class=\"pelers-section pelers-section-{}\" x=\"0\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                js_number_to_string(index),
                num(y),
                js_number_to_string(width),
                num(h),
                if index % 2.0 == 0.0 { "var(--_s)" } else { "none" }
            ));
        }
        let idx = categories
            .iter()
            .position(|(name, _)| *name == section)
            .unwrap_or(0);
        let label = &section_labels[idx];
        section_out.push_str(&label_svg_with(
            label,
            pad + 8.0 + label.width / 2.0,
            y + h / 2.0,
            &format!(
                " class=\"pelers-section-label\" data-id=\"{}\"",
                esc_text(&section)
            ),
            options.icons.is_some(),
        ));
        row = end;
    }

    let mut excluded = String::new();
    if (!db.excludes.is_empty() || !db.includes.is_empty())
        && add(&JsDate::from_ms(start), 6.0, super::dates::Unit::Y).time() > stop
    {
        let mut from: Option<JsDate> = None;
        let mut to: Option<JsDate> = None;
        let mut day = JsDate::from_ms(start);
        while day.time() <= stop {
            if db.is_invalid_date(&day)? {
                if from.is_none() {
                    from = Some(day);
                }
                to = Some(day);
            } else {
                flush_excluded(
                    &mut excluded,
                    &mut from,
                    &mut to,
                    &x,
                    left,
                    right,
                    plot_top,
                    plot_height,
                );
            }
            day = super::dates::add_days(&day, 1.0);
        }
        flush_excluded(
            &mut excluded,
            &mut from,
            &mut to,
            &x,
            left,
            right,
            plot_top,
            plot_height,
        );
    }

    let mut links: Vec<LinkInfo> = Vec::new();
    let mut tasks_out = String::new();
    for bar in &bars {
        let task = &all[bar.task];
        let cy = plot_top + bar.row as f64 * row_height + row_height / 2.0;
        let state = if task.active {
            "active"
        } else if task.done {
            "done"
        } else {
            ""
        };
        let mut classes = "pelers-task".to_owned();
        if !state.is_empty() {
            classes.push_str(&format!(" pelers-{state}"));
        }
        if task.crit {
            classes.push_str(" pelers-crit");
        }
        if task.milestone {
            classes.push_str(" pelers-milestone");
        }
        if task.classes.iter().any(|c| c == "clickable") {
            classes.push_str(" pelers-clickable");
        }

        let mut shape = String::new();
        let mut text_fill = String::new();
        if bar.x0.is_nan() {
            classes.push_str(" pelers-unscheduled");
            text_fill = " fill=\"var(--_m)\"".to_owned();
        } else {
            let color = if task.crit { CRITICAL } else { "var(--_c)" };
            let paint = if state == "done" {
                "fill=\"var(--_a)\"".to_owned()
            } else if state == "active" {
                format!("fill=\"{color}\" fill-opacity=\"0.25\"")
            } else {
                format!("fill=\"{color}\"")
            };
            let stroke = format!(
                "stroke=\"{}\"",
                if state == "done" && !task.crit {
                    "var(--_b)"
                } else {
                    color
                }
            );
            if task.milestone {
                let cx = (bar.x0 + bar.x1) / 2.0;
                let r = bar_height / 2.0;
                shape = format!(
                    "<path d=\"M{},{}L{},{}L{},{}L{},{}Z\" {paint} {stroke} stroke-linejoin=\"round\"/>",
                    num(cx),
                    num(cy - r),
                    num(cx + r),
                    num(cy),
                    num(cx),
                    num(cy + r),
                    num(cx - r),
                    num(cy)
                );
            } else {
                shape = format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" {paint} {stroke}{ROUNDED}/>",
                    num(bar.x0),
                    num(cy - bar_height / 2.0),
                    num(bar.x1 - bar.x0),
                    num(bar_height)
                );
            }
            if bar.inside {
                text_fill = if state.is_empty() {
                    " fill=\"var(--_bg)\"".to_owned()
                } else {
                    String::new()
                };
            } else if state == "done" {
                text_fill = " fill=\"var(--_m)\"".to_owned();
            }
        }

        let mut body = format!(
            "{shape}{}",
            label_svg_with(
                &bar.label,
                bar.label_x,
                cy,
                &format!(" class=\"pelers-label\"{text_fill}"),
                false
            )
        );
        if let Some(url) = db.links.get(&task.id).cloned() {
            body = linked(&body, &url, &task.id, &mut links, options, "");
        }
        tasks_out.push_str(&format!(
            "<g class=\"{classes}\" data-id=\"{}\">{body}</g>",
            esc_text(&task.id)
        ));
    }

    let mut today = String::new();
    let marker = db.today_marker.clone();
    if marker != "off" && now >= start && now <= stop && stop > start {
        let tx = x(now);
        let top = plot_top - grid_lead;
        let style = if marker.is_empty() {
            String::new()
        } else {
            let parts: Vec<&str> = marker.split(',').collect();
            resolve_style(&parts.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>()).line
        };
        let triangle = if style.is_empty() {
            format!(
                "<path d=\"M{},{}H{}L{},{}Z\" fill=\"var(--_c)\"/>",
                num(tx - 4.0),
                num(top - 5.0),
                num(tx + 4.0),
                num(tx),
                num(top + 1.0)
            )
        } else {
            String::new()
        };
        today = format!(
            "<g class=\"pelers-today\"><path d=\"M{},{}V{}\" fill=\"none\" stroke=\"var(--_c)\" stroke-width=\"2\"{style}/>{triangle}</g>",
            num(tx),
            num(top),
            num(plot_bottom)
        );
    }

    let axis_line = format!(
        "M{},{}H{}{}",
        num(left),
        num(plot_bottom),
        num(right),
        if top_axis {
            format!(
                "M{},{}H{}",
                num(left),
                num(plot_top - grid_lead),
                num(right)
            )
        } else {
            String::new()
        }
    );
    let mut content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    if !bands.is_empty() {
        content.push_str(&format!("<g class=\"pelers-sections\">{bands}</g>"));
    }
    if !excluded.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-excluded\" fill=\"var(--_a)\">{excluded}</g>"
        ));
    }
    content.push_str(&format!(
        "<g class=\"pelers-grid\" fill=\"none\"><path d=\"{grid}\" stroke=\"var(--_b)\" stroke-dasharray=\"1 3\"/><path d=\"{axis_line}\" stroke=\"var(--_b)\"/></g>"
    ));
    if !vert_lines.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-verts\" fill=\"none\" stroke=\"var(--_l)\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\">{vert_lines}</g>"
        ));
    }
    content.push_str(&today);
    content.push_str(&format!(
        "<g class=\"pelers-tasks\" font-size=\"{}\">{tasks_out}</g>",
        num(label_size)
    ));
    content.push_str(&format!(
        "<g class=\"pelers-section-labels\" font-size=\"{}\">{section_out}</g>",
        num(section_size)
    ));
    content.push_str(&format!(
        "<g class=\"pelers-axis\" font-size=\"{}\" fill=\"var(--_m)\">{axis_bottom}{axis_top}</g>",
        js_number_to_string(tick_size)
    ));
    if !vert_labels.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-vert-labels\" font-size=\"{}\">{vert_labels}</g>",
            num(label_size)
        ));
    }

    let svg = svg_document(
        "gantt",
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
    Ok(Drawn {
        svg,
        width,
        height,
        links,
    })
}

#[allow(clippy::too_many_arguments)]
fn flush_excluded(
    excluded: &mut String,
    from: &mut Option<JsDate>,
    to: &mut Option<JsDate>,
    x: &dyn Fn(f64) -> f64,
    left: f64,
    right: f64,
    plot_top: f64,
    plot_height: f64,
) {
    let (Some(from_day), Some(to_day)) = (*from, *to) else {
        return;
    };
    let mut day_start = from_day;
    day_start.set_hours(0.0, 0.0, 0.0, 0.0);
    let mut day_end = to_day;
    day_end.set_hours(23.0, 59.0, 59.0, 999.0);
    let ex0 = js_max(left, x(day_start.time()));
    let ex1 = js_min(right, x(day_end.time()));
    if ex1 > ex0 {
        excluded.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
            num(ex0),
            num(plot_top),
            num(ex1 - ex0),
            num(plot_height)
        ));
    }
    *from = None;
    *to = None;
}
