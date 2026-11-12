use super::members::ClassMember;
use crate::util::ordered::OrderedMap;

pub type RelationEnd = Option<u8>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClassNode {
    pub id: String,
    pub type_: String,
    pub label: String,
    pub css_classes: String,
    pub methods: Vec<ClassMember>,
    pub members: Vec<ClassMember>,
    pub annotations: Vec<String>,
    pub styles: Vec<String>,
    pub parent: Option<String>,
    pub link: Option<String>,
    pub link_target: Option<String>,
    pub have_callback: bool,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassRelation {
    pub id1: String,
    pub id2: String,
    pub relation_title1: String,
    pub relation_title2: String,
    pub title: Option<String>,
    pub type1: RelationEnd,
    pub type2: RelationEnd,
    pub line_type: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassNote {
    pub id: String,
    pub class: Option<String>,
    pub text: String,
    pub index: usize,
    pub parent: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassInterface {
    pub id: String,
    pub label: String,
    pub class_id: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NamespaceNode {
    pub id: String,
    pub label: String,
    pub classes: Vec<String>,
    pub notes: Vec<String>,
    pub children: Vec<String>,
    pub parent: Option<String>,
    pub explicit: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleClass {
    pub id: String,
    pub styles: Vec<String>,
    pub text_styles: Vec<String>,
}

pub type Classes = OrderedMap<ClassNode>;
pub type Notes = OrderedMap<ClassNote>;
pub type Namespaces = OrderedMap<NamespaceNode>;
pub type StyleClasses = OrderedMap<StyleClass>;
