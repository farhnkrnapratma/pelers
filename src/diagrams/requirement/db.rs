use std::collections::HashMap;

use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::is_js_space;
use crate::util::ordered::OrderedMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Requirement {
    pub name: String,
    pub type_: String,
    pub requirement_id: String,
    pub text: String,
    pub risk: String,
    pub verify_method: String,
    pub css_styles: Vec<String>,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub name: String,
    pub type_: String,
    pub doc_ref: String,
    pub css_styles: Vec<String>,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Relation {
    pub type_: String,
    pub src: String,
    pub dst: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequirementClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RequirementModel {
    pub direction: String,
    pub requirements: Vec<Requirement>,
    pub elements: Vec<Element>,
    pub relations: Vec<Relation>,
    pub classes: OrderedMap<RequirementClass>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
}

pub const REQUIREMENT_TYPE_REQUIREMENT: &str = "Requirement";
pub const REQUIREMENT_TYPE_FUNCTIONAL: &str = "Functional Requirement";
pub const REQUIREMENT_TYPE_INTERFACE: &str = "Interface Requirement";
pub const REQUIREMENT_TYPE_PERFORMANCE: &str = "Performance Requirement";
pub const REQUIREMENT_TYPE_PHYSICAL: &str = "Physical Requirement";
pub const REQUIREMENT_TYPE_DESIGN: &str = "Design Constraint";

const MAX_STYLE_COPIES: usize = 1_000_000;

#[derive(Clone, Copy)]
enum NodeRef {
    Requirement(usize),
    Element(usize),
}

#[derive(Default)]
pub struct RequirementDb {
    pub direction: String,
    pub requirements: Vec<Requirement>,
    requirement_index: HashMap<String, usize>,
    pub elements: Vec<Element>,
    element_index: HashMap<String, usize>,
    pub relations: Vec<Relation>,
    pub classes: OrderedMap<RequirementClass>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    new_id: String,
    new_text: String,
    new_risk: String,
    new_verify_method: String,
    new_element_type: String,
    new_doc_ref: String,
    style_copies: usize,
}

fn limit_error() -> PelersError {
    let mut error = PelersError::new(
        "Requirement diagram applies too many style declarations.",
        PelersErrorCode::Limit,
    );
    error.diagram_type = Some("requirement".to_owned());
    error
}

impl RequirementDb {
    pub fn new() -> Self {
        RequirementDb {
            direction: "TB".to_owned(),
            ..RequirementDb::default()
        }
    }

    fn copy(&mut self, target: NodeRef, styles: &[String]) -> Result<(), PelersError> {
        self.style_copies += styles.len();
        if self.style_copies > MAX_STYLE_COPIES {
            return Err(limit_error());
        }
        let list = match target {
            NodeRef::Requirement(i) => &mut self.requirements[i].css_styles,
            NodeRef::Element(i) => &mut self.elements[i].css_styles,
        };
        list.extend(styles.iter().cloned());
        Ok(())
    }

    fn node(&self, id: &str) -> Option<NodeRef> {
        if let Some(&i) = self.requirement_index.get(id) {
            return Some(NodeRef::Requirement(i));
        }
        self.element_index.get(id).map(|&i| NodeRef::Element(i))
    }

    fn node_classes_mut(&mut self, node: NodeRef) -> &mut Vec<String> {
        match node {
            NodeRef::Requirement(i) => &mut self.requirements[i].classes,
            NodeRef::Element(i) => &mut self.elements[i].classes,
        }
    }

    pub fn set_direction(&mut self, dir: &str) {
        self.direction = dir.to_owned();
    }

    pub fn add_requirement(&mut self, name: &str, type_: &str) {
        if !self.requirement_index.contains_key(name) {
            self.requirement_index
                .insert(name.to_owned(), self.requirements.len());
            self.requirements.push(Requirement {
                name: name.to_owned(),
                type_: type_.to_owned(),
                requirement_id: self.new_id.clone(),
                text: self.new_text.clone(),
                risk: self.new_risk.clone(),
                verify_method: self.new_verify_method.clone(),
                css_styles: Vec::new(),
                classes: vec!["default".to_owned()],
            });
        }
        self.new_id.clear();
        self.new_text.clear();
        self.new_risk.clear();
        self.new_verify_method.clear();
    }

    pub fn set_new_req_id(&mut self, id: &str) {
        self.new_id = id.to_owned();
    }

    pub fn set_new_req_text(&mut self, text: &str) {
        self.new_text = text.to_owned();
    }

    pub fn set_new_req_risk(&mut self, risk: &str) {
        self.new_risk = risk.to_owned();
    }

    pub fn set_new_req_verify_method(&mut self, verify: &str) {
        self.new_verify_method = verify.to_owned();
    }

    pub fn add_element(&mut self, name: &str) {
        if !self.element_index.contains_key(name) {
            self.element_index
                .insert(name.to_owned(), self.elements.len());
            self.elements.push(Element {
                name: name.to_owned(),
                type_: self.new_element_type.clone(),
                doc_ref: self.new_doc_ref.clone(),
                css_styles: Vec::new(),
                classes: vec!["default".to_owned()],
            });
        }
        self.new_element_type.clear();
        self.new_doc_ref.clear();
    }

    pub fn set_new_element_type(&mut self, type_: &str) {
        self.new_element_type = type_.to_owned();
    }

    pub fn set_new_element_doc_ref(&mut self, doc_ref: &str) {
        self.new_doc_ref = doc_ref.to_owned();
    }

    pub fn add_relationship(&mut self, type_: &str, src: &str, dst: &str) {
        self.relations.push(Relation {
            type_: type_.to_owned(),
            src: src.to_owned(),
            dst: dst.to_owned(),
        });
    }

    pub fn set_css_style(&mut self, ids: &[String], styles: &[String]) -> Result<(), PelersError> {
        for id in ids {
            let Some(node) = self.node(id) else {
                return Ok(());
            };
            for style in styles {
                let pieces: Vec<String> = if style.contains(',') {
                    style.split(',').map(str::to_owned).collect()
                } else {
                    vec![style.clone()]
                };
                self.copy(node, &pieces)?;
            }
        }
        Ok(())
    }

    pub fn set_class(&mut self, ids: &[String], class_names: &[String]) -> Result<(), PelersError> {
        for id in ids {
            let Some(node) = self.node(id) else {
                continue;
            };
            for name in class_names {
                self.node_classes_mut(node).push(name.clone());
                let styles = self.classes.get(name).map(|c| c.styles.clone());
                if let Some(styles) = styles {
                    self.copy(node, &styles)?;
                }
            }
        }
        Ok(())
    }

    pub fn define_class(&mut self, ids: &[String], style: &[String]) -> Result<(), PelersError> {
        for id in ids {
            if self.classes.get(id).is_none() {
                self.classes.set(
                    id,
                    RequirementClass {
                        id: id.clone(),
                        styles: Vec::new(),
                        text_styles: Vec::new(),
                    },
                );
            }
            let mut class = self.classes.get(id).cloned().unwrap_or(RequirementClass {
                id: id.clone(),
                styles: Vec::new(),
                text_styles: Vec::new(),
            });
            for s in style {
                if s.contains("color") {
                    class.text_styles.push(s.replacen("fill", "bgFill", 1));
                }
                class.styles.push(s.clone());
            }
            self.classes.set(id, class);
            let split: Vec<String> = style
                .iter()
                .flat_map(|s| s.split(',').map(str::to_owned))
                .collect();
            for i in 0..self.requirements.len() {
                if self.requirements[i].classes.iter().any(|c| c == id) {
                    self.copy(NodeRef::Requirement(i), &split)?;
                }
            }
            for i in 0..self.elements.len() {
                if self.elements[i].classes.iter().any(|c| c == id) {
                    self.copy(NodeRef::Element(i), &split)?;
                }
            }
        }
        Ok(())
    }

    pub fn into_model(self) -> RequirementModel {
        RequirementModel {
            direction: self.direction,
            requirements: self.requirements,
            elements: self.elements,
            relations: self.relations,
            classes: self.classes,
            title: self.title,
            acc_title: self.acc_title,
            acc_descr: self.acc_descr,
        }
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(text.trim_start_matches(is_js_space).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = Some(collapse_newline_indent(text));
    }
}

fn collapse_newline_indent(text: &str) -> String {
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
