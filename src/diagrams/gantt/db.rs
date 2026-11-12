use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

use super::dates::{Unit, WEEKDAYS, add, add_days, format, is_valid, iso_date, parse_strict};
use super::jsdate::JsDate;
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::trim;
use crate::util::ordered::OrderedMap;
use crate::util::url::safe_url;

const WS: &str = "\\t\\n\\x0b\\x0c\\r \\x{a0}\\x{1680}\\x{2000}-\\x{200a}\\x{2028}\\x{2029}\\x{202f}\\x{205f}\\x{3000}\\x{feff}";

const WORK_LIMIT: f64 = 4e8;
const DAY_STEP: f64 = 400.0;
const DATED_STEP: f64 = 200.0;
const FORMAT_STEP: f64 = 400.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    pub id: String,
    pub task: String,
    pub section: String,
    pub start_time: Option<JsDate>,
    pub end_time: Option<JsDate>,
    pub render_end_time: Option<JsDate>,
    pub manual_end_time: bool,
    pub processed: bool,
    pub active: bool,
    pub done: bool,
    pub crit: bool,
    pub milestone: bool,
    pub vert: bool,
    pub order: i64,
    pub classes: Vec<String>,
    pub prev_task_id: Option<String>,
    pub raw_data: String,
    pub raw_start: Option<String>,
    pub raw_end: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GanttDb {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub date_format: String,
    pub axis_format: String,
    pub tick_interval: Option<String>,
    pub today_marker: String,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub inclusive_end_dates: bool,
    pub top_axis: bool,
    pub display_mode: String,
    pub weekday: Option<String>,
    pub weekend: String,
    pub sections: Vec<String>,
    pub links: OrderedMap<String>,
    pub tasks: Vec<Task>,
    by_id: HashMap<String, usize>,
    current_section: String,
    task_count: usize,
    last_order: i64,
    last_task_id: Option<String>,
    now: f64,
    work: f64,
    include_set: HashSet<String>,
    exclude_set: HashSet<String>,
    excluded_days: [bool; 7],
    dated_tokens: bool,
}

impl Default for GanttDb {
    fn default() -> Self {
        Self::new()
    }
}

fn duration_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([0-9]+(?:\.[0-9]+)?)(M|d|h|m|s|w|y|ms)$").expect("pattern"))
}

fn tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"^[{WS}]*(active|done|crit|milestone|vert)[{WS}]*$"
        ))
        .expect("pattern")
    })
}

fn after_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^after[ \t\n\r\x0b\x0c\x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}]+([0-9A-Za-z_ -]+)").expect("pattern"))
}

fn until_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^until[ \t\n\r\x0b\x0c\x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}]+([0-9A-Za-z_ -]+)").expect("pattern"))
}

fn numeric_format_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[YMDHhmsSZXx\-/.:, T]*$").expect("pattern"))
}

fn digits_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]+$").expect("pattern"))
}

fn token_split_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!(r"[{WS},]+")).expect("pattern"))
}

pub fn parse_duration(text: &str) -> Option<(f64, Unit)> {
    let caps = duration_re().captures(trim(text))?;
    let value: f64 = caps[1].parse().ok()?;
    let unit = match &caps[2] {
        "M" => Unit::Month,
        "d" => Unit::D,
        "h" => Unit::H,
        "m" => Unit::M,
        "s" => Unit::S,
        "w" => Unit::W,
        "y" => Unit::Y,
        _ => Unit::Ms,
    };
    Some((value, unit))
}

fn merge_tokens(existing: &[String], text: &str) -> Vec<String> {
    let mut merged: Vec<String> = Vec::new();
    for token in existing {
        if !merged.contains(token) {
            merged.push(token.clone());
        }
    }
    for token in token_split_re().split(&text.to_lowercase()) {
        if !token.is_empty() && !merged.iter().any(|m| m == token) {
            merged.push(token.to_owned());
        }
    }
    merged
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_millis() as f64)
}

fn semantic(message: String) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("gantt".to_owned());
    error
}

fn limit(message: &str) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Limit);
    error.diagram_type = Some("gantt".to_owned());
    error
}

impl GanttDb {
    pub fn new() -> Self {
        Self {
            title: None,
            acc_title: None,
            acc_descr: None,
            date_format: String::new(),
            axis_format: String::new(),
            tick_interval: None,
            today_marker: String::new(),
            includes: Vec::new(),
            excludes: Vec::new(),
            inclusive_end_dates: false,
            top_axis: false,
            display_mode: String::new(),
            weekday: None,
            weekend: "saturday".to_owned(),
            sections: Vec::new(),
            links: OrderedMap::default(),
            tasks: Vec::new(),
            by_id: HashMap::new(),
            current_section: String::new(),
            task_count: 0,
            last_order: 0,
            last_task_id: None,
            now: 0.0,
            work: 0.0,
            include_set: HashSet::new(),
            exclude_set: HashSet::new(),
            excluded_days: [false; 7],
            dated_tokens: false,
        }
    }

