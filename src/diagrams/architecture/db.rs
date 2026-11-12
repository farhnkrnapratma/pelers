use std::collections::{HashMap, HashSet};

use super::parser::{ArchitectureAst, Side};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::is_js_space;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Service,
    Junction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchNode {
    pub id: String,
    pub kind: NodeKind,
    pub icon: Option<String>,
    pub icon_text: Option<String>,
    pub title: Option<String>,
    pub in_group: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchGroup {
    pub id: String,
    pub icon: Option<String>,
    pub title: Option<String>,
    pub in_group: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchEdge {
    pub lhs_id: String,
    pub lhs_dir: Side,
    pub lhs_into: bool,
    pub lhs_group: bool,
    pub rhs_id: String,
    pub rhs_dir: Side,
    pub rhs_into: bool,
    pub rhs_group: bool,
    pub title: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Row,
    Column,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Row => "row",
            Direction::Column => "column",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchLayoutHint {
    pub direction: Direction,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArchitectureModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub nodes: Vec<ArchNode>,
    pub groups: Vec<ArchGroup>,
    pub edges: Vec<ArchEdge>,
    pub layout_hints: Vec<ArchLayoutHint>,
}

fn fail(message: String) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("architecture".to_owned());
    error
}

#[derive(Default)]
pub struct ArchitectureDb {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    nodes: Vec<ArchNode>,
    node_index: HashMap<String, usize>,
    groups: Vec<ArchGroup>,
    edges: Vec<ArchEdge>,
    layout_hints: Vec<ArchLayoutHint>,
    registered: HashMap<String, bool>,
}

impl ArchitectureDb {
    fn register(&mut self, kind: &str, id: &str, parent: Option<&str>) -> Result<(), PelersError> {
        if let Some(existing) = self.registered.get(id) {
            let other = if *existing { "group" } else { "node" };
            return Err(fail(format!(
                "The {kind} id [{id}] is already in use by another {other}"
            )));
        }
        if let Some(parent) = parent {
            if id == parent {
                return Err(fail(format!(
                    "The {kind} [{id}] cannot be placed within itself"
                )));
            }
            match self.registered.get(parent) {
                None => {
                    return Err(fail(format!(
                        "The {kind} [{id}]'s parent does not exist. Please make sure the parent is created before this {kind}"
                    )));
                }
                Some(false) => {
                    return Err(fail(format!("The {kind} [{id}]'s parent is not a group")));
                }
                Some(true) => {}
            }
        }
        self.registered.insert(id.to_owned(), kind == "group");
        Ok(())
    }

    fn add_service(&mut self, node: ArchNode) -> Result<(), PelersError> {
        self.register("service", &node.id, node.in_group.as_deref())?;
        self.set_node(node);
        Ok(())
    }

    fn add_junction(&mut self, node: ArchNode) -> Result<(), PelersError> {
        self.register("junction", &node.id, node.in_group.as_deref())?;
        self.set_node(node);
        Ok(())
    }

    fn set_node(&mut self, node: ArchNode) {
        match self.node_index.get(&node.id) {
            Some(&at) => self.nodes[at] = node,
            None => {
                self.node_index.insert(node.id.clone(), self.nodes.len());
                self.nodes.push(node);
            }
        }
    }

    fn add_group(&mut self, group: ArchGroup) -> Result<(), PelersError> {
        self.register("group", &group.id, group.in_group.as_deref())?;
        self.groups.push(group);
        Ok(())
    }

    fn node(&self, id: &str) -> Option<&ArchNode> {
        self.node_index.get(id).map(|&at| &self.nodes[at])
    }

    fn add_edge(&mut self, edge: ArchEdge) -> Result<(), PelersError> {
        let ends = [
            ("left", edge.lhs_id.as_str(), edge.lhs_group),
            ("right", edge.rhs_id.as_str(), edge.rhs_group),
        ];
        for (side, id, _) in ends {
            if !self.registered.contains_key(id) {
                return Err(fail(format!(
                    "The {side}-hand id [{id}] does not yet exist. Please create the service/group before declaring an edge to it."
                )));
            }
        }
        for (side, id, _) in ends {
            if self.node(id).is_none() {
                return Err(fail(format!(
                    "The {side}-hand id [{id}] is a group. Add {{group}} to a service inside it instead."
                )));
            }
        }
        let lhs_parent = self.node(&edge.lhs_id).and_then(|n| n.in_group.clone());
        let rhs_parent = self.node(&edge.rhs_id).and_then(|n| n.in_group.clone());
        if lhs_parent.is_some() && lhs_parent == rhs_parent {
            for (side, id, crosses) in ends {
                if crosses {
                    return Err(fail(format!(
                        "The {side}-hand id [{id}] is modified to traverse the group boundary, but the edge does not pass through two groups."
                    )));
                }
            }
        }
        self.edges.push(edge);
        Ok(())
    }

    fn add_layout_hint(&mut self, hint: ArchLayoutHint) -> Result<(), PelersError> {
        if hint.members.len() < 2 {
            return Err(fail(format!(
                "An align directive requires at least two members; got {}",
                hint.members.len()
            )));
        }
        let mut seen: HashSet<&str> = HashSet::new();
        for id in &hint.members {
            if self.registered.get(id.as_str()) != Some(&false) {
                return Err(fail(format!(
                    "align {} references [{id}], which is not a service or junction",
                    hint.direction.as_str()
                )));
            }
            if !seen.insert(id.as_str()) {
                return Err(fail(format!(
                    "align {} lists [{id}] more than once",
                    hint.direction.as_str()
                )));
            }
        }
        self.layout_hints.push(hint);
        Ok(())
    }
}

fn collapse_indented_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' && chars.peek().is_some_and(|&n| is_js_space(n)) {
            out.push('\n');
            while chars.peek().is_some_and(|&n| is_js_space(n)) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn build_architecture(
    ast: &ArchitectureAst,
    title: Option<&str>,
) -> Result<ArchitectureModel, PelersError> {
    let mut db = ArchitectureDb::default();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title.to_owned());
    }
    if let Some(descr) = ast.acc_descr.as_deref().filter(|t| !t.is_empty()) {
        db.acc_descr = Some(collapse_indented_lines(descr));
    }
    if let Some(acc) = ast.acc_title.as_deref().filter(|t| !t.is_empty()) {
        db.acc_title = Some(acc.trim_start_matches(is_js_space).to_owned());
    }
    if let Some(title) = ast.title.as_deref().filter(|t| !t.is_empty()) {
        db.title = Some(title.to_owned());
    }
    for group in &ast.groups {
        db.add_group(ArchGroup {
            id: group.id.clone(),
            icon: group.icon.clone(),
            title: group.title.clone(),
            in_group: group.in_group.clone(),
        })?;
    }
    for service in &ast.services {
        db.add_service(ArchNode {
            id: service.id.clone(),
            kind: NodeKind::Service,
            icon: service.icon.clone(),
            icon_text: service.icon_text.clone(),
            title: service.title.clone(),
            in_group: service.in_group.clone(),
        })?;
    }
    for junction in &ast.junctions {
        db.add_junction(ArchNode {
            id: junction.id.clone(),
            kind: NodeKind::Junction,
            icon: None,
            icon_text: None,
            title: None,
            in_group: junction.in_group.clone(),
        })?;
    }
    for edge in &ast.edges {
        db.add_edge(ArchEdge {
            lhs_id: edge.lhs_id.clone(),
            lhs_dir: edge.lhs_dir,
            lhs_into: edge.lhs_into,
            lhs_group: edge.lhs_group,
            rhs_id: edge.rhs_id.clone(),
            rhs_dir: edge.rhs_dir,
            rhs_into: edge.rhs_into,
            rhs_group: edge.rhs_group,
            title: edge.title.clone(),
        })?;
    }
    for alignment in &ast.alignments {
        db.add_layout_hint(ArchLayoutHint {
            direction: if alignment.row {
                Direction::Row
            } else {
                Direction::Column
            },
            members: alignment.members.clone(),
        })?;
    }
    Ok(ArchitectureModel {
        title: db.title,
        acc_title: db.acc_title,
        acc_descr: db.acc_descr,
        nodes: db.nodes,
        groups: db.groups,
        edges: db.edges,
        layout_hints: db.layout_hints,
    })
}
