use std::collections::{HashMap, HashSet};

use super::shapes::is_valid_shape;
use super::types::{
    DocItem, FlowClassDef, FlowEdge, FlowNode, FlowSubgraph, FlowText, LabelType, LinkInfo,
};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::url::safe_url;
use crate::util::value::{Object, Value};
use crate::util::yaml::parse_yaml;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DbOptions {
    pub max_edges: Option<usize>,
    pub inherit_dir: bool,
    pub direction: Option<String>,
}

fn label_type(kind: Option<&str>) -> LabelType {
    match kind {
        Some("string") => LabelType::String,
        Some("text") => LabelType::Text,
        _ => LabelType::Markdown,
    }
}

fn unquote(text: &str) -> String {
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        text[1..text.len() - 1].to_owned()
    } else {
        text.to_owned()
    }
}

fn count_char(ch: char, s: &str) -> usize {
    s.chars().filter(|&c| c == ch).count()
}

fn js_to_number(text: &str) -> f64 {
    let t = text.trim();
    if t.is_empty() {
        return 0.0;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn value_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => *n,
        Value::String(s) => js_to_number(s),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null => 0.0,
        _ => f64::NAN,
    }
}

fn is_js_space(c: char) -> bool {
    crate::util::chars::is_js_space(c)
}

