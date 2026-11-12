use std::collections::{HashMap, HashSet};

use super::db::FlowDb;
use super::types::{FlowNode, LabelType};
use crate::util::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: Option<String>,
    pub label_type: LabelType,
    pub shape: String,
    pub is_group: bool,
    pub parent_id: Option<String>,
    pub css_styles: Vec<String>,
    pub css_compiled_styles: Vec<String>,
    pub css_classes: String,
    pub dir: Option<String>,
    pub link: Option<String>,
    pub link_target: Option<String>,
    pub tooltip: Option<String>,
    pub icon: Option<String>,
    pub pos: Option<String>,
    pub img: Option<String>,
    pub asset_width: Option<f64>,
    pub asset_height: Option<f64>,
    pub constraint: Option<String>,
    pub color_index: Option<usize>,
    pub metadata: Option<Vec<(String, Value)>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub id: String,
    pub is_user_defined_id: bool,
    pub start: String,
    pub end: String,
    pub kind: String,
    pub label: String,
    pub label_type: LabelType,
    pub thickness: Option<String>,
    pub pattern: Option<String>,
    pub minlen: Option<f64>,
    pub arrow_type_start: String,
    pub arrow_type_end: String,
    pub css_compiled_styles: Vec<String>,
    pub style: Vec<String>,
    pub classes: String,
    pub animate: Option<Value>,
    pub animation: Option<Value>,
    pub curve: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FlowGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

fn shape_of(vertex: &FlowNode) -> String {
    if vertex.img.is_some() {
        return "imageSquare".to_owned();
    }
    if vertex.icon.is_some() {
        return match vertex.form.as_deref() {
            Some("circle") => "iconCircle",
            Some("square") => "iconSquare",
            Some("rounded") => "iconRounded",
            _ => "icon",
        }
        .to_owned();
    }
    match vertex.kind.as_deref() {
        None | Some("square") => "squareRect".to_owned(),
        Some("round") => "roundedRect".to_owned(),
        Some(other) => other.to_owned(),
    }
}

fn arrows(kind: Option<&str>) -> (String, String) {
    match kind {
        Some(k @ ("arrow_point" | "arrow_circle" | "arrow_cross")) => {
            ("none".to_owned(), k.to_owned())
        }
        Some(k @ ("double_arrow_point" | "double_arrow_circle" | "double_arrow_cross")) => {
            let single = k.replacen("double_", "", 1);
            (single.clone(), single)
        }
        _ => ("none".to_owned(), "arrow_point".to_owned()),
    }
}

fn is_collapsed_meta(metadata: Option<&Vec<(String, Value)>>) -> bool {
    metadata
        .and_then(|m| m.iter().find(|(k, _)| k == "view"))
        .is_some_and(|(_, v)| v.as_str() == Some("collapsed"))
}

