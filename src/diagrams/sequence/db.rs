use std::collections::HashMap;

use super::colors::is_css_color;
use super::types::{
    Item, MessageBody, SeqActor, SeqBox, SeqBoxData, SeqMessage, SeqNote, SeqNumbering, SeqText,
    Statement, Who, linetype,
};
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::{is_js_space, trim, trim_start};
use crate::util::ordered::OrderedMap;
use crate::util::value::Value;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeqDbOptions {
    pub wrap: Option<bool>,
    pub sequence_wrap: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeqDb {
    pub actors: OrderedMap<SeqActor>,
    pub created_actors: OrderedMap<usize>,
    pub destroyed_actors: OrderedMap<usize>,
    pub boxes: Vec<SeqBox>,
    pub messages: Vec<SeqMessage>,
    pub notes: Vec<SeqNote>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    prev_actor: Option<String>,
    current_box: Option<usize>,
    last_created: Option<String>,
    last_destroyed: Option<String>,
    wrap_enabled: Option<bool>,
    sequence_wrap: bool,
    active: HashMap<String, i64>,
}

fn semantic(message: String) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("sequence".to_owned());
    error
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn strip_wrap_prefix(text: &str) -> Option<&str> {
    let rest = text.strip_prefix(':').unwrap_or(text);
    rest.strip_prefix("nowrap:")
        .or_else(|| rest.strip_prefix("wrap:"))
}

fn has_wrap_prefix(text: &str) -> Option<bool> {
    let rest = text.strip_prefix(':').unwrap_or(text);
    if rest.starts_with("wrap:") {
        Some(true)
    } else if rest.starts_with("nowrap:") {
        Some(false)
    } else {
        None
    }
}

fn extract_wrap(raw: &str) -> SeqText {
    let text = trim(raw);
    let wrap = has_wrap_prefix(text);
    let out = match wrap {
        None => text.to_owned(),
        Some(_) => trim(strip_wrap_prefix(text).unwrap_or(text)).to_owned(),
    };
    SeqText { text: out, wrap }
}

fn function_prefix_len(text: &str) -> Option<usize> {
    for name in ["rgba", "rgb", "hsla", "hsl"] {
        if let Some(rest) = text.strip_prefix(name) {
            let after = rest.trim_start_matches(is_js_space);
            if after.starts_with('(') {
                return Some(text.len() - after.len() + 1);
            }
        }
    }
    None
}

fn split_box(text: &str) -> Option<(String, String)> {
    let line_terminator = |s: &str| {
        s.chars()
            .any(|c| matches!(c, '\r' | '\u{2028}' | '\u{2029}'))
    };
    if let Some(prefix) = function_prefix_len(text)
        && let Some(close) = text.rfind(')')
        && close + 1 >= prefix
    {
        if line_terminator(&text[prefix..]) {
            return None;
        }
        return Some((text[..close + 1].to_owned(), text[close + 1..].to_owned()));
    }
    if line_terminator(text) {
        return None;
    }
    let word_len = text
        .chars()
        .take_while(|&c| is_word(c))
        .map(char::len_utf8)
        .sum::<usize>();
    Some((text[..word_len].to_owned(), text[word_len..].to_owned()))
}

fn js_slice(text: &str, start: i64, end: i64) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len() as i64;
    let norm = |i: i64| if i < 0 { (len + i).max(0) } else { i.min(len) };
    let (s, e) = (norm(start) as usize, norm(end) as usize);
    if s >= e {
        return String::new();
    }
    chars[s..e].iter().collect()
}

