use crate::diagrams::common::outline::{Decoration, OutlineDb, parse_outline};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::value::Value;
use crate::util::yaml::parse_yaml;

#[derive(Clone, Debug, PartialEq)]
pub struct KanbanNode {
    pub id: String,
    pub level: usize,
    pub label: String,
    pub icon: Option<String>,
    pub assigned: Option<String>,
    pub ticket: Option<String>,
    pub priority: Option<String>,
    pub css_classes: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KanbanSection {
    pub node: KanbanNode,
    pub items: Vec<KanbanNode>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Last {
    Section(usize),
    Item(usize, usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct KanbanDb {
    pub title: Option<String>,
    pub sections: Vec<KanbanSection>,
    last: Option<Last>,
    section_level: usize,
    misplaced: Option<String>,
}

fn semantic(message: String) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("kanban".to_owned());
    error
}

impl KanbanDb {
    fn node_mut(&mut self, at: Last) -> &mut KanbanNode {
        match at {
            Last::Section(s) => &mut self.sections[s].node,
            Last::Item(s, i) => &mut self.sections[s].items[i],
        }
    }

    fn metadata(node: &mut KanbanNode, shape_data: &str) -> Result<(), PelersError> {
        let yaml = if shape_data.contains('\n') {
            format!("{shape_data}\n")
        } else {
            format!("{{\n{shape_data}\n}}")
        };
        let doc = parse_yaml(&yaml).map_err(|message| {
            let mut error = PelersError::new(
                format!("Metadata of \"{}\" is not valid YAML. {message}", node.id),
                PelersErrorCode::Syntax,
            );
            error.diagram_type = Some("kanban".to_owned());
            error
        })?;
        let Value::Object(doc) = doc else {
            return Ok(());
        };
        let field = |key: &str| {
            doc.get(key)
                .filter(|v| v.is_truthy())
                .map(Value::to_js_string)
        };
        let shape = field("shape").unwrap_or_default();
        if shape != shape.to_lowercase() || shape.contains('_') {
            return Err(semantic(format!(
                "No such shape: {shape}. Shape names should be lowercase."
            )));
        }
        if let Some(label) = field("label") {
            node.label = label;
        }
        if let Some(icon) = field("icon") {
            node.icon = Some(icon);
        }
        if let Some(assigned) = field("assigned") {
            node.assigned = Some(assigned);
        }
        if let Some(ticket) = field("ticket") {
            node.ticket = Some(ticket);
        }
        if let Some(priority) = field("priority") {
            node.priority = Some(priority);
        }
        Ok(())
    }
}

impl OutlineDb for KanbanDb {
    fn add_node(
        &mut self,
        level: usize,
        id: &str,
        descr: &str,
        _kind: i32,
        shape_data: Option<&str>,
    ) -> Result<(), PelersError> {
        let mut node = KanbanNode {
            id: id.to_owned(),
            level,
            label: descr.to_owned(),
            icon: None,
            assigned: None,
            ticket: None,
            priority: None,
            css_classes: None,
        };
        if let Some(shape_data) = shape_data {
            Self::metadata(&mut node, shape_data)?;
        }
        if let Some(label) = &self.misplaced {
            return Err(semantic(format!(
                "Items without section detected, found section (\"{label}\")"
            )));
        }
        if self.sections.is_empty() {
            self.section_level = level;
        }
        if level == self.section_level {
            self.sections.push(KanbanSection {
                node,
                items: Vec::new(),
            });
            self.last = Some(Last::Section(self.sections.len() - 1));
        } else {
            let section = self.sections.len() - 1;
            let items = &mut self.sections[section].items;
            items.push(node.clone());
            self.last = Some(Last::Item(section, items.len() - 1));
            if level < self.section_level {
                self.misplaced = Some(node.label);
            }
        }
        Ok(())
    }

    fn decorate_node(&mut self, decoration: Decoration) {
        let Some(at) = self.last else {
            return;
        };
        let node = self.node_mut(at);
        if let Some(icon) = decoration.icon.filter(|i| !i.is_empty()) {
            node.icon = Some(icon);
        }
        if let Some(class) = decoration.class.filter(|c| !c.is_empty()) {
            node.css_classes = Some(class);
        }
    }
}

pub fn parse(source: &str, title: Option<String>) -> Result<KanbanDb, PelersError> {
    let mut db = KanbanDb {
        title,
        ..KanbanDb::default()
    };
    parse_outline(source, &mut db, true)?;
    Ok(db)
}