    pub fn set_date_format(&mut self, text: &str) {
        self.date_format = text.to_owned();
    }

    pub fn enable_inclusive_end_dates(&mut self) {
        self.inclusive_end_dates = true;
    }

    pub fn top_axis(&mut self) {
        self.top_axis = true;
    }

    pub fn set_axis_format(&mut self, text: &str) {
        self.axis_format = text.to_owned();
    }

    pub fn set_tick_interval(&mut self, text: &str) {
        self.tick_interval = Some(text.to_owned());
    }

    pub fn set_today_marker(&mut self, text: &str) {
        self.today_marker = text.to_owned();
    }

    pub fn set_includes(&mut self, text: &str) {
        self.includes = merge_tokens(&self.includes, text);
    }

    pub fn set_excludes(&mut self, text: &str) {
        self.excludes = merge_tokens(&self.excludes, text);
    }

    pub fn set_weekday(&mut self, text: &str) {
        self.weekday = Some(text.to_owned());
    }

    pub fn set_weekend(&mut self, start_day: &str) {
        self.weekend = start_day.to_owned();
    }

    pub fn set_diagram_title(&mut self, text: &str) {
        self.title = Some(text.to_owned());
    }

    pub fn set_acc_title(&mut self, text: &str) {
        self.acc_title = Some(crate::util::chars::trim_start(text).to_owned());
    }

    pub fn set_acc_description(&mut self, text: &str) {
        self.acc_descr = Some(crate::diagrams::common::dedent_descr(text));
    }

    pub fn add_section(&mut self, text: &str) {
        self.current_section = text.to_owned();
        self.sections.push(text.to_owned());
    }

    pub fn add_task(&mut self, descr: &str, data_str: &str) -> Result<(), PelersError> {
        let body = data_str.strip_prefix(':').unwrap_or(data_str);
        let mut data: Vec<String> = body.split(',').map(str::to_owned).collect();
        let mut task = Task {
            id: String::new(),
            task: descr.to_owned(),
            section: self.current_section.clone(),
            start_time: None,
            end_time: None,
            render_end_time: None,
            manual_end_time: false,
            processed: false,
            active: false,
            done: false,
            crit: false,
            milestone: false,
            vert: false,
            order: -1,
            classes: Vec::new(),
            prev_task_id: self.last_task_id.clone(),
            raw_data: data_str.to_owned(),
            raw_start: None,
            raw_end: String::new(),
        };
        while let Some(first) = data.first() {
            let Some(tag) = tag_re().captures(first).map(|c| c[1].to_owned()) else {
                break;
            };
            match tag.as_str() {
                "active" => task.active = true,
                "done" => task.done = true,
                "crit" => task.crit = true,
                "milestone" => task.milestone = true,
                _ => task.vert = true,
            }
            data.remove(0);
        }
        if data.is_empty() {
            return Err(semantic(format!(
                "Task \"{}\" needs a duration or an end date.",
                trim(descr)
            )));
        }
        if data.len() > 3 {
            return Err(semantic(format!(
                "Task \"{}\" has too many fields. A task takes an id, a start, and an end or duration.",
                trim(descr)
            )));
        }
        for item in data.iter_mut() {
            *item = trim(item).to_owned();
        }
        if data.len() == 3 {
            task.id = data[0].clone();
        } else {
            self.task_count += 1;
            task.id = format!("task{}", self.task_count);
        }
        if data.len() > 1 {
            task.raw_start = Some(data[data.len() - 2].clone());
        }
        task.raw_end = data[data.len() - 1].clone();

        if !task.vert {
            task.order = self.last_order;
            self.last_order += 1;
        }
        let id = task.id.clone();
        self.tasks.push(task);
        self.last_task_id = Some(id.clone());
        self.by_id.insert(id, self.tasks.len() - 1);
        Ok(())
    }

    pub fn set_link(&mut self, ids: &str, link: &str) {
        let href = safe_url(link);
        for id in ids.split(',') {
            if self.by_id.contains_key(id) {
                self.links.set(id, href.clone());
            }
        }
        self.set_class(ids, "clickable");
    }

    pub fn set_click_event(
        &mut self,
        ids: &str,
        _function_name: &str,
        _function_args: Option<&str>,
    ) {
        self.set_class(ids, "clickable");
    }

    pub fn set_class(&mut self, ids: &str, class_name: &str) {
        for id in ids.split(',') {
            if let Some(&at) = self.by_id.get(id) {
                self.tasks[at].classes.push(class_name.to_owned());
            }
        }
    }

