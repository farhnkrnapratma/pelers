use regex::Regex;
use std::sync::OnceLock;

use super::db::{FlowDb, VertexAttrs};
use super::lexer::tokenize;
use super::tokens::{T, token_name};
use super::types::{DocItem, FlowText, LabelType, LinkInfo};
use crate::errors::{Edit, PelersError, input_offset, lexical_error, syntax_error};
use crate::util::chars::{utf16_len, utf16_to_byte};

const ID: &[T] = &[
    T::NUM,
    T::NODE_STRING,
    T::DOWN,
    T::MINUS,
    T::DEFAULT,
    T::COMMA,
    T::COLON,
    T::AMP,
    T::BRKT,
    T::MULT,
    T::UNICODE_TEXT,
];

const ALPHANUM: &[T] = &[
    T::NUM,
    T::UNICODE_TEXT,
    T::NODE_STRING,
    T::DIR,
    T::DOWN,
    T::MINUS,
    T::COMMA,
    T::COLON,
    T::AMP,
    T::BRKT,
    T::MULT,
];

const NO_TAGS: &[T] = &[
    T::NUM,
    T::NODE_STRING,
    T::SPACE,
    T::MINUS,
    T::AMP,
    T::UNICODE_TEXT,
    T::COLON,
    T::MULT,
    T::BRKT,
    T::STYLE,
    T::LINKSTYLE,
    T::CLASSDEF,
    T::CLASS,
    T::CLICK,
    T::GRAPH,
    T::DIR,
    T::subgraph,
    T::end,
    T::DOWN,
    T::UP,
    T::START_LINK,
];

const STYLE_PART: &[T] = &[
    T::NUM,
    T::NODE_STRING,
    T::COLON,
    T::SPACE,
    T::BRKT,
    T::STYLE,
];

const TEXT_TOKENS: &str = "'UNICODE_TEXT', 'TEXT', 'TAGSTART'";
const DIRECTIONS: [&str; 5] = ["TB", "BT", "RL", "LR", "TD"];
const STATEMENT_START: &str = "'SEMI', 'NEWLINE', 'SPACE', 'EOF', 'subgraph', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'STYLE', 'LINKSTYLE', 'CLASSDEF', 'CLASS', 'CLICK', 'NODE_STRING'";

fn brace_trail() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\}\s*\n").expect("brace trail"))
}

struct Frame {
    id: Option<FlowText>,
    title: Option<FlowText>,
    title_is_id: bool,
    list: Vec<DocItem>,
    collect: bool,
}

enum Statement {
    Item(DocItem),
    Nothing,
}

struct Parser<'a> {
    source: &'a str,
    src: String,
    types: Vec<T>,
    texts: Vec<String>,
    starts: Vec<usize>,
    i: usize,
    pending_data: Option<String>,
    db: &'a mut FlowDb,
}