pub fn build_flow_graph(db: &FlowDb, default_curve: Option<&str>) -> FlowGraph {
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let subgraphs = &db.subgraphs;

    let compiled = |class_names: &[String]| -> Vec<String> {
        let mut out = Vec::new();
        for name in class_names {
            let Some(def) = db.class(name) else {
                continue;
            };
            for s in &def.styles {
                out.push(s.trim().to_owned());
            }
            for s in &def.text_styles {
                out.push(s.trim().to_owned());
            }
        }
        out
    };

    let mut subgraph_parent: HashMap<String, String> = HashMap::new();
    for sg in subgraphs {
        for child in &sg.nodes {
            if db.subgraph(child).is_some() {
                subgraph_parent.insert(child.clone(), sg.id.clone());
            }
        }
    }

    let mut declaration_index: HashMap<String, usize> = HashMap::new();
    let mut children_of: HashMap<String, Vec<String>> = HashMap::new();
    for sg in subgraphs {
        if let Some(parent) = subgraph_parent.get(&sg.id) {
            children_of
                .entry(parent.clone())
                .or_default()
                .push(sg.id.clone());
        }
    }
    let mut next_index = 0usize;
    for sg in subgraphs {
        if subgraph_parent.contains_key(&sg.id) {
            continue;
        }
        let mut pending = vec![sg.id.clone()];
        while let Some(id) = pending.pop() {
            if declaration_index.contains_key(&id) {
                continue;
            }
            declaration_index.insert(id.clone(), next_index);
            next_index += 1;
            if let Some(kids) = children_of.get(&id) {
                for kid in kids.iter().rev() {
                    pending.push(kid.clone());
                }
            }
        }
    }

    let is_collapsed = |id: &str| -> bool {
        db.subgraph(id)
            .is_some_and(|s| is_collapsed_meta(s.metadata_ref()))
    };
    let any_collapsed = subgraphs
        .iter()
        .any(|sg| is_collapsed_meta(sg.metadata_ref()));
    let mut outermost: HashMap<String, Option<String>> = HashMap::new();
    let outermost_collapsed =
        |id: &str, outermost: &mut HashMap<String, Option<String>>| -> Option<String> {
            let mut chain: Vec<String> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            let mut current: Option<String> = Some(id.to_owned());
            while let Some(cur) = current.clone() {
                if outermost.contains_key(&cur) || seen.contains(&cur) {
                    break;
                }
                seen.insert(cur.clone());
                chain.push(cur.clone());
                current = subgraph_parent.get(&cur).cloned();
            }
            let mut result: Option<String> = match &current {
                Some(c) => outermost.get(c).cloned().flatten(),
                None => None,
            };
            for item in chain.iter().rev() {
                if result.is_none() && is_collapsed(item) {
                    result = Some(item.clone());
                }
                outermost.insert(item.clone(), result.clone());
            }
            result
        };

    let mut hidden: HashSet<String> = HashSet::new();
    let mut collapsed_into: HashMap<String, String> = HashMap::new();
    if any_collapsed {
        for sg in subgraphs {
            let Some(ancestor) = outermost_collapsed(&sg.id, &mut outermost) else {
                continue;
            };
            if sg.id != ancestor {
                hidden.insert(sg.id.clone());
                collapsed_into.insert(sg.id.clone(), ancestor.clone());
            }
            for child in &sg.nodes {
                if *child == ancestor {
                    continue;
                }
                hidden.insert(child.clone());
                collapsed_into.insert(child.clone(), ancestor.clone());
            }
        }
    }

    let mut parent_of: HashMap<String, String> = HashMap::new();
    let mut groups: HashSet<String> = HashSet::new();
    for sg in subgraphs.iter().rev() {
        if hidden.contains(&sg.id) {
            continue;
        }
        if !sg.nodes.is_empty() {
            groups.insert(sg.id.clone());
        }
        for id in &sg.nodes {
            parent_of.insert(id.clone(), sg.id.clone());
        }
    }

    let mut by_id: HashMap<String, usize> = HashMap::new();
    for sg in subgraphs.iter().rev() {
        if hidden.contains(&sg.id) {
            continue;
        }
        let collapsed = is_collapsed_meta(sg.metadata_ref());
        let node = GraphNode {
            id: sg.id.clone(),
            label: Some(sg.title.clone()),
            label_type: sg.label_type,
            parent_id: parent_of.get(&sg.id).cloned(),
            css_styles: Vec::new(),
            css_compiled_styles: compiled(&sg.classes),
            css_classes: sg.classes.join(" "),
            shape: if collapsed { "collapsedGroup" } else { "rect" }.to_owned(),
            dir: sg.dir.clone(),
            link: None,
            link_target: None,
            tooltip: None,
            icon: None,
            pos: None,
            img: None,
            asset_width: None,
            asset_height: None,
            constraint: None,
            is_group: !collapsed,
            color_index: declaration_index.get(&sg.id).copied(),
            metadata: if collapsed {
                None
            } else {
                sg.metadata_ref().cloned()
            },
        };
        nodes.push(node);
        by_id.insert(sg.id.clone(), nodes.len() - 1);
    }

    for vertex in &db.nodes {
        if hidden.contains(&vertex.id) {
            continue;
        }
        if let Some(&idx) = by_id.get(&vertex.id) {
            let existing = &mut nodes[idx];
            existing.css_styles = vertex.styles.clone();
            existing.css_compiled_styles = compiled(&vertex.classes);
            existing.css_classes = vertex.classes.join(" ");
            continue;
        }
        let is_group = groups.contains(&vertex.id);
        let mut defaults = vec!["default".to_owned(), "node".to_owned()];
        defaults.extend(vertex.classes.iter().cloned());
        let node = GraphNode {
            id: vertex.id.clone(),
            label: vertex.text.clone(),
            label_type: vertex.label_type,
            parent_id: parent_of.get(&vertex.id).cloned(),
            css_styles: vertex.styles.clone(),
            css_compiled_styles: compiled(&defaults),
            css_classes: format!("default {}", vertex.classes.join(" ")),
            dir: vertex.dir.clone(),
            link: vertex.link.clone(),
            link_target: vertex.link_target.clone(),
            tooltip: db.tooltip(&vertex.id).map(str::to_owned),
            icon: vertex.icon.clone(),
            pos: vertex.pos.clone(),
            img: vertex.img.clone(),
            asset_width: vertex.asset_width,
            asset_height: vertex.asset_height,
            constraint: vertex.constraint.clone(),
            is_group,
            shape: if is_group {
                "rect".to_owned()
            } else {
                shape_of(vertex)
            },
            color_index: None,
            metadata: None,
        };
        nodes.push(node);
    }

    for (index, raw) in db.edges.iter().enumerate() {
        let start = collapsed_into
            .get(&raw.start)
            .cloned()
            .unwrap_or_else(|| raw.start.clone());
        let end = collapsed_into
            .get(&raw.end)
            .cloned()
            .unwrap_or_else(|| raw.end.clone());
        if start == end
            && (collapsed_into.contains_key(&raw.start) || collapsed_into.contains_key(&raw.end))
        {
            continue;
        }
        let (arrow_start, arrow_end) = arrows(raw.kind.as_deref());
        let mut styles: Vec<String> = db.default_edge_style.clone().unwrap_or_default();
        if let Some(extra) = &raw.style {
            styles.extend(extra.iter().cloned());
        }
        let plain =
            raw.stroke.as_deref() == Some("invisible") || raw.kind.as_deref() == Some("arrow_open");
        let id = match raw.id.as_deref() {
            Some(id) if !id.is_empty() => id.to_owned(),
            _ => format!("L_{start}_{end}_{index}"),
        };
        let curve = match &raw.interpolate {
            Some(v) if v.is_truthy() => Some(v.clone()),
            _ => match db.default_interpolate.as_deref() {
                Some(d) if !d.is_empty() => Some(Value::String(d.to_owned())),
                _ => default_curve
                    .filter(|c| !c.is_empty())
                    .map(|c| Value::String(c.to_owned())),
            },
        };
        edges.push(GraphEdge {
            id,
            is_user_defined_id: raw.is_user_defined_id,
            start,
            end,
            kind: raw.kind.clone().unwrap_or_else(|| "normal".to_owned()),
            label: raw.text.clone(),
            label_type: raw.label_type,
            thickness: raw.stroke.clone(),
            pattern: raw.stroke.clone(),
            minlen: raw.length,
            classes: if raw.stroke.as_deref() == Some("invisible") {
                String::new()
            } else {
                "edge-thickness-normal edge-pattern-solid flowchart-link".to_owned()
            },
            arrow_type_start: if plain {
                "none".to_owned()
            } else {
                arrow_start
            },
            arrow_type_end: if plain { "none".to_owned() } else { arrow_end },
            css_compiled_styles: compiled(&raw.classes),
            style: styles,
            animate: raw.animate.clone(),
            animation: raw.animation.clone(),
            curve,
        });
    }

    FlowGraph { nodes, edges }
}
