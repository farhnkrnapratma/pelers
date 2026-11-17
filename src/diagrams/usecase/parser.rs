use super::db::{
    DraftBoundary, DraftClassAssign, DraftClassDef, DraftElement, DraftEndpoint, DraftJson,
    DraftLabel, DraftMetadata, DraftNote, DraftProperty, DraftRelationship, DraftStyle,
    DraftTarget, DraftValue, Drafts, resolve_usecase,
};
use super::json::parse_ordered_json;
use super::lexer::{line_column, location_text, tokenize};
use super::tokens::*;
use super::types::{
    GraphStatement, LabelType, MetadataOccurrence, NodeOccurrence, RelationshipType, Span,
    StatementKind, UseCaseShape, UsecaseModel, arrow,
};
use crate::errors::{PelersError, PelersErrorCode};
use std::sync::OnceLock;

const LINE: u8 = 1;
const RELATION: u8 = 2;
const STYLE_PART: u8 = 4;
const FOLLOWS_NAME: u8 = 8;
const BOUNDARY_LINE: u8 = 16;
const CLOSES_LABEL: u8 = 32;
const METADATA_FIRST: u8 = 64;

const ARROWS: &[&str] = &[
    "DEPENDENCY_ARROW",
    "GENERALIZATION",
    "FORWARD_SOLID",
    "BACKWARD_SOLID",
    "MARKERLESS_SOLID",
    "FORWARD_CIRCLE",
    "BACKWARD_CIRCLE",
    "FORWARD_CROSS",
    "BACKWARD_CROSS",
];
const NAMES: &[&str] = &["IDENTIFIER", "PLAIN_STRING", "MARKDOWN_STRING"];
const LABELS: &str = "PLAIN_STRING|MARKDOWN_STRING|LABEL_TEXT";
const STATEMENTS: &[&str] = &[
    "ACC_TITLE_LINE",
    "ACC_DESCR_LINE",
    "ACC_DESCR_BLOCK",
    "DIRECTION",
    "ACTOR",
    "SYSTEM_BOUNDARY",
    "NOTE",
    "JSON_DECLARATION_START",
    "CLASS_DEF",
    "CLASS",
    "STYLE",
];
const STYLE_PARTS: &[&str] = &[
    "WORD",
    "PLAIN_STRING",
    "NUMBER",
    "HASH_COLOR",
    "CSS_IDENTIFIER",
    "CSS_ESCAPED_COMMA",
    "DASH",
    "DOT",
    "PERCENT",
    "CSS_PUNCTUATION",
    "COLON",
    "LPAREN",
    "RPAREN",
    "LBRACKET",
    "RBRACKET",
    "LBRACE",
    "RBRACE",
    "AT",
    "MARKERLESS_SOLID",
];
const FORBIDDEN: [&str; 5] = [
    "allowmixing",
    "newpage",
    "package",
    "rectangle",
    "skinparam",
];

fn token_index(name: &str) -> Option<usize> {
    TOKEN_NAMES.iter().position(|n| *n == name)
}

fn flags_of(t: usize) -> u8 {
    static TABLE: OnceLock<Vec<u8>> = OnceLock::new();
    let table = TABLE.get_or_init(|| (0..TOKEN_NAMES.len()).map(compute_flags).collect());
    table.get(t).copied().unwrap_or_else(|| compute_flags(t))
}

fn compute_flags(t: usize) -> u8 {
    let mut bits = 0u8;
    let mut set = |names: &[&str], bit: u8| {
        for name in names {
            if *name == "WORD" {
                if (USECASE..=IDENTIFIER).contains(&t) {
                    bits |= bit;
                }
            } else if token_index(name) == Some(t) {
                bits |= bit;
            }
        }
    };
    let mut line_names: Vec<&str> = vec!["NEWLINE", "COMMENT"];
    line_names.extend(STATEMENTS.iter().copied());
    line_names.extend(NAMES.iter().copied());
    set(&line_names, LINE);
    let mut relation_names = vec!["IDENTIFIER"];
    relation_names.extend(ARROWS.iter().copied());
    set(&relation_names, RELATION);
    set(STYLE_PARTS, STYLE_PART);
    let mut follows = vec!["STEREOTYPE_START", "CLASS_SEPARATOR", "IDENTIFIER"];
    follows.extend(ARROWS.iter().copied());
    follows.extend(["NEWLINE", "EOF"]);
    set(&follows, FOLLOWS_NAME);
    let mut boundary: Vec<&str> = vec!["NEWLINE", "COMMENT", "ACTOR"];
    boundary.extend(NAMES.iter().copied());
    set(&boundary, BOUNDARY_LINE);
    set(
        &[
            "FORWARD_SOLID",
            "MARKERLESS_SOLID",
            "FORWARD_CIRCLE",
            "FORWARD_CROSS",
        ],
        CLOSES_LABEL,
    );
    set(
        &["NEWLINE", "IDENTIFIER", "PLAIN_STRING", "RBRACE"],
        METADATA_FIRST,
    );
    bits
}

