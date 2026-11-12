use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::shapes::{is_valid_shape, resolve_shape_alias};
use super::types::{
    AgentClassDef, AgentEdge, AgentNode, AgentSubgraph, Diagnostic, DiagnosticId, DocItem,
    EdgeSemantic, EdgeStyle, ElementMapping, ElementPosition, FlowText, LabelType, LinkInfo, Loc,
    Metadata, SemanticConnector, SemanticEdge, SemanticModel, SemanticSubGraph, SemanticVertex,
    Severity, StatementType, VertexKind,
};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::{is_js_space, trim, trim_start, utf16_len, utf16_to_byte};
use crate::util::jsnum::js_number_to_string;
use crate::util::ordered::OrderedMap;
use crate::util::url::safe_url;
use crate::util::value::{Object, Value};
use crate::util::yaml::parse_yaml;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DbOptions {
    pub max_edges: Option<usize>,
    pub inherit_dir: bool,
    pub direction: Option<String>,
    pub line_offset: usize,
    pub source: String,
}

const PRESENTATION_KEYS: [&str; 11] = [
    "shape",
    "view",
    "icon",
    "img",
    "form",
    "pos",
    "w",
    "h",
    "class",
    "style",
    "labelType",
];
const TOOL_SHAPES: [&str; 5] = [
    "subroutine",
    "subprocess",
    "subproc",
    "framed-rectangle",
    "tool",
];

#[derive(Clone, Debug, PartialEq)]
pub struct AgentflowDb {
    pub direction: Option<String>,
    pub nodes: OrderedMap<AgentNode>,
    pub edges: Vec<AgentEdge>,
    pub subgraphs: Vec<AgentSubgraph>,
    pub connectors: HashSet<String>,
    pub classes: OrderedMap<AgentClassDef>,
    pub tooltips: OrderedMap<String>,
    pub default_edge_style: Option<EdgeStyle>,
    pub default_interpolate: Option<String>,
    pub mappings: Vec<ElementMapping>,
    pub diagnostics: Vec<Diagnostic>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub line_offset: usize,
    options: DbOptions,
    subgraph_lookup: HashMap<String, usize>,
    global_nodes: HashSet<String>,
    members: HashSet<String>,
    edge_by_id: HashMap<String, usize>,
    pair_count: HashMap<String, usize>,
    sub_count: usize,
    bare_mappings: HashSet<usize>,
    first_mapping: HashMap<String, usize>,
    reported: HashMap<String, Vec<Diagnostic>>,
}

fn label_type_of(value: Option<&Value>) -> LabelType {
    match value.and_then(Value::as_str) {
        Some("text") => LabelType::Text,
        Some("string") => LabelType::String,
        _ => LabelType::Markdown,
    }
}

fn unquote(text: &str) -> String {
    if text.len() >= 2 && text.starts_with('"') && text.ends_with('"') {
        text[1..text.len() - 1].to_owned()
    } else {
        text.to_owned()
    }
}

