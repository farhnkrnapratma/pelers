use std::collections::HashMap;

use crate::diagrams::common::dedent_descr;
use crate::util::jsnum::js_parse_int;

#[derive(Clone, Debug, PartialEq)]
pub struct Named {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Attr {
    Str(String),
    Named(Named),
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Element {
    pub alias: String,
    pub label: String,
    pub alias_attr: Option<Named>,
    pub label_attr: Option<Named>,
    pub parent: Option<usize>,
    pub kind: String,
    pub node_type: Option<String>,
    pub type_: Option<String>,
    pub descr: Option<String>,
    pub techn: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub bg_color: Option<String>,
    pub font_color: Option<String>,
    pub border_color: Option<String>,
    pub shadowing: Option<String>,
    pub shape: Option<String>,
    pub legend_text: Option<String>,
    pub legend_sprite: Option<String>,
    pub extra: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Rel {
    pub type_: String,
    pub from: String,
    pub to: String,
    pub label: String,
    pub from_attr: Option<Named>,
    pub to_attr: Option<Named>,
    pub label_attr: Option<Named>,
    pub techn: Option<String>,
    pub descr: Option<String>,
    pub sprite: Option<String>,
    pub tags: Option<String>,
    pub link: Option<String>,
    pub text_color: Option<String>,
    pub line_color: Option<String>,
    pub offset_x: Option<f64>,
    pub offset_y: Option<f64>,
    pub extra: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct C4Db {
    pub c4_type: Option<String>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub shapes: Vec<Element>,
    pub boundaries: Vec<Element>,
    pub rels: Vec<Rel>,
    pub shape_in_row: f64,
    pub boundary_in_row: f64,
    shape_index: HashMap<String, usize>,
    boundary_index: HashMap<String, usize>,
    rel_index: HashMap<(String, String), usize>,
    current: usize,
    stack: Vec<usize>,
}

impl Default for C4Db {
    fn default() -> Self {
        Self {
            c4_type: None,
            title: None,
            acc_title: None,
            acc_descr: None,
            shapes: Vec::new(),
            boundaries: vec![Element {
                alias: "global".to_owned(),
                label: "global".to_owned(),
                type_: Some("global".to_owned()),
                ..Element::default()
            }],
            rels: Vec::new(),
            shape_in_row: 4.0,
            boundary_in_row: 2.0,
            shape_index: HashMap::new(),
            boundary_index: HashMap::new(),
            rel_index: HashMap::new(),
            current: 0,
            stack: Vec::new(),
        }
    }
}

const ELEMENT_FIELDS: &[&str] = &[
    "label",
    "type",
    "descr",
    "techn",
    "sprite",
    "tags",
    "link",
    "bgColor",
    "fontColor",
    "borderColor",
    "shadowing",
    "shape",
    "legendText",
    "legendSprite",
];
const REL_FIELDS: &[&str] = &[
    "label",
    "descr",
    "techn",
    "sprite",
    "tags",
    "link",
    "textColor",
    "lineColor",
];

fn put_extra(extra: &mut Vec<(String, String)>, key: &str, value: &str) {
    match extra.iter_mut().find(|(k, _)| k == key) {
        Some(entry) => entry.1 = value.to_owned(),
        None => extra.push((key.to_owned(), value.to_owned())),
    }
}

fn element_field<'a>(el: &'a mut Element, name: &str) -> Option<&'a mut Option<String>> {
    Some(match name {
        "type" => &mut el.type_,
        "descr" => &mut el.descr,
        "techn" => &mut el.techn,
        "sprite" => &mut el.sprite,
        "tags" => &mut el.tags,
        "link" => &mut el.link,
        "bgColor" => &mut el.bg_color,
        "fontColor" => &mut el.font_color,
        "borderColor" => &mut el.border_color,
        "shadowing" => &mut el.shadowing,
        "shape" => &mut el.shape,
        "legendText" => &mut el.legend_text,
        "legendSprite" => &mut el.legend_sprite,
        _ => return None,
    })
}

fn rel_field<'a>(rel: &'a mut Rel, name: &str) -> Option<&'a mut Option<String>> {
    Some(match name {
        "descr" => &mut rel.descr,
        "techn" => &mut rel.techn,
        "sprite" => &mut rel.sprite,
        "tags" => &mut rel.tags,
        "link" => &mut rel.link,
        "textColor" => &mut rel.text_color,
        "lineColor" => &mut rel.line_color,
        _ => return None,
    })
}