impl Parser<'_> {
    fn fail(&self, expected: &str) -> PelersError {
        let t = self.types[self.i];
        let start = self.starts[self.i];
        let at = if self.src == self.source {
            start
        } else {
            let edits: Vec<Edit> = brace_trail()
                .find_iter(self.source)
                .map(|m| {
                    Edit::resized(
                        utf16_len(&self.source[..m.start()]),
                        utf16_len(&self.source[..m.end()]),
                        2,
                    )
                })
                .collect();
            input_offset(&[edits], start)
        };
        let at = utf16_to_byte(self.source, at);
        if t == T::ERROR {
            return lexical_error("flowchart", self.source, at);
        }
        syntax_error(
            "flowchart",
            self.source,
            at,
            format!("Expecting {expected}, got '{}'", token_name(t)),
        )
    }

    fn expect(&mut self, t: T, also: &str) -> Result<String, PelersError> {
        if self.types[self.i] != t {
            let extra = if also.is_empty() {
                String::new()
            } else {
                format!(", {also}")
            };
            return Err(self.fail(&format!("'{}'{extra}", token_name(t))));
        }
        let text = self.texts[self.i].clone();
        self.i += 1;
        Ok(text)
    }

    fn separator(&mut self) -> Result<(), PelersError> {
        let t = self.types[self.i];
        if t != T::NEWLINE && t != T::SEMI && t != T::EOF {
            return Err(self.fail("'SEMI', 'NEWLINE', 'EOF'"));
        }
        self.i += 1;
        Ok(())
    }

    fn id_string(&mut self) -> Result<String, PelersError> {
        if !ID.contains(&self.types[self.i]) {
            return Err(self.fail("'NODE_STRING'"));
        }
        let mut s = self.texts[self.i].clone();
        self.i += 1;
        while ID.contains(&self.types[self.i]) {
            s.push_str(&self.texts[self.i]);
            self.i += 1;
        }
        Ok(s)
    }

    fn alpha_num(&mut self) -> Result<String, PelersError> {
        if !ALPHANUM.contains(&self.types[self.i]) {
            return Err(self.fail("'NODE_STRING'"));
        }
        let mut s = self.texts[self.i].clone();
        self.i += 1;
        while ALPHANUM.contains(&self.types[self.i]) {
            s.push_str(&self.texts[self.i]);
            self.i += 1;
        }
        Ok(s)
    }

    fn text(&mut self) -> Result<FlowText, PelersError> {
        let t = self.types[self.i];
        let kind = match t {
            T::STR => LabelType::String,
            T::MD_STR => LabelType::Markdown,
            T::TEXT | T::TAGSTART | T::TAGEND | T::UNICODE_TEXT => LabelType::Text,
            _ => return Err(self.fail(&format!("{TEXT_TOKENS}, 'TAGEND', 'STR', 'MD_STR'"))),
        };
        let mut s = self.texts[self.i].clone();
        self.i += 1;
        loop {
            let n = self.types[self.i];
            if !matches!(n, T::TEXT | T::TAGSTART | T::TAGEND | T::UNICODE_TEXT) {
                break;
            }
            s.push_str(&self.texts[self.i]);
            self.i += 1;
        }
        Ok(FlowText { text: s, kind })
    }

    fn closed(&mut self, t: T) -> Result<(), PelersError> {
        self.expect(t, TEXT_TOKENS).map(|_| ())
    }

    fn edge_text(&mut self) -> Result<FlowText, PelersError> {
        let t = self.types[self.i];
        let kind = match t {
            T::STR => LabelType::String,
            T::MD_STR => LabelType::Markdown,
            T::EDGE_TEXT | T::UNICODE_TEXT => LabelType::Text,
            _ => return Err(self.fail("'EDGE_TEXT', 'STR', 'MD_STR'")),
        };
        let mut s = self.texts[self.i].clone();
        self.i += 1;
        while matches!(self.types[self.i], T::EDGE_TEXT | T::UNICODE_TEXT) {
            s.push_str(&self.texts[self.i]);
            self.i += 1;
        }
        Ok(FlowText { text: s, kind })
    }

    fn text_no_tags(&mut self) -> Result<FlowText, PelersError> {
        let t = self.types[self.i];
        let kind = match t {
            T::STR => LabelType::Text,
            T::MD_STR => LabelType::Markdown,
            _ if NO_TAGS.contains(&t) => LabelType::Text,
            _ => return Err(self.fail("'NODE_STRING', 'STR', 'MD_STR'")),
        };
        let mut s = self.texts[self.i].clone();
        self.i += 1;
        while NO_TAGS.contains(&self.types[self.i]) {
            s.push_str(&self.texts[self.i]);
            self.i += 1;
        }
        Ok(FlowText { text: s, kind })
    }

    fn shape_data(&mut self) -> String {
        let mut s = self.texts[self.i].clone();
        self.i += 1;
        while self.types[self.i] == T::SHAPE_DATA {
            s.push_str(&self.texts[self.i]);
            self.i += 1;
        }
        s
    }

    fn add_vertex(
        &mut self,
        id: &str,
        label: Option<&FlowText>,
        kind: Option<&str>,
    ) -> Result<(), PelersError> {
        self.db
            .add_vertex(id, label, kind, VertexAttrs::default(), None)
    }

    fn vertex(&mut self) -> Result<String, PelersError> {
        let id = self.id_string()?;
        match self.types[self.i] {
            T::SQS => {
                self.i += 1;
                let label = self.text()?;
                self.closed(T::SQE)?;
                self.add_vertex(&id, Some(&label), Some("square"))?;
            }
            T::DOUBLECIRCLESTART => {
                self.i += 1;
                let label = self.text()?;
                self.closed(T::DOUBLECIRCLEEND)?;
                self.add_vertex(&id, Some(&label), Some("doublecircle"))?;
            }
            T::PS => {
                self.i += 1;
                if self.types[self.i] == T::PS {
                    self.i += 1;
                    let label = self.text()?;
                    self.closed(T::PE)?;
                    self.expect(T::PE, "")?;
                    self.add_vertex(&id, Some(&label), Some("circle"))?;
                } else {
                    let label = self.text()?;
                    self.closed(T::PE)?;
                    self.add_vertex(&id, Some(&label), Some("round"))?;
                }
            }
            T::ELLIPSE_START => {
                self.i += 1;
                let label = self.text()?;
                self.closed(T::ELLIPSE_END)?;
                self.add_vertex(&id, Some(&label), Some("ellipse"))?;
            }
            T::STADIUMSTART => {
                self.i += 1;
                let label = self.text()?;
                self.closed(T::STADIUMEND)?;
                self.add_vertex(&id, Some(&label), Some("stadium"))?;
            }
            T::SUBROUTINESTART => {
                self.i += 1;
                let label = self.text()?;
                self.closed(T::SUBROUTINEEND)?;
                self.add_vertex(&id, Some(&label), Some("subroutine"))?;
            }
            T::VERTEX_WITH_PROPS_START => {
                self.i += 1;
                let field = self.expect(T::NODE_STRING, "")?;
                self.expect(T::COLON, "")?;
                let value = self.expect(T::NODE_STRING, "")?;
                self.expect(T::PIPE, "")?;
                let label = self.text()?;
                self.closed(T::SQE)?;
                self.db.add_vertex(
                    &id,
                    Some(&label),
                    Some("rect"),
                    VertexAttrs {
                        props: Some(vec![(field, value)]),
                        ..VertexAttrs::default()
                    },
                    None,
                )?;
            }
            T::CYLINDERSTART => {
                self.i += 1;
                let label = self.text()?;
                self.closed(T::CYLINDEREND)?;
                self.add_vertex(&id, Some(&label), Some("cylinder"))?;
            }
            T::DIAMOND_START => {
                self.i += 1;
                if self.types[self.i] == T::DIAMOND_START {
                    self.i += 1;
                    let label = self.text()?;
                    self.closed(T::DIAMOND_STOP)?;
                    self.expect(T::DIAMOND_STOP, "")?;
                    self.add_vertex(&id, Some(&label), Some("hexagon"))?;
                } else {
                    let label = self.text()?;
                    self.closed(T::DIAMOND_STOP)?;
                    self.add_vertex(&id, Some(&label), Some("diamond"))?;
                }
            }
            T::TAGEND => {
                self.i += 1;
                let label = self.text()?;
                self.closed(T::SQE)?;
                self.add_vertex(&id, Some(&label), Some("odd"))?;
            }
            T::TRAPSTART => {
                self.i += 1;
                let label = self.text()?;
                if self.types[self.i] == T::INVTRAPEND {
                    self.i += 1;
                    self.add_vertex(&id, Some(&label), Some("lean_right"))?;
                } else {
                    self.closed(T::TRAPEND)?;
                    self.add_vertex(&id, Some(&label), Some("trapezoid"))?;
                }
            }
            T::INVTRAPSTART => {
                self.i += 1;
                let label = self.text()?;
                if self.types[self.i] == T::TRAPEND {
                    self.i += 1;
                    self.add_vertex(&id, Some(&label), Some("lean_left"))?;
                } else {
                    self.closed(T::INVTRAPEND)?;
                    self.add_vertex(&id, Some(&label), Some("inv_trapezoid"))?;
                }
            }
            _ => self.add_vertex(&id, None, None)?,
        }
        Ok(id)
    }

    fn styled_vertex(&mut self) -> Result<String, PelersError> {
        let id = self.vertex()?;
        if self.types[self.i] == T::STYLE_SEPARATOR {
            self.i += 1;
            let class = self.id_string()?;
            self.db.set_class(&id, &class);
        }
        Ok(id)
    }

    fn node(&mut self) -> Result<Vec<String>, PelersError> {
        let mut ids = vec![self.styled_vertex()?];
        self.pending_data = None;
        loop {
            let data = if self.types[self.i] == T::SHAPE_DATA {
                Some(self.shape_data())
            } else {
                None
            };
            if self.types[self.i] != T::SPACE {
                self.pending_data = data;
                return Ok(ids);
            }
            self.i += 1;
            while self.types[self.i] == T::SPACE {
                self.i += 1;
            }
            if self.types[self.i] != T::AMP {
                if data.is_some() {
                    return Err(self.fail("'AMP'"));
                }
                return Ok(ids);
            }
            self.i += 1;
            if self.types[self.i] != T::SPACE {
                return Err(self.fail("'SPACE'"));
            }
            self.i += 1;
            while self.types[self.i] == T::SPACE {
                self.i += 1;
            }
            let next = self.styled_vertex()?;
            if let Some(data) = data {
                let last = ids.last().cloned().unwrap_or_default();
                self.db
                    .add_vertex(&last, None, None, VertexAttrs::default(), Some(&data))?;
            }
            ids.push(next);
        }
    }

    fn link(&mut self) -> Result<LinkInfo, PelersError> {
        let mut id: Option<String> = None;
        if self.types[self.i] == T::LINK_ID {
            id = Some(self.texts[self.i].clone());
            self.i += 1;
        }
        if self.types[self.i] == T::LINK {
            let end = self.texts[self.i].clone();
            self.i += 1;
            let mut info = self.db.destruct_link(&end, None);
            if let Some(id) = id {
                info.id = Some(id);
            }
            if self.types[self.i] == T::PIPE {
                self.i += 1;
                info.text = Some(self.text()?);
                self.closed(T::PIPE)?;
                if self.types[self.i] == T::SPACE {
                    self.i += 1;
                }
            }
            return Ok(info);
        }
        if self.types[self.i] == T::START_LINK {
            let start = self.texts[self.i].clone();
            self.i += 1;
            let label = self.edge_text()?;
            let end = self.expect(T::LINK, "'EDGE_TEXT'")?;
            let mut info = self.db.destruct_link(&end, Some(&start));
            info.text = Some(label);
            if let Some(id) = id {
                info.id = Some(id);
            }
            return Ok(info);
        }
        Err(self.fail("'LINK', 'START_LINK'"))
    }

    fn vertex_statement(&mut self, collect: bool) -> Result<Vec<String>, PelersError> {
        let mut stmt = self.node()?;
        if let Some(data) = self.pending_data.clone() {
            let last = stmt.last().cloned().unwrap_or_default();
            self.db
                .add_vertex(&last, None, None, VertexAttrs::default(), Some(&data))?;
        }
        let mut nodes = stmt.clone();
        loop {
            let t = self.types[self.i];
            if t != T::LINK && t != T::START_LINK && t != T::LINK_ID {
                return Ok(nodes);
            }
            let info = self.link()?;
            let next = self.node()?;
            if let Some(data) = self.pending_data.clone() {
                let last = next.last().cloned().unwrap_or_default();
                self.db
                    .add_vertex(&last, None, None, VertexAttrs::default(), Some(&data))?;
            }
            let first_start = stmt.clone();
            self.db.add_link(&first_start, &next, Some(&info))?;
            if collect {
                let mut combined = next.clone();
                combined.extend(nodes);
                nodes = combined;
            }
            stmt = next;
        }
    }

    fn style(&mut self) -> Result<String, PelersError> {
        if !STYLE_PART.contains(&self.types[self.i]) {
            return Err(self.fail("'NODE_STRING', 'NUM', 'COLON', 'BRKT'"));
        }
        let mut s = self.texts[self.i].clone();
        self.i += 1;
        while STYLE_PART.contains(&self.types[self.i]) {
            s.push_str(&self.texts[self.i]);
            self.i += 1;
        }
        Ok(s)
    }

    fn styles_opt(&mut self) -> Result<Vec<String>, PelersError> {
        let mut out = vec![self.style()?];
        while self.types[self.i] == T::COMMA {
            self.i += 1;
            out.push(self.style()?);
        }
        Ok(out)
    }

    fn link_style_statement(&mut self) -> Result<(), PelersError> {
        self.i += 1;
        self.expect(T::SPACE, "")?;
        let positions: Vec<String> = if self.types[self.i] == T::DEFAULT {
            let text = self.texts[self.i].clone();
            self.i += 1;
            vec![text]
        } else {
            let mut positions = vec![self.expect(T::NUM, "'DEFAULT'")?];
            while self.types[self.i] == T::COMMA {
                self.i += 1;
                positions.push(self.expect(T::NUM, "")?);
            }
            positions
        };
        self.expect(T::SPACE, "")?;
        if self.types[self.i] != T::INTERPOLATE {
            let styles = self.styles_opt()?;
            self.db.update_link(&positions, styles)?;
            return Ok(());
        }
        self.i += 1;
        self.expect(T::SPACE, "")?;
        let interpolate = self.alpha_num()?;
        if self.types[self.i] == T::SPACE {
            self.i += 1;
            let styles = self.styles_opt()?;
            self.db.update_link_interpolate(&positions, &interpolate)?;
            self.db.update_link(&positions, styles)?;
        } else {
            self.db.update_link_interpolate(&positions, &interpolate)?;
        }
        Ok(())
    }

    fn link_tail(&mut self, id: &str, url: &str) -> Result<(), PelersError> {
        if self.types[self.i] != T::SPACE {
            self.db.set_link(id, url, None);
            return Ok(());
        }
        self.i += 1;
        if self.types[self.i] == T::LINK_TARGET {
            let target = self.texts[self.i].clone();
            self.i += 1;
            self.db.set_link(id, url, Some(&target));
            return Ok(());
        }
        let tooltip = self.expect(T::STR, "'LINK_TARGET'")?;
        if self.types[self.i] == T::SPACE {
            self.i += 1;
            let target = self.expect(T::LINK_TARGET, "")?;
            self.db.set_link(id, url, Some(&target));
        } else {
            self.db.set_link(id, url, None);
        }
        self.db.set_tooltip(id, Some(&tooltip));
        Ok(())
    }

    fn click_statement(&mut self) -> Result<(), PelersError> {
        let id = self.texts[self.i].clone();
        self.i += 1;
        match self.types[self.i] {
            T::CALLBACKNAME => {
                self.i += 1;
                if self.types[self.i] == T::CALLBACKARGS {
                    self.i += 1;
                }
                let mut tooltip: Option<String> = None;
                if self.types[self.i] == T::SPACE {
                    self.i += 1;
                    tooltip = Some(self.expect(T::STR, "")?);
                }
                self.db.set_click_event(&id);
                if let Some(tooltip) = tooltip {
                    self.db.set_tooltip(&id, Some(&tooltip));
                }
                Ok(())
            }
            T::HREF => {
                self.i += 1;
                let url = self.expect(T::STR, "")?;
                self.link_tail(&id, &url)
            }
            T::STR => {
                let url = self.texts[self.i].clone();
                self.i += 1;
                self.link_tail(&id, &url)
            }
            _ => {
                let _name = self.alpha_num()?;
                if self.types[self.i] == T::SPACE {
                    self.i += 1;
                    let tooltip = self.expect(T::STR, "")?;
                    self.db.set_click_event(&id);
                    self.db.set_tooltip(&id, Some(&tooltip));
                } else {
                    self.db.set_click_event(&id);
                }
                Ok(())
            }
        }
    }

    fn subgraph_header(&mut self) -> Result<Frame, PelersError> {
        self.i += 1;
        let t = self.types[self.i];
        if t == T::NEWLINE || t == T::SEMI || t == T::EOF {
            self.i += 1;
            return Ok(Frame {
                id: None,
                title: None,
                title_is_id: false,
                list: Vec::new(),
                collect: false,
            });
        }
        self.expect(T::SPACE, "'SEMI', 'NEWLINE', 'EOF'")?;
        let id = self.text_no_tags()?;
        let mut title = id.clone();
        let mut title_is_id = true;
        if self.types[self.i] == T::SQS {
            self.i += 1;
            title = self.text()?;
            title_is_id = false;
            self.closed(T::SQE)?;
        }
        let t = self.types[self.i];
        if t != T::NEWLINE && t != T::SEMI && t != T::EOF {
            return Err(self.fail("'SEMI', 'NEWLINE', 'EOF', 'SQS'"));
        }
        self.i += 1;
        Ok(Frame {
            id: Some(id),
            title: Some(title),
            title_is_id,
            list: Vec::new(),
            collect: false,
        })
    }

    fn statement(&mut self, collect: bool) -> Result<Statement, PelersError> {
        let t = self.types[self.i];
        match t {
            T::STYLE => {
                self.i += 1;
                self.expect(T::SPACE, "")?;
                let id = self.id_string()?;
                self.expect(T::SPACE, "")?;
                let styles = self.styles_opt()?;
                self.db.add_vertex(
                    &id,
                    None,
                    None,
                    VertexAttrs {
                        style: Some(styles),
                        ..VertexAttrs::default()
                    },
                    None,
                )?;
                self.separator()?;
                Ok(Statement::Nothing)
            }
            T::LINKSTYLE => {
                self.link_style_statement()?;
                self.separator()?;
                Ok(Statement::Nothing)
            }
            T::CLASSDEF => {
                self.i += 1;
                self.expect(T::SPACE, "")?;
                let id = self.id_string()?;
                self.expect(T::SPACE, "")?;
                let styles = self.styles_opt()?;
                self.db.add_class(&id, &styles);
                self.separator()?;
                Ok(Statement::Nothing)
            }
            T::CLASS => {
                self.i += 1;
                self.expect(T::SPACE, "")?;
                let ids = self.id_string()?;
                self.expect(T::SPACE, "")?;
                let class = self.id_string()?;
                self.db.set_class(&ids, &class);
                self.separator()?;
                Ok(Statement::Nothing)
            }
            T::CLICK => {
                self.click_statement()?;
                self.separator()?;
                Ok(Statement::Nothing)
            }
            T::direction_tb
            | T::direction_bt
            | T::direction_rl
            | T::direction_lr
            | T::direction_td => {
                self.i += 1;
                let index = t as usize - T::direction_tb as usize;
                Ok(Statement::Item(DocItem::Dir(DIRECTIONS[index].to_owned())))
            }
            T::acc_title => {
                self.i += 1;
                let value = self.expect(T::acc_title_value, "")?.trim().to_owned();
                self.db.set_acc_title(&value);
                Ok(Statement::Item(DocItem::Id(value)))
            }
            T::acc_descr => {
                self.i += 1;
                let value = self.expect(T::acc_descr_value, "")?.trim().to_owned();
                self.db.set_acc_description(&value);
                Ok(Statement::Item(DocItem::Id(value)))
            }
            T::acc_descr_multiline_value => {
                let value = self.texts[self.i].trim().to_owned();
                self.i += 1;
                self.db.set_acc_description(&value);
                Ok(Statement::Item(DocItem::Id(value)))
            }
            _ => {
                if !ID.contains(&t) {
                    return Err(self.fail(STATEMENT_START));
                }
                let nodes = self.vertex_statement(collect)?;
                self.separator()?;
                if nodes.is_empty() {
                    Ok(Statement::Nothing)
                } else {
                    Ok(Statement::Item(DocItem::Ids(nodes)))
                }
            }
        }
    }

    fn document(&mut self) -> Result<(), PelersError> {
        let mut open: Vec<Frame> = Vec::new();
        let mut list: Vec<DocItem> = Vec::new();
        let mut collect = false;
        loop {
            let t = self.types[self.i];
            if t == T::END {
                break;
            }
            if t == T::end {
                let Some(frame) = open.pop() else {
                    break;
                };
                self.i += 1;
                let id = self.db.add_subgraph(
                    frame.id.as_ref().map(|f| f.text.as_str()),
                    &list,
                    frame.title.as_ref().map(|f| (f.text.as_str(), f.kind)),
                    frame.title_is_id,
                );
                list = frame.list;
                collect = frame.collect;
                if collect {
                    list.push(DocItem::Id(id));
                }
            } else if matches!(t, T::SEMI | T::NEWLINE | T::SPACE | T::EOF) {
                if collect {
                    list.push(DocItem::Id(self.texts[self.i].clone()));
                }
                self.i += 1;
            } else if t == T::subgraph {
                let mut frame = self.subgraph_header()?;
                frame.list = std::mem::take(&mut list);
                frame.collect = collect;
                open.push(frame);
                collect = true;
            } else {
                if let Statement::Item(item) = self.statement(collect)?
                    && collect
                {
                    list.push(item);
                }
            }
        }
        if !open.is_empty() {
            return Err(self.fail("'end'"));
        }
        Ok(())
    }
}