fn json_text(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) if number.is_finite() => js_number_to_string(*number),
        Value::Number(_) => "null".to_owned(),
        Value::String(text) => {
            let mut out = String::from("\"");
            for ch in text.chars() {
                match ch {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\u{8}' => out.push_str("\\b"),
                    '\u{c}' => out.push_str("\\f"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        }
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(json_text).collect::<Vec<_>>().join(",")
        ),
        Value::Object(object) => format!(
            "{{{}}}",
            object
                .iter()
                .map(|(key, entry)| format!(
                    "{}:{}",
                    json_text(&Value::String(key.to_owned())),
                    json_text(entry)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn strip_prototype_keys(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(strip_prototype_keys).collect()),
        Value::Object(object) => {
            let mut out = Object::new();
            for (key, entry) in object.iter() {
                if matches!(key, "__proto__" | "constructor" | "prototype") {
                    continue;
                }
                out.set(key, strip_prototype_keys(entry));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn merge(into: Option<Metadata>, doc: &Value) -> Metadata {
    let mut out = into.unwrap_or_default();
    match doc {
        Value::Object(object) => {
            for (key, entry) in object.iter() {
                out.set(key, entry.clone());
            }
        }
        Value::Array(items) => {
            for (index, entry) in items.iter().enumerate() {
                out.set(index.to_string(), entry.clone());
            }
        }
        Value::String(text) => {
            for (index, unit) in text.encode_utf16().enumerate() {
                out.set(
                    index.to_string(),
                    Value::String(String::from_utf16_lossy(&[unit])),
                );
            }
        }
        _ => {}
    }
    out
}

fn edge_semantic_of(kind: &str, stroke: &str) -> Option<EdgeSemantic> {
    match (kind, stroke) {
        ("arrow_point", "normal") => Some(EdgeSemantic::Sequence),
        ("arrow_cross", "normal") => Some(EdgeSemantic::Failure),
        ("arrow_open", "dotted") => Some(EdgeSemantic::Reference),
        _ => None,
    }
}

struct EndLink {
    kind: &'static str,
    stroke: &'static str,
    length: i64,
}

fn destruct_end_link(raw: &str) -> EndLink {
    let str = trim(raw);
    let last = str.chars().last();
    let mut line: &str = match last {
        Some(c) => &str[..str.len() - c.len_utf8()],
        None => "",
    };
    let mut kind = "arrow_open";
    match last {
        Some('x') => kind = "arrow_cross",
        Some('>') => kind = "arrow_point",
        Some('-') | Some('.') => line = str,
        _ => {}
    }
    let mut stroke = "normal";
    let mut length = line.encode_utf16().count() as i64 - 1;
    let dots = line.chars().filter(|&c| c == '.').count();
    if dots > 0 {
        stroke = "dotted";
        length = dots as i64;
    }
    EndLink {
        kind,
        stroke,
        length,
    }
}

fn semantic_metadata(metadata: &Option<Metadata>) -> Option<Metadata> {
    let metadata = metadata.as_ref()?;
    let mut out = Object::new();
    for (key, value) in metadata.iter() {
        if PRESENTATION_KEYS.contains(&key) {
            continue;
        }
        out.set(key, value.clone());
    }
    if out.is_empty() { None } else { Some(out) }
}

fn is_block_scalar_header(code: &str) -> bool {
    let t = code.trim_end_matches(is_js_space);
    let digits = t.trim_end_matches(|c: char| c.is_ascii_digit() || c == '+' || c == '-');
    let Some(rest) = digits.strip_suffix(['>', '|']) else {
        return false;
    };
    rest.trim_end_matches(is_js_space).ends_with(':')
}

fn strip_trailing_comma(code: &str) -> String {
    let trimmed = code.trim_end_matches([' ', '\t']);
    match trimmed.strip_suffix(',') {
        Some(head) => format!("{head}{}", &code[trimmed.len()..]),
        None => code.to_owned(),
    }
}

fn strip_line_trailing_commas(body: &str) -> String {
    let mut in_single = false;
    let mut in_double = false;
    let mut flow_depth: i64 = 0;
    let mut block_scalar_indent: Option<usize> = None;
    let mut out: Vec<String> = Vec::new();
    for line in body.split('\n') {
        let indent = utf16_len(line) - utf16_len(trim_start(line));
        if let Some(block) = block_scalar_indent {
            if trim(line).is_empty() || indent > block {
                out.push(line.to_owned());
                continue;
            }
            block_scalar_indent = None;
        }
        let bytes = line.as_bytes();
        let mut comment_start: Option<usize> = None;
        let mut i = 0;
        while i < bytes.len() {
            let ch = bytes[i];
            if in_double {
                if ch == b'\\' {
                    i += 1;
                } else if ch == b'"' {
                    in_double = false;
                }
            } else if in_single {
                if ch == b'\'' {
                    in_single = false;
                }
            } else if ch == b'"' {
                in_double = true;
            } else if ch == b'\'' {
                in_single = true;
            } else if ch == b'#' && (i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') {
                comment_start = Some(i);
                break;
            } else if ch == b'[' || ch == b'{' {
                flow_depth += 1;
            } else if ch == b']' || ch == b'}' {
                flow_depth = (flow_depth - 1).max(0);
            }
            i += 1;
        }
        if in_single || in_double || flow_depth > 0 {
            out.push(line.to_owned());
            continue;
        }
        let (code, comment) = match comment_start {
            Some(at) => (&line[..at], &line[at..]),
            None => (line, ""),
        };
        if is_block_scalar_header(code) {
            block_scalar_indent = Some(indent);
            out.push(line.to_owned());
            continue;
        }
        out.push(format!("{}{comment}", strip_trailing_comma(code)));
    }
    out.join("\n")
}

fn position_of(loc: Option<Loc>, line_offset: usize) -> ElementPosition {
    let start_line = loc.map_or(0, |l| l.first_line);
    let start_column = loc.map_or(0, |l| l.first_column);
    ElementPosition {
        start_line: start_line + line_offset,
        start_column,
        end_line: loc.map_or(start_line, |l| l.last_line) + line_offset,
        end_column: loc.map_or(start_column, |l| l.last_column),
        start_index: 0,
        end_index: 0,
    }
}

impl AgentflowDb {
    pub fn new(options: DbOptions) -> Self {
        let line_offset = options.line_offset;
        Self {
            direction: None,
            nodes: OrderedMap::default(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            connectors: HashSet::new(),
            classes: OrderedMap::default(),
            tooltips: OrderedMap::default(),
            default_edge_style: None,
            default_interpolate: None,
            mappings: Vec::new(),
            diagnostics: Vec::new(),
            title: None,
            acc_title: None,
            acc_descr: None,
            line_offset,
            options,
            subgraph_lookup: HashMap::new(),
            global_nodes: HashSet::new(),
            members: HashSet::new(),
            edge_by_id: HashMap::new(),
            pair_count: HashMap::new(),
            sub_count: 0,
            bare_mappings: HashSet::new(),
            first_mapping: HashMap::new(),
            reported: HashMap::new(),
        }
    }

    fn error_at(&self, loc: Option<Loc>) -> (usize, usize) {
        let Some(loc) = loc else {
            return (0, 0);
        };
        let source = self.options.source.as_str();
        let mut start = 0usize;
        for _ in 1..loc.first_line {
            start = match source[start..].find('\n') {
                Some(at) => start + at + 1,
                None => 0,
            };
        }
        let unit = utf16_len(&source[..start]) + loc.first_column;
        let (line, column, _) = crate::errors::locate(source, utf16_to_byte(source, unit));
        (line, column)
    }

    fn metadata(&self, id: &str, source: &str, loc: Option<Loc>) -> Result<Value, PelersError> {
        let multiline = source.contains('\n');
        let text = if multiline {
            format!("{source}\n")
        } else {
            format!("{{\n{source}\n}}")
        };
        let parsed = match parse_yaml(&text) {
            Ok(value) => value,
            Err(message) => {
                let stripped = if multiline {
                    strip_line_trailing_commas(source)
                } else {
                    source.to_owned()
                };
                let retried = if stripped != source {
                    parse_yaml(&format!("{stripped}\n")).ok()
                } else {
                    None
                };
                match retried {
                    Some(value) => value,
                    None => {
                        let (line, column) = self.error_at(loc);
                        let at = match loc {
                            Some(_) => format!(" ({line}:{column})"),
                            None => String::new(),
                        };
                        let mut error = PelersError::new(
                            format!("Metadata of \"{id}\" is not valid YAML. {message}{at}"),
                            PelersErrorCode::Syntax,
                        );
                        error.diagram_type = Some("agentflow".to_owned());
                        error.line = line;
                        error.column = column;
                        return Err(error);
                    }
                }
            }
        };
        let parsed = match parsed {
            Value::Null => Value::Object(Object::new()),
            other => other,
        };
        Ok(strip_prototype_keys(&parsed))
    }

    fn shape_error(&self, message: String, loc: Option<Loc>) -> PelersError {
        let (line, column) = self.error_at(loc);
        let mut error = PelersError::new(message, PelersErrorCode::Semantic);
        error.diagram_type = Some("agentflow".to_owned());
        error.line = line;
        error.column = column;
        error
    }

    pub fn add_vertex(
        &mut self,
        id: &str,
        text: Option<FlowText>,
        shape: Option<&str>,
        styles: Option<Vec<String>>,
        metadata: Option<(&str, Loc)>,
    ) -> Result<(), PelersError> {
        if trim(id).is_empty() {
            return Ok(());
        }
        let mut doc: Option<Value> = None;
        let mut authored_shape: Option<String> = None;
        if let Some((source, loc)) = metadata {
            let mut value = self.metadata(id, source, Some(loc))?;
            if let Value::Object(object) = &mut value
                && let Some(Value::String(shape_text)) = object.get("shape").cloned()
            {
                authored_shape = Some(shape_text.clone());
                let resolved = resolve_shape_alias(Some(&shape_text)).unwrap_or_default();
                object.set("shape", Value::String(resolved));
            }
            doc = Some(value);
        }
        let fields: Option<Object> = match &doc {
            Some(Value::Object(object)) => Some(object.clone()),
            _ => None,
        };
        let metadata_loc = metadata.map(|(_, loc)| loc);
        let truthy_doc = doc.as_ref().filter(|value| value.is_truthy()).cloned();

        if let Some(index) = self.subgraph_lookup.get(id).copied()
            && let Some(value) = &truthy_doc
        {
            let current = self.subgraphs[index].metadata.take();
            self.subgraphs[index].metadata = Some(merge(current, value));
            return Ok(());
        }

        if self.connectors.contains(id)
            && let Some(value) = &truthy_doc
        {
            let node = self.nodes.get_mut(id).expect("connector node");
            node.metadata = Some(merge(node.metadata.take(), value));
            return Ok(());
        }

        if id == "connectors" {
            return Ok(());
        }

        if let Some(index) = self.edge_by_id.get(id).copied() {
            if let Some(fields) = &fields {
                if let Some(animate) = fields.get("animate") {
                    self.edges[index].animate = Some(animate.clone());
                }
                if let Some(animation) = fields.get("animation") {
                    self.edges[index].animation = Some(animation.clone());
                }
                if let Some(curve) = fields.get("curve") {
                    self.edges[index].interpolate = if curve.is_truthy() {
                        Some(curve.to_js_string())
                    } else {
                        None
                    };
                }
            }
            if let Some(value) = &truthy_doc {
                let current = self.edges[index].metadata.take();
                self.edges[index].metadata = Some(merge(current, value));
            }
            return Ok(());
        }

        if self.nodes.get(id).is_none() {
            self.nodes.set(
                id,
                AgentNode {
                    id: id.to_owned(),
                    text: None,
                    label_type: LabelType::Text,
                    shape: None,
                    styles: Vec::new(),
                    classes: Vec::new(),
                    dir: None,
                    link: None,
                    link_target: None,
                    metadata: None,
                    is_connector: false,
                },
            );
        }
        let vertex = self.nodes.get_mut(id).expect("vertex node");
        if let Some(text) = &text {
            vertex.label_type = text.kind;
            vertex.text = Some(unquote(trim(&text.text)));
        } else if vertex.text.is_none() {
            vertex.text = Some(id.to_owned());
        }
        if let Some(shape) = shape {
            vertex.shape = Some(shape.to_owned());
        }
        if let Some(styles) = styles {
            vertex.styles.extend(styles);
        }

        let Some(value) = doc else {
            return Ok(());
        };
        if !value.is_truthy() {
            return Ok(());
        }
        vertex.metadata = Some(merge(vertex.metadata.take(), &value));
        let Some(fields) = fields else {
            return Ok(());
        };

        if let Some(shape_value) = fields.get("shape").filter(|v| v.is_truthy()).cloned() {
            let authored: Value = match &authored_shape {
                Some(text) => Value::String(text.clone()),
                None => shape_value.clone(),
            };
            let Value::String(authored_text) = &authored else {
                return Err(self.shape_error(
                    format!("No such shape: {}.", json_text(&authored)),
                    metadata_loc,
                ));
            };
            if *authored_text != authored_text.to_lowercase() || authored_text.contains('_') {
                return Err(self.shape_error(
                    format!("No such shape: {authored_text}. Shape names should be lowercase."),
                    metadata_loc,
                ));
            }
            let Value::String(shape_name) = &shape_value else {
                return Err(self.shape_error(
                    format!("No such shape: {}.", json_text(&shape_value)),
                    metadata_loc,
                ));
            };
            if !is_valid_shape(shape_name) {
                return Err(self.shape_error(format!("No such shape: {shape_name}."), metadata_loc));
            }
            let vertex = self.nodes.get_mut(id).expect("vertex node");
            vertex.shape = Some(shape_name.clone());
        }
        if let Some(label) = fields.get("label").filter(|v| v.is_truthy()).cloned() {
            let vertex = self.nodes.get_mut(id).expect("vertex node");
            vertex.text = Some(label.to_js_string());
            vertex.label_type = label_type_of(fields.get("labelType"));
        }
        Ok(())
    }

    fn add_single_link(
        &mut self,
        start: &str,
        end: &str,
        link: Option<&LinkInfo>,
        id: Option<String>,
    ) -> Result<(), PelersError> {
        let mut edge = AgentEdge {
            start: start.to_owned(),
            end: end.to_owned(),
            kind: None,
            text: String::new(),
            label_type: LabelType::Text,
            stroke: None,
            edge_semantic: None,
            length: None,
            style: None,
            classes: Vec::new(),
            id: None,
            is_user_defined_id: false,
            interpolate: self.default_interpolate.clone(),
            animate: None,
            animation: None,
            metadata: None,
        };
        if let Some(text) = link.and_then(|l| l.text.as_ref()) {
            edge.text = unquote(trim(&text.text));
            edge.label_type = text.kind;
        }
        if let Some(link) = link {
            edge.kind = Some(link.kind.clone());
            edge.stroke = Some(link.stroke.clone());
            edge.length = link
                .length
                .map(|length| if length > 10 { 10 } else { length });
            if let Some(semantic) = link.edge_semantic {
                edge.edge_semantic = Some(semantic);
            }
        }
        let pair = format!("{start}\0{end}");
        let existing = self.pair_count.get(&pair).copied().unwrap_or(0);
        match id {
            Some(id) if !self.edge_by_id.contains_key(&id) => {
                edge.id = Some(id);
                edge.is_user_defined_id = true;
            }
            _ => {
                let suffix = if existing == 0 { 0 } else { existing + 1 };
                edge.id = Some(format!("L_{start}_{end}_{suffix}"));
            }
        }
        if let Some(max) = self.options.max_edges
            && self.edges.len() >= max
        {
            let mut error = PelersError::new(
                format!("Edge limit exceeded. The limit is {max}."),
                PelersErrorCode::Limit,
            );
            error.diagram_type = Some("agentflow".to_owned());
            return Err(error);
        }
        let edge_id = edge.id.clone().unwrap_or_default();
        self.edges.push(edge);
        self.pair_count.insert(pair, existing + 1);
        let index = self.edges.len() - 1;
        self.edge_by_id.entry(edge_id).or_insert(index);
        Ok(())
    }

    pub fn add_link(
        &mut self,
        starts: &[String],
        ends: &[String],
        link: &LinkInfo,
    ) -> Result<(), PelersError> {
        let id = link
            .id
            .as_ref()
            .map(|id| id.replacen('@', "", 1))
            .filter(|id| !id.is_empty());
        let last_start = starts.last();
        let first_end = ends.first();
        for start in starts {
            for end in ends {
                let own = if Some(start) == last_start && Some(end) == first_end {
                    id.clone()
                } else {
                    None
                };
                self.add_single_link(start, end, Some(link), own)?;
            }
        }
        Ok(())
    }

    fn edge_at(&self, pos: &str) -> Result<usize, PelersError> {
        match pos.parse::<usize>().ok().filter(|&i| i < self.edges.len()) {
            Some(index) => Ok(index),
            None => {
                let mut error = PelersError::new(
                    format!(
                        "The index {pos} for linkStyle is out of bounds. Valid indices for linkStyle are between 0 and {}. (Help: Ensure that the index is within the range of existing edges.)",
                        self.edges.len() as i64 - 1
                    ),
                    PelersErrorCode::Semantic,
                );
                error.diagram_type = Some("agentflow".to_owned());
                Err(error)
            }
        }
    }

    pub fn update_link_interpolate(
        &mut self,
        positions: &[String],
        interpolate: &str,
    ) -> Result<(), PelersError> {
        for pos in positions {
            if pos == "default" {
                self.default_interpolate = Some(interpolate.to_owned());
            } else {
                let index = self.edge_at(pos)?;
                self.edges[index].interpolate = Some(interpolate.to_owned());
            }
        }
        Ok(())
    }

    pub fn update_link(
        &mut self,
        positions: &[String],
        style: Vec<String>,
    ) -> Result<(), PelersError> {
        let shared: EdgeStyle = Rc::new(RefCell::new(style));
        for pos in positions {
            if pos == "default" {
                self.default_edge_style = Some(Rc::clone(&shared));
                continue;
            }
            let index = self.edge_at(pos)?;
            let has_style = !shared.borrow().is_empty();
            let fill = shared.borrow().iter().any(|s| s.starts_with("fill"));
            self.edges[index].style = Some(Rc::clone(&shared));
            if has_style && !fill {
                shared.borrow_mut().push("fill:none".to_owned());
            }
        }
        Ok(())
    }

    pub fn add_class(&mut self, ids: &str, styles: &[String]) {
        let joined = styles.join(",").replace("\\,", "\u{a7}\u{a7}\u{a7}");
        let joined = joined.replace(',', ";").replace("\u{a7}\u{a7}\u{a7}", ",");
        let style: Vec<&str> = joined.split(';').collect();
        for id in ids.split(',') {
            if !self.classes.contains(id) {
                self.classes.set(
                    id,
                    AgentClassDef {
                        id: id.to_owned(),
                        styles: Vec::new(),
                        text_styles: Vec::new(),
                    },
                );
            }
            let class = self.classes.get_mut(id).expect("class");
            for s in &style {
                if s.contains("color") {
                    class.text_styles.push(s.replacen("fill", "bgFill", 1));
                }
                class.styles.push((*s).to_owned());
            }
        }
    }

    pub fn set_direction(&mut self, dir: &str) {
        let mut d = trim(dir).to_owned();
        if d.contains('<') {
            d = "RL".to_owned();
        }
        if d.contains('^') {
            d = "BT".to_owned();
        }
        if d.contains('>') {
            d = "LR".to_owned();
        }
        if d.contains('v') {
            d = "TB".to_owned();
        }
        if d == "TD" {
            d = "TB".to_owned();
        }
        self.direction = Some(d);
    }

    pub fn set_class(&mut self, ids: &str, class_name: &str) {
        for id in ids.split(',') {
            if let Some(node) = self.nodes.get_mut(id) {
                node.classes.push(class_name.to_owned());
            }
            if let Some(&index) = self.edge_by_id.get(id) {
                self.edges[index].classes.push(class_name.to_owned());
            }
            if let Some(&index) = self.subgraph_lookup.get(id) {
                self.subgraphs[index].classes.push(class_name.to_owned());
            }
        }
    }

    pub fn set_tooltip(&mut self, ids: &str, tooltip: &str) {
        for id in ids.split(',') {
            self.tooltips.set(id, tooltip.to_owned());
        }
    }

    pub fn set_link(&mut self, ids: &str, link: &str, target: Option<&str>) {
        let url = trim(link);
        for id in ids.split(',') {
            if let Some(node) = self.nodes.get_mut(id) {
                node.link = if url.is_empty() {
                    None
                } else {
                    Some(safe_url(url))
                };
                node.link_target = target.map(str::to_owned);
            }
        }
        self.set_class(ids, "clickable");
    }

    pub fn set_click_event(&mut self, ids: &str) {
        self.set_class(ids, "clickable");
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(trim_start(text).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\n' && chars.peek().is_some_and(|&next| is_js_space(next)) {
                while chars.peek().is_some_and(|&next| is_js_space(next)) {
                    chars.next();
                }
                out.push('\n');
            } else {
                out.push(ch);
            }
        }
        self.acc_descr = Some(out);
    }

    pub fn add_sub_graph(
        &mut self,
        id_text: Option<&FlowText>,
        list: Vec<DocItem>,
        title_text: Option<&FlowText>,
    ) -> String {
        let title = title_text.map_or(String::new(), |t| t.text.clone());

        let mut seen: HashSet<String> = HashSet::new();
        let mut node_list: Vec<String> = Vec::new();
        let mut dir: Option<String> = None;
        for entry in list {
            match entry {
                DocItem::Node(node) => {
                    if !trim(&node).is_empty() && seen.insert(node.clone()) {
                        node_list.push(node);
                    }
                }
                DocItem::Nodes(items) => {
                    for item in items {
                        if !trim(&item).is_empty() && seen.insert(item.clone()) {
                            node_list.push(item);
                        }
                    }
                }
                DocItem::Dir(value) => dir = Some(value),
            }
        }
        if dir.is_none() && self.options.inherit_dir {
            dir = self
                .direction
                .clone()
                .or_else(|| self.options.direction.clone());
        }

        let id = match id_text {
            Some(text) => trim(&text.text).to_owned(),
            None => format!("subGraph{}", self.sub_count),
        };
        self.sub_count += 1;

        let nodes: Vec<String> = node_list
            .into_iter()
            .filter(|node_id| {
                *node_id != id
                    && !self.global_nodes.contains(node_id)
                    && !self.members.contains(node_id)
            })
            .collect();
        for node_id in &nodes {
            self.members.insert(node_id.clone());
        }

        if let Some(&index) = self.subgraph_lookup.get(&id) {
            let existing = &mut self.subgraphs[index];
            existing.nodes.extend(nodes);
            if !trim(&title).is_empty() {
                existing.title = trim(&title).to_owned();
            }
            if let Some(dir) = dir.filter(|d| !d.is_empty()) {
                existing.dir = Some(dir);
            }
            return id;
        }
        let label_type = title_text.map_or(LabelType::Markdown, |t| t.kind);
        self.subgraphs.push(AgentSubgraph {
            id: id.clone(),
            nodes,
            title: trim(&title).to_owned(),
            classes: Vec::new(),
            dir,
            label_type,
            kind: "flow".to_owned(),
            metadata: None,
        });
        self.subgraph_lookup
            .insert(id.clone(), self.subgraphs.len() - 1);
        id
    }

    pub fn add_global(&mut self, list: Vec<DocItem>) {
        let mut release = false;
        let mut add = |this: &mut Self, item: &str| {
            let id = trim(item).to_owned();
            if id.is_empty() {
                return;
            }
            this.global_nodes.insert(id.clone());
            if this.members.remove(&id) {
                release = true;
            }
        };
        for entry in &list {
            match entry {
                DocItem::Node(node) => add(self, node),
                DocItem::Nodes(items) => {
                    for item in items {
                        add(self, item);
                    }
                }
                DocItem::Dir(_) => {}
            }
        }
        if !release {
            return;
        }
        for subgraph in &mut self.subgraphs {
            let global = &self.global_nodes;
            subgraph.nodes.retain(|node_id| !global.contains(node_id));
        }
    }

    pub fn add_connector(&mut self, text: Option<&FlowText>, title: Option<&FlowText>) -> String {
        let id = match text.map(|t| trim(&t.text)) {
            Some(id) if !id.is_empty() => id.to_owned(),
            _ => return String::new(),
        };
        let title = title.map_or(String::new(), |t| trim(&t.text).to_owned());
        if !self.connectors.contains(&id) {
            let created = AgentNode {
                id: id.clone(),
                text: Some(if title.is_empty() { id.clone() } else { title }),
                label_type: LabelType::Text,
                shape: None,
                styles: Vec::new(),
                classes: Vec::new(),
                dir: None,
                link: None,
                link_target: None,
                metadata: None,
                is_connector: true,
            };
            self.nodes.set(&id, created);
            self.connectors.insert(id.clone());
        } else if !title.is_empty()
            && let Some(node) = self.nodes.get_mut(&id)
        {
            node.text = Some(title);
        }
        id
    }

    pub fn destruct_link(&self, end: &str, start: Option<&str>) -> LinkInfo {
        let info = destruct_end_link(end);
        if let Some(start) = start.filter(|s| !s.is_empty()) {
            let stroke = if start.contains('.') {
                "dotted"
            } else {
                "normal"
            };
            if stroke != info.stroke {
                return LinkInfo {
                    kind: "INVALID".to_owned(),
                    stroke: "INVALID".to_owned(),
                    length: None,
                    edge_semantic: None,
                    text: None,
                    id: None,
                };
            }
        }
        LinkInfo {
            kind: info.kind.to_owned(),
            stroke: info.stroke.to_owned(),
            length: Some(info.length),
            edge_semantic: edge_semantic_of(info.kind, info.stroke),
            text: None,
            id: None,
        }
    }

    pub fn subgraph(&self, id: &str) -> Option<&AgentSubgraph> {
        self.subgraph_lookup
            .get(id)
            .map(|&index| &self.subgraphs[index])
    }

    pub fn vertex_kind(vertex: &AgentNode) -> VertexKind {
        if is_tool_definition(vertex) {
            return VertexKind::Tool;
        }
        match resolve_shape_alias(vertex.shape.as_deref()).as_deref() {
            Some("hexagon") | Some("hex") => VertexKind::Action,
            Some("lean-right") | Some("lean_right") => VertexKind::Input,
            Some("lin-doc") | Some("lined-document") => VertexKind::Refdoc,
            Some("diamond") => VertexKind::Decision,
            _ => VertexKind::Task,
        }
    }

    pub fn semantic_model(&self) -> SemanticModel {
        let mut vertices = Vec::new();
        let mut connectors = Vec::new();
        for (id, v) in self.nodes.iter() {
            if self.subgraph_lookup.contains_key(id) {
                continue;
            }
            let metadata = semantic_metadata(&v.metadata);
            if v.is_connector {
                let title = v.text.clone().filter(|text| text != id);
                connectors.push(SemanticConnector {
                    id: id.to_owned(),
                    title,
                    metadata,
                });
                continue;
            }
            vertices.push(SemanticVertex {
                id: id.to_owned(),
                label: v.text.clone(),
                shape: resolve_shape_alias(v.shape.as_deref()),
                vertex_kind: Self::vertex_kind(v).as_str(),
                metadata,
            });
        }
        let edges = self
            .edges
            .iter()
            .map(|e| SemanticEdge {
                start: e.start.clone(),
                end: e.end.clone(),
                id: e.id.clone(),
                label: Some(e.text.clone()).filter(|text| !text.is_empty()),
                kind: e.kind.clone(),
                stroke: e.stroke.clone(),
                edge_semantic: e.edge_semantic,
                length: e.length,
                metadata: e.metadata.clone().filter(|m| !m.is_empty()),
            })
            .collect();
        let sub_graphs = self
            .subgraphs
            .iter()
            .map(|sg| SemanticSubGraph {
                id: sg.id.clone(),
                kind: sg.kind.clone(),
                nodes: sg.nodes.clone(),
                title: sg.title.clone(),
                direction: sg.dir.clone(),
                metadata: semantic_metadata(&sg.metadata),
            })
            .collect();
        SemanticModel {
            direction: self.direction.clone(),
            vertices,
            edges,
            sub_graphs,
            connectors,
            diagnostics: self.diagnostics.clone(),
        }
    }

    fn push_mapping(&mut self, id: &str, kind: StatementType, position: ElementPosition) -> usize {
        self.mappings.push(ElementMapping {
            id: id.to_owned(),
            kind,
            position,
        });
        let index = self.mappings.len() - 1;
        self.first_mapping.entry(id.to_owned()).or_insert(index);
        index
    }

    pub fn add_vertex_mapping(&mut self, id: &str, shape: Option<&str>, loc: Option<Loc>) {
        if id.is_empty() {
            return;
        }
        let position = position_of(loc, self.line_offset);
        let index = self.push_mapping(id, StatementType::Vertex, position);
        if shape.is_none() {
            self.bare_mappings.insert(index);
        }
    }

    pub fn extend_vertex_mapping(&mut self, id: &str, loc: Option<Loc>) {
        let Some(loc) = loc else {
            return;
        };
        if id.is_empty() {
            return;
        }
        let end = position_of(Some(loc), self.line_offset);
        for index in (0..self.mappings.len()).rev() {
            if self.mappings[index].kind != StatementType::Vertex || self.mappings[index].id != id {
                continue;
            }
            if self.bare_mappings.remove(&index) {
                self.mappings[index].kind = StatementType::Attachment;
            }
            let p = &mut self.mappings[index].position;
            if end.end_line > p.end_line
                || (end.end_line == p.end_line && end.end_column > p.end_column)
            {
                p.end_line = end.end_line;
                p.end_column = end.end_column;
            }
            return;
        }
        self.push_mapping(id, StatementType::Attachment, end);
    }

    pub fn add_edge_mapping(&mut self, to_nodes: &[String], loc: Option<Loc>) {
        let ids: Vec<&str> = to_nodes
            .iter()
            .filter(|n| !n.is_empty())
            .map(String::as_str)
            .collect();
        let id = if ids.is_empty() {
            "edge".to_owned()
        } else {
            ids.join(">")
        };
        let position = position_of(loc, self.line_offset);
        self.push_mapping(&id, StatementType::Edge, position);
    }

    fn block_mapping(
        &mut self,
        id: &str,
        kind: StatementType,
        start_loc: Option<Loc>,
        end_loc: Option<Loc>,
    ) {
        if id.is_empty() {
            return;
        }
        let start = position_of(start_loc, self.line_offset);
        let end = match end_loc {
            Some(_) => position_of(end_loc, self.line_offset),
            None => start,
        };
        self.push_mapping(
            id,
            kind,
            ElementPosition {
                start_line: start.start_line,
                start_column: start.start_column,
                end_line: end.end_line,
                end_column: end.end_column,
                start_index: 0,
                end_index: 0,
            },
        );
    }

    pub fn add_subgraph_mapping(&mut self, id: &str, start_loc: Option<Loc>, end_loc: Option<Loc>) {
        self.block_mapping(id, StatementType::Subgraph, start_loc, end_loc);
    }

    pub fn add_connector_mapping(
        &mut self,
        id: Option<&FlowText>,
        start_loc: Option<Loc>,
        end_loc: Option<Loc>,
    ) {
        let text = id.map_or(String::new(), |t| trim(&t.text).to_owned());
        self.block_mapping(&text, StatementType::Connector, start_loc, end_loc);
    }

    fn emit(
        &mut self,
        id: DiagnosticId,
        severity: Severity,
        message: String,
        node: Option<&str>,
        edge: Option<&str>,
    ) {
        let anchor = node.or(edge).map(str::to_owned);
        let mut diagnostic = Diagnostic {
            id,
            severity,
            message,
            node_id: None,
            edge_id: None,
            position: None,
        };
        if let Some(node) = node {
            diagnostic.node_id = Some(node.to_owned());
        } else if let Some(edge) = edge {
            diagnostic.edge_id = Some(edge.to_owned());
        }
        if let Some(index) = anchor.as_ref().and_then(|a| self.first_mapping.get(a)) {
            diagnostic.position = Some(self.mappings[*index].position);
        }
        let key = anchor.unwrap_or_default();
        match self.reported.get_mut(&key) {
            None => {
                self.reported.insert(key, vec![diagnostic.clone()]);
            }
            Some(same) => {
                if same.iter().any(|d| {
                    d.id == diagnostic.id
                        && d.severity == diagnostic.severity
                        && d.message == diagnostic.message
                        && d.node_id == diagnostic.node_id
                        && d.edge_id == diagnostic.edge_id
                }) {
                    return;
                }
                same.push(diagnostic.clone());
            }
        }
        self.diagnostics.push(diagnostic);
    }

    pub fn emit_warning(
        &mut self,
        id: DiagnosticId,
        message: String,
        node: Option<&str>,
        edge: Option<&str>,
    ) {
        self.emit(id, Severity::Warning, message, node, edge);
    }

    pub fn emit_error(
        &mut self,
        id: DiagnosticId,
        message: String,
        node: Option<&str>,
        edge: Option<&str>,
    ) {
        self.emit(id, Severity::Error, message, node, edge);
    }

    pub fn is_connector(&self, id: &str) -> bool {
        self.connectors.contains(id)
    }

    pub fn node_kind(&self, id: &str) -> Option<&'static str> {
        if self.connectors.contains(id) {
            return Some("connector");
        }
        self.nodes
            .get(id)
            .map(|vertex| Self::vertex_kind(vertex).as_str())
    }
}

pub fn is_tool_definition(vertex: &AgentNode) -> bool {
    vertex
        .shape
        .as_deref()
        .is_some_and(|shape| TOOL_SHAPES.contains(&shape))
}
