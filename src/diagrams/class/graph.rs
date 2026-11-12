use std::collections::HashMap;

use super::db::{ClassDb, RELATION_LOLLIPOP};
use super::types::RelationEnd;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeShape {
    Rect,
    ClassBox,
    Note,
    Interface,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndMarker {
    Aggregation,
    Extension,
    Composition,
    Dependency,
    Lollipop,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub is_group: bool,
    pub shape: NodeShape,
    pub parent_id: Option<String>,
    pub parent: i64,
    pub class_id: Option<String>,
    pub color_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub id: String,
    pub start: String,
    pub end: String,
    pub source: i64,
    pub target: i64,
    pub label: Option<String>,
    pub arrow_type_start: EndMarker,
    pub arrow_type_end: EndMarker,
    pub start_label_right: String,
    pub end_label_left: String,
    pub pattern: Pattern,
    pub classes: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClassGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

fn marker(kind: RelationEnd) -> EndMarker {
    match kind {
        Some(0) => EndMarker::Aggregation,
        Some(1) => EndMarker::Extension,
        Some(2) => EndMarker::Composition,
        Some(3) => EndMarker::Dependency,
        Some(4) => EndMarker::Lollipop,
        _ => EndMarker::None,
    }
}

pub fn build_class_graph(model: &ClassDb, hierarchical: bool) -> ClassGraph {
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edges: Vec<GraphEdge> = Vec::new();
    let mut namespace_at: HashMap<String, i64> = HashMap::new();
    let mut class_at: HashMap<String, i64> = HashMap::new();
    let mut interface_at: HashMap<String, i64> = HashMap::new();
    let mut declared: HashMap<String, Option<String>> = HashMap::new();

    let declared_ancestor =
        |declared: &mut HashMap<String, Option<String>>, id: Option<&str>| -> Option<String> {
            if hierarchical {
                return id.map(str::to_owned);
            }
            let mut current = id.map(str::to_owned);
            let mut chain: Vec<String> = Vec::new();
            let mut found: Option<String> = None;
            while let Some(cur) = current.clone().filter(|c| !c.is_empty()) {
                if let Some(known) = declared.get(&cur) {
                    found = known.clone();
                    break;
                }
                let Some(ns) = model.namespaces.get(&cur) else {
                    break;
                };
                if ns.explicit {
                    found = Some(cur);
                    break;
                }
                current = ns.parent.clone();
                chain.push(cur);
            }
            for visited in chain {
                declared.insert(visited, found.clone());
            }
            found
        };
    let group = |namespace_at: &HashMap<String, i64>, id: Option<&str>| -> i64 {
        match id {
            None => -1,
            Some(id) => namespace_at.get(id).copied().unwrap_or(-1),
        }
    };

    for ns in model.namespaces.values() {
        if !hierarchical && !ns.explicit {
            continue;
        }
        namespace_at.insert(ns.id.clone(), nodes.len() as i64);
        nodes.push(GraphNode {
            id: ns.id.clone(),
            label: if hierarchical {
                ns.label.clone()
            } else {
                ns.id.clone()
            },
            is_group: true,
            shape: NodeShape::Rect,
            parent_id: if hierarchical {
                ns.parent.clone()
            } else {
                None
            },
            parent: -1,
            class_id: None,
            color_index: None,
        });
    }
    for node in nodes.iter_mut() {
        node.parent = group(&namespace_at, node.parent_id.as_deref());
    }

    for (color_index, class_node) in model.classes.values().enumerate() {
        let parent_id = declared_ancestor(&mut declared, class_node.parent.as_deref());
        class_at.insert(class_node.id.clone(), nodes.len() as i64);
        let parent = group(&namespace_at, parent_id.as_deref());
        nodes.push(GraphNode {
            id: class_node.id.clone(),
            label: class_node.label.clone(),
            is_group: false,
            shape: NodeShape::ClassBox,
            parent_id,
            parent,
            class_id: Some(class_node.id.clone()),
            color_index: Some(color_index),
        });
    }

    for note in model.notes.values() {
        let parent_id = declared_ancestor(&mut declared, note.parent.as_deref());
        let target = note
            .class
            .as_deref()
            .and_then(|class| class_at.get(class).copied());
        if let Some(target) = target {
            edges.push(GraphEdge {
                id: format!("edgeNote{}", note.index),
                start: note.id.clone(),
                end: nodes[target as usize].id.clone(),
                source: nodes.len() as i64,
                target,
                label: None,
                arrow_type_start: EndMarker::None,
                arrow_type_end: EndMarker::None,
                start_label_right: String::new(),
                end_label_left: String::new(),
                pattern: Pattern::Dotted,
                classes: "relation".to_owned(),
            });
        }
        let parent = group(&namespace_at, parent_id.as_deref());
        nodes.push(GraphNode {
            id: note.id.clone(),
            label: note.text.clone(),
            is_group: false,
            shape: NodeShape::Note,
            parent_id,
            parent,
            class_id: None,
            color_index: None,
        });
    }

    for item in &model.interfaces {
        let owner = class_at.get(&item.class_id).copied();
        interface_at.insert(item.id.clone(), nodes.len() as i64);
        let parent = match owner {
            None => -1,
            Some(owner) => nodes[owner as usize].parent,
        };
        nodes.push(GraphNode {
            id: item.id.clone(),
            label: item.label.clone(),
            is_group: false,
            shape: NodeShape::Interface,
            parent_id: None,
            parent,
            class_id: None,
            color_index: None,
        });
    }

    for (count, relation) in model.relations.iter().enumerate() {
        let (type1, type2) = (relation.type1, relation.type2);
        let start_is_interface = type1 == Some(RELATION_LOLLIPOP) && type2.is_none();
        let end_is_interface =
            !start_is_interface && type2 == Some(RELATION_LOLLIPOP) && type1.is_none();
        let start_map = if start_is_interface {
            &interface_at
        } else {
            &class_at
        };
        let end_map = if end_is_interface {
            &interface_at
        } else {
            &class_at
        };
        edges.push(GraphEdge {
            id: format!("id_{}_{}_{}", relation.id1, relation.id2, count + 1),
            start: relation.id1.clone(),
            end: relation.id2.clone(),
            source: start_map.get(&relation.id1).copied().unwrap_or(-1),
            target: end_map.get(&relation.id2).copied().unwrap_or(-1),
            label: relation.title.clone(),
            arrow_type_start: marker(type1),
            arrow_type_end: marker(type2),
            start_label_right: if relation.relation_title1 == "none" {
                String::new()
            } else {
                relation.relation_title1.clone()
            },
            end_label_left: if relation.relation_title2 == "none" {
                String::new()
            } else {
                relation.relation_title2.clone()
            },
            pattern: if relation.line_type == 1 {
                Pattern::Dashed
            } else {
                Pattern::Solid
            },
            classes: "relation".to_owned(),
        });
    }

    ClassGraph { nodes, edges }
}
