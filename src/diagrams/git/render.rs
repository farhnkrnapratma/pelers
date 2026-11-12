use std::f64::consts::FRAC_1_SQRT_2;

use super::db::{GitModel, commit_type};
use super::parser::Direction;
use crate::diagrams::common::Drawn;
use crate::diagrams::common::fit::plain_label;
use crate::diagrams::common::fit_width::turn_to_fit;
use crate::preprocess::Config;
use crate::svg::builder::{esc_text, label_svg, num};
use crate::svg::palette::series_ink_fill;
use crate::svg::root::{Accessible, RenderOptions, svg_document};
use crate::svg::theme::{ROUNDED, series_color};
use crate::text::label::layout_label;
use crate::text::measurer::BOLD_STYLE;
use crate::util::jsnum::{js_max, js_min, js_number_to_string, js_round};
use crate::util::value::Value;

const DOT: f64 = 7.0;
const STEP: f64 = 44.0;
const LANE: f64 = 44.0;
const CURVE: f64 = 36.0;
const GAP: f64 = 8.0;
const LABEL_GAP: f64 = 20.0;
const TAIL: f64 = 20.0;
const NOTCH: f64 = 7.0;
const TAG_GAP: f64 = 6.0;
const CLASSES: [&str; 5] = ["normal", "reverse", "highlight", "merge", "cherry-pick"];

struct View {
    commit: usize,
    index: usize,
    lane: usize,
    m: f64,
    label: Option<Label>,
    tags: Vec<Label>,
}

use crate::text::label::Label;

fn flag(config: &Config, key: &str, fallback: bool) -> bool {
    match config
        .get("gitGraph")
        .and_then(Value::as_object)
        .and_then(|section| section.get(key))
    {
        Some(Value::Bool(value)) => *value,
        _ => fallback,
    }
}

fn tag_width(tag: &Label) -> f64 {
    NOTCH + 4.0 + tag.width + 6.0
}

fn symbol(kind: u8, x: f64, y: f64, color: &str) -> String {
    let at = format!("cx=\"{}\" cy=\"{}\"", num(x), num(y));
    match kind {
        commit_type::HIGHLIGHT => format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{color}\"/>",
            num(x - DOT - 1.0),
            num(y - DOT - 1.0),
            num(2.0 * DOT + 2.0),
            num(2.0 * DOT + 2.0)
        ),
        commit_type::MERGE => format!(
            "<circle {at} r=\"{}\" fill=\"var(--_bg)\" stroke=\"{color}\" stroke-width=\"2\"/><circle {at} r=\"2.5\" fill=\"{color}\"/>",
            num(DOT - 1.0)
        ),
        commit_type::REVERSE => format!(
            "<circle {at} r=\"{}\" fill=\"{color}\"/><path d=\"M{},{}l6,6M{},{}l6,-6\" fill=\"none\" stroke=\"var(--_bg)\" stroke-width=\"1.5\" stroke-linecap=\"round\"/>",
            num(DOT),
            num(x - 3.0),
            num(y - 3.0),
            num(x - 3.0),
            num(y + 3.0)
        ),
        commit_type::CHERRY_PICK => format!(
            "<circle {at} r=\"{}\" fill=\"{color}\"/><path d=\"M{},{}L{},{}L{},{}\" fill=\"none\" stroke=\"var(--_bg)\" stroke-linejoin=\"round\"/><circle cx=\"{}\" cy=\"{}\" r=\"1.8\" fill=\"var(--_bg)\"/><circle cx=\"{}\" cy=\"{}\" r=\"1.8\" fill=\"var(--_bg)\"/>",
            num(DOT),
            num(x - 2.6),
            num(y + 1.0),
            num(x + 0.8),
            num(y - 4.2),
            num(x + 2.6),
            num(y + 1.0),
            num(x - 2.6),
            num(y + 2.2),
            num(x + 2.6),
            num(y + 2.2)
        ),
        _ => format!("<circle {at} r=\"{}\" fill=\"{color}\"/>", num(DOT)),
    }
}

pub fn render_git(model: &GitModel, config: &Config, options: &RenderOptions) -> Drawn {
    if model.direction != Direction::Lr {
        return draw(model, config, options, model.direction);
    }
    turn_to_fit(options, |down| {
        draw(
            model,
            config,
            options,
            if down { Direction::Tb } else { Direction::Lr },
        )
    })
}