fn trim_js(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

#[derive(Clone, Debug, PartialEq)]
pub struct StartLink {
    pub kind: String,
    pub stroke: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EndLink {
    pub kind: String,
    pub stroke: String,
    pub length: f64,
}

pub fn destruct_start_link(raw: &str) -> StartLink {
    let mut s = raw.trim();
    let mut kind = "arrow_open";
    match s.chars().next() {
        Some('<') => {
            kind = "arrow_point";
            s = &s[1..];
        }
        Some('x') => {
            kind = "arrow_cross";
            s = &s[1..];
        }
        Some('o') => {
            kind = "arrow_circle";
            s = &s[1..];
        }
        _ => {}
    }
    let mut stroke = "normal";
    if s.contains('=') {
        stroke = "thick";
    }
    if s.contains('.') {
        stroke = "dotted";
    }
    StartLink {
        kind: kind.to_owned(),
        stroke: stroke.to_owned(),
    }
}

pub fn destruct_end_link(raw: &str) -> EndLink {
    let s = raw.trim();
    let mut line: &str = if s.is_empty() {
        ""
    } else {
        &s[..s.len() - s.chars().last().map_or(0, char::len_utf8)]
    };
    let last = s.chars().last();
    let mut kind = "arrow_open".to_owned();
    match last {
        Some('x') => {
            kind = "arrow_cross".to_owned();
            if s.starts_with('x') {
                kind = format!("double_{kind}");
                line = &line[1..];
            }
        }
        Some('>') => {
            kind = "arrow_point".to_owned();
            if s.starts_with('<') {
                kind = format!("double_{kind}");
                line = &line[1..];
            }
        }
        Some('o') => {
            kind = "arrow_circle".to_owned();
            if s.starts_with('o') {
                kind = format!("double_{kind}");
                line = &line[1..];
            }
        }
        _ => {}
    }
    let mut stroke = "normal";
    let mut length = line.chars().count() as f64 - 1.0;
    if line.starts_with('=') {
        stroke = "thick";
    }
    if line.starts_with('~') {
        stroke = "invisible";
    }
    let dots = count_char('.', line);
    if dots > 0 {
        stroke = "dotted";
        length = dots as f64;
    }
    EndLink {
        kind,
        stroke: stroke.to_owned(),
        length,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FlowDb {
    pub direction: Option<String>,
    pub nodes: Vec<FlowNode>,
    node_index: HashMap<String, usize>,
    pub edges: Vec<FlowEdge>,
    pub subgraphs: Vec<FlowSubgraph>,
    pub classes: Vec<FlowClassDef>,
    class_index: HashMap<String, usize>,
    pub tooltips: Vec<(String, String)>,
    pub default_edge_style: Option<Vec<String>>,
    pub default_interpolate: Option<String>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub warnings: Vec<String>,
    subgraph_lookup: HashMap<String, usize>,
    members: HashSet<String>,
    edge_by_id: HashMap<String, usize>,
    pair_count: HashMap<String, usize>,
    sub_count: usize,
    options: DbOptions,
}

#[derive(Default)]
pub struct VertexAttrs<'a> {
    pub style: Option<Vec<String>>,
    pub classes: Option<Vec<String>>,
    pub dir: Option<&'a str>,
    pub props: Option<Vec<(String, String)>>,
}

impl FlowDb {
    pub fn new(options: DbOptions) -> Self {
        Self {
            options,
            ..Self::default()
        }
    }

    fn node_mut(&mut self, id: &str) -> Option<&mut FlowNode> {
        let idx = *self.node_index.get(id)?;
        self.nodes.get_mut(idx)
    }

    fn class_entry(&mut self, id: &str) -> &mut FlowClassDef {
        let idx = match self.class_index.get(id) {
            Some(&idx) => idx,
            None => {
                self.classes.push(FlowClassDef {
                    id: id.to_owned(),
                    styles: Vec::new(),
                    text_styles: Vec::new(),
                });
                let idx = self.classes.len() - 1;
                self.class_index.insert(id.to_owned(), idx);
                idx
            }
        };
        &mut self.classes[idx]
    }

    pub fn add_vertex(
        &mut self,
        id: &str,
        text_obj: Option<&FlowText>,
        kind: Option<&str>,
        attrs: VertexAttrs<'_>,
        metadata: Option<&str>,
    ) -> Result<(), PelersError> {
        let VertexAttrs {
            style,
            classes,
            dir,
            props,
        } = attrs;
        if trim_js(id).is_empty() {
            return Ok(());
        }

        let mut doc: Option<Object> = None;
        if let Some(metadata) = metadata {
            let yaml = if metadata.contains('\n') {
                format!("{metadata}\n")
            } else {
                format!("{{\n{metadata}\n}}")
            };
            let parsed = parse_yaml(&yaml).map_err(|message| {
                let mut error = PelersError::new(
                    format!("Metadata of \"{id}\" is not valid YAML. {message}"),
                    PelersErrorCode::Syntax,
                );
                error.diagram_type = Some("flowchart".to_owned());
                error
            })?;
            doc = match parsed {
                Value::Object(object) => Some(object),
                _ => None,
            };
        }

        if let Some(doc) = doc.as_ref()
            && let Some(idx) = self.subgraph_lookup.get(id).copied()
        {
            for (key, value) in doc.iter() {
                let metadata = &mut self.subgraphs[idx].metadata;
                match metadata.iter_mut().find(|(k, _)| k.as_str() == key) {
                    Some(slot) => slot.1 = value.clone(),
                    None => metadata.push((key.to_owned(), value.clone())),
                }
            }
            return Ok(());
        }

        if let Some(&edge_idx) = self.edge_by_id.get(id) {
            if let Some(doc) = doc.as_ref() {
                if let Some(v) = doc.get("animate") {
                    self.edges[edge_idx].animate = Some(v.clone());
                }
                if let Some(v) = doc.get("animation") {
                    self.edges[edge_idx].animation = Some(v.clone());
                }
                if let Some(v) = doc.get("curve") {
                    self.edges[edge_idx].interpolate = Some(v.clone());
                }
            }
            return Ok(());
        }

        if !self.node_index.contains_key(id) {
            if text_obj.is_none() && kind.is_none() && style.is_some() {
                self.warnings.push(format!(
                    "Style applied to unknown node \"{id}\". This may indicate a typo. The node will be created automatically."
                ));
            }
            self.nodes.push(FlowNode {
                id: id.to_owned(),
                label_type: LabelType::Text,
                text: None,
                kind: None,
                styles: Vec::new(),
                classes: Vec::new(),
                dir: None,
                props: Vec::new(),
                link: None,
                link_target: None,
                icon: None,
                form: None,
                pos: None,
                img: None,
                constraint: None,
                asset_width: None,
                asset_height: None,
            });
            self.node_index.insert(id.to_owned(), self.nodes.len() - 1);
        }

        let vertex = self.node_mut(id).expect("vertex was just ensured");
        if let Some(text) = text_obj {
            vertex.label_type = text.kind;
            vertex.text = Some(unquote(trim_js(&text.text)));
        } else if vertex.text.is_none() {
            vertex.text = Some(id.to_owned());
        }
        if let Some(kind) = kind {
            vertex.kind = Some(kind.to_owned());
        }
        if let Some(style) = style {
            vertex.styles.extend(style);
        }
        if let Some(classes) = classes {
            vertex.classes.extend(classes);
        }
        if let Some(dir) = dir {
            vertex.dir = Some(dir.to_owned());
        }
        for (key, value) in props.unwrap_or_default() {
            match vertex.props.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => vertex.props.push((key, value)),
            }
        }

        let Some(doc) = doc else {
            return Ok(());
        };

        if let Some(shape) = doc.get("shape").filter(|v| v.is_truthy()) {
            let shape = shape.to_js_string();
            if shape != shape.to_lowercase() || shape.contains('_') {
                let mut error = PelersError::new(
                    format!("No such shape: {shape}. Shape names should be lowercase."),
                    PelersErrorCode::Semantic,
                );
                error.diagram_type = Some("flowchart".to_owned());
                return Err(error);
            }
            if !is_valid_shape(&shape) {
                let mut error = PelersError::new(
                    format!("No such shape: {shape}."),
                    PelersErrorCode::Semantic,
                );
                error.diagram_type = Some("flowchart".to_owned());
                return Err(error);
            }
            self.node_mut(id).expect("vertex exists").kind = Some(shape);
        }

        let label = doc.get("label").and_then(Value::as_str).map(str::to_owned);
        let label_blank = label.as_deref().is_none_or(|l| trim_js(l).is_empty());
        if let Some(label_value) = doc.get("label").filter(|v| v.is_truthy()) {
            let vertex = self.node_mut(id).expect("vertex exists");
            vertex.text = Some(label_value.to_js_string());
            vertex.label_type = label_type(doc.get("labelType").and_then(Value::as_str));
        }
        if let Some(icon) = doc.get("icon").filter(|v| v.is_truthy()) {
            let vertex = self.node_mut(id).expect("vertex exists");
            vertex.icon = Some(icon.to_js_string());
            if label_blank && vertex.text.as_deref() == Some(id) {
                vertex.text = Some(String::new());
            }
        }
        if let Some(form) = doc.get("form").filter(|v| v.is_truthy()) {
            self.node_mut(id).expect("vertex exists").form = Some(form.to_js_string());
        }
        if let Some(pos) = doc.get("pos").filter(|v| v.is_truthy()) {
            self.node_mut(id).expect("vertex exists").pos = Some(pos.to_js_string());
        }
        if let Some(img) = doc.get("img").filter(|v| v.is_truthy()) {
            let vertex = self.node_mut(id).expect("vertex exists");
            vertex.img = Some(img.to_js_string());
            if label_blank && vertex.text.as_deref() == Some(id) {
                vertex.text = Some(String::new());
            }
        }
        if let Some(constraint) = doc.get("constraint").filter(|v| v.is_truthy()) {
            self.node_mut(id).expect("vertex exists").constraint = Some(constraint.to_js_string());
        }
        if let Some(w) = doc.get("w").filter(|v| v.is_truthy()) {
            self.node_mut(id).expect("vertex exists").asset_width = Some(value_number(w));
        }
        if let Some(h) = doc.get("h").filter(|v| v.is_truthy()) {
            self.node_mut(id).expect("vertex exists").asset_height = Some(value_number(h));
        }
        Ok(())
    }

    pub fn add_single_link(
        &mut self,
        start: &str,
        end: &str,
        link: Option<&LinkInfo>,
        id: Option<&str>,
    ) -> Result<(), PelersError> {
        let mut edge = FlowEdge {
            start: start.to_owned(),
            end: end.to_owned(),
            kind: None,
            text: String::new(),
            label_type: LabelType::Text,
            classes: Vec::new(),
            style: None,
            stroke: None,
            length: None,
            id: None,
            is_user_defined_id: false,
            interpolate: self.default_interpolate.clone().map(Value::String),
            animate: None,
            animation: None,
        };
        if let Some(text) = link.and_then(|l| l.text.as_ref()) {
            edge.text = unquote(trim_js(&text.text));
            edge.label_type = text.kind;
        }
        if let Some(link) = link {
            edge.kind = Some(link.kind.clone());
            edge.stroke = Some(link.stroke.clone());
            edge.length = link.length.map(|l| if l > 10.0 { 10.0 } else { l });
        }
        let pair = format!("{start}\0{end}");
        let count = self.pair_count.get(&pair).copied().unwrap_or(0);
        match id {
            Some(id) if !id.is_empty() && !self.edge_by_id.contains_key(id) => {
                edge.id = Some(id.to_owned());
                edge.is_user_defined_id = true;
            }
            _ => {
                let suffix = if count == 0 { 0 } else { count + 1 };
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
            error.diagram_type = Some("flowchart".to_owned());
            return Err(error);
        }
        let edge_id = edge.id.clone().expect("edge id is set");
        self.edges.push(edge);
        self.pair_count.insert(pair, count + 1);
        if !self.edge_by_id.contains_key(&edge_id) {
            self.edge_by_id.insert(edge_id, self.edges.len() - 1);
        }
        Ok(())
    }

    pub fn add_link(
        &mut self,
        starts: &[String],
        ends: &[String],
        link: Option<&LinkInfo>,
    ) -> Result<(), PelersError> {
        let id = link
            .and_then(|l| l.id.as_deref())
            .map(|s| s.replacen('@', "", 1));
        let last_start = starts.last();
        let first_end = ends.first();
        for start in starts {
            for end in ends {
                let is_last = Some(start) == last_start && Some(end) == first_end;
                let edge_id = if is_last { id.as_deref() } else { None };
                self.add_single_link(start, end, link, edge_id)?;
            }
        }
        Ok(())
    }

    fn edge_at(&self, pos: &str) -> Result<usize, PelersError> {
        let n = js_to_number(pos);
        let len = self.edges.len();
        if n.fract() == 0.0 && n >= 0.0 && (n as usize) < len {
            return Ok(n as usize);
        }
        let mut error = PelersError::new(
            format!(
                "The index {pos} for linkStyle is out of bounds. Valid indices for linkStyle are between 0 and {}. (Help: Ensure that the index is within the range of existing edges.)",
                len as i64 - 1
            ),
            PelersErrorCode::Semantic,
        );
        error.diagram_type = Some("flowchart".to_owned());
        Err(error)
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
                let idx = self.edge_at(pos)?;
                self.edges[idx].interpolate = Some(Value::String(interpolate.to_owned()));
            }
        }
        Ok(())
    }

    pub fn update_link(
        &mut self,
        positions: &[String],
        style: Vec<String>,
    ) -> Result<(), PelersError> {
        let mut shared = style;
        let mut default_assigned = false;
        let mut assigned: Vec<usize> = Vec::new();
        for pos in positions {
            if pos == "default" {
                default_assigned = true;
                self.default_edge_style = Some(shared.clone());
                continue;
            }
            let idx = self.edge_at(pos)?;
            self.edges[idx].style = Some(shared.clone());
            assigned.push(idx);
            if !shared.is_empty() && !shared.iter().any(|s| s.starts_with("fill")) {
                shared.push("fill:none".to_owned());
                if default_assigned {
                    self.default_edge_style = Some(shared.clone());
                }
                for &i in &assigned {
                    self.edges[i].style = Some(shared.clone());
                }
            }
        }
        Ok(())
    }

    pub fn add_class(&mut self, ids: &str, styles: &[String]) {
        let joined = styles.join(",");
        let escaped = joined
            .replace("\\,", "\u{a7}\u{a7}\u{a7}")
            .replace(',', ";")
            .replace("\u{a7}\u{a7}\u{a7}", ",");
        let style: Vec<String> = escaped.split(';').map(str::to_owned).collect();
        for id in ids.split(',') {
            let class_node = self.class_entry(id);
            for s in &style {
                if s.contains("color") {
                    class_node.text_styles.push(s.replacen("fill", "bgFill", 1));
                }
                class_node.styles.push(s.clone());
            }
        }
    }

    pub fn set_direction(&mut self, dir: &str) {
        let mut d = dir.trim().to_owned();
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
            if let Some(&idx) = self.node_index.get(id) {
                self.nodes[idx].classes.push(class_name.to_owned());
            }
            if let Some(&idx) = self.edge_by_id.get(id) {
                self.edges[idx].classes.push(class_name.to_owned());
            }
            if let Some(&idx) = self.subgraph_lookup.get(id) {
                self.subgraphs[idx].classes.push(class_name.to_owned());
            }
        }
    }

    pub fn set_tooltip(&mut self, ids: &str, tooltip: Option<&str>) {
        let Some(tooltip) = tooltip else {
            return;
        };
        for id in ids.split(',') {
            match self.tooltips.iter_mut().find(|(k, _)| k == id) {
                Some(slot) => slot.1 = tooltip.to_owned(),
                None => self.tooltips.push((id.to_owned(), tooltip.to_owned())),
            }
        }
    }

    pub fn set_link(&mut self, ids: &str, link: &str, target: Option<&str>) {
        let url = link.trim();
        for id in ids.split(',') {
            if let Some(vertex) = self.node_mut(id) {
                vertex.link = if url.is_empty() {
                    None
                } else {
                    Some(safe_url(url))
                };
                vertex.link_target = target.map(str::to_owned);
            }
        }
        self.set_class(ids, "clickable");
    }

    pub fn set_click_event(&mut self, ids: &str) {
        self.set_class(ids, "clickable");
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(text.trim_start_matches(is_js_space).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\n' {
                while chars.peek().is_some_and(|&n| is_js_space(n)) {
                    chars.next();
                }
            }
            out.push(c);
        }
        self.acc_descr = Some(out);
    }

    pub fn add_subgraph(
        &mut self,
        id_text: Option<&str>,
        list: &[DocItem],
        title_text: Option<(&str, LabelType)>,
        title_is_id: bool,
    ) -> String {
        let mut id: Option<String> = id_text.map(|t| trim_js(t).to_owned());
        let title_source = title_text.map(|(t, _)| t).unwrap_or("");
        if id_text.is_some() && title_is_id && title_source.chars().any(is_js_space) {
            id = None;
        }

        let mut seen: HashSet<String> = HashSet::new();
        let mut node_list: Vec<String> = Vec::new();
        let mut dir: Option<String> = None;
        let take = |s: &str, seen: &mut HashSet<String>, node_list: &mut Vec<String>| {
            if !trim_js(s).is_empty() && !seen.contains(s) {
                seen.insert(s.to_owned());
                node_list.push(s.to_owned());
            }
        };
        for entry in list {
            match entry {
                DocItem::Id(s) => take(s, &mut seen, &mut node_list),
                DocItem::Ids(items) => {
                    for item in items {
                        take(item, &mut seen, &mut node_list);
                    }
                }
                DocItem::Dir(d) => dir = Some(d.clone()),
            }
        }
        if dir.is_none() {
            dir = if self.options.inherit_dir {
                self.direction
                    .clone()
                    .or_else(|| self.options.direction.clone())
            } else {
                None
            };
        }

        let id = id.unwrap_or_else(|| format!("subGraph{}", self.sub_count));
        let title = title_source.to_owned();
        self.sub_count += 1;

        let members = &mut self.members;
        let filtered: Vec<String> = node_list
            .into_iter()
            .filter(|node_id| !members.contains(node_id) && *node_id != id)
            .collect();
        for node_id in &filtered {
            members.insert(node_id.clone());
        }

        let subgraph = FlowSubgraph {
            id: id.clone(),
            nodes: filtered,
            title: trim_js(&title).to_owned(),
            classes: Vec::new(),
            dir,
            label_type: title_text.map_or(LabelType::Markdown, |(_, k)| k),
            metadata: Vec::new(),
        };
        match self.subgraph_lookup.get(&id).copied() {
            Some(existing) => {
                let extra = subgraph.nodes;
                self.subgraphs[existing].nodes.extend(extra);
            }
            None => {
                self.subgraphs.push(subgraph);
                self.subgraph_lookup
                    .insert(id.clone(), self.subgraphs.len() - 1);
            }
        }
        id
    }

    pub fn destruct_link(&self, end: &str, start: Option<&str>) -> LinkInfo {
        let info = destruct_end_link(end);
        let invalid = LinkInfo {
            kind: "INVALID".to_owned(),
            stroke: "INVALID".to_owned(),
            ..LinkInfo::default()
        };
        let Some(start) = start.filter(|s| !s.is_empty()) else {
            return LinkInfo {
                kind: info.kind,
                stroke: info.stroke,
                length: Some(info.length),
                ..LinkInfo::default()
            };
        };
        let mut start_info = destruct_start_link(start);
        if start_info.stroke != info.stroke {
            return invalid;
        }
        if start_info.kind == "arrow_open" {
            start_info.kind = info.kind.clone();
        } else {
            if start_info.kind != info.kind {
                return invalid;
            }
            start_info.kind = format!("double_{}", start_info.kind);
        }
        if start_info.kind == "double_arrow" {
            start_info.kind = "double_arrow_point".to_owned();
        }
        LinkInfo {
            kind: start_info.kind,
            stroke: start_info.stroke,
            length: Some(info.length),
            ..LinkInfo::default()
        }
    }

    pub fn class(&self, id: &str) -> Option<&FlowClassDef> {
        self.class_index.get(id).map(|&idx| &self.classes[idx])
    }

    pub fn tooltip(&self, id: &str) -> Option<&str> {
        self.tooltips
            .iter()
            .find(|(k, _)| k == id)
            .map(|(_, v)| v.as_str())
    }

    pub fn subgraph(&self, id: &str) -> Option<&FlowSubgraph> {
        self.subgraph_lookup
            .get(id)
            .map(|&idx| &self.subgraphs[idx])
    }
}
