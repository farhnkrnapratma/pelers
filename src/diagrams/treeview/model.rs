use std::collections::HashMap;

use super::boxdrawing::{line_start, preprocess_box_drawing};
use super::parser::{TreeViewAst, parse_treeview};
use crate::errors::{PelersError, locate};
use crate::util::chars::{is_js_space, utf16_to_byte};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    File,
    Directory,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub id: usize,
    pub level: i64,
    pub name: String,
    pub node_type: NodeType,
    pub icon: Option<String>,
    pub css_class: Option<String>,
    pub description: Option<String>,
    pub children: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeViewModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub nodes: Vec<TreeNode>,
}

impl TreeViewModel {
    pub fn root(&self) -> &TreeNode {
        &self.nodes[0]
    }
}

fn populate(ast: &TreeViewAst) -> Vec<TreeNode> {
    let mut nodes = vec![TreeNode {
        id: 0,
        level: -1,
        name: "/".to_owned(),
        node_type: NodeType::Directory,
        icon: None,
        css_class: None,
        description: None,
        children: Vec::new(),
    }];
    let mut stack = vec![0usize];
    for (position, node) in ast.nodes.iter().enumerate() {
        let count = position + 1;
        let level = node.indent.map_or(0, |indent| indent as i64);
        let (name, node_type) = match node.name.strip_suffix('/') {
            Some(name) => (name, NodeType::Directory),
            None => (node.name.as_str(), NodeType::File),
        };
        let icon = node.icon_annotation.as_ref().map(|raw| {
            if raw.is_empty() {
                "none".to_owned()
            } else {
                raw.clone()
            }
        });
        let css_class = node.class_annotation.clone().filter(|c| !c.is_empty());
        let description = node.desc_annotation.clone().filter(|d| !d.is_empty());
        while stack.last().is_some_and(|&top| level <= nodes[top].level) {
            stack.pop();
        }
        let index = nodes.len();
        nodes.push(TreeNode {
            id: count,
            level,
            name: name.to_owned(),
            node_type,
            icon,
            css_class,
            description,
            children: Vec::new(),
        });
        let parent = *stack.last().unwrap_or(&0);
        nodes[parent].children.push(index);
        stack.push(index);
    }
    nodes
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn remap_error_lines(message: &str, lines: &HashMap<usize, usize>) -> String {
    let bytes = message.as_bytes();
    let mut out = String::with_capacity(message.len());
    let mut at = 0;
    let mut copied = 0;
    while at < bytes.len() {
        let before_ok = at == 0 || !is_word_byte(bytes[at - 1]);
        if before_ok
            && bytes
                .get(at..at + 4)
                .is_some_and(|word| word.eq_ignore_ascii_case(b"line"))
        {
            let mut j = at + 4;
            let ws_start = j;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            let digits_start = j;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let boundary = j == bytes.len() || !is_word_byte(bytes[j]);
            if j > digits_start && j > ws_start && boundary {
                let number: usize = message[digits_start..j].parse().unwrap_or(0);
                if let Some(&original) = lines.get(&number).filter(|&&o| o != 0) {
                    out.push_str(&message[copied..at]);
                    out.push_str(&format!("line {original}"));
                    copied = j;
                }
                at = j;
                continue;
            }
        }
        at += 1;
    }
    out.push_str(&message[copied..]);
    out
}

pub fn read_treeview(source: &str) -> Result<TreeViewAst, PelersError> {
    let preprocessed = preprocess_box_drawing(source)?;
    let error = match parse_treeview(&preprocessed.text) {
        Ok(ast) => return Ok(ast),
        Err(error) => error,
    };
    if preprocessed.text == source {
        return Err(error);
    }
    let Some(&line) = preprocessed.line_map.get(&error.line) else {
        return Err(error);
    };
    let at = if error.column > 0 {
        preprocessed
            .columns
            .get(&error.line)
            .map_or(error.column - 1, |column| column.map(error.column - 1))
    } else {
        0
    };
    let start = line_start(source, line);
    let original_line = source[start..].split('\n').next().unwrap_or("");
    let (written_line, written_column, written_snippet) =
        locate(source, start + utf16_to_byte(original_line, at));

    let head = error.message.find('\n').unwrap_or(error.message.len());
    let rest = &error.message[head..];
    let mut lines = HashMap::new();
    lines.insert(error.line, written_line);
    let remapped = remap_error_lines(&error.message[..head], &lines);
    let tail = if error.snippet.is_empty() {
        rest.to_owned()
    } else {
        rest.replacen(&error.snippet, &written_snippet, 1)
    };
    let mut out = PelersError::new(format!("{remapped}{tail}"), error.code);
    out.diagram_type = error.diagram_type.clone();
    out.line = written_line;
    out.column = if error.column > 0 { written_column } else { 0 };
    out.snippet = if error.snippet.is_empty() {
        String::new()
    } else {
        written_snippet
    };
    Err(out)
}

pub fn build_treeview(ast: &TreeViewAst, title: Option<&str>) -> TreeViewModel {
    let title = match &ast.title {
        Some(title) if !title.is_empty() => Some(title.clone()),
        _ => title.map(str::to_owned),
    };
    TreeViewModel {
        title,
        acc_title: ast
            .acc_title
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| t.trim_start_matches(is_js_space).to_owned()),
        acc_descr: ast
            .acc_descr
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(collapse_indented_lines),
        nodes: populate(ast),
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
