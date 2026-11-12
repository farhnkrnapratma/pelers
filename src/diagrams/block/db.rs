use std::collections::{HashMap, HashSet};

use super::types::{Block, BlockClassDef, Statement};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::trim;
use crate::util::jsnum::js_max;
use crate::util::ordered::OrderedMap;

const MAX_SPACES: f64 = 100000.0;

pub fn type_str_2_type(type_str: Option<&str>) -> &'static str {
    match type_str {
        Some("[]") => "square",
        Some("()") => "round",
        Some("(())") => "circle",
        Some(">]") => "rect_left_inv_arrow",
        Some("{}") => "diamond",
        Some("{{}}") => "hexagon",
        Some("([])") => "stadium",
        Some("[[]]") => "subroutine",
        Some("[()]") => "cylinder",
        Some("((()))") => "doublecircle",
        Some("[//]") => "lean_right",
        Some("[\\\\]") => "lean_left",
        Some("[/\\]") => "trapezoid",
        Some("[\\/]") => "inv_trapezoid",
        Some("<[]>") => "block_arrow",
        _ => "na",
    }
}

pub fn edge_str_to_edge_data(type_str: &str) -> &'static str {
    match trim(type_str).chars().last() {
        Some('x') => "arrow_cross",
        Some('o') => "arrow_circle",
        Some('>') => "arrow_point",
        _ => "",
    }
}

pub fn edge_str_to_edge_start_data(type_str: &str) -> &'static str {
    match trim(type_str).chars().next() {
        Some('x') => "arrow_cross",
        Some('o') => "arrow_circle",
        Some('<') => "arrow_point",
        _ => "arrow_open",
    }
}

pub fn edge_str_to_thickness(type_str: &str) -> &'static str {
    if type_str.contains("==") {
        "thick"
    } else {
        "normal"
    }
}

pub fn edge_str_to_pattern(type_str: &str) -> &'static str {
    if type_str.contains(".-") {
        "dotted"
    } else {
        "solid"
    }
}

struct Frame {
    list: Vec<usize>,
    at: usize,
    parent: usize,
    children: Vec<usize>,
}

