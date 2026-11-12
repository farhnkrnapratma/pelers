use std::cmp::Ordering;

use super::parser::SankeyDb;
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::jsnum::{js_max, js_min};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Alignment {
    Justify,
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, Debug)]
pub struct LayoutOptions {
    pub width: f64,
    pub height: f64,
    pub node_width: f64,
    pub node_padding: f64,
    pub label_height: f64,
    pub alignment: Alignment,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedNode {
    pub layer: usize,
    pub value: f64,
    pub x: f64,
    pub y: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedLink {
    pub source: usize,
    pub target: usize,
    pub value: f64,
    pub thickness: f64,
    pub source_y: f64,
    pub target_y: f64,
}

pub struct SankeyLayout {
    pub nodes: Vec<PlacedNode>,
    pub links: Vec<PlacedLink>,
}

const PASSES: usize = 6;

fn tie_break(difference: f64, a: usize, b: usize) -> Ordering {
    if difference != 0.0 && !difference.is_nan() {
        difference.partial_cmp(&0.0).unwrap_or(Ordering::Equal)
    } else {
        a.cmp(&b)
    }
}

struct State {
    nodes: Vec<PlacedNode>,
    links: Vec<PlacedLink>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    source_offset: Vec<f64>,
    target_offset: Vec<f64>,
    margin: Vec<f64>,
    columns: Vec<Vec<usize>>,
    gap: f64,
    height: f64,
    n: usize,
}

impl State {
    fn center(&self, i: usize) -> f64 {
        self.nodes[i].y + self.nodes[i].height / 2.0
    }

    fn stack_out(&mut self, i: usize) {
        let mut list = std::mem::take(&mut self.outgoing[i]);
        list.sort_by(|&a, &b| {
            tie_break(
                self.center(self.links[a].target) - self.center(self.links[b].target),
                a,
                b,
            )
        });
        let mut y = 0.0f64;
        for &k in &list {
            self.source_offset[k] = y + self.links[k].thickness / 2.0;
            y += self.links[k].thickness;
        }
        self.outgoing[i] = list;
    }

    fn stack_in(&mut self, i: usize) {
        let mut list = std::mem::take(&mut self.incoming[i]);
        list.sort_by(|&a, &b| {
            tie_break(
                self.center(self.links[a].source) - self.center(self.links[b].source),
                a,
                b,
            )
        });
        let mut y = 0.0f64;
        for &k in &list {
            self.target_offset[k] = y + self.links[k].thickness / 2.0;
            y += self.links[k].thickness;
        }
        self.incoming[i] = list;
    }

    fn stack_all(&mut self) {
        for i in 0..self.n {
            self.stack_out(i);
            self.stack_in(i);
        }
    }

    fn settle(&mut self, column: usize, pull: f64, forward: bool) {
        let members = self.columns[column].clone();
        for &i in &members {
            let list: Vec<usize> = if forward {
                self.incoming[i].clone()
            } else {
                self.outgoing[i].clone()
            };
            if list.is_empty() {
                continue;
            }
            if forward {
                self.stack_in(i);
            } else {
                self.stack_out(i);
            }
            let list: Vec<usize> = if forward {
                self.incoming[i].clone()
            } else {
                self.outgoing[i].clone()
            };
            let mut weighted = 0.0f64;
            let mut weight = 0.0f64;
            let mut plain = 0.0f64;
            for &k in &list {
                let level = if forward {
                    self.nodes[self.links[k].source].y + self.source_offset[k]
                        - self.target_offset[k]
                } else {
                    self.nodes[self.links[k].target].y + self.target_offset[k]
                        - self.source_offset[k]
                };
                weighted += level * self.links[k].thickness;
                weight += self.links[k].thickness;
                let other = if forward {
                    self.links[k].source
                } else {
                    self.links[k].target
                };
                plain += self.center(other) - self.nodes[i].height / 2.0;
            }
            let wanted = if weight > 0.0 {
                weighted / weight
            } else {
                plain / list.len() as f64
            };
            self.nodes[i].y += (wanted - self.nodes[i].y) * pull;
        }

        let mut sorted = std::mem::take(&mut self.columns[column]);
        sorted.sort_by(|&a, &b| tie_break(self.center(a) - self.center(b), a, b));
        let mut y = 0.0f64;
        for &i in &sorted {
            if self.nodes[i].y - self.margin[i] < y {
                self.nodes[i].y = y + self.margin[i];
            }
            y = self.nodes[i].y + self.nodes[i].height + self.margin[i] + self.gap;
        }
        y = self.height;
        for k in (0..sorted.len()).rev() {
            let i = sorted[k];
            if self.nodes[i].y + self.nodes[i].height + self.margin[i] > y {
                self.nodes[i].y = y - self.nodes[i].height - self.margin[i];
            }
            y = self.nodes[i].y - self.margin[i] - self.gap;
        }
        self.columns[column] = sorted;
    }
}

pub fn layout_sankey(
    model: &SankeyDb,
    options: LayoutOptions,
) -> Result<SankeyLayout, PelersError> {
    let n = model.nodes.len();
    let mut links: Vec<PlacedLink> = model
        .links
        .iter()
        .map(|link| PlacedLink {
            source: link.source,
            target: link.target,
            value: if link.value.is_finite() && link.value > 0.0 {
                link.value
            } else {
                0.0
            },
            thickness: 0.0,
            source_y: 0.0,
            target_y: 0.0,
        })
        .collect();
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut incoming: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (k, link) in links.iter().enumerate() {
        outgoing[link.source].push(k);
        incoming[link.target].push(k);
    }

    let mut depth = vec![0i64; n];
    let mut pending: Vec<i64> = incoming.iter().map(|list| list.len() as i64).collect();
    let mut order: Vec<usize> = (0..n).filter(|&i| pending[i] == 0).collect();
    let mut q = 0usize;
    while q < order.len() {
        let i = order[q];
        for &k in &outgoing[i] {
            let t = links[k].target;
            if depth[t] < depth[i] + 1 {
                depth[t] = depth[i] + 1;
            }
            pending[t] -= 1;
            if pending[t] == 0 {
                order.push(t);
            }
        }
        q += 1;
    }
    if order.len() < n {
        let mut stuck = 0usize;
        while pending[stuck] == 0 {
            stuck += 1;
        }
        let mut seen = vec![false; n];
        while !seen[stuck] {
            seen[stuck] = true;
            for &k in &incoming[stuck] {
                if pending[links[k].source] > 0 {
                    stuck = links[k].source;
                    break;
                }
            }
        }
        let mut error = PelersError::new(
            format!(
                "Sankey diagrams cannot have circular links, and \"{}\" is part of one.",
                model.nodes[stuck].id
            ),
            PelersErrorCode::Semantic,
        );
        error.diagram_type = Some("sankey".to_owned());
        return Err(error);
    }
    let mut layers = 1i64;
    for &d in &depth {
        if d + 1 > layers {
            layers = d + 1;
        }
    }
    let layer_count = layers as usize;
    let mut reach = vec![0i64; n];
    for q in (0..n).rev() {
        let i = order[q];
        for &k in &outgoing[i] {
            let target = links[k].target;
            if reach[i] < reach[target] + 1 {
                reach[i] = reach[target] + 1;
            }
        }
    }

    let mut largest = 0.0f64;
    for link in &links {
        if link.value > largest {
            largest = link.value;
        }
    }
    let unit = if largest > 0.0 { 1.0 / largest } else { 0.0 };

    let mut nodes: Vec<PlacedNode> = Vec::with_capacity(n);
    let mut flow = vec![0.0f64; n];
    let mut columns: Vec<Vec<usize>> = vec![Vec::new(); layer_count];
    let step = if layers > 1 {
        (options.width - options.node_width) / (layers - 1) as f64
    } else {
        0.0
    };
    for i in 0..n {
        let layer: i64 = match options.alignment {
            Alignment::Left => depth[i],
            Alignment::Right => layers - 1 - reach[i],
            Alignment::Center => {
                let mut layer = depth[i];
                if incoming[i].is_empty() && !outgoing[i].is_empty() {
                    layer = layers;
                    for &k in &outgoing[i] {
                        let target = links[k].target;
                        if depth[target] - 1 < layer {
                            layer = depth[target] - 1;
                        }
                    }
                }
                layer
            }
            Alignment::Justify => {
                if !outgoing[i].is_empty() {
                    depth[i]
                } else {
                    layers - 1
                }
            }
        };
        let mut into = 0.0f64;
        let mut out = 0.0f64;
        let mut value = 0.0f64;
        let mut value_out = 0.0f64;
        for &k in &incoming[i] {
            into += links[k].value * unit;
            value += links[k].value;
        }
        for &k in &outgoing[i] {
            out += links[k].value * unit;
            value_out += links[k].value;
        }
        flow[i] = js_max(into, out);
        nodes.push(PlacedNode {
            layer: layer as usize,
            value: js_max(value, value_out),
            x: layer as f64 * step,
            y: 0.0,
            height: 0.0,
        });
        columns[layer as usize].push(i);
    }

    let mut tallest = 1usize;
    for column in &columns {
        if column.len() > tallest {
            tallest = column.len();
        }
    }
    let gap = if tallest > 1 {
        js_min(options.node_padding, options.height / (tallest - 1) as f64)
    } else {
        options.node_padding
    };
    let mut scale = f64::INFINITY;
    for column in &columns {
        let mut sum = 0.0f64;
        for &i in column {
            sum += flow[i];
        }
        if sum > 0.0 {
            scale = js_min(
                scale,
                (options.height - (column.len() - 1) as f64 * gap) / sum,
            );
        }
    }
    if !scale.is_finite() || scale < 0.0 {
        scale = 0.0;
    }

    let mut claim = js_max(0.0, options.label_height - gap);
    let fits = |s: f64| -> bool {
        for column in &columns {
            let mut extent = (column.len() - 1) as f64 * gap;
            for &i in column {
                extent += js_max(flow[i] * s, claim);
            }
            if extent > options.height + 1e-6 {
                return false;
            }
        }
        true
    };
    if claim > 0.0 && !fits(scale) {
        if fits(0.0) {
            let mut low = 0.0f64;
            for _ in 0..40 {
                let mid = (low + scale) / 2.0;
                if fits(mid) {
                    low = mid;
                } else {
                    scale = mid;
                }
            }
            scale = low;
        } else {
            claim = 0.0;
        }
    }

    let mut margin = vec![0.0f64; n];
    for link in &mut links {
        link.thickness = link.value * unit * scale;
    }
    for column in &columns {
        let mut y = 0.0f64;
        for &i in column {
            nodes[i].height = flow[i] * scale;
            margin[i] = js_max(0.0, claim - nodes[i].height) / 2.0;
            nodes[i].y = y + margin[i];
            y += nodes[i].height + 2.0 * margin[i] + gap;
        }
        let spare = (options.height - y + gap) / (column.len() + 1) as f64;
        for (k, &i) in column.iter().enumerate() {
            nodes[i].y += spare * (k + 1) as f64;
        }
    }

    let link_count = links.len();
    let mut state = State {
        nodes,
        links,
        outgoing,
        incoming,
        source_offset: vec![0.0f64; link_count],
        target_offset: vec![0.0f64; link_count],
        margin,
        columns,
        gap,
        height: options.height,
        n,
    };
    state.stack_all();
    for pass in 0..PASSES {
        let pull = 0.99f64.powf(pass as f64);
        for l in (0..layer_count.saturating_sub(1)).rev() {
            state.settle(l, pull, false);
        }
        state.stack_all();
        for l in 1..layer_count {
            state.settle(l, pull, true);
        }
        state.stack_all();
    }
    for k in 0..state.links.len() {
        let link = state.links[k];
        state.links[k].source_y = state.nodes[link.source].y + state.source_offset[k];
        state.links[k].target_y = state.nodes[link.target].y + state.target_offset[k];
    }

    Ok(SankeyLayout {
        nodes: state.nodes,
        links: state.links,
    })
}