fn draw(model: &GitModel, config: &Config, options: &RenderOptions, direction: Direction) -> Drawn {
    let size = options.font_size.unwrap_or(16.0);
    let small = js_round(size * 0.75);
    let pad = options.padding.unwrap_or(8.0);

    let show_branches = flag(config, "showBranches", true);
    let show_commit_label = flag(config, "showCommitLabel", true);
    let rotate = flag(config, "rotateCommitLabel", true);
    let parallel = flag(config, "parallelCommits", false);
    let vertical = direction != Direction::Lr;
    let flip = direction == Direction::Bt;

    let lane_count = model.lanes.len();
    let medium = js_round(size * 0.875);
    let names: Vec<Label> = model
        .lanes
        .iter()
        .map(|name| {
            plain_label(
                Some(if show_branches { name } else { "" }),
                medium,
                4000.0,
                0,
            )
        })
        .collect();
    let lane_of = |branch: &str| {
        model
            .lanes
            .iter()
            .position(|lane| lane == branch)
            .unwrap_or(0)
    };

    let mut commits_sorted: Vec<&super::db::GitCommit> = model.commits.values().collect();
    commits_sorted.sort_by_key(|commit| commit.seq);
    let mut views: Vec<View> = Vec::with_capacity(commits_sorted.len());
    for (index, commit) in commits_sorted.iter().enumerate() {
        let labelled = show_commit_label
            && commit.kind != commit_type::CHERRY_PICK
            && (commit.kind != commit_type::MERGE || commit.custom_id);
        let mut tags = Vec::new();
        for tag in &commit.tags {
            let label = plain_label(Some(tag), small, 4000.0, 0);
            if !label.lines.is_empty() {
                tags.push(label);
            }
        }
        views.push(View {
            commit: index,
            index,
            lane: lane_of(&commit.branch),
            m: 0.0,
            label: labelled.then(|| plain_label(Some(&commit.id), small, 4000.0, 0)),
            tags,
        });
    }
    let position_of = |id: &str| commits_sorted.iter().position(|commit| commit.id == id);

    let n = views.len();
    let mut half = vec![0.0f64; n];
    let mut tag_half = vec![0.0f64; n];
    let mut lo = vec![DOT; lane_count];
    let mut hi = vec![DOT; lane_count];
    for (k, view) in views.iter().enumerate() {
        let label = view.label.as_ref();
        let tags = &view.tags;
        let mut tags_main = 0.0f64;
        let mut tags_cross = 0.0f64;
        for tag in tags {
            let w = tag_width(tag);
            if vertical {
                tags_main = js_max(tags_main, tag.height);
                tags_cross += w + 4.0;
            } else {
                tags_main = js_max(tags_main, w);
                tags_cross += tag.height + 3.0;
            }
        }
        let mut label_main = 0.0f64;
        let mut label_cross = 0.0f64;
        if let Some(label) = label.filter(|l| !l.lines.is_empty()) {
            if rotate {
                label_cross = 3.0 + FRAC_1_SQRT_2 * (label.width + label.height);
            } else {
                label_main = if vertical { label.height } else { label.width };
                label_cross = 6.0 + if vertical { label.width } else { label.height };
            }
        }
        half[k] = js_max(js_max(DOT, tags_main / 2.0), label_main / 2.0);
        tag_half[k] = tags_main / 2.0;
        let tag_side = DOT
            + if tags.is_empty() {
                0.0
            } else {
                TAG_GAP + tags_cross
            };
        let label_side = DOT + label_cross;
        let lane = view.lane;
        lo[lane] = js_max(lo[lane], if vertical { label_side } else { tag_side });
        hi[lane] = js_max(hi[lane], if vertical { tag_side } else { label_side });
    }

    let mut lead = vec![0.0f64; n];
    if !vertical {
        for (k, view) in views.iter().enumerate() {
            if tag_half[k] == 0.0 {
                continue;
            }
            for id in &commits_sorted[view.commit].parents {
                if let Some(p) = position_of(id)
                    && views[p].lane < view.lane
                {
                    lead[k] = tag_half[k] + 4.0;
                }
            }
        }
    }
    let space = |p: usize, k: usize| -> f64 {
        let mut value = js_max(STEP, half[p] + half[k] + GAP);
        if lead[k] > 0.0 {
            value = js_max(value, half[p] + CURVE + lead[k]);
        }
        value
    };

    for k in 0..n {
        if !parallel {
            views[k].m = if k == 0 {
                0.0
            } else {
                views[k - 1].m + space(k - 1, k)
            };
            continue;
        }
        let mut m = 0.0f64;
        for id in &commits_sorted[k].parents {
            if let Some(p) = position_of(id) {
                if p >= k {
                    continue;
                }
                m = js_max(m, views[p].m + space(p, k));
            }
        }
        views[k].m = m;
    }
    let mut end = 0.0f64;
    for view in &views {
        end = js_max(end, view.m);
    }

    let mut cross = vec![0.0f64; lane_count];
    for i in 0..lane_count {
        let name = &names[i];
        let reach = if name.lines.is_empty() {
            0.0
        } else if vertical {
            name.width / 2.0 + 8.0
        } else {
            name.height / 2.0 + 2.0
        };
        lo[i] = js_max(lo[i], reach);
        hi[i] = js_max(hi[i], reach);
        if i > 0 {
            cross[i] = cross[i - 1] + js_max(LANE, hi[i - 1] + lo[i] + GAP);
        }
    }

    let px = |m: f64, c: f64| if vertical { c } else { m };
    let py = |m: f64, c: f64| {
        if vertical {
            if flip { -m } else { m }
        } else {
            c
        }
    };
    let point = |m: f64, c: f64| format!("{},{}", num(px(m, c)), num(py(m, c)));

    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut grow = |x0: f64, y0: f64, x1: f64, y1: f64| {
        if x0 < min_x {
            min_x = x0;
        }
        if y0 < min_y {
            min_y = y0;
        }
        if x1 > max_x {
            max_x = x1;
        }
        if y1 > max_y {
            max_y = y1;
        }
    };

    let mut branches = String::new();
    if show_branches {
        for i in 0..lane_count {
            let name = &names[i];
            let color = series_color(i);
            let c = cross[i];
            let w = name.width + 16.0;
            let h = name.height + 4.0;
            let from = if name.lines.is_empty() {
                -LABEL_GAP
            } else {
                -LABEL_GAP - if vertical { h } else { w }
            };
            let cx = px(from + (if vertical { h } else { w }) / 2.0, c);
            let cy = py(from + (if vertical { h } else { w }) / 2.0, c);
            grow(
                px(-LABEL_GAP, c),
                py(-LABEL_GAP, c),
                px(-LABEL_GAP, c),
                py(-LABEL_GAP, c),
            );
            grow(
                px(end + TAIL, c),
                py(end + TAIL, c),
                px(end + TAIL, c),
                py(end + TAIL, c),
            );
            branches.push_str(&format!(
                "<g class=\"pelers-branch\" data-id=\"{}\"><path class=\"pelers-lane\" d=\"M{}L{}\" stroke=\"{color}\" stroke-dasharray=\"2 4\"/>",
                esc_text(&model.lanes[i]),
                point(-LABEL_GAP, c),
                point(end + TAIL, c)
            ));
            if !name.lines.is_empty() {
                grow(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
                let ink = series_ink_fill(i);
                let ink = if ink.is_empty() {
                    " fill=\"var(--_bg)\"".to_owned()
                } else {
                    ink
                };
                branches.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\"{ROUNDED}/>{}",
                    num(cx - w / 2.0),
                    num(cy - h / 2.0),
                    num(w),
                    num(h),
                    label_svg(name, cx, cy, &format!(" class=\"pelers-label\"{ink}"))
                ));
            }
            branches.push_str("</g>");
        }
    }

    let mut taken: Vec<Vec<f64>> = vec![Vec::new(); lane_count];
    for view in &views {
        taken[view.lane].push(view.m);
    }
    if parallel {
        for list in &mut taken {
            list.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        }
    }
    let busy = |lane: usize, from: f64, to: f64| -> bool {
        let list = &taken[lane];
        let mut low = 0usize;
        let mut high = list.len();
        while low < high {
            let mid = (low + high) >> 1;
            if list[mid] <= from {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        low < list.len() && list[low] < to
    };

    let mut curves = String::new();
    let mut lines = String::new();
    for view in &views {
        let commit = commits_sorted[view.commit];
        let c2 = cross[view.lane];
        let m2 = view.m;
        for (j, id) in commit.parents.iter().enumerate() {
            let Some(p) = position_of(id) else { continue };
            if p == view.index {
                continue;
            }
            let parent = &views[p];
            let c1 = cross[parent.lane];
            let m1 = parent.m;
            if parent.lane == view.lane {
                lines.push_str(&format!(
                    "<path class=\"pelers-edge\" d=\"M{}L{}\" stroke=\"{}\"/>",
                    point(m1, c1),
                    point(m2, c2),
                    series_color(view.lane)
                ));
                continue;
            }
            let merged = j > 0 && commit.kind == commit_type::MERGE;
            let picked = j > 0 && !merged;
            let mut d = format!("M{}", point(m1, c1));
            if m2 <= m1 {
                let mid = (m1 + m2) / 2.0;
                d.push_str(&format!(
                    "C{} {} {}",
                    point(mid, c1),
                    point(mid, c2),
                    point(m2, c2)
                ));
            } else if !merged && !busy(view.lane, m1, m2) {
                let s = js_min(CURVE, m2 - m1);
                d.push_str(&format!(
                    "C{} {} {}L{}",
                    point(m1 + s / 2.0, c1),
                    point(m1 + s / 2.0, c2),
                    point(m1 + s, c2),
                    point(m2, c2)
                ));
            } else {
                let lead_here = lead[view.index];
                let to = if parent.lane < view.lane && m2 - lead_here > m1 {
                    m2 - lead_here
                } else {
                    m2
                };
                let mut s = js_min(CURVE, to - m1);
                let mut along = c1;
                if !merged || busy(parent.lane, m1, m2) {
                    along = if c1 > c2 {
                        c2 + hi[view.lane] + GAP / 2.0
                    } else {
                        c2 - lo[view.lane] - GAP / 2.0
                    };
                    s = js_min(CURVE, (to - m1) / 2.0);
                    d.push_str(&format!(
                        "C{} {} {}",
                        point(m1 + s / 2.0, c1),
                        point(m1 + s / 2.0, along),
                        point(m1 + s, along)
                    ));
                }
                d.push_str(&format!(
                    "L{}C{} {} {}",
                    point(to - s, along),
                    point(to - s / 2.0, along),
                    point(to - s / 2.0, c2),
                    point(to, c2)
                ));
                if to < m2 {
                    d.push_str(&format!("L{}", point(m2, c2)));
                }
            }
            let class = if merged {
                " pelers-edge-merge"
            } else if picked {
                " pelers-edge-cherry-pick"
            } else {
                ""
            };
            let lane_color = series_color(if merged { parent.lane } else { view.lane });
            let dash = if picked {
                " stroke-dasharray=\"4 3\""
            } else {
                ""
            };
            curves.push_str(&format!(
                "<path class=\"pelers-edge{class}\" d=\"{d}\" stroke=\"{lane_color}\"{dash}/>"
            ));
        }
    }

    let mut commits = String::new();
    for view in &views {
        let commit = commits_sorted[view.commit];
        let x = px(view.m, cross[view.lane]);
        let y = py(view.m, cross[view.lane]);
        let kind = commit.custom_type.unwrap_or(commit.kind);
        grow(x - DOT - 1.0, y - DOT - 1.0, x + DOT + 1.0, y + DOT + 1.0);
        let written = commit.kind < commit_type::MERGE && !commit.message.is_empty();
        let mut body = if written {
            format!("<title>{}</title>", esc_text(&commit.message))
        } else {
            String::new()
        };
        body.push_str(&symbol(kind, x, y, &series_color(view.lane)));

        if let Some(label) = view.label.as_ref().filter(|l| !l.lines.is_empty()) {
            let w = label.width;
            let h = label.height;
            let back = |cx: f64, cy: f64| {
                format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"3\" fill=\"var(--_bg)\"/>",
                    num(cx - w / 2.0 - 2.0),
                    num(cy - h / 2.0 + 2.0),
                    num(w + 4.0),
                    num(h - 4.0)
                )
            };
            if rotate {
                let k = (FRAC_1_SQRT_2 * h) / 2.0;
                let ax = if vertical { x - DOT - 3.0 - k } else { x + 3.0 };
                let ay = if vertical { y } else { y + DOT + 3.0 + k };
                let reach = FRAC_1_SQRT_2 * w + k;
                grow(
                    ax - reach,
                    if flip { ay - reach } else { ay - k },
                    ax + k,
                    if flip { ay + k } else { ay + reach },
                );
                body.push_str(&format!(
                    "<g class=\"pelers-commit-label\" transform=\"rotate({} {} {})\">{}{}</g>",
                    if flip { 45 } else { -45 },
                    num(ax),
                    num(ay),
                    back(ax - w / 2.0, ay),
                    label_svg(label, ax - w / 2.0, ay, "")
                ));
            } else {
                let cx = if vertical { x - DOT - 6.0 - w / 2.0 } else { x };
                let cy = if vertical { y } else { y + DOT + 6.0 + h / 2.0 };
                grow(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
                body.push_str(&format!(
                    "<g class=\"pelers-commit-label\">{}{}</g>",
                    back(cx, cy),
                    label_svg(label, cx, cy, "")
                ));
            }
        }

        let tags = &view.tags;
        let mut offset = DOT + TAG_GAP;
        let mut t: isize = if vertical { 0 } else { tags.len() as isize - 1 };
        while t >= 0 && (t as usize) < tags.len() {
            let tag = &tags[t as usize];
            let w = tag_width(tag);
            let h = tag.height;
            let left = if vertical { x + offset } else { x - w / 2.0 };
            let cy = if vertical { y } else { y - offset - h / 2.0 };
            offset += if vertical { w + 4.0 } else { h + 3.0 };
            grow(left, cy - h / 2.0, left + w, cy + h / 2.0);
            body.push_str(&format!(
                "<g class=\"pelers-tag\"><path d=\"M{},{}L{},{}H{}V{}H{}Z\" fill=\"var(--_s)\" stroke=\"var(--_b)\" stroke-linejoin=\"round\"/><circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"var(--_b)\"/>{}</g>",
                num(left),
                num(cy),
                num(left + NOTCH),
                num(cy - h / 2.0),
                num(left + w),
                num(cy + h / 2.0),
                num(left + NOTCH),
                num(left + NOTCH),
                num(cy),
                label_svg(tag, left + NOTCH + 4.0 + tag.width / 2.0, cy, " fill=\"var(--_fg)\"")
            ));
            t += if vertical { 1 } else { -1 };
        }

        let merge = if commit.kind == commit_type::MERGE && kind != commit_type::MERGE {
            " pelers-commit-merge"
        } else {
            ""
        };
        let class = CLASSES.get(kind as usize).copied().unwrap_or(CLASSES[0]);
        commits.push_str(&format!(
            "<g class=\"pelers-commit pelers-commit-{class}{merge}\" data-id=\"{}\">{body}</g>",
            esc_text(&commit.id)
        ));
    }

    if min_x > max_x {
        if 0.0 < min_x {
            min_x = 0.0;
        }
        if 0.0 < min_y {
            min_y = 0.0;
        }
        if 0.0 > max_x {
            max_x = 0.0;
        }
        if 0.0 > max_y {
            max_y = 0.0;
        }
    }
    let title = layout_label(model.title.as_deref(), false, size, 4000.0, BOLD_STYLE);
    let title_height = if title.height > 0.0 {
        title.height + 16.0
    } else {
        0.0
    };
    let inner = js_max(max_x - min_x, title.width);
    let width = (inner + 2.0 * pad).ceil();
    let height = (max_y - min_y + title_height + 2.0 * pad).ceil();
    let dx = pad - min_x + (inner - (max_x - min_x)) / 2.0;
    let dy = pad + title_height - min_y;

    let mut content = label_svg(
        &title,
        width / 2.0,
        pad + title.height / 2.0,
        " class=\"pelers-title\" font-weight=\"var(--_tw)\"",
    );
    content.push_str(&format!(
        "<g transform=\"translate({},{})\">",
        num(dx),
        num(dy)
    ));
    if !branches.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-branches\" fill=\"none\" font-size=\"{}\">{branches}</g>",
            js_number_to_string(medium)
        ));
    }
    if !curves.is_empty() || !lines.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-edges\" fill=\"none\" stroke-width=\"2\" stroke-linecap=\"round\">{curves}{lines}</g>"
        ));
    }
    if !commits.is_empty() {
        content.push_str(&format!(
            "<g class=\"pelers-commits\" font-size=\"{}\" fill=\"var(--_m)\">{commits}</g>",
            js_number_to_string(small)
        ));
    }
    content.push_str("</g>");

    let svg = svg_document(
        "gitGraph",
        width,
        height,
        size,
        options,
        Accessible {
            acc_title: model.acc_title.as_deref(),
            acc_descr: model.acc_descr.as_deref(),
        },
        &content,
    );
    Drawn {
        svg,
        width,
        height,
        links: Vec::new(),
    }
}