fn flag(t: usize, bit: u8) -> bool {
    flags_of(t) & bit != 0
}

fn is_word(type_: usize) -> bool {
    (USECASE..=IDENTIFIER).contains(&type_)
}

fn is_label_text(type_: usize) -> bool {
    (USECASE..=LABEL_SYMBOL).contains(&type_)
}

fn is_key(type_: usize) -> bool {
    type_ == IDENTIFIER || type_ == PLAIN_STRING
}

fn generated_id(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn statement_sequences() -> String {
    let mut out: Vec<String> = STATEMENTS.iter().map(|s| (*s).to_owned()).collect();
    let mut follows: Vec<&str> = vec!["STEREOTYPE_START", "CLASS_SEPARATOR", "IDENTIFIER"];
    follows.extend(ARROWS.iter().copied());
    follows.extend(["NEWLINE", "EOF"]);
    for name in NAMES {
        for next in ["NEWLINE", "IDENTIFIER", "PLAIN_STRING", "RBRACE"] {
            out.push(format!("{name}, METADATA_START, {next}"));
        }
    }
    for (i, name) in NAMES.iter().enumerate() {
        let nexts: Vec<&str> = if i == 0 {
            let mut v = vec!["LPAREN", "LBRACKET"];
            v.extend(follows.iter().copied());
            v
        } else {
            follows.clone()
        };
        for next in nexts {
            out.push(format!("{name}, {next}"));
        }
    }
    for name in NAMES {
        for next in ["NEWLINE", "IDENTIFIER", "PLAIN_STRING", "RBRACE"] {
            out.push(format!("{name}, METADATA_START, {next}"));
        }
    }
    out.join("|")
}

struct Item {
    id: String,
    label: DraftLabel,
    span: Span,
    generated: bool,
    shape: Option<UseCaseShape>,
    metadata: Option<Vec<DraftProperty>>,
    stereotype: Option<String>,
    stereotype_span: Option<Span>,
    classes: Vec<String>,
    class_spans: Vec<Span>,
    explicit: bool,
    node_span: Span,
}

struct Tail {
    explicit_id: Option<String>,
    explicit_id_span: Option<Span>,
    type_: RelationshipType,
    arrow_type: u8,
    label: Option<DraftLabel>,
    minlen: f64,
    target: Item,
    span: Span,
}

type PResult<T> = Result<T, PelersError>;

struct Parser<'a> {
    src: &'a str,
    units: Vec<u16>,
    types: Vec<usize>,
    starts: Vec<usize>,
    ends: Vec<usize>,
    p: usize,
    last_end: usize,
    drafts: Drafts,
    parent: Option<DraftBoundary>,
    pending_json: Vec<(usize, usize)>,
    edge_count: usize,
    note_count: usize,
}

