use std::collections::HashMap;

use super::parser::{CynefinAst, DomainName};
use crate::util::chars::is_js_space;

#[derive(Clone, Debug, PartialEq)]
pub struct CynefinItem {
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CynefinDomain {
    pub name: DomainName,
    pub items: Vec<CynefinItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CynefinTransition {
    pub from: DomainName,
    pub to: DomainName,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CynefinModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub domains: HashMap<DomainName, CynefinDomain>,
    pub transitions: Vec<CynefinTransition>,
}

pub fn build_cynefin(ast: &CynefinAst, title: Option<&str>) -> CynefinModel {
    let mut domains = HashMap::new();
    for block in &ast.domains {
        domains.insert(
            block.name,
            CynefinDomain {
                name: block.name,
                items: block
                    .items
                    .iter()
                    .map(|label| CynefinItem {
                        label: label.clone(),
                    })
                    .collect(),
            },
        );
    }
    let transitions = ast
        .transitions
        .iter()
        .filter(|t| t.from != t.to)
        .map(|t| CynefinTransition {
            from: t.from,
            to: t.to,
            label: t.label.clone().filter(|label| !label.is_empty()),
        })
        .collect();
    let title = match &ast.title {
        Some(title) if !title.is_empty() => Some(title.clone()),
        _ => title.map(str::to_owned),
    };
    CynefinModel {
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
        domains,
        transitions,
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
