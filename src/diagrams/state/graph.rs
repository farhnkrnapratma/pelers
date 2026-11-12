use std::collections::HashMap;

use super::db::{ClassDef, StateRec};
use super::types::{Desc, StateStmt, Stmt, truthy};
use crate::errors::{PelersError, PelersErrorCode};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateNode {
    pub id: String,
    pub shape: Option<String>,
    pub label: Option<String>,
    pub description: Vec<String>,
    pub css_classes: String,
    pub css_compiled_styles: Vec<String>,
    pub css_styles: Vec<String>,
    pub is_group: bool,
    pub parent_id: Option<String>,
    pub dir: Option<String>,
    pub position: Option<String>,
    pub note_for: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateEdge {
    pub id: String,
    pub start: String,
    pub end: String,
    pub label: String,
    pub classes: String,
    pub dashed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateGraph {
    pub nodes: Vec<StateNode>,
    pub edges: Vec<StateEdge>,
}

#[derive(Clone)]
struct Draft {
    shape: Option<String>,
    description: Desc,
    css_classes: String,
    css_styles: Vec<String>,
    group: bool,
    dir: Option<String>,
}

fn direction(doc: &[Stmt]) -> String {
    doc.iter()
        .rev()
        .find_map(|item| match item {
            Stmt::Dir(value) => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "TB".to_owned())
}

struct Frame<'a> {
    item: Option<&'a StateStmt>,
    doc: &'a [Stmt],
    at: usize,
    alt: bool,
}

struct Builder<'a> {
    states: &'a HashMap<String, StateRec>,
    classes: &'a HashMap<String, ClassDef>,
    nodes: Vec<StateNode>,
    edges: Vec<StateEdge>,
    by_id: HashMap<String, usize>,
    drafts: HashMap<String, Draft>,
    counter: usize,
}

impl Builder<'_> {
    fn insert(&mut self, mut node: StateNode) {
        if node.id.is_empty() || node.id == "</join></fork>" || node.id == "</choice>" {
            return;
        }
        if !node.css_classes.is_empty() {
            let mut compiled: Option<Vec<String>> = None;
            for name in node.css_classes.split(' ') {
                let Some(def) = self.classes.get(name) else {
                    continue;
                };
                compiled
                    .get_or_insert_with(|| node.css_compiled_styles.clone())
                    .extend(def.styles.iter().cloned());
            }
            if let Some(styles) = compiled {
                node.css_compiled_styles = styles;
            }
        }
        match self.by_id.get(&node.id).copied() {
            Some(index) => {
                let existing = &mut self.nodes[index];
                let parent_id = node
                    .parent_id
                    .clone()
                    .or_else(|| existing.parent_id.clone());
                let position = node.position.clone().or_else(|| existing.position.clone());
                let note_for = node.note_for.clone().or_else(|| existing.note_for.clone());
                *existing = StateNode {
                    parent_id,
                    position,
                    note_for,
                    ..node
                };
            }
            None => {
                self.by_id.insert(node.id.clone(), self.nodes.len());
                self.nodes.push(node);
            }
        }
    }

    fn visit(&mut self, parent: Option<&str>, item: &StateStmt, alt: bool) {
        let id = item.id.as_str();
        if id == "root" {
            return;
        }
        let (class_str, styles) = match self.states.get(id) {
            Some(known) => (known.classes.join(" "), known.styles.clone()),
            None => (String::new(), Vec::new()),
        };

        let mut draft = match self.drafts.get(id) {
            Some(draft) => draft.clone(),
            None => {
                let mut shape = Some("rect".to_owned());
                if item.start == Some(true) {
                    shape = Some("stateStart".to_owned());
                } else if item.start == Some(false) {
                    shape = Some("stateEnd".to_owned());
                }
                if item.type_.as_deref() != Some("default") {
                    shape = item.type_.clone();
                }
                Draft {
                    shape,
                    description: Desc::Text(id.to_owned()),
                    css_classes: format!("{class_str} statediagram-state"),
                    css_styles: styles,
                    group: false,
                    dir: None,
                }
            }
        };

        if let Some(incoming) = item.description.clone()
            && truthy(&incoming)
        {
            let current = draft.description.clone();
            draft.description = match current {
                Desc::List(mut list) => {
                    draft.shape = Some("rectWithTitle".to_owned());
                    push_desc(&mut list, incoming);
                    Desc::List(list)
                }
                Desc::Text(current) if !current.is_empty() => {
                    draft.shape = Some("rectWithTitle".to_owned());
                    let mut list = if current == id {
                        Vec::new()
                    } else {
                        vec![current]
                    };
                    push_desc(&mut list, incoming);
                    Desc::List(list)
                }
                Desc::Text(_) => {
                    draft.shape = Some("rect".to_owned());
                    incoming
                }
            };
        }
        if let Desc::List(list) = &draft.description
            && list.len() == 1
            && draft.shape.as_deref() == Some("rectWithTitle")
        {
            draft.shape = Some(
                if draft.group {
                    "roundedWithTitle"
                } else {
                    "rect"
                }
                .to_owned(),
            );
        }

        if !draft.group && item.doc.is_some() {
            draft.group = true;
            draft.dir = Some(direction(item.doc.as_deref().unwrap_or(&[])));
            draft.shape = Some(
                if item.type_.as_deref() == Some("divider") {
                    "divider"
                } else {
                    "roundedWithTitle"
                }
                .to_owned(),
            );
            draft.css_classes = format!(
                "{} statediagram-cluster {}",
                draft.css_classes,
                if alt { "statediagram-cluster-alt" } else { "" }
            );
        }

        let (label, description) = match &draft.description {
            Desc::Text(text) => (Some(text.clone()), Vec::new()),
            Desc::List(list) => (
                list.first().cloned(),
                list.iter().skip(1).cloned().collect(),
            ),
        };
        let label = if draft.shape.as_deref() == Some("divider") {
            Some(String::new())
        } else {
            label
        };
        let mut node = StateNode {
            id: id.to_owned(),
            shape: draft.shape.clone(),
            label,
            description,
            css_classes: draft.css_classes.clone(),
            css_compiled_styles: Vec::new(),
            css_styles: draft.css_styles.clone(),
            is_group: draft.group,
            parent_id: None,
            dir: draft.dir.clone(),
            position: None,
            note_for: None,
        };
        if let Some(parent) = parent.filter(|p| *p != "root") {
            node.parent_id = Some(parent.to_owned());
        }
        self.drafts.insert(id.to_owned(), draft.clone());

        let Some(note) = &item.note else {
            self.insert(node);
            return;
        };
        let aside =
            |aside_id: String, shape: &str, css_classes: String, is_group: bool| StateNode {
                id: aside_id,
                shape: Some(shape.to_owned()),
                label: Some(note.text.clone()),
                description: Vec::new(),
                css_classes,
                css_compiled_styles: Vec::new(),
                css_styles: Vec::new(),
                is_group,
                parent_id: None,
                dir: None,
                position: Some(note.position.clone()),
                note_for: None,
            };
        let note_id = format!("{id}----note-{}", self.counter);
        self.counter += 1;
        let group_id = format!("{id}----parent");
        let mut note_node = aside(note_id, "note", "statediagram-note".to_owned(), false);
        note_node.parent_id = Some(group_id.clone());
        note_node.note_for = Some(id.to_owned());
        self.insert(aside(
            group_id,
            "noteGroup",
            draft.css_classes.clone(),
            true,
        ));
        let note_key = note_node.id.clone();
        self.insert(note_node);
        self.insert(node);
        let left = note.position == "left of";
        let (from, to) = if left {
            (note_key, id.to_owned())
        } else {
            (id.to_owned(), note_key)
        };
        self.edges.push(StateEdge {
            id: format!("{from}-{to}"),
            start: from,
            end: to,
            label: String::new(),
            classes: "transition note-edge".to_owned(),
            dashed: true,
        });
    }
}