impl Parser<'_> {
    fn ty(&self, k: usize) -> usize {
        self.types.get(k).copied().unwrap_or(EOF)
    }

    fn text(&self, a: usize, b: usize) -> String {
        String::from_utf16_lossy(&self.units[a.min(self.units.len())..b.min(self.units.len())])
    }

    fn image(&self, k: usize) -> String {
        self.text(self.starts[k], self.ends[k])
    }

    fn span(&self, k: usize) -> Span {
        (self.starts[k], self.ends[k])
    }

    fn fail<T>(&self, detail: &str) -> PResult<T> {
        let (line, column) = line_column(self.src, self.starts[self.p]);
        let at = location_text(self.src, self.starts[self.p], self.ends[self.p]);
        let mut error = PelersError::new(
            format!("Error parsing usecase diagram: {detail} at {at}"),
            PelersErrorCode::Syntax,
        );
        error.diagram_type = Some("usecase".to_owned());
        error.line = line;
        error.column = column;
        Err(error)
    }

    fn mismatch<T>(&self, name: &str) -> PResult<T> {
        let found = self.image(self.p);
        self.fail(&format!(
            "Expecting token of type --> {name} <-- but found --> '{found}' <--"
        ))
    }

    fn expect(&mut self, type_: usize) -> PResult<usize> {
        if self.ty(self.p) != type_ {
            return self.mismatch(TOKEN_NAMES[type_]);
        }
        let at = self.p;
        self.p += 1;
        Ok(at)
    }

    fn no_alternative<T>(&self, expected: &str) -> PResult<T> {
        let list: Vec<String> = expected
            .split('|')
            .enumerate()
            .map(|(i, sequence)| format!("  {}. [{sequence}]", i + 1))
            .collect();
        let found = self.image(self.p);
        self.fail(&format!(
            "Expecting: one of these possible Token sequences:\n{}\nbut found: '{found}'",
            list.join("\n")
        ))
    }

    fn line_end(&mut self) -> PResult<()> {
        self.last_end = self.ends[self.p - 1];
        if self.ty(self.p) == NEWLINE {
            self.p += 1;
        } else if self.ty(self.p) != EOF {
            return self.no_alternative("NEWLINE|EOF");
        }
        Ok(())
    }

    fn content(&self, k: usize) -> Span {
        let trim = match self.ty(k) {
            MARKDOWN_STRING => 2,
            PLAIN_STRING => 1,
            _ => 0,
        };
        (self.starts[k] + trim, self.ends[k] - trim)
    }

    fn token_label(&self, k: usize) -> DraftLabel {
        let at = self.content(k);
        DraftLabel {
            text: self.text(at.0, at.1),
            type_: if self.ty(k) == MARKDOWN_STRING {
                LabelType::Markdown
            } else {
                LabelType::Text
            },
            span: at,
        }
    }

    fn label(&mut self) -> PResult<DraftLabel> {
        let type_ = self.ty(self.p);
        if type_ == PLAIN_STRING || type_ == MARKDOWN_STRING {
            let k = self.p;
            self.p += 1;
            return Ok(self.token_label(k));
        }
        if !is_label_text(type_) {
            return self.no_alternative(LABELS);
        }
        let first = self.p;
        while is_label_text(self.ty(self.p)) {
            self.p += 1;
        }
        let at: Span = (self.starts[first], self.ends[self.p - 1]);
        Ok(DraftLabel {
            text: self.text(at.0, at.1),
            type_: LabelType::Text,
            span: at,
        })
    }

    fn property(&mut self) -> PResult<DraftProperty> {
        let key = self.p;
        if !is_key(self.ty(self.p)) {
            return self.no_alternative("IDENTIFIER|PLAIN_STRING");
        }
        self.p += 1;
        self.expect(COLON)?;
        let value_token = self.p;
        let type_ = self.ty(self.p);
        if !is_key(type_) && type_ != TRUE && type_ != FALSE {
            return self.no_alternative("IDENTIFIER|PLAIN_STRING|TRUE|FALSE");
        }
        self.p += 1;
        let key_span = self.content(key);
        let value_span = self.content(value_token);
        let value = match type_ {
            TRUE => DraftValue::Bool(true),
            FALSE => DraftValue::Bool(false),
            _ => DraftValue::Text(self.text(value_span.0, value_span.1)),
        };
        Ok(DraftProperty {
            key: self.text(key_span.0, key_span.1),
            value,
            span: (self.starts[key], self.ends[value_token]),
            key_span,
            value_span,
        })
    }

    fn separator_ahead(&self) -> bool {
        let a = self.ty(self.p);
        let b = self.ty(self.p + 1);
        let c = self.ty(self.p + 2);
        if a == COMMA {
            return is_key(b) || (b == NEWLINE && (c == NEWLINE || is_key(c)));
        }
        if a != NEWLINE {
            return false;
        }
        b == COMMA || is_key(b) || (b == NEWLINE && (c == NEWLINE || c == COMMA || is_key(c)))
    }

    fn metadata(&mut self) -> PResult<Vec<DraftProperty>> {
        self.expect(METADATA_START)?;
        let mut properties = Vec::new();
        while self.ty(self.p) == NEWLINE {
            self.p += 1;
        }
        if is_key(self.ty(self.p)) {
            properties.push(self.property()?);
            while self.separator_ahead() {
                if self.ty(self.p) == COMMA {
                    self.p += 1;
                } else {
                    while self.ty(self.p) == NEWLINE {
                        self.p += 1;
                    }
                    if self.ty(self.p) == COMMA {
                        self.p += 1;
                    }
                }
                while self.ty(self.p) == NEWLINE {
                    self.p += 1;
                }
                properties.push(self.property()?);
            }
            if self.ty(self.p) == COMMA {
                self.p += 1;
            }
            while self.ty(self.p) == NEWLINE {
                self.p += 1;
            }
        }
        self.expect(RBRACE)?;
        Ok(properties)
    }

    fn class_suffix(&mut self, classes: &mut Vec<String>, spans: &mut Vec<Span>) -> PResult<()> {
        loop {
            self.p += 1;
            let id = self.expect(IDENTIFIER)?;
            classes.push(self.image(id));
            spans.push(self.span(id));
            if self.ty(self.p) != COMMA {
                break;
            }
        }
        Ok(())
    }

    fn name(&mut self, brackets: bool) -> PResult<Item> {
        let k = self.p;
        let type_ = self.ty(self.p);
        let mut item = Item {
            id: String::new(),
            label: DraftLabel {
                text: String::new(),
                type_: LabelType::Text,
                span: (0, 0),
            },
            span: self.content(k),
            generated: type_ != IDENTIFIER,
            shape: None,
            metadata: None,
            stereotype: None,
            stereotype_span: None,
            classes: Vec::new(),
            class_spans: Vec::new(),
            explicit: false,
            node_span: (0, 0),
        };
        if type_ == IDENTIFIER {
            self.p += 1;
            item.id = self.image(k);
            let open = self.ty(self.p);
            if open == LPAREN || (brackets && open == LBRACKET) {
                self.p += 1;
                item.label = self.label()?;
                let close = if open == LPAREN { RPAREN } else { RBRACKET };
                self.expect(close)?;
                item.shape = Some(if open == LPAREN {
                    UseCaseShape::Ellipse
                } else {
                    UseCaseShape::Rect
                });
            } else {
                item.label = self.token_label(k);
            }
        } else if type_ == PLAIN_STRING || type_ == MARKDOWN_STRING {
            self.p += 1;
            item.label = self.token_label(k);
            item.id = generated_id(&item.label.text);
        } else {
            return self.no_alternative("IDENTIFIER|PLAIN_STRING|MARKDOWN_STRING");
        }
        Ok(item)
    }

    fn element(&mut self, actor: bool) -> PResult<Item> {
        let first = self.p;
        let mut item = self.name(!actor)?;
        if actor {
            item.shape = None;
        }
        if self.ty(self.p) == METADATA_START {
            item.metadata = Some(self.metadata()?);
        }
        if self.ty(self.p) == STEREOTYPE_START {
            self.p += 1;
            let text = self.expect(STEREOTYPE_TEXT)?;
            self.expect(STEREOTYPE_END)?;
            item.stereotype = Some(self.image(text).trim().to_owned());
            item.stereotype_span = Some(self.span(text));
        }
        if self.ty(self.p) == CLASS_SEPARATOR {
            let mut classes = std::mem::take(&mut item.classes);
            let mut spans = std::mem::take(&mut item.class_spans);
            self.class_suffix(&mut classes, &mut spans)?;
            item.classes = classes;
            item.class_spans = spans;
        }
        item.explicit =
            item.shape.is_some() || item.metadata.is_some() || item.stereotype.is_some();
        item.node_span = (self.starts[first], self.ends[self.p - 1]);
        Ok(item)
    }

    fn labeled_right(&self, closers: u8) -> bool {
        let t = self.ty(self.p);
        if !(MARKDOWN_STRING_RANGE_START..=MARKDOWN_STRING).contains(&t) {
            return false;
        }
        let mut k = self.p + 1;
        loop {
            let type_ = self.ty(k);
            if type_ == MARKERLESS_SOLID || flag_closers(type_, closers) {
                return true;
            }
            if !(USECASE..=MARKDOWN_STRING).contains(&type_) {
                return false;
            }
            k += 1;
        }
    }

    fn minimum_length(&self, k: usize) -> f64 {
        let dashes = (self.ends[k] - self.starts[k]) as i64
            - if self.ty(k) == MARKERLESS_SOLID { 0 } else { 1 };
        f64::max(1.0, (dashes - 1) as f64)
    }

    fn relation_tail(&mut self) -> PResult<Tail> {
        let first = self.p;
        let mut tail = Tail {
            explicit_id: None,
            explicit_id_span: None,
            type_: RelationshipType::Association,
            arrow_type: arrow::SOLID_ARROW,
            label: None,
            minlen: 1.0,
            target: Item {
                id: String::new(),
                label: DraftLabel {
                    text: String::new(),
                    type_: LabelType::Text,
                    span: (0, 0),
                },
                span: (0, 0),
                generated: false,
                shape: None,
                metadata: None,
                stereotype: None,
                stereotype_span: None,
                classes: Vec::new(),
                class_spans: Vec::new(),
                explicit: false,
                node_span: (0, 0),
            },
            span: (0, 0),
        };
        if self.ty(self.p) == IDENTIFIER && self.ty(self.p + 1) == AT {
            tail.explicit_id = Some(self.image(self.p));
            tail.explicit_id_span = Some(self.span(self.p));
            self.p += 2;
        }
        let k = self.p;
        let type_ = self.ty(self.p);
        self.p += 1;
        match type_ {
            DEPENDENCY_ARROW => {
                self.expect(COLON)?;
                let next = self.ty(self.p);
                if next != INCLUDE && next != EXTEND {
                    return self.no_alternative("INCLUDE|EXTEND");
                }
                tail.type_ = if next == INCLUDE {
                    RelationshipType::Include
                } else {
                    RelationshipType::Extend
                };
                let name = if next == INCLUDE { "include" } else { "extend" };
                tail.label = Some(DraftLabel {
                    text: name.to_owned(),
                    type_: LabelType::Text,
                    span: self.span(self.p),
                });
                self.p += 1;
            }
            GENERALIZATION => tail.type_ = RelationshipType::Generalization,
            FORWARD_SOLID => tail.minlen = self.minimum_length(k),
            BACKWARD_SOLID => {
                tail.arrow_type = arrow::BACK_ARROW;
                if self.ends[k] - self.starts[k] == 3 && self.labeled_right(0) {
                    tail.label = Some(self.label()?);
                    let close = self.expect(MARKERLESS_SOLID)?;
                    tail.minlen = self.minimum_length(close);
                } else {
                    tail.minlen = self.minimum_length(k);
                }
            }
            MARKERLESS_SOLID => {
                tail.arrow_type = arrow::LINE_SOLID;
                if self.ends[k] - self.starts[k] == 2 && self.labeled_right(CLOSES_LABEL) {
                    tail.label = Some(self.label()?);
                    let close = self.ty(self.p);
                    if close == FORWARD_SOLID {
                        tail.arrow_type = arrow::SOLID_ARROW;
                    } else if close == FORWARD_CIRCLE {
                        tail.arrow_type = arrow::CIRCLE_ARROW;
                    } else if close == FORWARD_CROSS {
                        tail.arrow_type = arrow::CROSS_ARROW;
                    } else if close != MARKERLESS_SOLID {
                        return self.no_alternative(
                            "FORWARD_SOLID|MARKERLESS_SOLID|FORWARD_CIRCLE|FORWARD_CROSS",
                        );
                    }
                    if tail.arrow_type == arrow::SOLID_ARROW || tail.arrow_type == arrow::LINE_SOLID
                    {
                        tail.minlen = self.minimum_length(self.p);
                    }
                    self.p += 1;
                } else {
                    tail.minlen = self.minimum_length(k);
                }
            }
            FORWARD_CIRCLE => tail.arrow_type = arrow::CIRCLE_ARROW,
            FORWARD_CROSS => tail.arrow_type = arrow::CROSS_ARROW,
            BACKWARD_CIRCLE | BACKWARD_CROSS => {
                tail.arrow_type = if type_ == BACKWARD_CIRCLE {
                    arrow::CIRCLE_ARROW_REVERSED
                } else {
                    arrow::CROSS_ARROW_REVERSED
                };
                if self.labeled_right(0) {
                    tail.label = Some(self.label()?);
                    self.expect(MARKERLESS_SOLID)?;
                }
            }
            _ => {
                self.p -= 1;
                return self.no_alternative(&ARROWS.join("|"));
            }
        }
        tail.target = self.element(false)?;
        tail.span = (self.starts[first], self.ends[self.p - 1]);
        Ok(tail)
    }
}