fn named_element(el: &mut Element, attr: &Named) {
    let (key, value) = (attr.key.as_str(), attr.value.as_str());
    if key == "label" {
        el.label = value.to_owned();
    } else if ELEMENT_FIELDS.contains(&key) {
        if let Some(slot) = element_field(el, key) {
            *slot = Some(value.to_owned());
        }
    } else {
        put_extra(&mut el.extra, key, value);
    }
}

fn named_rel(rel: &mut Rel, attr: &Named) {
    let (key, value) = (attr.key.as_str(), attr.value.as_str());
    if key == "offsetX" {
        rel.offset_x = Some(js_parse_int(value));
    } else if key == "offsetY" {
        rel.offset_y = Some(js_parse_int(value));
    } else if key == "label" {
        rel.label = value.to_owned();
    } else if REL_FIELDS.contains(&key) {
        if let Some(slot) = rel_field(rel, key) {
            *slot = Some(value.to_owned());
        }
    } else {
        put_extra(&mut rel.extra, key, value);
    }
}

fn put_element(el: &mut Element, field: &str, attr: Option<&Attr>) {
    match attr {
        None => {}
        Some(Attr::Str(value)) => {
            if let Some(slot) = element_field(el, field) {
                *slot = Some(value.clone());
            }
        }
        Some(Attr::Named(named)) => named_element(el, named),
    }
}

fn put_rel(rel: &mut Rel, field: &str, attr: Option<&Attr>) {
    match attr {
        None => {}
        Some(Attr::Str(value)) => {
            if let Some(slot) = rel_field(rel, field) {
                *slot = Some(value.clone());
            }
        }
        Some(Attr::Named(named)) => named_rel(rel, named),
    }
}

fn text_element(el: &mut Element, field: &str, attr: Option<&Attr>) {
    match attr {
        None => {
            if let Some(slot) = element_field(el, field) {
                slot.get_or_insert_with(String::new);
            }
        }
        _ => put_element(el, field, attr),
    }
}

fn text_rel(rel: &mut Rel, field: &str, attr: Option<&Attr>) {
    match attr {
        None => {
            if let Some(slot) = rel_field(rel, field) {
                slot.get_or_insert_with(String::new);
            }
        }
        _ => put_rel(rel, field, attr),
    }
}

fn label_element(el: &mut Element, attr: Option<&Attr>) {
    el.label_attr = None;
    match attr {
        Some(Attr::Named(named)) => {
            el.label = String::new();
            el.label_attr = Some(named.clone());
            named_element(el, named);
        }
        Some(Attr::Str(value)) => el.label = value.clone(),
        None => el.label = String::new(),
    }
}

fn label_rel(rel: &mut Rel, attr: Option<&Attr>) {
    rel.label_attr = None;
    match attr {
        Some(Attr::Named(named)) => {
            rel.label = String::new();
            rel.label_attr = Some(named.clone());
            named_rel(rel, named);
        }
        Some(Attr::Str(value)) => rel.label = value.clone(),
        None => rel.label = String::new(),
    }
}

fn attr_value(attr: Option<&Attr>) -> String {
    match attr {
        Some(Attr::Str(value)) => value.clone(),
        Some(Attr::Named(named)) => named.value.clone(),
        None => String::new(),
    }
}

fn str_at(attrs: &[Attr], at: usize) -> Option<&Attr> {
    attrs.get(at)
}

fn alias_text(attr: Option<&Attr>) -> Option<&str> {
    match attr {
        Some(Attr::Str(value)) => Some(value),
        _ => None,
    }
}

impl C4Db {
    pub fn set_c4_type(&mut self, c4_type: &str) {
        self.c4_type = Some(c4_type.to_owned());
    }

