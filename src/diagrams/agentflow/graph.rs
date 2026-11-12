use std::collections::{HashMap, HashSet};

use super::color_slots::{container_slot, kind_slot};
use super::db::AgentflowDb;
use super::shapes::{DEFAULT_SHAPE, is_allowed_shape, is_removed_shape, resolve_shape_alias};
use super::types::{AgentSubgraph, DiagnosticId, LabelType, Metadata};
use crate::util::value::{Object, Value};

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
    pub color_index: Option<usize>,
    pub kind: Option<&'static str>,
    pub metadata: Option<Metadata>,
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
    pub minlen: Option<i64>,
    pub arrow_type_start: &'static str,
    pub arrow_type_end: &'static str,
    pub css_compiled_styles: Vec<String>,
    pub style: Vec<String>,
    pub classes: String,
    pub animate: Option<Value>,
    pub animation: Option<Value>,
    pub curve: Option<String>,
    pub metadata: Option<Metadata>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct AgentGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

fn truthy(text: Option<&String>) -> Option<&str> {
    text.map(String::as_str).filter(|t| !t.is_empty())
}

fn view_is_collapsed(metadata: &Option<Metadata>) -> bool {
    matches!(
        metadata.as_ref().and_then(|m| m.get("view")),
        Some(Value::String(view)) if view == "collapsed"
    )
}

fn compiled(db: &AgentflowDb, class_names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in class_names {
        if let Some(def) = db.classes.get(name) {
            out.extend(def.styles.iter().cloned());
            out = out
                .iter()
                .map(|s| crate::util::chars::trim(s).to_owned())
                .collect();
            out.extend(def.text_styles.iter().cloned());
            out = out
                .iter()
                .map(|s| crate::util::chars::trim(s).to_owned())
                .collect();
        }
    }
    out
}

fn normalise_node_shapes(nodes: &mut [GraphNode], db: &mut AgentflowDb) {
    for node in nodes.iter_mut() {
        if node.is_group {
            continue;
        }
        if node.shape.is_empty() || node.shape == "squareRect" || node.shape == "rect" {
            node.shape = DEFAULT_SHAPE.to_owned();
        }
        if is_removed_shape(&node.shape) {
            let message = format!(
                "shape \"{}\" was removed in v0.8.1, using \"{DEFAULT_SHAPE}\"",
                node.shape
            );
            db.emit_error(
                DiagnosticId::ShapeRemoved,
                message,
                Some(node.id.as_str()),
                None,
            );
            node.shape = DEFAULT_SHAPE.to_owned();
        } else if !is_allowed_shape(&node.shape) {
            let message = format!(
                "shape \"{}\" is not supported, using \"{DEFAULT_SHAPE}\"",
                node.shape
            );
            db.emit_warning(
                DiagnosticId::ShapeUnsupported,
                message,
                Some(node.id.as_str()),
                None,
            );
            node.shape = DEFAULT_SHAPE.to_owned();
        }
    }
}