const MARKDOWN_STRING_RANGE_START: usize = USECASE;

fn flag_closers(type_: usize, closers: u8) -> bool {
    closers != 0 && flag(type_, closers)
}

fn statement(kind: StatementKind) -> GraphStatement {
    GraphStatement::new(kind, (0, 0))
}

fn occurrences(properties: &[DraftProperty]) -> Vec<MetadataOccurrence> {
    properties.iter().map(DraftProperty::occurrence).collect()
}

fn draft_element(
    item: &Item,
    kind: super::types::SymbolKind,
    parent: &Option<DraftBoundary>,
) -> DraftElement {
    DraftElement {
        id: item.id.clone(),
        label: item.label.clone(),
        span: item.span,
        generated: item.generated,
        kind,
        parent: parent.clone(),
        shape: item.shape,
        metadata: item.metadata.clone(),
        stereotype: item.stereotype.clone(),
        classes: item.classes.clone(),
    }
}

fn endpoint(item: &Item, declares: bool) -> DraftEndpoint {
    DraftEndpoint {
        id: item.id.clone(),
        label: item.label.clone(),
        span: item.span,
        generated: item.generated,
        declares,
        classed: !item.classes.is_empty(),
    }
}

fn occurrence(item: &Item, defines: bool) -> NodeOccurrence {
    NodeOccurrence {
        id: item.id.clone(),
        span: item.node_span,
        id_span: item.span,
        label_span: Some(item.label.span),
        defines: if defines { Some(true) } else { None },
        stereotype_span: item.stereotype_span,
        metadata: item.metadata.as_ref().map(|m| occurrences(m)),
        class_spans: if item.class_spans.is_empty() {
            None
        } else {
            Some(item.class_spans.clone())
        },
    }
}