fn entries(value: &Value) -> Vec<(String, Value)> {
    match value {
        Value::String(text) => text
            .chars()
            .enumerate()
            .map(|(i, ch)| (i.to_string(), Value::String(ch.to_string())))
            .collect(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, item)| (i.to_string(), item.clone()))
            .collect(),
        Value::Object(object) => object
            .iter()
            .map(|(k, v)| (k.to_owned(), v.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

impl SeqDb {
    pub fn new(options: SeqDbOptions) -> Self {
        Self {
            wrap_enabled: options.wrap,
            sequence_wrap: options.sequence_wrap,
            ..Self::default()
        }
    }

    pub fn auto_wrap(&self) -> bool {
        self.wrap_enabled.unwrap_or(self.sequence_wrap)
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(trim_start(text).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        let mut out = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            out.push(c);
            if c == '\n' {
                while chars.peek().is_some_and(|&next| is_js_space(next)) {
                    chars.next();
                }
            }
        }
        self.acc_descr = Some(out);
    }

    pub fn set_diagram_title(&mut self, text: &str) {
        self.title = Some(text.to_owned());
    }

    pub fn parse_message(&self, text: &str) -> SeqText {
        extract_wrap(text)
    }

    pub fn parse_box_data(&self, text: &str) -> SeqBoxData {
        let matched = split_box(text);
        let mut color = match matched.as_ref().map(|m| m.0.as_str()) {
            Some(first) if !first.is_empty() => trim(first).to_owned(),
            _ => "transparent".to_owned(),
        };
        let mut title = match matched.as_ref().map(|m| m.1.as_str()) {
            Some(second) if !second.is_empty() => Some(trim(second).to_owned()),
            _ => None,
        };
        if !is_css_color(&color) {
            color = "transparent".to_owned();
            title = Some(trim(text).to_owned());
        }
        let Some(title) = title else {
            return SeqBoxData {
                text: None,
                color,
                wrap: None,
            };
        };
        let extracted = extract_wrap(&title);
        SeqBoxData {
            text: if extracted.text.is_empty() {
                None
            } else {
                Some(extracted.text)
            },
            color,
            wrap: extracted.wrap,
        }
    }

    pub fn add_box(&mut self, data: &SeqBoxData) {
        let wrap = data.wrap.unwrap_or_else(|| self.auto_wrap());
        self.boxes.push(SeqBox {
            name: data.text.clone(),
            wrap,
            fill: data.color.clone(),
            actor_keys: Vec::new(),
        });
        self.current_box = Some(self.boxes.len() - 1);
    }

    pub fn add_actor(
        &mut self,
        id: &str,
        name: &str,
        description: Option<(String, Option<bool>)>,
        type_: Option<String>,
        metadata: Option<&str>,
    ) -> Result<(), PelersError> {
        let mut assigned_box = self.current_box;
        let mut type_ = type_;
        let mut description = description;
        let mut doc: Option<Value> = None;
        if let Some(metadata) = metadata {
            let source = if metadata.contains('\n') {
                format!("{metadata}\n")
            } else {
                format!("{{\n{metadata}\n}}")
            };
            match crate::util::yaml::parse_yaml(&source) {
                Ok(parsed) => {
                    if matches!(parsed, Value::Object(_)) {
                        doc = Some(parsed);
                    }
                }
                Err(message) => {
                    let mut error = PelersError::new(
                        format!("Config of participant \"{id}\" is not valid YAML. {message}"),
                        PelersErrorCode::Syntax,
                    );
                    error.diagram_type = Some("sequence".to_owned());
                    return Err(error);
                }
            }
        }
        if let Some(Value::Object(object)) = &doc {
            if let Some(value) = object.get("type")
                && !matches!(value, Value::Null)
            {
                type_ = Some(value.to_js_string());
            }
            if let Some(alias) = object.get("alias")
                && alias.is_truthy()
                && description.as_ref().is_none_or(|(text, _)| text == name)
            {
                let wrap = description.as_ref().and_then(|(_, wrap)| *wrap);
                description = Some((alias.to_js_string(), wrap));
            }
        }

        if let Some(old) = self.actors.get(id).cloned() {
            if let (Some(current), Some(old_box)) = (self.current_box, old.box_index)
                && current != old_box
            {
                let old_name = self.boxes[old_box]
                    .name
                    .clone()
                    .unwrap_or_else(|| "undefined".to_owned());
                let current_name = self.boxes[current]
                    .name
                    .clone()
                    .unwrap_or_else(|| "undefined".to_owned());
                return Err(semantic(format!(
                    "A same participant should only be defined in one Box: {} can't be in '{old_name}' and in '{current_name}' at the same time.",
                    old.name
                )));
            }
            assigned_box = old.box_index.or(self.current_box);
            if let Some(actor) = self.actors.get_mut(id) {
                actor.box_index = assigned_box;
            }
            if name == old.name && description.is_none() {
                return Ok(());
            }
        }
        if description.is_none() || type_.is_none() {
            description = Some((name.to_owned(), None));
        }
        let (description_text, description_wrap) = description.unwrap_or_default();
        let wrap = description_wrap.unwrap_or_else(|| self.auto_wrap());
        self.actors.set(
            id,
            SeqActor {
                box_index: assigned_box,
                name: name.to_owned(),
                description: description_text,
                wrap,
                prev_actor: self.prev_actor.clone(),
                next_actor: None,
                links: OrderedMap::default(),
                properties: OrderedMap::default(),
                details: None,
                type_: type_.unwrap_or_else(|| "participant".to_owned()),
            },
        );
        if let Some(prev) = self.prev_actor.clone()
            && let Some(prev_actor) = self.actors.get_mut(&prev)
        {
            prev_actor.next_actor = Some(id.to_owned());
        }
        if let Some(current) = self.current_box {
            self.boxes[current].actor_keys.push(id.to_owned());
        }
        self.prev_actor = Some(id.to_owned());
        Ok(())
    }

    fn add_signal(
        &mut self,
        from: Option<&str>,
        to: Option<&str>,
        message: Option<&SeqText>,
        type_: Option<i32>,
        activate: bool,
        central_connection: Option<i32>,
    ) -> Result<(), PelersError> {
        if type_ == Some(linetype::ACTIVE_END) {
            let count = from.map_or(0, |f| self.active.get(f).copied().unwrap_or(0));
            if count < 1 {
                return Err(semantic(format!(
                    "Trying to inactivate an inactive participant ({})",
                    from.unwrap_or("undefined")
                )));
            }
            if let Some(from) = from {
                self.active.insert(from.to_owned(), count - 1);
            }
        } else if type_ == Some(linetype::ACTIVE_START)
            && let Some(from) = from
        {
            let count = self.active.get(from).copied().unwrap_or(0);
            self.active.insert(from.to_owned(), count + 1);
        }
        let wrap = message
            .and_then(|m| m.wrap)
            .unwrap_or_else(|| self.auto_wrap());
        self.messages.push(SeqMessage {
            id: self.messages.len().to_string(),
            from: from.map(str::to_owned),
            to: to.map(str::to_owned),
            message: MessageBody::Text(message.map_or(String::new(), |m| m.text.clone())),
            wrap,
            type_,
            activate: Some(activate),
            placement: None,
            central_connection: Some(central_connection.unwrap_or(0)),
        });
        Ok(())
    }

    fn add_note(&mut self, actor: Who, placement: i32, message: &SeqText) {
        let wrap = message.wrap.unwrap_or_else(|| self.auto_wrap());
        let (from, to) = match &actor {
            Who::One(name) => (Some(name.clone()), Some(name.clone())),
            Who::Many(names) => (names.first().cloned(), names.get(1).cloned()),
        };
        self.notes.push(SeqNote {
            actor,
            placement,
            message: message.text.clone(),
            wrap,
        });
        self.messages.push(SeqMessage {
            id: self.messages.len().to_string(),
            from,
            to,
            message: MessageBody::Text(message.text.clone()),
            wrap,
            type_: Some(linetype::NOTE),
            activate: None,
            placement: Some(placement),
            central_connection: None,
        });
    }

    fn add_links(&mut self, actor_id: &str, text: &SeqText) {
        let Some(actor) = self.actors.get_mut(actor_id) else {
            return;
        };
        if let Some(parsed) = crate::util::json::parse(&text.text) {
            for (key, value) in entries(&parsed) {
                actor.links.set(&key, value);
            }
        }
    }

    fn add_a_link(&mut self, actor_id: &str, text: &SeqText) {
        let Some(actor) = self.actors.get_mut(actor_id) else {
            return;
        };
        let sep = text.text.find('@').map_or(-1i64, |at| at as i64);
        let key = trim(&js_slice(&text.text, 0, sep - 1)).to_owned();
        let value = trim(&js_slice(&text.text, sep + 1, i64::MAX / 2)).to_owned();
        actor.links.set(&key, Value::String(value));
    }

    fn add_properties(&mut self, actor_id: &str, text: &SeqText) {
        let Some(actor) = self.actors.get_mut(actor_id) else {
            return;
        };
        if let Some(parsed) = crate::util::json::parse(&text.text) {
            for (key, value) in entries(&parsed) {
                actor.properties.set(&key, value);
            }
        }
    }

    fn add_details(&mut self, actor_id: &str, text: &SeqText) {
        if let Some(actor) = self.actors.get_mut(actor_id) {
            actor.details = Some(text.text.clone());
        }
    }

    pub fn apply(&mut self, item: &Item) -> Result<(), PelersError> {
        match item {
            Item::Text(_) => Ok(()),
            Item::List(items) => {
                for child in items {
                    self.apply(child)?;
                }
                Ok(())
            }
            Item::Statement(statement) => self.apply_one(statement),
        }
    }

    fn apply_one(&mut self, param: &Statement) -> Result<(), PelersError> {
        let actor_name = match &param.actor {
            Some(Who::One(name)) => name.clone(),
            _ => String::new(),
        };
        let actor_id = actor_name.as_str();
        match param.type_.as_str() {
            "sequenceIndex" => {
                self.messages.push(SeqMessage {
                    id: self.messages.len().to_string(),
                    from: None,
                    to: None,
                    message: MessageBody::Numbering(SeqNumbering {
                        start: param.sequence_index,
                        step: param.sequence_index_step,
                        visible: param.sequence_visible == Some(true),
                    }),
                    wrap: false,
                    type_: param.signal_type,
                    activate: None,
                    placement: None,
                    central_connection: None,
                });
            }
            "addParticipant" => {
                let description = param.description.as_ref().map(|d| (d.text.clone(), d.wrap));
                self.add_actor(
                    actor_id,
                    actor_id,
                    description,
                    param.draw.clone(),
                    param.config.as_deref(),
                )?;
            }
            "createParticipant" => {
                if self.actors.contains(actor_id) {
                    return Err(semantic(
                        "It is not possible to have actors with the same id, even if one is destroyed before the next is created. Use 'AS' aliases to simulate the behavior".to_owned(),
                    ));
                }
                self.last_created = Some(actor_name.clone());
                let description = param.description.as_ref().map(|d| (d.text.clone(), d.wrap));
                self.add_actor(
                    actor_id,
                    actor_id,
                    description,
                    param.draw.clone(),
                    param.config.as_deref(),
                )?;
                self.created_actors.set(actor_id, self.messages.len());
            }
            "destroyParticipant" => {
                self.last_destroyed = Some(actor_name.clone());
                self.destroyed_actors.set(actor_id, self.messages.len());
            }
            "activeStart" | "activeEnd" | "centralConnection" | "centralConnectionReverse" => {
                self.add_signal(Some(actor_id), None, None, param.signal_type, false, None)?;
            }
            "addNote" => {
                if let (Some(actor), Some(placement), Some(text)) =
                    (&param.actor, param.placement, &param.text)
                {
                    self.add_note(actor.clone(), placement, text);
                }
            }
            "addLinks" => {
                if let Some(text) = &param.text {
                    self.add_links(actor_id, text);
                }
            }
            "addALink" => {
                if let Some(text) = &param.text {
                    self.add_a_link(actor_id, text);
                }
            }
            "addProperties" => {
                if let Some(text) = &param.text {
                    self.add_properties(actor_id, text);
                }
            }
            "addDetails" => {
                if let Some(text) = &param.text {
                    self.add_details(actor_id, text);
                }
            }
            "addMessage" => {
                if let Some(created) = self.last_created.clone() {
                    if param.to.as_deref() != Some(created.as_str()) {
                        return Err(semantic(format!(
                            "The created participant {created} does not have an associated creating message after its declaration. Please check the sequence diagram."
                        )));
                    }
                    self.last_created = None;
                } else if let Some(destroyed) = self.last_destroyed.clone() {
                    if param.to.as_deref() != Some(destroyed.as_str())
                        && param.from.as_deref() != Some(destroyed.as_str())
                    {
                        return Err(semantic(format!(
                            "The destroyed participant {destroyed} does not have an associated destroying message after its declaration. Please check the sequence diagram."
                        )));
                    }
                    self.last_destroyed = None;
                }
                self.add_signal(
                    param.from.as_deref(),
                    param.to.as_deref(),
                    param.msg.as_ref(),
                    param.signal_type,
                    param.activate.unwrap_or(false),
                    param.central_connection,
                )?;
            }
            "boxStart" => {
                if let Some(data) = &param.box_data {
                    self.add_box(data);
                }
            }
            "boxEnd" => self.current_box = None,
            "loopStart" => self.add_signal(
                None,
                None,
                param.loop_text.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "rectStart" => self.add_signal(
                None,
                None,
                param.color.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "optStart" => self.add_signal(
                None,
                None,
                param.opt_text.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "altStart" | "else" => self.add_signal(
                None,
                None,
                param.alt_text.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "parStart" | "and" => self.add_signal(
                None,
                None,
                param.par_text.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "criticalStart" => self.add_signal(
                None,
                None,
                param.critical_text.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "option" => self.add_signal(
                None,
                None,
                param.option_text.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "breakStart" => self.add_signal(
                None,
                None,
                param.break_text.as_ref(),
                param.signal_type,
                false,
                None,
            )?,
            "loopEnd" | "rectEnd" | "optEnd" | "altEnd" | "parEnd" | "criticalEnd" | "breakEnd" => {
                self.add_signal(None, None, None, param.signal_type, false, None)?
            }
            _ => {}
        }
        Ok(())
    }
}
