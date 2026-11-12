use std::collections::{HashMap, HashSet};

use super::model::{ErAttribute, ErDb, Identification};

#[derive(Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub alias: String,
    pub attributes: Vec<ErAttribute>,
    pub is_group: bool,
    pub parent_id: Option<String>,
    pub css_styles: Vec<String>,
    pub css_compiled_styles: Vec<String>,
    pub css_classes: String,
    pub dir: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub id: String,
    pub start: String,
    pub end: String,
    pub label: String,
    pub arrow_type_start: &'static str,
    pub arrow_type_end: &'static str,
    pub dashed: bool,
}

pub struct ErGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

fn compiled<'a>(db: &ErDb, class_names: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut out = Vec::new();
    for name in class_names {
        let Some(def) = db.classes.get(name) else {
            continue;
        };
        out.extend(def.styles.iter().map(|style| style.trim().to_owned()));
        out.extend(def.text_styles.iter().map(|style| style.trim().to_owned()));
    }
    out
}

pub fn build_er_graph(db: &ErDb) -> ErGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let subgraphs = &db.subgraphs;

    let mut parent_of: HashMap<String, String> = HashMap::new();
    let mut subgraph_ids: HashSet<String> = HashSet::new();
    for subgraph in subgraphs.iter().rev() {
        subgraph_ids.insert(subgraph.id.clone());
        for id in &subgraph.nodes {
            parent_of.insert(id.clone(), subgraph.id.clone());
        }
    }

    for subgraph in subgraphs.iter().rev() {
        nodes.push(GraphNode {
            id: subgraph.id.clone(),
            label: subgraph.title.clone(),
            alias: String::new(),
            attributes: Vec::new(),
            is_group: true,
            parent_id: parent_of.get(&subgraph.id).cloned(),
            css_styles: subgraph.css_styles.clone(),
            css_compiled_styles: compiled(db, subgraph.classes.iter().map(String::as_str)),
            css_classes: subgraph.classes.join(" "),
            dir: subgraph.dir.clone(),
        });
    }

    let mut moved: HashMap<String, String> = HashMap::new();
    for (name, entity) in db.entities.iter() {
        if subgraph_ids.contains(name) {
            moved.insert(entity.id.clone(), name.to_owned());
            continue;
        }
        nodes.push(GraphNode {
            id: entity.id.clone(),
            label: entity.label.clone(),
            alias: entity.alias.clone(),
            attributes: entity.attributes.clone(),
            is_group: false,
            parent_id: parent_of.get(name).cloned(),
            css_styles: entity.css_styles.clone(),
            css_compiled_styles: compiled(db, entity.css_classes.split(' ')),
            css_classes: entity.css_classes.clone(),
            dir: None,
        });
    }

    for (index, relationship) in db.relationships.iter().enumerate() {
        let spec = relationship.rel_spec;
        edges.push(GraphEdge {
            id: format!(
                "id_{}_{}_{index}",
                relationship.entity_a, relationship.entity_b
            ),
            start: moved
                .get(&relationship.entity_a)
                .cloned()
                .unwrap_or_else(|| relationship.entity_a.clone()),
            end: moved
                .get(&relationship.entity_b)
                .cloned()
                .unwrap_or_else(|| relationship.entity_b.clone()),
            label: relationship.role_a.clone(),
            arrow_type_start: spec.card_b.lower(),
            arrow_type_end: spec.card_a.lower(),
            dashed: spec.rel_type == Identification::NonIdentifying,
        });
    }

    ErGraph { nodes, edges }
}