    pub fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_owned());
    }

    pub fn set_acc_description(&mut self, descr: &str) {
        self.acc_descr = Some(dedent_descr(descr));
    }

    fn element_in(
        list: &mut Vec<Element>,
        index: &mut HashMap<String, usize>,
        alias: Option<&Attr>,
        title: Option<&Attr>,
    ) -> usize {
        let found = alias_text(alias).and_then(|a| index.get(a).copied());
        let at = match found {
            Some(at) => at,
            None => {
                list.push(Element::default());
                let at = list.len() - 1;
                match alias {
                    Some(Attr::Str(value)) => {
                        list[at].alias = value.clone();
                        index.insert(value.clone(), at);
                    }
                    Some(Attr::Named(named)) => {
                        list[at].alias_attr = Some(named.clone());
                        named_element(&mut list[at], named);
                    }
                    None => {}
                }
                at
            }
        };
        label_element(&mut list[at], title);
        at
    }

    pub fn add_person_or_system(&mut self, kind: &str, attrs: &[Attr]) {
        let at = Self::element_in(
            &mut self.shapes,
            &mut self.shape_index,
            attrs.first(),
            attrs.get(1),
        );
        text_element(&mut self.shapes[at], "descr", str_at(attrs, 2));
        self.finish_shape(at, kind, attrs, 3);
    }

    pub fn add_container(&mut self, kind: &str, attrs: &[Attr]) {
        let at = Self::element_in(
            &mut self.shapes,
            &mut self.shape_index,
            attrs.first(),
            attrs.get(1),
        );
        text_element(&mut self.shapes[at], "techn", str_at(attrs, 2));
        text_element(&mut self.shapes[at], "descr", str_at(attrs, 3));
        self.finish_shape(at, kind, attrs, 4);
    }

    pub fn add_component(&mut self, kind: &str, attrs: &[Attr]) {
        self.add_container(kind, attrs);
    }

    fn finish_shape(&mut self, at: usize, kind: &str, attrs: &[Attr], slot: usize) {
        let shape = &mut self.shapes[at];
        put_element(shape, "sprite", str_at(attrs, slot));
        put_element(shape, "tags", str_at(attrs, slot + 1));
        put_element(shape, "link", str_at(attrs, slot + 2));
        shape.kind = kind.to_owned();
        shape.parent = Some(self.current);
    }

    fn boundary(&mut self, attrs: &[Attr], fallback: &str) -> usize {
        let at = Self::element_in(
            &mut self.boundaries,
            &mut self.boundary_index,
            attrs.first(),
            attrs.get(1),
        );
        match str_at(attrs, 2) {
            None => self.boundaries[at].type_ = Some(fallback.to_owned()),
            attr => put_element(&mut self.boundaries[at], "type", attr),
        }
        at
    }

    fn open(&mut self, boundary: usize) {
        self.boundaries[boundary].parent = Some(self.current);
        self.stack.push(self.current);
        self.current = boundary;
    }

    pub fn add_person_or_system_boundary(&mut self, attrs: &[Attr], fallback: &str) {
        let at = self.boundary(attrs, fallback);
        put_element(&mut self.boundaries[at], "tags", str_at(attrs, 3));
        put_element(&mut self.boundaries[at], "link", str_at(attrs, 4));
        self.open(at);
    }

    pub fn add_container_boundary(&mut self, attrs: &[Attr]) {
        self.add_person_or_system_boundary(attrs, "container");
    }

    pub fn add_deployment_node(&mut self, node_type: &str, attrs: &[Attr]) {
        let at = self.boundary(attrs, "node");
        let boundary = &mut self.boundaries[at];
        text_element(boundary, "descr", str_at(attrs, 3));
        put_element(boundary, "sprite", str_at(attrs, 4));
        put_element(boundary, "tags", str_at(attrs, 5));
        put_element(boundary, "link", str_at(attrs, 6));
        boundary.node_type = Some(node_type.to_owned());
        self.open(at);
    }

    pub fn pop_boundary_parse_stack(&mut self) {
        self.current = self.stack.pop().unwrap_or(0);
    }

    pub fn add_rel(&mut self, rel_type: &str, attrs: &[Attr]) {
        let (Some(from), Some(to)) = (attrs.first(), attrs.get(1)) else {
            return;
        };
        if attrs.get(2).is_none() {
            return;
        }
        let known = match (from, to) {
            (Attr::Str(a), Attr::Str(b)) => Some((a.clone(), b.clone())),
            _ => None,
        };
        let found = known
            .as_ref()
            .and_then(|key| self.rel_index.get(key).copied());
        let at = match found {
            Some(at) => at,
            None => {
                self.rels.push(Rel::default());
                let at = self.rels.len() - 1;
                if let Some(key) = known {
                    self.rel_index.insert(key, at);
                }
                match from {
                    Attr::Str(value) => self.rels[at].from = value.clone(),
                    Attr::Named(named) => self.rels[at].from_attr = Some(named.clone()),
                }
                match to {
                    Attr::Str(value) => self.rels[at].to = value.clone(),
                    Attr::Named(named) => self.rels[at].to_attr = Some(named.clone()),
                }
                at
            }
        };
        let rel = &mut self.rels[at];
        rel.type_ = rel_type.to_owned();
        label_rel(rel, str_at(attrs, 2));
        text_rel(rel, "techn", str_at(attrs, 3));
        text_rel(rel, "descr", str_at(attrs, 4));
        put_rel(rel, "sprite", str_at(attrs, 5));
        put_rel(rel, "tags", str_at(attrs, 6));
        put_rel(rel, "link", str_at(attrs, 7));
    }

    pub fn find_element(&self, alias: &str) -> Option<(bool, usize)> {
        if let Some(&at) = self.shape_index.get(alias) {
            return Some((true, at));
        }
        self.boundary_index.get(alias).map(|&at| (false, at))
    }

    fn element_mut(&mut self, found: (bool, usize)) -> &mut Element {
        if found.0 {
            &mut self.shapes[found.1]
        } else {
            &mut self.boundaries[found.1]
        }
    }

    pub fn update_el_style(&mut self, attrs: &[Attr]) {
        let Some(Attr::Str(alias)) = attrs.first() else {
            return;
        };
        let Some(found) = self.find_element(alias) else {
            return;
        };
        let el = self.element_mut(found);
        put_element(el, "bgColor", str_at(attrs, 1));
        put_element(el, "fontColor", str_at(attrs, 2));
        put_element(el, "borderColor", str_at(attrs, 3));
        put_element(el, "shadowing", str_at(attrs, 4));
        put_element(el, "shape", str_at(attrs, 5));
        put_element(el, "sprite", str_at(attrs, 6));
        put_element(el, "techn", str_at(attrs, 7));
        put_element(el, "legendText", str_at(attrs, 8));
        put_element(el, "legendSprite", str_at(attrs, 9));
    }

    pub fn update_rel_style(&mut self, attrs: &[Attr]) {
        let (Some(Attr::Str(from)), Some(Attr::Str(to))) = (attrs.first(), attrs.get(1)) else {
            return;
        };
        let Some(&at) = self.rel_index.get(&(from.clone(), to.clone())) else {
            return;
        };
        let rel = &mut self.rels[at];
        put_rel(rel, "textColor", str_at(attrs, 2));
        put_rel(rel, "lineColor", str_at(attrs, 3));
        match str_at(attrs, 4) {
            Some(Attr::Str(x)) => rel.offset_x = Some(js_parse_int(x)),
            Some(Attr::Named(named)) => named_rel(rel, named),
            None => {}
        }
        match str_at(attrs, 5) {
            Some(Attr::Str(y)) => rel.offset_y = Some(js_parse_int(y)),
            Some(Attr::Named(named)) => named_rel(rel, named),
            None => {}
        }
    }

    pub fn update_layout_config(&mut self, attrs: &[Attr]) {
        let count = |attr: Option<&Attr>| js_parse_int(&attr_value(attr));
        let shapes = count(attrs.first());
        let boundaries = count(attrs.get(1));
        if shapes >= 1.0 {
            self.shape_in_row = shapes;
        }
        if boundaries >= 1.0 {
            self.boundary_in_row = boundaries;
        }
    }
}