fn flatten(statements: &[Statement]) -> Vec<usize> {
    let mut out = Vec::new();
    for statement in statements {
        match statement {
            Statement::One(block) => out.push(*block),
            Statement::Many(blocks) => out.extend(blocks.iter().copied()),
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockDb {
    pub title: Option<String>,
    pub arena: Vec<Block>,
    pub root: usize,
    pub blocks: Vec<usize>,
    pub edges: Vec<usize>,
    pub classes: OrderedMap<BlockClassDef>,
    pub by_id: OrderedMap<usize>,
    pending: HashSet<String>,
    edge_count: HashMap<String, usize>,
    ids: usize,
    colors: usize,
    spaces: f64,
}

impl Default for BlockDb {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockDb {
    pub fn new() -> Self {
        let root = Block {
            id: Some("root".to_owned()),
            kind: Some("composite".to_owned()),
            columns: Some(-1.0),
            ..Block::default()
        };
        Self {
            title: None,
            arena: vec![root],
            root: 0,
            blocks: Vec::new(),
            edges: Vec::new(),
            classes: OrderedMap::default(),
            by_id: OrderedMap::default(),
            pending: HashSet::new(),
            edge_count: HashMap::new(),
            ids: 0,
            colors: 0,
            spaces: 0.0,
        }
    }

    pub fn add_block(&mut self, block: Block) -> usize {
        self.arena.push(block);
        self.arena.len() - 1
    }

    pub fn generate_id(&mut self) -> String {
        self.ids += 1;
        format!("id-{}", self.ids)
    }

    fn add_style_class(&mut self, id: &str, style_attributes: &str) {
        if !self.classes.contains(id) {
            self.classes.set(
                id,
                BlockClassDef {
                    id: id.to_owned(),
                    styles: Vec::new(),
                    text_styles: Vec::new(),
                },
            );
        }
        let Some(found) = self.classes.get_mut(id) else {
            return;
        };
        for attribute in style_attributes.split(',') {
            let fixed = match attribute.find(';') {
                None => attribute.to_owned(),
                Some(semi) => format!("{}{}", &attribute[..semi], &attribute[semi + 1..]),
            };
            let fixed = trim(&fixed).to_owned();
            if attribute.contains("color") {
                found.text_styles.push(
                    fixed
                        .replacen("fill", "bgFill", 1)
                        .replacen("color", "fill", 1),
                );
            }
            found.styles.push(fixed);
        }
    }

    fn add_style2node(&mut self, id: &str, styles: &str) {
        let block = self.named(id);
        self.arena[block].styles = Some(styles.split(',').map(str::to_owned).collect());
    }

    fn set_css_class(&mut self, item_ids: &str, css_class_name: &str) {
        for id in item_ids.split(',') {
            let block = self.named(trim(id));
            self.arena[block]
                .classes
                .get_or_insert_with(Vec::new)
                .push(css_class_name.to_owned());
        }
    }

    fn named(&mut self, id: &str) -> usize {
        if let Some(&block) = self.by_id.get(id) {
            return block;
        }
        let block = self.add_block(Block {
            id: Some(id.to_owned()),
            kind: Some("na".to_owned()),
            ..Block::default()
        });
        self.by_id.set(id, block);
        self.pending.insert(id.to_owned());
        block
    }

    pub fn set_hierarchy(&mut self, statements: &[Statement]) -> Result<(), PelersError> {
        let root = self.root;
        let mut stack = vec![Frame {
            list: flatten(statements),
            at: 0,
            parent: root,
            children: Vec::new(),
        }];
        while let Some(top) = stack.len().checked_sub(1) {
            if stack[top].at == stack[top].list.len() {
                let finished = stack.pop().expect("the frame is on the stack");
                self.arena[finished.parent].children = finished.children;
                continue;
            }
            let block = stack[top].list[stack[top].at];
            stack[top].at += 1;
            let kind = self.arena[block].kind.clone();
            match kind.as_deref() {
                Some("classDef") => {
                    let id = self.arena[block].id.clone().unwrap_or_default();
                    let css = self.arena[block].css.clone().unwrap_or_default();
                    self.add_style_class(&id, &css);
                    continue;
                }
                Some("applyClass") => {
                    let id = self.arena[block].id.clone().unwrap_or_default();
                    let class = self.arena[block].style_class.clone().unwrap_or_default();
                    self.set_css_class(&id, &class);
                    continue;
                }
                Some("applyStyles") => {
                    let id = self.arena[block].id.clone().unwrap_or_default();
                    if let Some(styles) = self.arena[block]
                        .styles_str
                        .clone()
                        .filter(|s| !s.is_empty())
                    {
                        self.add_style2node(&id, &styles);
                    }
                    continue;
                }
                Some("column-setting") => {
                    let columns = self.arena[block].columns.unwrap_or(-1.0);
                    let parent = stack[top].parent;
                    self.arena[parent].columns = Some(columns);
                    continue;
                }
                Some("edge") => {
                    let id = self.arena[block].id.clone().unwrap_or_default();
                    let count = self.edge_count.get(&id).copied().unwrap_or(0) + 1;
                    self.edge_count.insert(id.clone(), count);
                    self.arena[block].id = Some(format!("{count}-{id}"));
                    self.edges.push(block);
                    continue;
                }
                _ => {}
            }

            if self.arena[block].id.is_none() {
                let generated = self.generate_id();
                self.arena[block].id = Some(generated);
            }
            let id = self.arena[block].id.clone().unwrap_or_default();
            if self.arena[block].label.as_deref().is_none_or(str::is_empty) {
                let composite = kind.as_deref() == Some("composite");
                self.arena[block].label = Some(if composite { String::new() } else { id.clone() });
            }
            let known = self.by_id.get(&id).copied();
            let fresh = known.is_none() || self.pending.remove(&id);
            match known {
                Some(known) if !fresh => {
                    if kind.as_deref() != Some("na") {
                        self.arena[known].kind = kind.clone();
                    }
                    let label = self.arena[block].label.clone();
                    if label.as_deref() != Some(id.as_str()) {
                        self.arena[known].label = label;
                    }
                }
                _ => {
                    if let Some(known) = known {
                        self.arena[block].classes = self.arena[known].classes.clone();
                        self.arena[block].styles = self.arena[known].styles.clone();
                    }
                    if kind.as_deref() == Some("composite") {
                        self.arena[block].color_index = Some(self.colors);
                        self.colors += 1;
                    }
                    self.by_id.set(&id, block);
                }
            }

            let inner = self.arena[block].inner.take();
            self.arena[block].children = Vec::new();
            if kind.as_deref() == Some("space") {
                let width = self.arena[block].width.unwrap_or(1.0);
                if width.is_nan() || width > MAX_SPACES - self.spaces {
                    let mut error = PelersError::new(
                        format!("A block diagram can hold at most {MAX_SPACES} space blocks."),
                        PelersErrorCode::Limit,
                    );
                    error.diagram_type = Some("block".to_owned());
                    return Err(error);
                }
                self.spaces += js_max(width, 0.0);
                let mut j = 0usize;
                while (j as f64) < width {
                    let mut copy = self.arena[block].clone();
                    copy.id = Some(format!("{id}-{j}"));
                    copy.children = Vec::new();
                    let copy_id = copy.id.clone().unwrap_or_default();
                    let at = self.add_block(copy);
                    self.by_id.set(&copy_id, at);
                    stack[top].children.push(at);
                    j += 1;
                }
            } else if fresh {
                stack[top].children.push(block);
            }
            if let Some(inner) = inner.filter(|inner| !inner.is_empty()) {
                stack.push(Frame {
                    list: flatten(&inner),
                    at: 0,
                    parent: block,
                    children: Vec::new(),
                });
            }
        }
        self.blocks = self.arena[root].children.clone();
        Ok(())
    }
}