pub fn parse_flowchart(source: &str, db: &mut FlowDb) -> Result<(), PelersError> {
    let src = if source.contains('}') {
        brace_trail().replace_all(source, "}\n").into_owned()
    } else {
        source.to_owned()
    };
    let tokens = tokenize(&src);
    let mut parser = Parser {
        source,
        src,
        types: tokens.types,
        texts: tokens.texts,
        starts: tokens.starts,
        i: 0,
        pending_data: None,
        db,
    };
    while parser.types[parser.i] == T::SPACE || parser.types[parser.i] == T::NEWLINE {
        parser.i += 1;
    }
    parser.expect(T::GRAPH, "'SPACE', 'NEWLINE'")?;
    if parser.types[parser.i] == T::NODIR {
        parser.i += 1;
        parser.db.set_direction("TB");
    } else {
        let dir = parser.expect(T::DIR, "'NODIR'")?;
        let mut t = parser.types[parser.i];
        if t == T::SPACE {
            parser.i += 1;
            while parser.types[parser.i] == T::SPACE {
                parser.i += 1;
            }
            t = parser.types[parser.i];
            if t != T::NEWLINE {
                return Err(parser.fail("'NEWLINE', 'SPACE'"));
            }
        } else if t != T::SEMI && t != T::NEWLINE {
            return Err(parser.fail("'SEMI', 'NEWLINE', 'SPACE'"));
        }
        parser.i += 1;
        parser.db.set_direction(&dir);
    }
    parser.document()?;
    if parser.types[parser.i] != T::END {
        return Err(parser.fail(STATEMENT_START));
    }
    Ok(())
}
