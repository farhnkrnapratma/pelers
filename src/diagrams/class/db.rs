use std::collections::HashMap;

use super::members::ClassMember;
use super::types::{
    ClassInterface, ClassNode, ClassNote, ClassRelation, Classes, NamespaceNode, Namespaces, Notes,
    StyleClass, StyleClasses,
};
use crate::text::entities::decode_entities;
use crate::util::chars::{trim, trim_start};
use crate::util::url::sanitize_url;

pub const RELATION_LOLLIPOP: u8 = 4;

fn split_name(id: &str) -> (String, String) {
    match id.find('~') {
        None | Some(0) => (id.to_owned(), String::new()),
        Some(at) => {
            let end = id[at + 1..].find('~').map_or(id.len(), |e| at + 1 + e);
            (id[..at].to_owned(), id[at + 1..end].to_owned())
        }
    }
}

fn lookup_id(id: &str) -> String {
    if id.as_bytes().first().is_some_and(u8::is_ascii_digit) {
        split_name(&format!("classId-{id}")).0
    } else {
        split_name(id).0
    }
}

fn set_description(text: &str) -> String {
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
    out
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassDb {
    pub direction: String,
    pub classes: Classes,
    pub relations: Vec<ClassRelation>,
    pub notes: Notes,
    pub namespaces: Namespaces,
    pub interfaces: Vec<ClassInterface>,
    pub style_classes: StyleClasses,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    namespace_stack: Vec<String>,
    with_css_class: HashMap<String, Vec<String>>,
}

impl ClassDb {
    pub fn new() -> Self {
        Self {
            direction: "TB".to_owned(),
            classes: Classes::default(),
            relations: Vec::new(),
            notes: Notes::default(),
            namespaces: Namespaces::default(),
            interfaces: Vec::new(),
            style_classes: StyleClasses::default(),
            title: None,
            acc_title: None,
            acc_descr: None,
            namespace_stack: Vec::new(),
            with_css_class: HashMap::new(),
        }
    }

    pub fn add_class(&mut self, id: &str) {
        let (class_name, type_) = split_name(id);
        if self.classes.contains(&class_name) {
            return;
        }
        let node = ClassNode {
            id: class_name.clone(),
            type_,
            label: class_name.clone(),
            css_classes: "default".to_owned(),
            ..ClassNode::default()
        };
        self.classes.set(&class_name, node);
        self.tag(&class_name, "default");
    }

    fn tag(&mut self, node_id: &str, css_class: &str) {
        let tagged = self.with_css_class.entry(css_class.to_owned()).or_default();
        if !tagged.iter().any(|id| id == node_id) {
            tagged.push(node_id.to_owned());
        }
    }

    fn ensure(&mut self, id: &str) -> String {
        self.add_class(id);
        split_name(id).0
    }

    pub fn set_class_label(&mut self, id: &str, label: &str) {
        let key = self.ensure(id);
        if let Some(node) = self.classes.get_mut(&key) {
            node.label = label.to_owned();
        }
    }

    pub fn add_relation(&mut self, mut relation: ClassRelation) {
        if relation.type1 == Some(RELATION_LOLLIPOP) && relation.type2.is_none() {
            self.add_class(&relation.id2);
            relation.id1 = self.add_interface(&relation.id1, &relation.id2);
        } else if relation.type2 == Some(RELATION_LOLLIPOP) && relation.type1.is_none() {
            self.add_class(&relation.id1);
            relation.id2 = self.add_interface(&relation.id2, &relation.id1);
        } else {
            self.add_class(&relation.id1);
            self.add_class(&relation.id2);
        }
        relation.id1 = split_name(&relation.id1).0;
        relation.id2 = split_name(&relation.id2).0;
        relation.relation_title1 = trim(&relation.relation_title1).to_owned();
        relation.relation_title2 = trim(&relation.relation_title2).to_owned();
        self.relations.push(relation);
    }

    fn add_interface(&mut self, label: &str, class_id: &str) -> String {
        let id = format!("interface{}", self.interfaces.len());
        self.interfaces.push(ClassInterface {
            id: id.clone(),
            label: label.to_owned(),
            class_id: class_id.to_owned(),
        });
        id
    }

    pub fn add_annotation(&mut self, class_name: &str, annotation: &str) {
        let key = self.ensure(class_name);
        if let Some(node) = self.classes.get_mut(&key) {
            node.annotations.push(annotation.to_owned());
        }
    }

    pub fn add_member(&mut self, class_name: &str, member: &str) {
        let key = self.ensure(class_name);
        let text = trim(member);
        if text.starts_with("<<") && text.ends_with(">>") {
            let annotation = text[2..text.len() - 2].to_owned();
            if let Some(node) = self.classes.get_mut(&key) {
                node.annotations.push(annotation);
            }
        } else if text.find(')').is_some_and(|at| at > 0) {
            let parsed = ClassMember::new(text, true);
            if let Some(node) = self.classes.get_mut(&key) {
                if parsed.method {
                    node.methods.push(parsed);
                } else {
                    node.members.push(parsed);
                }
            }
        } else if !text.is_empty() {
            let parsed = ClassMember::new(text, false);
            if let Some(node) = self.classes.get_mut(&key) {
                node.members.push(parsed);
            }
        }
    }

    pub fn add_members(&mut self, class_name: &str, members: &[String]) {
        for member in members.iter().rev() {
            self.add_member(class_name, member);
        }
    }

    pub fn add_note(&mut self, text: &str, class_name: Option<&str>) -> String {
        let index = self.notes.len();
        let id = format!("note{index}");
        self.notes.set(
            &id,
            ClassNote {
                id: id.clone(),
                class: class_name.map(str::to_owned),
                text: text.to_owned(),
                index,
                parent: None,
            },
        );
        id
    }

    pub fn cleanup_label(&self, label: &str) -> String {
        trim(label.strip_prefix(':').unwrap_or(label)).to_owned()
    }

    pub fn set_css_class(&mut self, ids: &str, class_name: &str) {
        for id in ids.split(',') {
            let key = lookup_id(id);
            let Some(node) = self.classes.get_mut(&key) else {
                continue;
            };
            node.css_classes.push(' ');
            node.css_classes.push_str(class_name);
            for word in class_name.split(' ') {
                self.tag(&key, word);
            }
        }
    }

    pub fn define_class(&mut self, ids: &[String], style: &[String]) {
        for id in ids {
            if !self.style_classes.contains(id) {
                self.style_classes.set(
                    id,
                    StyleClass {
                        id: id.clone(),
                        ..StyleClass::default()
                    },
                );
            }
            if let Some(style_class) = self.style_classes.get_mut(id) {
                for s in style {
                    if s.contains("color") {
                        style_class
                            .text_styles
                            .push(s.replacen("fill", "bgFill", 1));
                    }
                    style_class.styles.push(s.clone());
                }
            }
            let Some(tagged) = self.with_css_class.get(id).cloned() else {
                continue;
            };
            let parts: Vec<String> = style
                .iter()
                .flat_map(|s| s.split(',').map(str::to_owned))
                .collect();
            for node_id in tagged {
                if let Some(node) = self.classes.get_mut(&node_id) {
                    node.styles.extend(parts.iter().cloned());
                }
            }
        }
    }

    pub fn set_tooltip(&mut self, ids: &str, tooltip: Option<&str>) {
        let Some(tooltip) = tooltip else {
            return;
        };
        for id in ids.split(',') {
            let key = split_name(id).0;
            if let Some(node) = self.classes.get_mut(&key) {
                node.tooltip = Some(tooltip.to_owned());
            }
        }
    }

    pub fn set_link(&mut self, ids: &str, link: &str, target: Option<&str>) {
        let url = trim(&decode_entities(link)).to_owned();
        for id in ids.split(',') {
            let key = lookup_id(id);
            if let Some(node) = self.classes.get_mut(&key) {
                node.link = if url.is_empty() {
                    None
                } else {
                    Some(sanitize_url(&url))
                };
                node.link_target = Some(target.unwrap_or("_blank").to_owned());
            }
        }
        self.set_css_class(ids, "clickable");
    }

    pub fn set_click_event(&mut self, ids: &str) {
        for id in ids.split(',') {
            let key = split_name(id).0;
            if let Some(node) = self.classes.get_mut(&key) {
                node.have_callback = true;
            }
        }
        self.set_css_class(ids, "clickable");
    }

    pub fn set_direction(&mut self, dir: &str) {
        self.direction = dir.to_owned();
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(trim_start(text).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = Some(set_description(text));
    }

    pub fn add_namespace(&mut self, id: &str, label: Option<&str>) -> String {
        let prefix = self
            .namespace_stack
            .last()
            .cloned()
            .filter(|p| !p.is_empty());
        let qualified = match &prefix {
            Some(prefix) => format!("{prefix}.{id}"),
            None => id.to_owned(),
        };
        self.namespace_stack.push(qualified.clone());

        if self.namespaces.contains(&qualified) {
            if let Some(existing) = self.namespaces.get_mut(&qualified) {
                existing.explicit = true;
                if let Some(label) = label.filter(|l| !l.is_empty()) {
                    existing.label = label.to_owned();
                }
            }
            return qualified;
        }

        let mut parent_id = prefix.clone();
        let mut from = prefix.as_ref().map_or(0, |p| p.len() + 1);
        loop {
            let dot = qualified[from..].find('.').map(|d| from + d);
            let leaf = dot.is_none();
            let current = match dot {
                None => qualified.clone(),
                Some(d) => qualified[..d].to_owned(),
            };
            if !self.namespaces.contains(&current) {
                let part = qualified[from..dot.unwrap_or(qualified.len())].to_owned();
                let node_label = match label.filter(|l| leaf && !l.is_empty()) {
                    Some(label) => label.to_owned(),
                    None => part,
                };
                self.namespaces.set(
                    &current,
                    NamespaceNode {
                        id: current.clone(),
                        label: node_label,
                        parent: parent_id.clone(),
                        explicit: leaf,
                        ..NamespaceNode::default()
                    },
                );
            } else if leaf && let Some(node) = self.namespaces.get_mut(&current) {
                node.explicit = true;
            }
            if let Some(parent_key) = parent_id.clone()
                && self.namespaces.contains(&parent_key)
            {
                if let Some(parent) = self.namespaces.get_mut(&parent_key)
                    && !parent.children.contains(&current)
                {
                    parent.children.push(current.clone());
                }
                if let Some(node) = self.namespaces.get_mut(&current)
                    && node.parent.is_none()
                {
                    node.parent = Some(parent_key);
                }
            }
            if leaf {
                return qualified;
            }
            parent_id = Some(current);
            from = dot.unwrap_or(from) + 1;
        }
    }

    pub fn pop_namespace(&mut self) {
        self.namespace_stack.pop();
    }

    pub fn add_classes_to_namespace(
        &mut self,
        id: &str,
        class_names: &[String],
        note_names: &[String],
    ) {
        if !self.namespaces.contains(id) {
            return;
        }
        for name in class_names {
            let class_name = split_name(name).0;
            let Some(node) = self.classes.get_mut(&class_name) else {
                continue;
            };
            node.parent = Some(id.to_owned());
            if let Some(namespace) = self.namespaces.get_mut(id)
                && !namespace.classes.contains(&class_name)
            {
                namespace.classes.push(class_name);
            }
        }
        for name in note_names {
            let Some(note) = self.notes.get_mut(name) else {
                continue;
            };
            note.parent = Some(id.to_owned());
            if let Some(namespace) = self.namespaces.get_mut(id)
                && !namespace.notes.contains(name)
            {
                namespace.notes.push(name.clone());
            }
        }
    }

    pub fn set_css_style(&mut self, id: &str, styles: &[String]) {
        let Some(node) = self.classes.get_mut(id) else {
            return;
        };
        for s in styles {
            node.styles.extend(s.split(',').map(str::to_owned));
        }
    }
}

impl Default for ClassDb {
    fn default() -> Self {
        Self::new()
    }
}
