use crate::diagrams::common::outline::{Decoration, OutlineDb, parse_outline};
use crate::errors::{PelersError, PelersErrorCode};

const SECTIONS: usize = 11;

#[derive(Clone, Debug, PartialEq)]
pub struct MindmapNode {
    pub id: usize,
    pub node_id: String,
    pub level: i64,
    pub descr: String,
    pub kind: i32,
    pub children: Vec<usize>,
    pub is_root: bool,
    pub section: Option<usize>,
    pub class: Option<String>,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MindmapDb {
    pub title: Option<String>,
    pub nodes: Vec<MindmapNode>,
    base_level: i64,
    open: Vec<usize>,
}

impl OutlineDb for MindmapDb {
    fn add_node(
        &mut self,
        level: usize,
        id: &str,
        descr: &str,
        kind: i32,
        _shape_data: Option<&str>,
    ) -> Result<(), PelersError> {
        let is_root = self.nodes.is_empty();
        if is_root {
            self.base_level = level as i64;
        }
        let level = level as i64 - self.base_level;

        let mut depth = self.open.len();
        while depth > 0 && self.nodes[self.open[depth - 1]].level >= level {
            depth -= 1;
        }
        let parent = (depth > 0).then(|| self.open[depth - 1]);
        if parent.is_none() && !is_root {
            let mut error = PelersError::new(
                format!("There can be only one root. No parent could be found for (\"{descr}\")"),
                PelersErrorCode::Semantic,
            );
            error.diagram_type = Some("mindmap".to_owned());
            return Err(error);
        }
        let node_index = self.nodes.len();
        let mut node = MindmapNode {
            id: node_index,
            node_id: id.to_owned(),
            level,
            descr: descr.to_owned(),
            kind,
            children: Vec::new(),
            is_root,
            section: None,
            class: None,
            icon: None,
        };
        if let Some(parent) = parent {
            let parent_node = &mut self.nodes[parent];
            node.section = if parent_node.is_root {
                Some(parent_node.children.len() % SECTIONS)
            } else {
                parent_node.section
            };
            parent_node.children.push(node_index);
        }
        self.open.truncate(depth);
        self.open.push(node_index);
        self.nodes.push(node);
        Ok(())
    }

    fn decorate_node(&mut self, decoration: Decoration) {
        let Some(node) = self.nodes.last_mut() else {
            return;
        };
        if let Some(icon) = decoration.icon.filter(|i| !i.is_empty()) {
            node.icon = Some(icon);
        }
        if let Some(class) = decoration.class.filter(|c| !c.is_empty()) {
            node.class = Some(class);
        }
    }
}

pub fn parse(source: &str, title: Option<String>) -> Result<MindmapDb, PelersError> {
    let mut db = MindmapDb {
        title,
        ..MindmapDb::default()
    };
    parse_outline(source, &mut db, false)?;
    Ok(db)
}