pub fn build_agent_graph(
    db: &mut AgentflowDb,
    palette_length: usize,
    default_curve: Option<&str>,
) -> AgentGraph {
    let subgraphs: Vec<AgentSubgraph> = db.subgraphs.clone();
    let lookup: HashMap<String, usize> = subgraphs
        .iter()
        .enumerate()
        .map(|(i, sg)| (sg.id.clone(), i))
        .collect();

    let mut hidden: HashSet<String> = HashSet::new();
    let mut hidden_order: Vec<String> = Vec::new();
    let mut owner: HashMap<String, String> = HashMap::new();
    let mut walked: HashSet<String> = HashSet::new();
    for sg in &subgraphs {
        if !view_is_collapsed(&sg.metadata) || hidden.contains(&sg.id) {
            continue;
        }
        let mut pending = vec![sg.id.clone()];
        while let Some(current) = pending.pop() {
            let Some(&index) = lookup.get(&current) else {
                continue;
            };
            for child in subgraphs[index].nodes.clone() {
                if child == sg.id || hidden.contains(&child) {
                    continue;
                }
                hidden.insert(child.clone());
                hidden_order.push(child.clone());
                owner.insert(child.clone(), sg.id.clone());
                if !walked.contains(&child) {
                    pending.push(child);
                }
            }
        }
        walked.insert(sg.id.clone());
    }
    let mut collapsed_into: HashMap<String, String> = HashMap::new();
    for id in &hidden_order {
        let mut path = vec![id.clone()];
        let mut top = owner[id].clone();
        loop {
            let known = collapsed_into.get(&top).cloned();
            let up = known.clone().or_else(|| owner.get(&top).cloned());
            let Some(up) = up else {
                break;
            };
            if known.is_none() {
                path.push(top.clone());
            }
            top = up;
            if known.is_some() {
                break;
            }
        }
        for p in path {
            collapsed_into.insert(p, top.clone());
        }
    }

    let position: HashMap<String, usize> = subgraphs
        .iter()
        .enumerate()
        .map(|(i, sg)| (sg.id.clone(), i))
        .collect();
    let mut parent_of: HashMap<String, String> = HashMap::new();
    let closes_loop =
        |parent_of: &HashMap<String, String>, child: &str, parent: &str, at: usize| -> bool {
            let Some(&child_at) = position.get(child) else {
                return false;
            };
            if child_at < at {
                return false;
            }
            let mut current: Option<String> = Some(parent.to_owned());
            let mut hops = 0;
            while let Some(node) = current.clone() {
                if hops > subgraphs.len() {
                    break;
                }
                if node == child {
                    return true;
                }
                current = parent_of.get(&node).cloned();
                hops += 1;
            }
            false
        };
    for i in (0..subgraphs.len()).rev() {
        let sg = &subgraphs[i];
        if hidden.contains(&sg.id) {
            continue;
        }
        for id in &sg.nodes {
            if closes_loop(&parent_of, id, &sg.id, i) {
                let message = format!(
                    "Container \"{}\" cannot contain \"{id}\" because \"{id}\" already contains it. The nesting that would close the loop is dropped.",
                    sg.id
                );
                db.emit_warning(
                    DiagnosticId::ContainmentViolation,
                    message,
                    Some(id.as_str()),
                    None,
                );
                continue;
            }
            parent_of.insert(id.clone(), sg.id.clone());
        }
    }

    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut by_id: HashMap<String, usize> = HashMap::new();
    for i in (0..subgraphs.len()).rev() {
        let sg = &subgraphs[i];
        if hidden.contains(&sg.id) {
            continue;
        }
        let collapsed = view_is_collapsed(&sg.metadata);
        let metadata = if collapsed {
            let mut out: Object = sg.metadata.clone().unwrap_or_default();
            out.set("containerType", Value::String(sg.kind.clone()));
            Some(out)
        } else {
            sg.metadata.clone()
        };
        nodes.push(GraphNode {
            id: sg.id.clone(),
            label: Some(sg.title.clone()),
            label_type: sg.label_type,
            shape: if collapsed {
                "collapsedGroup".to_owned()
            } else {
                "flowGroup".to_owned()
            },
            is_group: !collapsed,
            parent_id: parent_of.get(&sg.id).cloned(),
            css_styles: Vec::new(),
            css_compiled_styles: compiled(db, &sg.classes),
            css_classes: sg.classes.join(" "),
            dir: sg.dir.clone(),
            link: None,
            link_target: None,
            tooltip: None,
            color_index: None,
            kind: None,
            metadata,
        });
        by_id.insert(sg.id.clone(), nodes.len() - 1);
    }

    let vertices: Vec<_> = db
        .nodes
        .iter()
        .map(|(id, v)| (id.to_owned(), v.clone()))
        .collect();
    for (id, vertex) in vertices {
        if hidden.contains(&id) {
            continue;
        }
        if let Some(&index) = by_id.get(&id) {
            let existing = &mut nodes[index];
            existing.css_styles = vertex.styles.clone();
            existing.css_compiled_styles = compiled(db, &vertex.classes);
            existing.css_classes = vertex.classes.join(" ");
            continue;
        }
        let resolved = resolve_shape_alias(vertex.shape.as_deref());
        let shape = match resolved.as_deref() {
            None | Some("square") => "squareRect".to_owned(),
            Some(other) => other.to_owned(),
        };
        let mut default_classes = vec!["default".to_owned(), "node".to_owned()];
        default_classes.extend(vertex.classes.iter().cloned());
        nodes.push(GraphNode {
            id: id.clone(),
            label: vertex.text.clone(),
            label_type: vertex.label_type,
            shape,
            is_group: false,
            parent_id: parent_of.get(&id).cloned(),
            css_styles: vertex.styles.clone(),
            css_compiled_styles: compiled(db, &default_classes),
            css_classes: format!("default {}", vertex.classes.join(" ")),
            dir: vertex.dir.clone(),
            link: vertex.link.clone(),
            link_target: vertex.link_target.clone(),
            tooltip: db.tooltips.get(&id).cloned(),
            color_index: None,
            kind: None,
            metadata: vertex.metadata.clone(),
        });
        by_id.insert(id.clone(), nodes.len() - 1);
    }

    let mut edges: Vec<GraphEdge> = Vec::new();
    let raw_edges = db.edges.clone();
    let default_style: Vec<String> = db
        .default_edge_style
        .as_ref()
        .map(|style| style.borrow().clone())
        .unwrap_or_default();
    for (index, raw) in raw_edges.iter().enumerate() {
        let start = collapsed_into
            .get(&raw.start)
            .cloned()
            .unwrap_or_else(|| raw.start.clone());
        let end = collapsed_into
            .get(&raw.end)
            .cloned()
            .unwrap_or_else(|| raw.end.clone());
        if start == end && raw.start != raw.end {
            continue;
        }
        let mut style = default_style.clone();
        if let Some(extra) = raw.style.as_ref() {
            style.extend(extra.borrow().iter().cloned());
        }
        let kind = raw.kind.clone().unwrap_or_else(|| "normal".to_owned());
        let curve = [
            raw.interpolate.as_deref(),
            db.default_interpolate.as_deref(),
            default_curve,
        ]
        .into_iter()
        .flatten()
        .find(|c| !c.is_empty())
        .map(str::to_owned);
        let id = match truthy(raw.id.as_ref()) {
            Some(id) => id.to_owned(),
            None => format!("L_{start}_{end}_{index}"),
        };
        let compiled_styles = compiled(db, &raw.classes);
        edges.push(GraphEdge {
            id,
            is_user_defined_id: raw.is_user_defined_id,
            start,
            end,
            kind,
            label: raw.text.clone(),
            label_type: raw.label_type,
            thickness: raw.stroke.clone(),
            pattern: raw.stroke.clone(),
            minlen: raw.length,
            arrow_type_start: "none",
            arrow_type_end: match raw.kind.as_deref() {
                Some("arrow_open") => "none",
                Some("arrow_cross") => "arrow_cross",
                _ => "arrow_point",
            },
            css_compiled_styles: compiled_styles,
            style,
            classes: "edge-thickness-normal edge-pattern-solid flowchart-link".to_owned(),
            animate: raw.animate.clone(),
            animation: raw.animation.clone(),
            curve,
            metadata: raw.metadata.clone(),
        });
    }

    normalise_node_shapes(&mut nodes, db);

    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for sg in &subgraphs {
        if let Some(parent) = parent_of.get(&sg.id) {
            children
                .entry(parent.clone())
                .or_default()
                .push(sg.id.clone());
        }
    }
    let mut order: HashMap<String, usize> = HashMap::new();
    for sg in &subgraphs {
        if parent_of.contains_key(&sg.id) {
            continue;
        }
        let mut pending = vec![sg.id.clone()];
        while let Some(id) = pending.pop() {
            if order.contains_key(&id) {
                continue;
            }
            let next = order.len();
            order.insert(id.clone(), next);
            if let Some(kids) = children.get(&id) {
                for kid in kids.iter().rev() {
                    pending.push(kid.clone());
                }
            }
        }
    }

    let mut fallback = order.len();
    for node in nodes.iter_mut() {
        if node.is_group || node.shape == "collapsedGroup" {
            let n = match order.get(&node.id) {
                Some(&n) => n,
                None => {
                    let current = fallback;
                    fallback += 1;
                    current
                }
            };
            node.color_index = Some(container_slot(n, palette_length));
            continue;
        }
        let kind = if db.is_connector(&node.id) {
            Some("connector")
        } else {
            db.nodes
                .get(&node.id)
                .map(|vertex| AgentflowDb::vertex_kind(vertex).as_str())
        };
        if let Some(kind) = kind.filter(|kind| kind_slot(kind).is_some()) {
            node.kind = Some(kind);
        }
    }

    AgentGraph { nodes, edges }
}
