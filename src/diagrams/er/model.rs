use std::collections::{HashMap, HashSet};

use crate::diagrams::common::dedent_descr;
use crate::util::chars::{trim, trim_start};
use crate::util::ordered::OrderedMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cardinality {
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
    OnlyOne,
    MdParent,
}

impl Cardinality {
    pub fn lower(self) -> &'static str {
        match self {
            Self::ZeroOrOne => "zero_or_one",
            Self::ZeroOrMore => "zero_or_more",
            Self::OneOrMore => "one_or_more",
            Self::OnlyOne => "only_one",
            Self::MdParent => "md_parent",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Identification {
    NonIdentifying,
    Identifying,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ErAttribute {
    pub attr_type: String,
    pub name: String,
    pub keys: Vec<String>,
    pub comment: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ErEntity {
    pub id: String,
    pub label: String,
    pub alias: String,
    pub attributes: Vec<ErAttribute>,
    pub css_classes: String,
    pub css_styles: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelSpec {
    pub card_a: Cardinality,
    pub rel_type: Identification,
    pub card_b: Cardinality,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ErRelationship {
    pub entity_a: String,
    pub role_a: String,
    pub entity_b: String,
    pub rel_spec: RelSpec,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ErClassDef {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ErSubgraph {
    pub id: String,
    pub nodes: Vec<String>,
    pub title: String,
    pub classes: Vec<String>,
    pub css_styles: Vec<String>,
    pub dir: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DocItem {
    Text(String),
    Dir(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ErDb {
    pub direction: String,
    pub entities: OrderedMap<ErEntity>,
    pub relationships: Vec<ErRelationship>,
    pub classes: OrderedMap<ErClassDef>,
    pub subgraphs: Vec<ErSubgraph>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    subgraph_lookup: HashMap<String, usize>,
    claimed: HashSet<String>,
}

impl Default for ErDb {
    fn default() -> Self {
        Self {
            direction: "TB".to_owned(),
            entities: OrderedMap::default(),
            relationships: Vec::new(),
            classes: OrderedMap::default(),
            subgraphs: Vec::new(),
            title: None,
            acc_title: None,
            acc_descr: None,
            subgraph_lookup: HashMap::new(),
            claimed: HashSet::new(),
        }
    }
}

impl ErDb {
    pub fn add_entity(&mut self, name: &str, alias: &str) -> String {
        if let Some(entity) = self.entities.get_mut(name) {
            if entity.alias.is_empty() && !alias.is_empty() {
                entity.alias = alias.to_owned();
            }
            return entity.id.clone();
        }
        let id = format!("entity-{name}-{}", self.entities.len());
        self.entities.set(
            name,
            ErEntity {
                id: id.clone(),
                label: name.to_owned(),
                alias: alias.to_owned(),
                attributes: Vec::new(),
                css_classes: "default".to_owned(),
                css_styles: Vec::new(),
            },
        );
        id
    }

    pub fn add_attributes(&mut self, name: &str, attributes: Vec<ErAttribute>) {
        self.add_entity(name, "");
        if let Some(entity) = self.entities.get_mut(name) {
            for attribute in attributes.into_iter().rev() {
                entity.attributes.push(attribute);
            }
        }
    }

    pub fn add_relationship(&mut self, ent_a: &str, role_a: &str, ent_b: &str, rel_spec: RelSpec) {
        let entity_a = if self.subgraph_lookup.contains_key(ent_a) {
            ent_a.to_owned()
        } else {
            self.add_entity(ent_a, "")
        };
        let entity_b = if self.subgraph_lookup.contains_key(ent_b) {
            ent_b.to_owned()
        } else {
            self.add_entity(ent_b, "")
        };
        self.relationships.push(ErRelationship {
            entity_a,
            role_a: role_a.to_owned(),
            entity_b,
            rel_spec,
        });
    }

    pub fn set_direction(&mut self, dir: &str) {
        self.direction = dir.to_owned();
    }

    pub fn add_css_styles(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            if let Some(entity) = self.entities.get_mut(id) {
                entity.css_styles.extend(styles.iter().cloned());
            }
            if let Some(&index) = self.subgraph_lookup.get(id) {
                self.subgraphs[index]
                    .css_styles
                    .extend(styles.iter().cloned());
            }
        }
    }

    pub fn add_class(&mut self, ids: &[String], styles: &[String]) {
        for id in ids {
            if !self.classes.contains(id) {
                self.classes.set(
                    id,
                    ErClassDef {
                        id: id.clone(),
                        styles: Vec::new(),
                        text_styles: Vec::new(),
                    },
                );
            }
            if let Some(def) = self.classes.get_mut(id) {
                for style in styles {
                    if style.contains("color") {
                        def.text_styles.push(style.replacen("fill", "bgFill", 1));
                    }
                    def.styles.push(style.clone());
                }
            }
        }
    }

    pub fn set_class(&mut self, ids: &[String], class_names: &[String]) {
        for id in ids {
            if let Some(entity) = self.entities.get_mut(id) {
                for name in class_names {
                    entity.css_classes.push(' ');
                    entity.css_classes.push_str(name);
                }
            }
            if let Some(&index) = self.subgraph_lookup.get(id) {
                self.subgraphs[index]
                    .classes
                    .extend(class_names.iter().cloned());
            }
        }
    }

    pub fn add_sub_graph(&mut self, id_text: &str, list: &[DocItem], title_text: &str) -> String {
        let id = trim(id_text).to_owned();
        let mut seen: HashSet<&str> = HashSet::new();
        let mut nodes: Vec<String> = Vec::new();
        let mut dir: Option<String> = None;
        for item in list {
            let text = match item {
                DocItem::Dir(value) => {
                    dir = Some(value.clone());
                    continue;
                }
                DocItem::Text(text) => text,
            };
            let trimmed = trim(text);
            if trimmed.is_empty() || !seen.insert(trimmed) {
                continue;
            }
            if !self.claimed.contains(text) {
                nodes.push(text.clone());
            }
        }
        for node in &nodes {
            self.claimed.insert(node.clone());
        }
        self.subgraphs.push(ErSubgraph {
            id: id.clone(),
            nodes,
            title: trim(title_text).to_owned(),
            classes: Vec::new(),
            css_styles: Vec::new(),
            dir,
        });
        self.subgraph_lookup
            .insert(id.clone(), self.subgraphs.len() - 1);
        id
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(trim_start(text).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = Some(dedent_descr(text));
    }
}