    pub fn get_tasks(&mut self, now: Option<f64>) -> Result<(), PelersError> {
        self.now = match now {
            Some(given) if given.is_finite() => given,
            _ => now_ms(),
        };
        self.work = 0.0;
        self.include_set = self.includes.iter().cloned().collect();
        self.exclude_set = self.excludes.iter().cloned().collect();
        let weekend_start: Option<i64> = match self.weekend.as_str() {
            "friday" => Some(5),
            "saturday" => Some(6),
            _ => None,
        };
        let weekends = self.exclude_set.contains("weekends");
        for (day, name) in WEEKDAYS.iter().enumerate() {
            let iso = if day == 0 { 7 } else { day as i64 };
            self.excluded_days[day] = (weekends
                && weekend_start.is_some_and(|start| iso == start || iso == start + 1))
                || self.exclude_set.contains(&name.to_lowercase());
        }
        self.dated_tokens =
            !self.includes.is_empty() || !numeric_format_re().is_match(trim(&self.date_format));
        for token in &self.excludes {
            if token != "weekends" && !WEEKDAYS.iter().any(|d| d.to_lowercase() == *token) {
                self.dated_tokens = true;
            }
        }

        for task in self.tasks.iter_mut() {
            task.start_time = None;
            task.end_time = None;
            task.render_end_time = None;
            task.processed = false;
            task.manual_end_time = false;
        }

        let mut all_processed = self.compile_tasks()?;
        let mut i = 0;
        while !all_processed && i < 10 {
            all_processed = self.compile_tasks()?;
            i += 1;
        }
        Ok(())
    }

    pub fn is_invalid_date(&mut self, date: &JsDate) -> Result<bool, PelersError> {
        let weekday = self.excluded_days[date.fields().map_or(0, |f| f.weekday) as usize];
        if !self.dated_tokens {
            return Ok(weekday);
        }
        let template = trim(&self.date_format).to_owned();
        let date_only = iso_date(date);
        let mut formatted: Option<String> = Some(date_only.clone());
        self.spend(DATED_STEP)?;
        if template != "YYYY-MM-DD" {
            self.spend(FORMAT_STEP + template.chars().count() as f64 * 8.0)?;
            formatted = format(date, &template);
        }
        let listed = |set: &HashSet<String>| {
            set.contains(&date_only) || formatted.as_ref().is_some_and(|f| set.contains(f))
        };
        if listed(&self.include_set) {
            return Ok(false);
        }
        Ok(weekday || listed(&self.exclude_set))
    }

    fn spend(&mut self, units: f64) -> Result<(), PelersError> {
        self.work += units;
        if self.work > WORK_LIMIT {
            return Err(limit("The gantt chart takes too much work to schedule."));
        }
        Ok(())
    }

    fn parse(&mut self, text: &str, template: &str) -> Result<Option<JsDate>, PelersError> {
        self.spend((template.chars().count() as f64 + 1.0) * (text.chars().count() as f64 + 1.0))?;
        Ok(parse_strict(text, template, &JsDate::from_ms(self.now)))
    }

    fn today(&self) -> JsDate {
        let mut today = JsDate::from_ms(self.now);
        today.set_hours(0.0, 0.0, 0.0, 0.0);
        today
    }

    fn compile_tasks(&mut self) -> Result<bool, PelersError> {
        let mut all_processed = true;
        for at in 0..self.tasks.len() {
            self.compile_task(at)?;
            all_processed = all_processed && self.tasks[at].processed;
        }
        Ok(all_processed)
    }

    fn compile_task(&mut self, at: usize) -> Result<(), PelersError> {
        match self.tasks[at].raw_start.clone() {
            None => {
                let previous = self.tasks[at]
                    .prev_task_id
                    .clone()
                    .and_then(|id| self.by_id.get(&id).copied());
                self.tasks[at].start_time = match previous {
                    Some(prev) => self.tasks[prev].end_time,
                    None => Some(self.today()),
                };
            }
            Some(start) => {
                if let Some(start_time) = self.get_start_date(&start)? {
                    self.tasks[at].start_time = Some(start_time);
                }
            }
        }
        let Some(start_time) = self.tasks[at].start_time else {
            return Ok(());
        };
        let raw_end = self.tasks[at].raw_end.clone();
        self.tasks[at].end_time = self.get_end_date(&start_time, &raw_end)?;
        if self.tasks[at].end_time.is_none() {
            return Ok(());
        }
        self.tasks[at].processed = true;
        let template = "YYYY-MM-DD";
        self.tasks[at].manual_end_time = self.parse(&raw_end, template)?.is_some();
        self.check_task_dates(at)?;
        Ok(())
    }