fn push_desc(list: &mut Vec<String>, incoming: Desc) {
    match incoming {
        Desc::Text(text) => list.push(text),
        Desc::List(items) => list.extend(items),
    }
}

pub fn build_state_graph(
    root: &[Stmt],
    states: &HashMap<String, StateRec>,
    classes: &HashMap<String, ClassDef>,
) -> Result<StateGraph, PelersError> {
    let mut builder = Builder {
        states,
        classes,
        nodes: Vec::new(),
        edges: Vec::new(),
        by_id: HashMap::new(),
        drafts: HashMap::new(),
        counter: 0,
    };
    let mut stack = vec![Frame {
        item: None,
        doc: root,
        at: 0,
        alt: true,
    }];
    while let Some(frame) = stack.last_mut() {
        let doc = frame.doc;
        if frame.at >= doc.len() {
            stack.pop();
            continue;
        }
        let parent = frame.item;
        let alt = frame.alt;
        let item = &doc[frame.at];
        frame.at += 1;
        let parent_id = parent.map(|p| p.id.as_str());
        match item {
            Stmt::State(state) => {
                builder.visit(parent_id, state, alt);
                if let Some(children) = &state.doc {
                    stack.push(Frame {
                        item: Some(state),
                        doc: children,
                        at: 0,
                        alt: !alt,
                    });
                }
            }
            Stmt::Relation(relation) => {
                builder.visit(parent_id, &relation.state1, alt);
                builder.visit(parent_id, &relation.state2, alt);
                builder.edges.push(StateEdge {
                    id: format!("edge{}", builder.counter),
                    start: relation.state1.id.clone(),
                    end: relation.state2.id.clone(),
                    label: relation.description.clone().unwrap_or_default(),
                    classes: "transition".to_owned(),
                    dashed: false,
                });
                builder.counter += 1;
            }
            _ => {}
        }
    }

    for node in &builder.nodes {
        if node.is_group && !node.description.is_empty() {
            let mut error = PelersError::new(
                format!(
                    "Group nodes can only have label. Remove the additional description for node [{}]",
                    node.id
                ),
                PelersErrorCode::Semantic,
            );
            error.diagram_type = Some("state".to_owned());
            return Err(error);
        }
    }
    Ok(StateGraph {
        nodes: builder.nodes,
        edges: builder.edges,
    })
}
