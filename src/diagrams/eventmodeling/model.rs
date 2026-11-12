use std::collections::HashMap;

use super::parser::{EventModelAst, Ref};
use crate::diagrams::common::tokens::{acc_descr_value, acc_title_value, title_value};
use crate::errors::{PelersError, PelersErrorCode, locate};
use crate::util::chars::{is_js_space, trim, trim_end};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Ui,
    Processor,
    Command,
    Readmodel,
    Event,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Ui => "ui",
            EntityKind::Processor => "processor",
            EntityKind::Command => "command",
            EntityKind::Readmodel => "readmodel",
            EntityKind::Event => "event",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaneGroup {
    Trigger,
    State,
    Stream,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmLane {
    pub id: usize,
    pub index: i64,
    pub label: String,
    pub group: LaneGroup,
    pub namespace: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmBox {
    pub id: String,
    pub kind: EntityKind,
    pub entity: String,
    pub name: String,
    pub namespace: Option<String>,
    pub reset: bool,
    pub lane: usize,
    pub data: Option<String>,
    pub data_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmRelation {
    pub source: usize,
    pub target: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmNote {
    pub frame: String,
    pub text: String,
    pub data_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EmSpecification {
    pub frame: String,
    pub given: Vec<String>,
    pub when: Vec<String>,
    pub then: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventModelingModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub boxes: Vec<EmBox>,
    pub lanes: Vec<EmLane>,
    pub relations: Vec<EmRelation>,
    pub notes: Vec<EmNote>,
    pub specifications: Vec<EmSpecification>,
    pub entities: Vec<String>,
}

impl EventModelingModel {
    pub fn lane(&self, id: usize) -> &EmLane {
        self.lanes
            .iter()
            .find(|lane| lane.id == id)
            .expect("every box's lane is in the lane list")
    }
}

fn kind_of(name: &str) -> EntityKind {
    match name {
        "ui" => EntityKind::Ui,
        "pcr" | "processor" => EntityKind::Processor,
        "rmo" | "readmodel" => EntityKind::Readmodel,
        "cmd" | "command" => EntityKind::Command,
        "evt" | "event" => EntityKind::Event,
        _ => EntityKind::Event,
    }
}

fn group_of(kind: EntityKind) -> LaneGroup {
    match kind {
        EntityKind::Ui | EntityKind::Processor => LaneGroup::Trigger,
        EntityKind::Event => LaneGroup::Stream,
        _ => LaneGroup::State,
    }
}

fn group_info(group: LaneGroup) -> (i64, &'static str, &'static str) {
    match group {
        LaneGroup::Trigger => (0, "UI/Automation", "UI/A: "),
        LaneGroup::State => (100, "Command/Read Model", "C/RM: "),
        LaneGroup::Stream => (200, "Events", "Stream: "),
    }
}

fn block_text(raw: &str) -> String {
    let open = raw.find('{');
    let close = raw.rfind('}');
    let inner: String = match (open, close) {
        (Some(o), Some(c)) if c > o => raw[o + 1..c].to_owned(),
        (None, _) if raw.chars().count() >= 2 => {
            let mut chars: Vec<char> = raw.chars().collect();
            chars.pop();
            chars.remove(0);
            chars.into_iter().collect()
        }
        _ => String::new(),
    };
    let mut lines: Vec<String> = inner
        .replace('\r', "")
        .split('\n')
        .map(str::to_owned)
        .collect();
    while lines.first().is_some_and(|line| trim(line).is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| trim(line).is_empty()) {
        lines.pop();
    }
    let mut indent = usize::MAX;
    for line in &lines {
        if trim(line).is_empty() {
            continue;
        }
        let leading = line.chars().take_while(|&c| is_js_space(c)).count();
        indent = indent.min(leading);
    }
    lines
        .iter()
        .map(|line| {
            let skip = indent.min(line.chars().count());
            trim_end(&line.chars().skip(skip).collect::<String>()).to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct LaneTable {
    lanes: Vec<EmLane>,
    by_key: HashMap<String, usize>,
    next: HashMap<&'static str, i64>,
}

impl LaneTable {
    fn lane(&mut self, kind: EntityKind, namespace: Option<&str>) -> usize {
        let group = group_of(kind);
        let group_key = match group {
            LaneGroup::Trigger => "trigger",
            LaneGroup::State => "state",
            LaneGroup::Stream => "stream",
        };
        let key = match namespace {
            None => group_key.to_owned(),
            Some(ns) => format!("{group_key} {ns}"),
        };
        if let Some(&id) = self.by_key.get(&key) {
            return id;
        }
        let (first, label, prefix) = group_info(group);
        let index = match namespace {
            None => first,
            Some(_) => {
                let index = self.next.get(group_key).copied().unwrap_or(first + 1);
                self.next.insert(group_key, index + 1);
                index
            }
        };
        let id = self.lanes.len();
        self.lanes.push(EmLane {
            id,
            index,
            label: match namespace {
                None => label.to_owned(),
                Some(ns) => format!("{prefix}{ns}"),
            },
            group,
            namespace: namespace.map(str::to_owned),
        });
        self.by_key.insert(key, id);
        id
    }
}

pub fn build_eventmodel(
    ast: &EventModelAst,
    title: Option<&str>,
) -> Result<EventModelingModel, PelersError> {
    let mut ids: Vec<&str> = Vec::new();
    for frame in &ast.frames {
        if ids.contains(&frame.name.as_str()) {
            let (line, _, _) = locate(&ast.source, frame.offset);
            let mut error = PelersError::new(
                format!(
                    "Duplicate event modeling frame ID \"{}\" on line {line}",
                    frame.name
                ),
                PelersErrorCode::Semantic,
            );
            error.diagram_type = Some("eventmodeling".to_owned());
            error.line = line;
            return Err(error);
        }
        ids.push(&frame.name);
    }

    let mut data: HashMap<&str, &str> = HashMap::new();
    for entity in &ast.data_entities {
        data.entry(entity.name.as_str())
            .or_insert(entity.data_block_value.as_str());
    }

    let mut table = LaneTable {
        lanes: Vec::new(),
        by_key: HashMap::new(),
        next: HashMap::new(),
    };
    let mut boxes: Vec<EmBox> = Vec::new();
    let mut relations: Vec<EmRelation> = Vec::new();
    for (index, frame) in ast.frames.iter().enumerate() {
        let parts: Vec<&str> = frame.entity_identifier.split('.').collect();
        let namespace = (parts.len() == 2).then(|| parts[0]);
        let kind = kind_of(&frame.model_entity_type);
        let mut text = frame.data_inline_value.as_deref().map(block_text);
        if let Some(reference) = &frame.data_reference
            && let Some(block) = data.get(reference.text.as_str())
        {
            text = Some(block_text(block));
        }
        let lane = table.lane(kind, namespace);
        let entity_box = EmBox {
            id: frame.name.clone(),
            kind,
            entity: frame.entity_identifier.clone(),
            name: if parts.len() == 2 {
                parts[1].to_owned()
            } else {
                frame.entity_identifier.clone()
            },
            namespace: namespace.map(str::to_owned),
            reset: frame.reset,
            lane,
            data: text,
            data_type: frame.data_type.clone(),
        };
        boxes.push(entity_box);
        let target = boxes.len() - 1;

        if !frame.source_frames.is_empty() {
            let wanted: Vec<&str> = frame
                .source_frames
                .iter()
                .map(|r: &Ref| r.text.as_str())
                .collect();
            for (source, boxed) in boxes.iter().enumerate() {
                if wanted.contains(&boxed.id.as_str()) {
                    relations.push(EmRelation { source, target });
                }
            }
        } else if index > 0 && !boxes[target].reset {
            for source in (0..index).rev() {
                if boxes[source].lane != boxes[target].lane {
                    relations.push(EmRelation { source, target });
                    break;
                }
            }
        }
    }

    let mut lanes = table.lanes;
    lanes.sort_by_key(|lane| lane.index);

    let title = match ast
        .title
        .as_deref()
        .map(title_value)
        .filter(|t| !t.is_empty())
    {
        Some(title) => Some(title),
        None => title.map(str::to_owned),
    };
    Ok(EventModelingModel {
        title,
        acc_title: ast.acc_title.as_deref().filter(|t| !t.is_empty()).map(|t| {
            acc_title_value(t)
                .trim_start_matches(is_js_space)
                .to_owned()
        }),
        acc_descr: ast
            .acc_descr
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| collapse_indented_lines(&acc_descr_value(t))),
        boxes,
        lanes,
        relations,
        notes: ast
            .note_entities
            .iter()
            .map(|note| EmNote {
                frame: note.source_frame.text.clone(),
                text: block_text(&note.data_block_value),
                data_type: note.data_type.clone(),
            })
            .collect(),
        specifications: ast
            .gwt_entities
            .iter()
            .map(|gwt| EmSpecification {
                frame: gwt.source_frame.text.clone(),
                given: gwt.given.clone(),
                when: gwt.when.clone(),
                then: gwt.then.clone(),
            })
            .collect(),
        entities: ast.model_entities.clone(),
    })
}

fn collapse_indented_lines(text: &str) -> String {
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