impl Parser<'_> {
    fn draft_for(&self, item: &Item, kind: super::types::SymbolKind) -> DraftElement {
        draft_element(item, kind, &self.parent)
    }

    fn relationship(
        &mut self,
        source: &Item,
        declaration: bool,
        tail: Tail,
        nodes: Vec<NodeOccurrence>,
    ) -> GraphStatement {
        let target = &tail.target;
        if target.explicit {
            let draft = self.draft_for(target, super::types::SymbolKind::UseCase);
            self.drafts.elements.push(draft);
        }
        self.drafts.relationships.push(DraftRelationship {
            source: endpoint(source, declaration),
            target: endpoint(target, target.explicit),
            span: tail.span,
            id: tail.explicit_id.clone(),
            id_span: tail.explicit_id_span,
            type_: tail.type_,
            arrow_type: tail.arrow_type,
            label: tail.label.clone(),
            minlen: tail.minlen,
        });
        let mut nodes = nodes;
        nodes.push(occurrence(target, target.explicit));
        let mut out = statement(StatementKind::Edge);
        out.nodes = Some(nodes);
        let id = match &tail.explicit_id {
            Some(id) => id.clone(),
            None => {
                let generated = format!("edge-{}", self.edge_count);
                self.edge_count += 1;
                generated
            }
        };
        let mut edge = super::types::EdgeOccurrence {
            id,
            span: out.span,
            label_span: tail.label.as_ref().map(|l| l.span),
            id_span: None,
            metadata: None,
        };
        if let Some(explicit_span) = tail.explicit_id_span {
            edge.id_span = Some(explicit_span);
        }
        out.edges = Some(vec![edge]);
        out
    }

    fn declaration(&self, nodes: Vec<NodeOccurrence>) -> GraphStatement {
        let mut out = statement(StatementKind::Node);
        out.nodes = Some(nodes);
        out
    }

    fn actors(&mut self, top: bool) -> PResult<GraphStatement> {
        self.p += 1;
        let mut items = vec![self.element(true)?];
        let mut tail: Option<Tail> = None;
        if self.ty(self.p) == COMMA {
            loop {
                self.p += 1;
                items.push(self.element(true)?);
                if self.ty(self.p) != COMMA {
                    break;
                }
            }
        } else if top && flag(self.ty(self.p), RELATION) {
            tail = Some(self.relation_tail()?);
        }
        self.line_end()?;
        for item in &items {
            let draft = self.draft_for(item, super::types::SymbolKind::Actor);
            self.drafts.elements.push(draft);
        }
        let nodes: Vec<NodeOccurrence> = items.iter().map(|item| occurrence(item, true)).collect();
        Ok(match tail {
            Some(tail) => self.relationship(&items[0], true, tail, nodes),
            None => self.declaration(nodes),
        })
    }

    fn entity(&mut self, top: bool) -> PResult<GraphStatement> {
        let mut source = self.element(false)?;
        let tail = if top && flag(self.ty(self.p), RELATION) {
            Some(self.relation_tail()?)
        } else {
            None
        };
        self.line_end()?;
        if tail.is_none() {
            source.explicit = true;
            if source.shape.is_none() {
                source.shape = Some(UseCaseShape::Ellipse);
            }
        }
        if source.explicit {
            let draft = self.draft_for(&source, super::types::SymbolKind::UseCase);
            self.drafts.elements.push(draft);
        }
        let nodes = vec![occurrence(&source, source.explicit)];
        Ok(match tail {
            Some(tail) => self.relationship(&source, source.explicit, tail, nodes),
            None => self.declaration(nodes),
        })
    }

    fn boundary(&mut self) -> PResult<GraphStatement> {
        self.p += 1;
        let mut named = self.name(true)?;
        let inline = if self.ty(self.p) == METADATA_START {
            Some(self.metadata()?)
        } else {
            None
        };
        if self.ty(self.p) == CLASS_SEPARATOR {
            let mut classes = std::mem::take(&mut named.classes);
            let mut spans = std::mem::take(&mut named.class_spans);
            self.class_suffix(&mut classes, &mut spans)?;
            named.classes = classes;
            named.class_spans = spans;
        }
        let group = DraftBoundary {
            id: named.id.clone(),
            label: named.label.clone(),
            span: named.span,
            generated: named.generated,
            classes: named.classes.clone(),
        };
        self.line_end()?;
        self.drafts.boundaries.push(group.clone());
        self.parent = Some(group.clone());
        let mut children: Vec<GraphStatement> = Vec::new();
        while flag(self.ty(self.p), BOUNDARY_LINE) {
            children.push(self.line(false)?);
        }
        self.parent = None;
        let end = self.expect(END)?;
        self.line_end()?;
        let mut out = statement(StatementKind::Group);
        out.group = Some(group.id.clone());
        out.id_span = Some(group.span);
        out.title_span = Some(group.label.span);
        out.end_span = Some(self.span(end));
        out.class_spans = if named.class_spans.is_empty() {
            None
        } else {
            Some(named.class_spans.clone())
        };
        if !children.is_empty() {
            out.children = Some(children);
        }
        if let Some(inline) = inline {
            out.metadata = Some(occurrences(&inline));
            self.drafts.metadata.push(DraftMetadata {
                target: group.id.clone(),
                span: group.span,
                properties: inline,
                statement: usize::MAX,
            });
        }
        Ok(out)
    }

    fn style(&mut self) -> PResult<String> {
        let first = self.p;
        let type_ = self.ty(self.p);
        if is_word(type_) || type_ == CSS_IDENTIFIER {
            self.p += 1;
        } else if type_ == MARKERLESS_SOLID {
            self.p += 1;
            if !is_word(self.ty(self.p)) {
                return self.mismatch("WORD");
            }
            self.p += 1;
        } else {
            return self.no_alternative("WORD|CSS_IDENTIFIER|MARKERLESS_SOLID");
        }
        self.expect(COLON)?;
        if !flag(self.ty(self.p), STYLE_PART) {
            let found = self.image(self.p);
            return self.fail(&format!(
                "Expecting: expecting at least one iteration which starts with one of these possible Token sequences::\n  <[{}]>\nbut found: '{found}'",
                STYLE_PARTS.join("] ,[")
            ));
        }
        while flag(self.ty(self.p), STYLE_PART) {
            self.p += 1;
        }
        Ok(self
            .text(self.starts[first], self.ends[self.p - 1])
            .replace("\\,", ","))
    }

    fn styles(&mut self) -> PResult<Vec<String>> {
        let mut out = vec![self.style()?];
        while self.ty(self.p) == COMMA {
            self.p += 1;
            out.push(self.style()?);
        }
        Ok(out)
    }

    fn identifiers(&mut self) -> PResult<Vec<usize>> {
        let mut out = vec![self.expect(IDENTIFIER)?];
        while self.ty(self.p) == COMMA {
            self.p += 1;
            out.push(self.expect(IDENTIFIER)?);
        }
        Ok(out)
    }

    fn reference(&self, k: usize) -> NodeOccurrence {
        let at = self.content(k);
        NodeOccurrence {
            id: self.image(k),
            span: at,
            id_span: at,
            label_span: None,
            defines: None,
            stereotype_span: None,
            metadata: None,
            class_spans: None,
        }
    }

    fn top_statement(&mut self) -> PResult<GraphStatement> {
        let k = self.p;
        let type_ = self.ty(self.p);
        let out: GraphStatement;
        if type_ == ACC_TITLE_LINE || type_ == ACC_DESCR_LINE || type_ == ACC_DESCR_BLOCK {
            self.p += 1;
            let text = self.image(k);
            if type_ == ACC_DESCR_BLOCK {
                let open = text.find('{').map_or(0, |i| i + 1);
                let close = text.rfind('}').unwrap_or(text.len());
                self.drafts.acc_description =
                    crate::util::chars::trim(&text[open.min(close)..close]).to_owned();
            } else if type_ == ACC_DESCR_LINE {
                let colon = text.find(':').map_or(0, |i| i + 1);
                self.drafts.acc_description = crate::util::chars::trim(&text[colon..]).to_owned();
            } else {
                let colon = text.find(':').map_or(0, |i| i + 1);
                self.drafts.acc_title = crate::util::chars::trim(&text[colon..]).to_owned();
            }
            out = statement(if type_ == ACC_TITLE_LINE {
                StatementKind::AccTitle
            } else {
                StatementKind::AccDescr
            });
        } else if type_ == DIRECTION {
            self.p += 1;
            let value = self.ty(self.p);
            if !(TD..=RL).contains(&value) {
                return self.no_alternative("TD|TB|BT|LR|RL");
            }
            self.drafts.direction = Some(if value == TD {
                "TB".to_owned()
            } else {
                self.image(self.p)
            });
            self.p += 1;
            out = statement(StatementKind::Direction);
        } else if type_ == ACTOR {
            return self.actors(true);
        } else if type_ == SYSTEM_BOUNDARY {
            return self.boundary();
        } else if type_ == NOTE {
            self.p += 1;
            self.expect(FOR)?;
            let target = self.expect(IDENTIFIER)?;
            let text = self.label()?;
            let target_name = self.image(target);
            self.drafts.notes.push(DraftNote {
                target: target_name,
                span: self.span(target),
                label: text.clone(),
            });
            let mut note = statement(StatementKind::Note);
            note.ref_ = Some(format!("note-{}", self.note_count));
            self.note_count += 1;
            note.ref_span = Some(text.span);
            note.nodes = Some(vec![self.reference(target)]);
            out = note;
        } else if type_ == JSON_DECLARATION_START {
            self.p += 1;
            let literal = self.expect(JSON_OBJECT_LITERAL)?;
            let mut classes = Vec::new();
            let mut class_spans = Vec::new();
            if self.ty(self.p) == CLASS_SEPARATOR {
                self.class_suffix(&mut classes, &mut class_spans)?;
            }
            let mut from = self.starts[k] + 4;
            while self.units.get(from).is_some_and(|&c| c == 32 || c == 9) {
                from += 1;
            }
            let mut to = from;
            while self.units.get(to).is_some_and(|&c| {
                c < 128 && ((c as u8).is_ascii_alphanumeric() || c == b'_' as u16)
            }) {
                to += 1;
            }
            let id = self.text(from, to);
            self.drafts.jsons.push(DraftJson {
                id: id.clone(),
                value: crate::util::value::Value::Object(crate::util::value::Object::new()),
                property_order: crate::util::ordered::OrderedMap::default(),
                span: (from, to),
                classes: classes.clone(),
            });
            let json_index = self.drafts.jsons.len() - 1;
            self.pending_json.push((json_index, literal));
            let mut json_statement = statement(StatementKind::Json);
            json_statement.nodes = Some(vec![NodeOccurrence {
                id,
                span: (from, to),
                id_span: (from, to),
                label_span: None,
                defines: Some(true),
                stereotype_span: None,
                metadata: None,
                class_spans: if class_spans.is_empty() {
                    None
                } else {
                    Some(class_spans.clone())
                },
            }]);
            json_statement.class_spans = if class_spans.is_empty() {
                None
            } else {
                Some(class_spans)
            };
            out = json_statement;
        } else if type_ == CLASS_DEF {
            self.p += 1;
            let ids = self.identifiers()?;
            let styles = self.styles()?;
            self.drafts.class_defs.push(DraftClassDef {
                ids: ids.iter().map(|&i| self.image(i)).collect(),
                styles,
            });
            let mut stmt = statement(StatementKind::ClassDef);
            stmt.ref_ = Some(self.image(ids[0]));
            stmt.ref_span = Some(self.span(ids[0]));
            out = stmt;
        } else if type_ == CLASS {
            self.p += 1;
            let targets = self.identifiers()?;
            let classes = self.identifiers()?;
            self.drafts.classes.push(DraftClassAssign {
                targets: targets
                    .iter()
                    .map(|&id| DraftTarget {
                        id: self.image(id),
                        span: self.span(id),
                    })
                    .collect(),
                classes: classes.iter().map(|&c| self.image(c)).collect(),
            });
            let mut stmt = statement(StatementKind::ClassAssign);
            stmt.ref_ = Some(self.image(classes[0]));
            stmt.ref_span = Some(self.span(classes[0]));
            stmt.nodes = Some(targets.iter().map(|&t| self.reference(t)).collect());
            out = stmt;
        } else if type_ == STYLE {
            self.p += 1;
            let target = self.expect(IDENTIFIER)?;
            let styles = self.styles()?;
            self.drafts.styles.push(DraftStyle {
                target: self.image(target),
                span: self.span(target),
                styles,
            });
            let mut stmt = statement(StatementKind::Style);
            stmt.nodes = Some(vec![self.reference(target)]);
            out = stmt;
        } else {
            let next = self.ty(self.p + 1);
            if next == METADATA_START {
                if !flag(self.ty(self.p + 2), METADATA_FIRST) {
                    return self.no_alternative(&statement_sequences());
                }
                self.p += 1;
                let mut target = self.reference(k);
                if type_ != IDENTIFIER {
                    target.id = generated_id(&self.text(target.span.0, target.span.1));
                }
                let assigned = self.metadata()?;
                let mut stmt = statement(StatementKind::Metadata);
                stmt.metadata = Some(occurrences(&assigned));
                stmt.nodes = Some(vec![target.clone()]);
                self.drafts.metadata.push(DraftMetadata {
                    target: target.id.clone(),
                    span: target.span,
                    properties: assigned,
                    statement: usize::MAX,
                });
                out = stmt;
            } else {
                let follows = flag(next, FOLLOWS_NAME)
                    || (type_ == IDENTIFIER && (next == LPAREN || next == LBRACKET));
                if !follows
                    || (type_ == IDENTIFIER
                        && FORBIDDEN.contains(&self.image(k).to_lowercase().as_str()))
                {
                    return self.no_alternative(&statement_sequences());
                }
                return self.entity(true);
            }
        }
        self.line_end()?;
        Ok(out)
    }

    fn line(&mut self, top: bool) -> PResult<GraphStatement> {
        let k = self.p;
        let type_ = self.ty(self.p);
        if type_ == NEWLINE {
            self.p += 1;
            let mut blank = statement(StatementKind::Blank);
            blank.span = self.span(k);
            return Ok(blank);
        }
        if type_ == COMMENT {
            self.p += 1;
            self.line_end()?;
            let mut comment = statement(StatementKind::Comment);
            comment.span = self.span(k);
            return Ok(comment);
        }
        let mut out = if top {
            self.top_statement()?
        } else if type_ == ACTOR {
            self.actors(false)?
        } else {
            self.entity(false)?
        };
        out.span = (self.starts[k], self.last_end);
        if let Some(edges) = out.edges.as_mut() {
            for edge in edges.iter_mut() {
                edge.span = out.span;
            }
        }
        Ok(out)
    }
}

