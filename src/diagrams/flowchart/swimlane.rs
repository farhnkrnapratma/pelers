use crate::diagrams::common::Drawn;
use crate::diagrams::flowchart::db::FlowDb;
use crate::diagrams::flowchart::graph::{FlowGraph, GraphNode};
use crate::diagrams::flowchart::render::{FlowVariant, render_variant};
use crate::diagrams::flowchart::types::LabelType;
use crate::layout::lanes::{LANE_PAD, lane_layout};
use crate::layout::{CEdge, CNode, CompoundResult, Dir, LayeredOptions};
use crate::preprocess::Config;
use crate::svg::builder::{label_svg_with, num};
use crate::svg::ink::ink_fill;
use crate::svg::root::RenderOptions;
use crate::svg::theme::ResolvedStyle;
use crate::text::label::Label;

const DEFAULT_LANE: &str = "__swimlane_default__";

fn prepare(graph: &mut FlowGraph) {
    let mut loose = false;
    for node in &mut graph.nodes {
        if node.is_group {
            node.dir = None;
        } else if node.parent_id.is_none() {
            loose = true;
        }
    }
    if !loose {
        return;
    }
    for node in &mut graph.nodes {
        if !node.is_group && node.parent_id.is_none() {
            node.parent_id = Some(DEFAULT_LANE.to_owned());
        }
    }
    graph.nodes.push(GraphNode {
        id: DEFAULT_LANE.to_owned(),
        label: None,
        label_type: LabelType::Text,
        shape: "rect".to_owned(),
        is_group: true,
        parent_id: None,
        css_styles: Vec::new(),
        css_compiled_styles: Vec::new(),
        css_classes: String::new(),
        dir: None,
        link: None,
        link_target: None,
        tooltip: None,
        icon: None,
        pos: None,
        img: None,
        asset_width: None,
        asset_height: None,
        constraint: None,
        color_index: None,
        metadata: None,
    });
}

fn layout(
    nodes: &mut [CNode],
    edges: &mut [CEdge],
    dir: Dir,
    options: &LayeredOptions,
    tight: bool,
) -> CompoundResult {
    lane_layout(
        nodes,
        edges,
        dir,
        options,
        if tight { 12.0 } else { LANE_PAD },
    )
}

fn sideways(dir: Dir) -> bool {
    matches!(dir, Dir::LR | Dir::RL)
}

fn draw_lane(
    c: &CNode,
    label: &Label,
    style: &ResolvedStyle,
    classes: &str,
    id: &str,
    dir: Dir,
    icons: bool,
) -> String {
    let left = c.x - c.w / 2.0;
    let top = c.y - c.h / 2.0;
    let across = sideways(dir);
    let cx = if across { left + c.pad_top / 2.0 } else { c.x };
    let cy = if across { c.y } else { top + c.pad_top / 2.0 };
    let attrs = format!(
        " class=\"pelers-cluster-label\"{}{}",
        ink_fill(&style.shape, &style.text, Some("var(--_m)")),
        style.text
    );
    let text = label_svg_with(label, cx, cy, &attrs, icons);
    let text = if across && !text.is_empty() {
        format!(
            "<g transform=\"rotate(-90 {} {})\">{text}</g>",
            num(cx),
            num(cy)
        )
    } else {
        text
    };
    let (band_w, band_h) = if across {
        (c.pad_top, c.h)
    } else {
        (c.w, c.pad_top)
    };
    format!(
        "<g class=\"pelers-cluster pelers-lane{classes}\" data-id=\"{id}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"var(--_b)\"/><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"var(--_a)\" stroke=\"var(--_b)\"{}/>{text}</g>",
        num(left),
        num(top),
        num(c.w),
        num(c.h),
        num(left),
        num(top),
        num(band_w),
        num(band_h),
        style.shape
    )
}

fn filled_box(c: &CNode, dir: Dir) -> [f64; 4] {
    let left = c.x - c.w / 2.0;
    let top = c.y - c.h / 2.0;
    if sideways(dir) {
        [left, top, left + c.pad_top, top + c.h]
    } else {
        [left, top, left + c.w, top + c.pad_top]
    }
}

pub static VARIANT: FlowVariant = FlowVariant {
    kind: "swimlane",
    prepare,
    layout,
    draw_group: draw_lane,
    filled_box,
};

pub fn render_swimlane(db: &FlowDb, config: &Config, options: &RenderOptions) -> Drawn {
    render_variant(db, config, options, Some(&VARIANT))
}
