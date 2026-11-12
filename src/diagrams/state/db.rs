use std::collections::HashMap;

use super::graph::{StateGraph, build_state_graph};
use super::types::{StateStmt, Stmt};
use crate::errors::PelersError;
use crate::util::chars::{trim, trim_start};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateRec {
    pub classes: Vec<String>,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClassDef {
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub url: String,
    pub tooltip: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateDb {
    pub root_doc: Vec<Stmt>,
    pub states: HashMap<String, StateRec>,
    pub classes: HashMap<String, ClassDef>,
    pub links: HashMap<String, Link>,
    pub graph: StateGraph,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    divider_count: usize,
}

impl StateDb {
    pub fn direction(&self) -> String {
        self.root_doc
            .iter()
            .rev()
            .find_map(|item| match item {
                Stmt::Dir(value) => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "TB".to_owned())
    }

    pub fn set_root_doc(&mut self, doc: Vec<Stmt>) -> Result<(), PelersError> {
        self.root_doc = doc;
        self.translate();
        self.extract()
    }

    fn extract(&mut self) -> Result<(), PelersError> {
        self.states.clear();
        self.classes.clear();
        let doc = std::mem::take(&mut self.root_doc);
        for item in &doc {
            match item {
                Stmt::State(state) => self.add_state(trim(&state.id), None),
                Stmt::Relation(relation) => {
                    self.add_state(&relation.state1.id, relation.state1.classes.as_deref());
                    self.add_state(&relation.state2.id, relation.state2.classes.as_deref());
                }
                Stmt::ClassDef { id, classes } => self.add_style_class(trim(id), classes),
                Stmt::Style { id, style_class } => {
                    let styles: Vec<String> = style_class
                        .split(',')
                        .map(|s| trim(&s.replace(';', "")).to_owned())
                        .collect();
                    for key in self.named(trim(id)) {
                        if let Some(state) = self.states.get_mut(&key) {
                            state.styles = styles.clone();
                        }
                    }
                }
                Stmt::ApplyClass { id, style_class } => self.set_css_class(trim(id), style_class),
                Stmt::Click { id, url, tooltip } => {
                    self.links.insert(
                        id.clone(),
                        Link {
                            url: url.clone(),
                            tooltip: tooltip.clone(),
                        },
                    );
                }
                Stmt::Text(_) | Stmt::Dir(_) => {}
            }
        }
        let graph = build_state_graph(&doc, &self.states, &self.classes);
        self.root_doc = doc;
        self.graph = graph?;
        Ok(())
    }

    fn translate(&mut self) {
        let doc = std::mem::take(&mut self.root_doc);
        match self.regions(doc) {
            Ok((mut regions, trailing)) => {
                self.translate_items("root", &mut regions);
                self.root_doc = restore(regions, trailing);
            }
            Err(mut items) => {
                self.translate_items("root", &mut items);
                self.root_doc = items;
            }
        }
    }

    fn translate_items(&mut self, parent: &str, items: &mut [Stmt]) {
        for item in items.iter_mut() {
            match item {
                Stmt::State(state) => self.translate_state(parent, state, true),
                Stmt::Relation(relation) => {
                    self.translate_state(parent, &mut relation.state1, true);
                    self.translate_state(parent, &mut relation.state2, false);
                }
                _ => {}
            }
        }
    }

    fn translate_state(&mut self, parent: &str, state: &mut StateStmt, first: bool) {
        if state.id == "[*]" {
            state.id = format!("{parent}{}", if first { "_start" } else { "_end" });
            state.start = Some(first);
        } else {
            state.id = trim(&state.id).to_owned();
        }
        let Some(doc) = state.doc.take() else {
            return;
        };
        state.doc = Some(match self.regions(doc) {
            Ok((regions, _)) => regions,
            Err(items) => items,
        });
        let id = state.id.clone();
        if let Some(children) = state.doc.as_mut() {
            self.translate_items(&id, children);
        }
    }

    fn regions(&mut self, doc: Vec<Stmt>) -> Result<(Vec<Stmt>, bool), Vec<Stmt>> {
        let mut regions = Vec::new();
        let mut current = Vec::new();
        for item in doc {
            match item {
                Stmt::State(mut divider)
                    if divider.type_.as_deref() == Some("divider") && divider.doc.is_none() =>
                {
                    divider.doc = Some(std::mem::take(&mut current));
                    regions.push(Stmt::State(divider));
                }
                other => current.push(other),
            }
        }
        if regions.is_empty() {
            return Err(current);
        }
        let trailing = !current.is_empty();
        if trailing {
            let id = self.get_divider_id();
            regions.push(Stmt::State(StateStmt {
                id,
                type_: Some("divider".to_owned()),
                doc: Some(current),
                ..StateStmt::default()
            }));
        }
        Ok((regions, trailing))
    }

    fn add_state(&mut self, id: &str, classes: Option<&[String]>) {
        let trimmed = trim(id);
        self.states.entry(trimmed.to_owned()).or_default();
        if let Some(classes) = classes {
            for name in classes {
                self.set_css_class(trimmed, trim(name));
            }
        }
    }

    fn add_style_class(&mut self, id: &str, style_attributes: &str) {
        let found = self.classes.entry(id.to_owned()).or_default();
        if style_attributes.is_empty() {
            return;
        }
        for attrib in style_attributes.split(',') {
            found
                .styles
                .push(trim(&attrib.replacen(';', "", 1)).to_owned());
        }
    }

    fn set_css_class(&mut self, item_ids: &str, name: &str) {
        for key in self.named(item_ids) {
            if let Some(state) = self.states.get_mut(&key) {
                state.classes.push(name.to_owned());
            }
        }
    }

    fn named(&mut self, ids: &str) -> Vec<String> {
        let mut out = Vec::new();
        for id in ids.split(',') {
            if self.states.contains_key(id) {
                out.push(id.to_owned());
            } else {
                self.add_state(id, None);
                out.push(trim(id).to_owned());
            }
        }
        out
    }

    pub fn set_direction(&mut self, dir: &str) {
        for item in self.root_doc.iter_mut() {
            if let Stmt::Dir(value) = item {
                *value = dir.to_owned();
                return;
            }
        }
        self.root_doc.insert(0, Stmt::Dir(dir.to_owned()));
    }

    pub fn get_divider_id(&mut self) -> String {
        self.divider_count += 1;
        format!("divider-id-{}", self.divider_count)
    }

    pub fn trim_colon(&self, text: &str) -> String {
        match text.strip_prefix(':') {
            Some(rest) => trim(rest).to_owned(),
            None => trim(text).to_owned(),
        }
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(trim_start(text).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            out.push(c);
            if c == '\n' {
                while chars
                    .peek()
                    .is_some_and(|&next| crate::util::chars::is_js_space(next))
                {
                    chars.next();
                }
            }
        }
        self.acc_descr = Some(out);
    }
}

fn restore(regions: Vec<Stmt>, trailing: bool) -> Vec<Stmt> {
    let count = regions.len();
    let mut out = Vec::new();
    for (k, region) in regions.into_iter().enumerate() {
        let Stmt::State(mut state) = region else {
            continue;
        };
        out.extend(state.doc.take().unwrap_or_default());
        if !(trailing && k + 1 == count) {
            out.push(Stmt::State(state));
        }
    }
    out
}