    fn references(&self, list: &str) -> Vec<usize> {
        let mut found = Vec::new();
        for id in list.split(' ') {
            if id.is_empty() {
                continue;
            }
            if let Some(&at) = self.by_id.get(id) {
                found.push(at);
            }
        }
        found
    }

    fn get_start_date(&mut self, raw: &str) -> Result<Option<JsDate>, PelersError> {
        let text = trim(raw).to_owned();
        let template = trim(&self.date_format).to_owned();

        if (template == "x" || template == "X") && digits_re().is_match(&text) {
            let date = JsDate::from_ms(text.parse::<f64>().unwrap_or(f64::NAN));
            return self.checked(date, &text).map(Some);
        }

        if let Some(caps) = after_re().captures(&text) {
            let tasks = self.references(&caps[1]);
            let mut latest: Option<usize> = None;
            for at in tasks {
                let is_later = match (
                    latest,
                    self.tasks[at].end_time,
                    latest.and_then(|l| self.tasks[l].end_time),
                ) {
                    (None, _, _) => true,
                    (Some(_), Some(end), Some(best)) => end.time() > best.time(),
                    _ => false,
                };
                if is_later {
                    latest = Some(at);
                }
            }
            return Ok(match latest {
                Some(at) => self.tasks[at].end_time,
                None => Some(self.today()),
            });
        }

        if let Some(parsed) = self.parse(&text, &template)? {
            return Ok(Some(parsed));
        }

        let date = JsDate::from_ms(super::legacy_date::parse(&text));
        let year = date.fields().map_or(0, |f| f.year);
        if !is_valid(&date) || !(-10000..=10000).contains(&year) {
            return Err(semantic(format!("Invalid date:{text}")));
        }
        Ok(Some(date))
    }

    fn checked(&self, date: JsDate, text: &str) -> Result<JsDate, PelersError> {
        if !is_valid(&date) {
            return Err(semantic(format!("Invalid date:{text}")));
        }
        Ok(date)
    }

    fn get_end_date(
        &mut self,
        start_time: &JsDate,
        raw: &str,
    ) -> Result<Option<JsDate>, PelersError> {
        let text = trim(raw).to_owned();

        if let Some(caps) = until_re().captures(&text) {
            let tasks = self.references(&caps[1]);
            let mut earliest: Option<usize> = None;
            for at in tasks {
                let is_earlier = match (
                    earliest,
                    self.tasks[at].start_time,
                    earliest.and_then(|e| self.tasks[e].start_time),
                ) {
                    (None, _, _) => true,
                    (Some(_), Some(start), Some(best)) => start.time() < best.time(),
                    _ => false,
                };
                if is_earlier {
                    earliest = Some(at);
                }
            }
            return Ok(match earliest {
                Some(at) => self.tasks[at].start_time,
                None => Some(self.today()),
            });
        }

        let template = trim(&self.date_format).to_owned();
        if let Some(parsed) = self.parse(&text, &template)? {
            return Ok(Some(if self.inclusive_end_dates {
                add_days(&parsed, 1.0)
            } else {
                parsed
            }));
        }

        let Some((value, unit)) = parse_duration(&text) else {
            return Ok(Some(*start_time));
        };
        let end = add(start_time, value, unit);
        Ok(Some(if is_valid(&end) { end } else { *start_time }))
    }

    fn check_task_dates(&mut self, at: usize) -> Result<(), PelersError> {
        if self.excludes.is_empty() || self.tasks[at].manual_end_time {
            return Ok(());
        }
        let start = self.tasks[at]
            .start_time
            .expect("a scheduled task has a start");
        let mut cursor = add_days(&start, 1.0);
        let mut end_time = self.tasks[at]
            .end_time
            .expect("a scheduled task has an end");
        let max_end_time = add_days(&end_time, 10000.0).time();
        let mut invalid = false;
        let mut render_end_time: Option<f64> = None;
        while cursor.time() <= end_time.time() {
            self.spend(DAY_STEP)?;
            if !invalid {
                render_end_time = Some(end_time.time());
            }
            invalid = self.is_invalid_date(&cursor)?;
            if invalid {
                let day = end_time.fields().map_or(0, |f| f.day);
                end_time.set_date((day + 1) as f64);
                if end_time.time() > max_end_time {
                    return Err(semantic(
                        "Failed to find a valid date that was not excluded by `excludes` after 10,000 iterations."
                            .to_owned(),
                    ));
                }
            }
            let day = cursor.fields().map_or(0, |f| f.day);
            cursor.set_date((day + 1) as f64);
        }
        self.tasks[at].end_time = Some(end_time);
        self.tasks[at].render_end_time = render_end_time.map(JsDate::from_ms);
        Ok(())
    }
}