pub fn parse_usecase(source: &str, title: Option<String>) -> PResult<UsecaseModel> {
    let tokens = tokenize(source)?;
    let units: Vec<u16> = source.encode_utf16().collect();
    let mut parser = Parser {
        src: source,
        units,
        types: tokens.types,
        starts: tokens.starts,
        ends: tokens.ends,
        p: 0,
        last_end: 0,
        drafts: Drafts::default(),
        parent: None,
        pending_json: Vec::new(),
        edge_count: 0,
        note_count: 0,
    };
    let header = parser.expect(USECASE)?;
    parser.line_end()?;
    let mut statements: Vec<GraphStatement> = Vec::new();
    while flag(parser.ty(parser.p), LINE) {
        let index = statements.len();
        let metadata_before = parser.drafts.metadata.len();
        let statement = parser.line(true)?;
        for draft in parser.drafts.metadata[metadata_before..].iter_mut() {
            draft.statement = index;
        }
        statements.push(statement);
    }
    if parser.ty(parser.p) != EOF {
        let found = parser.image(parser.p);
        return parser.fail(&format!(
            "Redundant input, expecting EOF but found: {found}"
        ));
    }
    let pending = std::mem::take(&mut parser.pending_json);
    for (json_index, literal) in pending {
        let text = parser.image(literal);
        let literal_start = parser.starts[literal];
        let parsed =
            parse_ordered_json(&text, |offset| line_column(source, literal_start + offset))?;
        parser.drafts.jsons[json_index].value = parsed.value;
        parser.drafts.jsons[json_index].property_order = parsed.property_order;
    }
    let header_span = parser.span(header);
    let drafts = std::mem::take(&mut parser.drafts);
    resolve_usecase(source, &drafts, header_span, &mut statements, title)
}
