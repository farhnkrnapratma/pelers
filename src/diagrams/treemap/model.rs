use super::parser::{Item, Row, TreemapAst};
use crate::diagrams::common::{acc_descr, acc_title};

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub name: String,
    pub children: Option<Vec<usize>>,
    pub value: Option<f64>,
    pub class_selector: Option<String>,
    pub css_styles: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreemapModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub nodes: Vec<TreeNode>,
    pub roots: Vec<usize>,
}

struct Pending {
    name: String,
    leaf: bool,
    level: usize,
    value: Option<f64>,
    class_selector: Option<String>,
    css_styles: Option<Vec<String>>,
}

fn declarations(text: &str) -> Vec<String> {
    let text = text
        .replace("\\,", "§§§")
        .replace(',', ";")
        .replace("§§§", ",");
    text.split(';').map(str::to_owned).collect()
}

fn hierarchy(items: Vec<Pending>, nodes: &mut Vec<TreeNode>) -> Vec<usize> {
    let mut roots = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for item in items {
        let index = nodes.len();
        nodes.push(TreeNode {
            name: item.name,
            children: if item.leaf { None } else { Some(Vec::new()) },
            value: item.value,
            class_selector: item.class_selector,
            css_styles: item.css_styles,
        });
        while stack.last().is_some_and(|&(_, level)| level >= item.level) {
            stack.pop();
        }
        match stack.last() {
            None => roots.push(index),
            Some(&(parent, _)) => nodes[parent]
                .children
                .get_or_insert_with(Vec::new)
                .push(index),
        }
        if !item.leaf {
            stack.push((index, item.level));
        }
    }
    roots
}

pub fn build_treemap(ast: TreemapAst, title: Option<String>) -> TreemapModel {
    let mut classes: Vec<(String, Vec<String>)> = Vec::new();
    for row in &ast.rows {
        if let Row::ClassDef {
            class_name,
            style_text,
            ..
        } = row
        {
            let at = match classes.iter().position(|(name, _)| name == class_name) {
                Some(at) => at,
                None => {
                    classes.push((class_name.clone(), Vec::new()));
                    classes.len() - 1
                }
            };
            let text = style_text.clone().unwrap_or_default();
            classes[at].1.extend(declarations(&text));
        }
    }

    let mut items: Vec<Pending> = Vec::new();
    for row in &ast.rows {
        let Row::Node { indent, item } = row else {
            continue;
        };
        let (name, leaf, value, selector) = match item {
            Item::Section {
                name,
                class_selector,
            } => (name, false, None, class_selector),
            Item::Leaf {
                name,
                value,
                class_selector,
            } => (name, true, Some(*value), class_selector),
        };
        let styles = selector.as_ref().and_then(|selector| {
            classes
                .iter()
                .find(|(class, _)| class == selector)
                .map(|(_, styles)| styles.clone())
        });
        items.push(Pending {
            name: name.clone(),
            leaf,
            level: indent.unwrap_or(0),
            value,
            class_selector: selector.clone(),
            css_styles: styles.filter(|s| !s.is_empty()),
        });
    }

    let mut nodes = Vec::new();
    let roots = hierarchy(items, &mut nodes);
    TreemapModel {
        title: ast.title.filter(|t| !t.is_empty()).or(title),
        acc_title: acc_title(ast.acc_title),
        acc_descr: acc_descr(ast.acc_descr),
        nodes,
        roots,
    }
}
