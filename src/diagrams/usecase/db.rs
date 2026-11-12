use std::collections::HashMap;

use super::lexer::{line_column, location_text};
use super::types::{
    Actor, ActorType, Animation, BoundaryType, ClassDef, GraphStatement, LabelType,
    MetadataOccurrence, Relationship, RelationshipType, Span, StatementKind, SymbolKind,
    SystemBoundary, UseCase, UseCaseShape, UsecaseJsonNode, UsecaseModel, UsecaseNote, empty_model,
};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::ordered::OrderedMap;
use crate::util::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct DraftLabel {
    pub text: String,
    pub type_: LabelType,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DraftValue {
    Text(String),
    Bool(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftProperty {
    pub key: String,
    pub value: DraftValue,
    pub span: Span,
    pub key_span: Span,
    pub value_span: Span,
}

impl DraftProperty {
    pub fn occurrence(&self) -> MetadataOccurrence {
        MetadataOccurrence {
            key: self.key.clone(),
            span: self.span,
            key_span: self.key_span,
            value_span: self.value_span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftBoundary {
    pub id: String,
    pub label: DraftLabel,
    pub span: Span,
    pub generated: bool,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftElement {
    pub id: String,
    pub label: DraftLabel,
    pub span: Span,
    pub generated: bool,
    pub kind: SymbolKind,
    pub parent: Option<DraftBoundary>,
    pub shape: Option<UseCaseShape>,
    pub metadata: Option<Vec<DraftProperty>>,
    pub stereotype: Option<String>,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftJson {
    pub id: String,
    pub value: Value,
    pub property_order: OrderedMap<Vec<String>>,
    pub span: Span,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftEndpoint {
    pub id: String,
    pub label: DraftLabel,
    pub span: Span,
    pub generated: bool,
    pub declares: bool,
    pub classed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftRelationship {
    pub source: DraftEndpoint,
    pub target: DraftEndpoint,
    pub span: Span,
    pub id: Option<String>,
    pub id_span: Option<Span>,
    pub type_: RelationshipType,
    pub arrow_type: u8,
    pub label: Option<DraftLabel>,
    pub minlen: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftNote {
    pub target: String,
    pub span: Span,
    pub label: DraftLabel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftMetadata {
    pub target: String,
    pub span: Span,
    pub properties: Vec<DraftProperty>,
    pub statement: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftTarget {
    pub id: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftClassAssign {
    pub targets: Vec<DraftTarget>,
    pub classes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftStyle {
    pub target: String,
    pub span: Span,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DraftClassDef {
    pub ids: Vec<String>,
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drafts {
    pub elements: Vec<DraftElement>,
    pub boundaries: Vec<DraftBoundary>,
    pub jsons: Vec<DraftJson>,
    pub relationships: Vec<DraftRelationship>,
    pub notes: Vec<DraftNote>,
    pub metadata: Vec<DraftMetadata>,
    pub classes: Vec<DraftClassAssign>,
    pub styles: Vec<DraftStyle>,
    pub class_defs: Vec<DraftClassDef>,
    pub direction: Option<String>,
    pub acc_title: String,
    pub acc_description: String,
}

#[derive(Clone, Debug)]
struct Origin {
    kind: SymbolKind,
    span: Span,
    generated: bool,
    label: Option<String>,
}

#[derive(Clone, Debug)]
struct ElementState {
    kind: SymbolKind,
    id: String,
    label: DraftLabel,
    span: Span,
    parent: Option<DraftBoundary>,
    shape: Option<UseCaseShape>,
    stereotype: Option<String>,
    stereotype_span: Option<Span>,
    classes: Vec<String>,
    actor_type: Option<ActorType>,
    icon: Option<String>,
    business: Option<bool>,
}

#[derive(Clone, Debug)]
struct BoundaryState {
    boundary: DraftBoundary,
    type_: Option<BoundaryType>,
    members: Vec<String>,
}

#[derive(Clone, Copy, Debug)]
enum Declaration {
    Element(usize),
    Boundary(usize),
    Json(usize),
    EdgeId(usize),
}

fn push_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}

fn actor_type_name(kind: ActorType) -> &'static str {
    match kind {
        ActorType::Normal => "normal",
        ActorType::Hollow => "hollow",
        ActorType::Awesome => "awesome",
        ActorType::Icon => "icon",
    }
}

fn infer_kind(properties: &[DraftProperty]) -> Option<&'static str> {
    let mut possible: Vec<&'static str> = vec!["actor", "usecase", "boundary", "edge"];
    for property in properties {
        let key = property.key.as_str();
        match key {
            "icon" => possible = vec!["actor"],
            "animate" | "animation" => possible = vec!["edge"],
            "type" => {
                let value = match &property.value {
                    DraftValue::Text(text) => text.as_str(),
                    DraftValue::Bool(_) => "",
                };
                possible.clear();
                if value == "rect" || value == "package" {
                    possible.push("boundary");
                } else if value == "normal" || value == "hollow" || value == "awesome" {
                    possible.push("actor");
                }
            }
            "business" => possible.retain(|k| *k != "boundary" && *k != "edge"),
            _ => return None,
        }
    }
    if possible.len() == 1 {
        Some(possible[0])
    } else {
        None
    }
}

fn normalize_description(text: &str) -> String {
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

struct Resolver<'a> {
    src: &'a str,
    symbols: HashMap<String, Origin>,
    elements: OrderedMap<ElementState>,
    boundaries: OrderedMap<BoundaryState>,
    edges: HashMap<String, usize>,
    first: HashMap<String, usize>,
    model: UsecaseModel,
}

impl Resolver<'_> {
    fn at(&self, span: Span) -> String {
        location_text(self.src, span.0, span.1)
    }

    fn error(&self, message: String, span: Span) -> PelersError {
        let (line, column) = line_column(self.src, span.0);
        let mut error = PelersError::new(message, PelersErrorCode::Semantic);
        error.diagram_type = Some("usecase".to_owned());
        error.line = line;
        error.column = column;
        error
    }

    fn fail(&self, message: &str, span: Span) -> PelersError {
        self.error(format!("{message} at {}", self.at(span)), span)
    }

    fn conflict(
        &self,
        message: &str,
        current: Span,
        previous: Span,
        current_label: Option<&str>,
        previous_label: Option<&str>,
    ) -> PelersError {
        let suffix = |label: Option<&str>| match label {
            None => String::new(),
            Some(label) => format!(" (label \"{label}\")"),
        };
        self.error(
            format!(
                "{message} at {}{}; previous declaration at {}{}",
                self.at(current),
                suffix(current_label),
                self.at(previous),
                suffix(previous_label)
            ),
            current,
        )
    }

    fn invalid(&self, id: &str, kind: &str, property: &DraftProperty) -> PelersError {
        self.fail(
            &format!(
                "Metadata property '{}' is invalid for {kind} '{id}'",
                property.key
            ),
            property.key_span,
        )
    }

    fn mention(&mut self, id: &str, offset: usize) {
        match self.first.get(id) {
            Some(&previous) if previous <= offset => {}
            _ => {
                self.first.insert(id.to_owned(), offset);
            }
        }
    }

    fn order_of(&self, id: &str) -> usize {
        self.first.get(id).copied().unwrap_or(0)
    }

    fn register(&mut self, id: &str, kind: SymbolKind, span: Span, generated: bool, label: &str) {
        self.symbols.insert(
            id.to_owned(),
            Origin {
                kind,
                span,
                generated,
                label: Some(label.to_owned()),
            },
        );
    }

    fn claim(
        &self,
        id: &str,
        label: &DraftLabel,
        span: Span,
        generated: bool,
        kind: SymbolKind,
    ) -> Result<(), PelersError> {
        let Some(origin) = self.symbols.get(id) else {
            return Ok(());
        };
        let label_text = if generated {
            Some(label.text.as_str())
        } else {
            None
        };
        let origin_label = if origin.generated {
            origin.label.as_deref()
        } else {
            None
        };
        if origin.kind != kind {
            return Err(self.conflict(
                &format!(
                    "ID '{id}' is declared as both {} and {}",
                    origin.kind.name(),
                    kind.name()
                ),
                span,
                origin.span,
                label_text,
                origin_label,
            ));
        }
        if origin.generated || generated {
            return Err(self.conflict(
                &format!("Generated ID '{id}' collides with another declaration"),
                span,
                origin.span,
                label_text,
                origin_label,
            ));
        }
        Ok(())
    }

    fn unique(&mut self, id: &str, kind: SymbolKind, span: Span) -> Result<(), PelersError> {
        if let Some(previous) = self.symbols.get(id).cloned() {
            return Err(self.conflict(
                &format!(
                    "ID '{id}' is declared more than once ({} and {})",
                    previous.kind.name(),
                    kind.name()
                ),
                span,
                previous.span,
                None,
                if previous.generated {
                    previous.label.as_deref()
                } else {
                    None
                },
            ));
        }
        self.symbols.insert(
            id.to_owned(),
            Origin {
                kind,
                span,
                generated: false,
                label: None,
            },
        );
        Ok(())
    }

    fn actor_property(
        &self,
        state: &mut ElementState,
        property: &DraftProperty,
        replace: bool,
    ) -> Result<(), PelersError> {
        let key = property.key.as_str();
        match (key, &property.value) {
            ("type", DraftValue::Text(value))
                if value == "normal" || value == "hollow" || value == "awesome" =>
            {
                let new_type = match value.as_str() {
                    "normal" => ActorType::Normal,
                    "hollow" => ActorType::Hollow,
                    _ => ActorType::Awesome,
                };
                if !replace
                    && let Some(previous) = state.actor_type
                    && previous != new_type
                {
                    return Err(self.conflict(
                        &format!("Actor '{}' has conflicting {key} metadata", state.id),
                        property.key_span,
                        state.span,
                        None,
                        None,
                    ));
                }
                state.actor_type = Some(new_type);
                Ok(())
            }
            ("icon", DraftValue::Text(value)) => {
                if !replace
                    && let Some(previous) = &state.icon
                    && previous != value
                {
                    return Err(self.conflict(
                        &format!("Actor '{}' has conflicting {key} metadata", state.id),
                        property.key_span,
                        state.span,
                        None,
                        None,
                    ));
                }
                state.icon = Some(value.clone());
                Ok(())
            }
            ("business", DraftValue::Bool(value)) => {
                if !replace
                    && let Some(previous) = state.business
                    && previous != *value
                {
                    return Err(self.conflict(
                        &format!("Actor '{}' has conflicting {key} metadata", state.id),
                        property.key_span,
                        state.span,
                        None,
                        None,
                    ));
                }
                state.business = Some(*value);
                Ok(())
            }
            _ => Err(self.invalid(&state.id, "actor", property)),
        }
    }

    fn element_metadata(
        &self,
        state: &mut ElementState,
        properties: &[DraftProperty],
        replace: bool,
    ) -> Result<(), PelersError> {
        for property in properties {
            if state.kind == SymbolKind::Actor {
                self.actor_property(state, property, replace)?;
            } else if let (true, DraftValue::Bool(value)) =
                (property.key == "business", &property.value)
            {
                if !replace
                    && let Some(previous) = state.business
                    && previous != *value
                {
                    return Err(self.conflict(
                        &format!("Use case '{}' has conflicting business metadata", state.id),
                        property.key_span,
                        state.span,
                        None,
                        None,
                    ));
                }
                state.business = Some(*value);
            } else {
                return Err(self.invalid(&state.id, "usecase", property));
            }
        }
        Ok(())
    }

    fn publish(&mut self, state: &ElementState) {
        if state.kind == SymbolKind::Actor {
            self.model.use_cases.remove(&state.id);
            let styles = self
                .model
                .actors
                .get(&state.id)
                .map(|actor| actor.styles.clone())
                .unwrap_or_default();
            let type_ = if state.icon.is_some() {
                ActorType::Icon
            } else {
                state.actor_type.unwrap_or(ActorType::Normal)
            };
            let actor = Actor {
                id: state.id.clone(),
                label: state.label.text.clone(),
                label_type: state.label.type_,
                type_,
                icon: state.icon.clone(),
                business: state.business.unwrap_or(false),
                stereotype: None,
                parent_id: None,
                classes: state.classes.clone(),
                styles,
            };
            self.model.actors.set(&state.id, actor);
            let element = self
                .model
                .actors
                .get_mut(&state.id)
                .expect("actor just set");
            if let Some(stereotype) = &state.stereotype {
                element.stereotype = Some(stereotype.clone());
            }
            if let Some(parent) = &state.parent {
                element.parent_id = Some(parent.id.clone());
            }
        } else {
            self.model.actors.remove(&state.id);
            let styles = self
                .model
                .use_cases
                .get(&state.id)
                .map(|case| case.styles.clone())
                .unwrap_or_default();
            let element = UseCase {
                id: state.id.clone(),
                label: state.label.text.clone(),
                label_type: state.label.type_,
                shape: state.shape.unwrap_or(UseCaseShape::Ellipse),
                business: state.business.unwrap_or(false),
                stereotype: None,
                parent_id: None,
                classes: state.classes.clone(),
                styles,
            };
            self.model.use_cases.set(&state.id, element);
            let element = self
                .model
                .use_cases
                .get_mut(&state.id)
                .expect("use case just set");
            if let Some(stereotype) = &state.stereotype {
                element.stereotype = Some(stereotype.clone());
            }
            if let Some(parent) = &state.parent {
                element.parent_id = Some(parent.id.clone());
            }
        }
    }

    fn declare_element(&mut self, draft: &DraftElement) -> Result<(), PelersError> {
        self.claim(
            &draft.id,
            &draft.label,
            draft.span,
            draft.generated,
            draft.kind,
        )?;
        let parent = match &draft.parent {
            Some(parent) if !parent.id.is_empty() => Some(parent.clone()),
            _ => None,
        };
        let existing = self.elements.get(&draft.id).cloned();
        let Some(mut existing) = existing else {
            let mut state = ElementState {
                kind: draft.kind,
                id: draft.id.clone(),
                label: draft.label.clone(),
                span: draft.span,
                parent,
                shape: draft.shape,
                stereotype: draft.stereotype.clone(),
                stereotype_span: if draft.stereotype.is_some() {
                    Some(draft.span)
                } else {
                    None
                },
                classes: draft.classes.clone(),
                actor_type: None,
                icon: None,
                business: None,
            };
            if let Some(metadata) = &draft.metadata {
                self.element_metadata(&mut state, metadata, false)?;
            }
            self.elements.set(&draft.id, state.clone());
            let origin_label = draft.label.text.clone();
            self.register(
                &draft.id,
                draft.kind,
                draft.span,
                draft.generated,
                &origin_label,
            );
            return Ok(());
        };
        if existing.label.text != draft.label.text || existing.label.type_ != draft.label.type_ {
            return Err(self.conflict(
                &format!("ID '{}' has conflicting labels", draft.id),
                draft.span,
                existing.span,
                None,
                None,
            ));
        }
        if let (Some(shape), Some(previous)) = (draft.shape, existing.shape)
            && shape != previous
        {
            return Err(self.conflict(
                &format!("Use case '{}' has conflicting shapes", draft.id),
                draft.span,
                existing.span,
                None,
                None,
            ));
        }
        if let (Some(parent), Some(previous)) = (&parent, &existing.parent)
            && !previous.id.is_empty()
            && parent.id != previous.id
        {
            return Err(self.conflict(
                &format!(
                    "Element '{}' belongs to more than one system boundary",
                    draft.id
                ),
                parent.span,
                previous.span,
                None,
                None,
            ));
        }
        if let (Some(stereotype), Some(previous)) = (&draft.stereotype, &existing.stereotype)
            && stereotype != previous
        {
            return Err(self.conflict(
                &format!("Element '{}' has conflicting stereotypes", draft.id),
                draft.span,
                existing.stereotype_span.unwrap_or(existing.span),
                None,
                None,
            ));
        }
        if existing.shape.is_none() {
            existing.shape = draft.shape;
        }
        if existing.parent.is_none() {
            existing.parent = draft.parent.clone();
        }
        if existing.stereotype.is_none() {
            existing.stereotype = draft.stereotype.clone();
        }
        if existing.stereotype_span.is_none() && draft.stereotype.is_some() {
            existing.stereotype_span = Some(draft.span);
        }
        push_unique(&mut existing.classes, &draft.classes);
        if let Some(metadata) = &draft.metadata {
            self.element_metadata(&mut existing, metadata, false)?;
        }
        self.elements.set(&draft.id, existing);
        Ok(())
    }

    fn declare_boundary(&mut self, draft: &DraftBoundary) -> Result<(), PelersError> {
        self.claim(
            &draft.id,
            &draft.label,
            draft.span,
            draft.generated,
            SymbolKind::Boundary,
        )?;
        if let Some(existing) = self.boundaries.get(&draft.id).cloned() {
            if existing.boundary.label.text != draft.label.text
                || existing.boundary.label.type_ != draft.label.type_
            {
                return Err(self.conflict(
                    &format!("Boundary '{}' has conflicting titles", draft.id),
                    draft.span,
                    existing.boundary.span,
                    None,
                    None,
                ));
            }
            let mut state = existing;
            push_unique(&mut state.boundary.classes, &draft.classes);
            self.boundaries.set(&draft.id, state);
            return Ok(());
        }
        let mut boundary = draft.clone();
        boundary.classes = draft.classes.clone();
        self.boundaries.set(
            &draft.id,
            BoundaryState {
                boundary,
                type_: None,
                members: Vec::new(),
            },
        );
        let label = draft.label.text.clone();
        self.register(
            &draft.id,
            SymbolKind::Boundary,
            draft.span,
            draft.generated,
            &label,
        );
        Ok(())
    }

    fn endpoint(&mut self, end: &DraftEndpoint) -> Result<SymbolKind, PelersError> {
        if end.classed && !end.declares {
            return Err(self.fail(
                &format!(
                    "Relationship endpoint '{}' uses ::: without declaring the node",
                    end.id
                ),
                end.span,
            ));
        }
        if let Some(origin) = self.symbols.get(&end.id) {
            return Ok(origin.kind);
        }
        self.elements.set(
            &end.id,
            ElementState {
                kind: SymbolKind::UseCase,
                id: end.id.clone(),
                label: end.label.clone(),
                span: end.span,
                parent: None,
                shape: Some(UseCaseShape::Ellipse),
                stereotype: None,
                stereotype_span: None,
                classes: Vec::new(),
                actor_type: None,
                icon: None,
                business: None,
            },
        );
        let label = end.label.text.clone();
        self.register(
            &end.id,
            SymbolKind::UseCase,
            end.span,
            end.generated,
            &label,
        );
        self.mention(&end.id, end.span.0);
        Ok(SymbolKind::UseCase)
    }

    fn stylable(
        &mut self,
        id: &str,
        span: Span,
    ) -> Result<(&mut Vec<String>, &mut Vec<String>), PelersError> {
        let kind = self.symbols.get(id).map(|origin| origin.kind);
        let missing = self.fail(
            &format!("Class/style target '{id}' is unresolved or anonymous"),
            span,
        );
        let edge_index = self.edges.get(id).copied();
        let model = &mut self.model;
        let target = match kind {
            Some(SymbolKind::Actor) => model
                .actors
                .get_mut(id)
                .map(|a| (&mut a.classes, &mut a.styles)),
            Some(SymbolKind::UseCase) => model
                .use_cases
                .get_mut(id)
                .map(|u| (&mut u.classes, &mut u.styles)),
            Some(SymbolKind::Boundary) => model
                .system_boundaries
                .get_mut(id)
                .map(|b| (&mut b.classes, &mut b.styles)),
            Some(SymbolKind::Json) => model
                .json_nodes
                .get_mut(id)
                .map(|j| (&mut j.classes, &mut j.styles)),
            Some(SymbolKind::Edge) => match edge_index {
                Some(index) => {
                    let relationship = &mut model.relationships[index];
                    Some((&mut relationship.classes, &mut relationship.styles))
                }
                None => None,
            },
            None => None,
        };
        target.ok_or(missing)
    }
}

pub fn resolve_usecase(
    src: &str,
    drafts: &Drafts,
    header_span: Span,
    statements: &mut [GraphStatement],
    title: Option<String>,
) -> Result<UsecaseModel, PelersError> {
    let mut model = empty_model();
    model.statements = Vec::new();
    let mut resolver = Resolver {
        src,
        symbols: HashMap::new(),
        elements: OrderedMap::default(),
        boundaries: OrderedMap::default(),
        edges: HashMap::new(),
        first: HashMap::new(),
        model,
    };

    for relationship in &drafts.relationships {
        resolver.mention(&relationship.source.id, relationship.source.span.0);
        resolver.mention(&relationship.target.id, relationship.target.span.0);
    }
    let mut declarations: Vec<(usize, Declaration)> = Vec::new();
    for (index, draft) in drafts.elements.iter().enumerate() {
        resolver.mention(&draft.id, draft.span.0);
        declarations.push((draft.span.0, Declaration::Element(index)));
    }
    for (index, draft) in drafts.boundaries.iter().enumerate() {
        resolver.mention(&draft.id, draft.span.0);
        declarations.push((draft.span.0, Declaration::Boundary(index)));
    }
    for (index, draft) in drafts.jsons.iter().enumerate() {
        resolver.mention(&draft.id, draft.span.0);
        declarations.push((draft.span.0, Declaration::Json(index)));
    }
    for (index, relationship) in drafts.relationships.iter().enumerate() {
        if let (Some(_), Some(id_span)) = (&relationship.id, relationship.id_span) {
            declarations.push((id_span.0, Declaration::EdgeId(index)));
        }
    }
    declarations.sort_by_key(|(offset, _)| *offset);
    for (_, declaration) in declarations {
        match declaration {
            Declaration::Element(index) => resolver.declare_element(&drafts.elements[index])?,
            Declaration::Boundary(index) => resolver.declare_boundary(&drafts.boundaries[index])?,
            Declaration::Json(index) => {
                let draft = &drafts.jsons[index];
                resolver.unique(&draft.id, SymbolKind::Json, draft.span)?;
            }
            Declaration::EdgeId(index) => {
                let relationship = &drafts.relationships[index];
                if let (Some(id), Some(id_span)) = (&relationship.id, relationship.id_span) {
                    resolver.unique(id, SymbolKind::Edge, id_span)?;
                }
            }
        }
    }

    let element_ids = |resolver: &Resolver| -> Vec<String> {
        let mut ids: Vec<String> = resolver
            .elements
            .iter()
            .map(|(id, _)| id.to_owned())
            .collect();
        ids.sort_by_key(|id| resolver.order_of(id));
        ids
    };
    for id in element_ids(&resolver) {
        let state = resolver.elements.get(&id).cloned().expect("element exists");
        resolver.publish(&state);
    }
    let mut boundary_ids: Vec<String> = resolver
        .boundaries
        .iter()
        .map(|(id, _)| id.to_owned())
        .collect();
    boundary_ids.sort_by_key(|id| resolver.order_of(id));
    for id in &boundary_ids {
        let state = resolver
            .boundaries
            .get(id)
            .cloned()
            .expect("boundary exists");
        resolver.model.system_boundaries.set(
            id,
            SystemBoundary {
                id: state.boundary.id.clone(),
                label: state.boundary.label.text.clone(),
                label_type: state.boundary.label.type_,
                type_: BoundaryType::Rect,
                members: Vec::new(),
                classes: Vec::new(),
                styles: Vec::new(),
            },
        );
    }
    let mut json_order: Vec<usize> = (0..drafts.jsons.len()).collect();
    json_order.sort_by_key(|&index| resolver.order_of(&drafts.jsons[index].id));
    for index in json_order {
        let draft = &drafts.jsons[index];
        resolver.model.json_nodes.set(
            &draft.id,
            UsecaseJsonNode {
                id: draft.id.clone(),
                value: draft.value.clone(),
                property_order: draft.property_order.clone(),
                classes: draft.classes.clone(),
                styles: Vec::new(),
            },
        );
    }

    let mut anonymous = 0usize;
    for draft in &drafts.relationships {
        let source_kind = resolver.endpoint(&draft.source)?;
        let target_kind = resolver.endpoint(&draft.target)?;
        let related = |kind: SymbolKind| {
            matches!(
                kind,
                SymbolKind::Actor | SymbolKind::UseCase | SymbolKind::Json
            )
        };
        if !related(source_kind) {
            let origin_span = resolver
                .symbols
                .get(&draft.source.id)
                .map(|o| o.span)
                .unwrap_or(draft.source.span);
            return Err(resolver.conflict(
                &format!(
                    "Relationship source '{}' cannot be {}",
                    draft.source.id,
                    source_kind.name()
                ),
                draft.source.span,
                origin_span,
                None,
                None,
            ));
        }
        if !related(target_kind) {
            let origin_span = resolver
                .symbols
                .get(&draft.target.id)
                .map(|o| o.span)
                .unwrap_or(draft.target.span);
            return Err(resolver.conflict(
                &format!(
                    "Relationship target '{}' cannot be {}",
                    draft.target.id,
                    target_kind.name()
                ),
                draft.target.span,
                origin_span,
                None,
                None,
            ));
        }
        let plain_use_cases =
            source_kind != SymbolKind::UseCase || target_kind != SymbolKind::UseCase;
        if matches!(
            draft.type_,
            RelationshipType::Include | RelationshipType::Extend
        ) && plain_use_cases
        {
            let name = if draft.type_ == RelationshipType::Include {
                "include"
            } else {
                "extend"
            };
            return Err(resolver.fail(
                &format!("{name} relationship requires use-case endpoints"),
                draft.span,
            ));
        }
        if draft.type_ == RelationshipType::Generalization
            && (source_kind == SymbolKind::Json || source_kind != target_kind)
        {
            return Err(resolver.fail(
                "Generalization requires actor-to-actor or use-case-to-use-case endpoints",
                draft.span,
            ));
        }
        if draft.type_ == RelationshipType::Association
            && (source_kind == SymbolKind::Json || target_kind == SymbolKind::Json)
            && draft.arrow_type > 2
        {
            return Err(resolver.fail(
                &format!(
                    "JSON relationship '{}' to '{}' permits only point, reversed-point, or markerless solid association",
                    draft.source.id, draft.target.id
                ),
                draft.span,
            ));
        }
        let id = match &draft.id {
            Some(id) => id.clone(),
            None => {
                let generated = format!("edge-{anonymous}");
                anonymous += 1;
                generated
            }
        };
        let mut relationship = Relationship {
            id: id.clone(),
            explicit_id: draft.id.is_some(),
            source: draft.source.id.clone(),
            target: draft.target.id.clone(),
            type_: draft.type_,
            arrow_type: draft.arrow_type,
            label: None,
            label_type: None,
            minlen: draft.minlen,
            classes: Vec::new(),
            styles: Vec::new(),
            animate: false,
            animation: None,
        };
        if let Some(label) = &draft.label {
            relationship.label = Some(label.text.clone());
            relationship.label_type = Some(label.type_);
        }
        resolver.model.relationships.push(relationship);
        let index = resolver.model.relationships.len() - 1;
        resolver.edges.insert(id, index);
    }

    let declared_actors = resolver.model.actors.clone();
    let declared_use_cases = resolver.model.use_cases.clone();
    resolver.model.actors = OrderedMap::default();
    resolver.model.use_cases = OrderedMap::default();
    for id in element_ids(&resolver) {
        if let Some(actor) = declared_actors.get(&id) {
            resolver.model.actors.set(&id, actor.clone());
        } else if let Some(use_case) = declared_use_cases.get(&id) {
            resolver.model.use_cases.set(&id, use_case.clone());
        } else {
            let state = resolver.elements.get(&id).cloned().expect("element exists");
            resolver.publish(&state);
        }
    }

    for draft in &drafts.metadata {
        let Some(origin) = resolver.symbols.get(&draft.target).cloned() else {
            let inferred = infer_kind(&draft.properties);
            let suffix = match inferred {
                Some(kind) => format!(" (metadata implies {kind})"),
                None => String::new(),
            };
            return Err(resolver.fail(
                &format!("Metadata target '{}' is unresolved{suffix}", draft.target),
                draft.span,
            ));
        };
        let kind = origin.kind;
        if kind == SymbolKind::Actor || kind == SymbolKind::UseCase {
            let mut state = resolver
                .elements
                .get(&draft.target)
                .cloned()
                .expect("element exists");
            resolver.element_metadata(&mut state, &draft.properties, true)?;
            resolver.elements.set(&draft.target, state);
        } else if kind == SymbolKind::Edge {
            let statement = &mut statements[draft.statement];
            statement.kind = StatementKind::EdgeMetadata;
            statement.edges = Some(vec![super::types::EdgeOccurrence {
                id: draft.target.clone(),
                span: statement.span,
                label_span: None,
                id_span: Some(draft.span),
                metadata: statement.metadata.clone(),
            }]);
            statement.nodes = None;
        }
        if kind == SymbolKind::Actor || kind == SymbolKind::UseCase {
            continue;
        }
        for property in &draft.properties {
            let key = property.key.as_str();
            match (kind, key, &property.value) {
                (SymbolKind::Boundary, "type", DraftValue::Text(value))
                    if value == "rect" || value == "package" =>
                {
                    let kind_value = if value == "rect" {
                        BoundaryType::Rect
                    } else {
                        BoundaryType::Package
                    };
                    if let Some(state) = resolver.boundaries.get_mut(&draft.target) {
                        state.type_ = Some(kind_value);
                    }
                }
                (SymbolKind::Edge, "animate", DraftValue::Bool(value)) => {
                    let index = resolver.edges[&draft.target];
                    resolver.model.relationships[index].animate = *value;
                }
                (SymbolKind::Edge, "animation", DraftValue::Text(value))
                    if value == "fast" || value == "slow" =>
                {
                    let index = resolver.edges[&draft.target];
                    let relationship = &mut resolver.model.relationships[index];
                    relationship.animation = Some(if value == "fast" {
                        Animation::Fast
                    } else {
                        Animation::Slow
                    });
                    relationship.animate = true;
                }
                _ => {
                    return Err(resolver.invalid(&draft.target, kind.name(), property));
                }
            }
        }
    }
    for relationship in resolver.model.relationships.iter_mut() {
        if relationship.animation.is_some() {
            relationship.animate = true;
        }
    }

    for id in element_ids(&resolver) {
        let state = resolver.elements.get(&id).cloned().expect("element exists");
        if state.kind == SymbolKind::Actor {
            if let (Some(_), Some(actor_type)) = (&state.icon, state.actor_type)
                && actor_type != ActorType::Normal
            {
                return Err(resolver.fail(
                    &format!(
                        "Actor '{}' cannot combine icon with type '{}'",
                        state.id,
                        actor_type_name(actor_type)
                    ),
                    state.span,
                ));
            }
            if state.business == Some(true)
                && (state.icon.is_some() || state.actor_type == Some(ActorType::Awesome))
            {
                return Err(resolver.fail(
                    &format!(
                        "Business actor '{}' must use normal or hollow geometry",
                        state.id
                    ),
                    state.span,
                ));
            }
        } else if state.shape == Some(UseCaseShape::Rect) && state.business == Some(true) {
            return Err(resolver.fail(
                &format!(
                    "Rectangular use case '{}' cannot be a business use case",
                    state.id
                ),
                state.span,
            ));
        }
        resolver.publish(&state);
    }

    let mut ordered_elements: Vec<&DraftElement> = drafts.elements.iter().collect();
    ordered_elements.sort_by_key(|draft| draft.span.0);
    for draft in ordered_elements {
        let Some(parent) = &draft.parent else {
            continue;
        };
        if parent.id.is_empty() {
            continue;
        }
        if let Some(state) = resolver.boundaries.get_mut(&parent.id)
            && !state.members.contains(&draft.id)
        {
            state.members.push(draft.id.clone());
        }
    }
    for id in boundary_ids_all(&resolver) {
        let state = resolver
            .boundaries
            .get(&id)
            .cloned()
            .expect("boundary exists");
        if let Some(boundary) = resolver.model.system_boundaries.get_mut(&id) {
            boundary.type_ = state.type_.unwrap_or(BoundaryType::Rect);
            boundary.members = state.members.clone();
            boundary.classes = state.boundary.classes.clone();
        }
    }

    for (notes, draft) in drafts.notes.iter().enumerate() {
        let Some(origin) = resolver.symbols.get(&draft.target).cloned() else {
            return Err(resolver.fail(
                &format!("Note target '{}' is unresolved", draft.target),
                draft.span,
            ));
        };
        if origin.kind != SymbolKind::Actor && origin.kind != SymbolKind::UseCase {
            return Err(resolver.conflict(
                &format!(
                    "Note target '{}' must be an actor or use case, not {}",
                    draft.target,
                    origin.kind.name()
                ),
                draft.span,
                origin.span,
                None,
                None,
            ));
        }
        let id = format!("note-{notes}");
        resolver.model.notes.set(
            &id,
            UsecaseNote {
                id: id.clone(),
                target: draft.target.clone(),
                label: draft.label.text.clone(),
                label_type: draft.label.type_,
            },
        );
    }

    for definition in &drafts.class_defs {
        for id in &definition.ids {
            resolver.model.class_defs.set(
                id,
                ClassDef {
                    id: id.clone(),
                    styles: definition.styles.clone(),
                },
            );
        }
    }
    for assignment in &drafts.classes {
        for target in &assignment.targets {
            let (classes, _) = resolver.stylable(&target.id, target.span)?;
            push_unique(classes, &assignment.classes);
        }
    }
    for assignment in &drafts.styles {
        let (_, styles) = resolver.stylable(&assignment.target, assignment.span)?;
        styles.extend(assignment.styles.iter().cloned());
    }

    let mut model = resolver.model;
    model.direction = drafts.direction.clone().unwrap_or(model.direction);
    model.title = title;
    model.acc_title = drafts.acc_title.clone();
    model.acc_description = drafts.acc_description.clone();
    model.acc_descr = normalize_description(&drafts.acc_description);
    model.source = src.to_owned();
    model.header_span = header_span;
    model.statements = statements.to_vec();
    Ok(model)
}

fn boundary_ids_all(resolver: &Resolver) -> Vec<String> {
    let mut ids: Vec<String> = resolver
        .boundaries
        .iter()
        .map(|(id, _)| id.to_owned())
        .collect();
    ids.sort_by_key(|id| resolver.order_of(id));
    ids
}
