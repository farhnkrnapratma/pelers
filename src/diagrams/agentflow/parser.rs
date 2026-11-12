use super::db::AgentflowDb;
use super::lexer::{Tokens, tokenize};
use super::tokens::*;
use super::types::{DocItem, FlowText, LabelType, LinkInfo, Loc};
use crate::errors::{Edit, PelersError, input_offset, lexical_error, syntax_error};
use crate::util::chars::{is_js_space, trim, trim_end, utf16_len, utf16_to_byte};

const SEPARATORS: &str = "'COMMENT', 'SEMI', 'NEWLINE', 'EOF'";
const TEXT_TOKENS: &str = "'UNICODE_TEXT', 'TEXT', 'TAGSTART'";
const STATEMENT_START: &str = "'COMMENT', 'SEMI', 'NEWLINE', 'SPACE', 'EOF', 'flow', 'global', 'connector', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'STYLE', 'LINKSTYLE', 'CLASSDEF', 'CLASS', 'CLICK', 'NODE_STRING'";
const DIRECTION_NAMES: [&str; 5] = ["TB", "BT", "RL", "LR", "TD"];

fn is_id(t: usize) -> bool {
    matches!(
        t,
        NUM | NODE_STRING
            | DOWN
            | MINUS
            | DEFAULT
            | COMMA
            | COLON
            | AMP
            | BRKT
            | MULT
            | UNICODE_TEXT
    )
}

fn is_alphanum(t: usize) -> bool {
    matches!(
        t,
        NUM | UNICODE_TEXT | NODE_STRING | DIR | DOWN | MINUS | COMMA | COLON | AMP | BRKT | MULT
    )
}

fn is_no_tags(t: usize) -> bool {
    matches!(
        t,
        NUM | NODE_STRING
            | SPACE
            | MINUS
            | AMP
            | UNICODE_TEXT
            | COLON
            | MULT
            | BRKT
            | STYLE
            | LINKSTYLE
            | CLASSDEF
            | CLASS
            | CLICK
            | GRAPH
            | DIR
            | FLOW
            | CONNECTOR
            | GLOBAL
            | END_KW
            | DOWN
            | UP
            | START_LINK
    )
}

fn is_style_part(t: usize) -> bool {
    matches!(t, NUM | NODE_STRING | COLON | SPACE | BRKT | STYLE)
}

fn is_separator(t: usize) -> bool {
    matches!(t, NEWLINE | SEMI | EOF | COMMENT)
}

fn is_trailing(t: usize) -> bool {
    matches!(t, COMMENT | NEWLINE | SPACE | EOF)
}

fn is_text(t: usize) -> bool {
    matches!(t, TEXT | TAGSTART | TAGEND | UNICODE_TEXT)
}

fn brace_trail_matches(source: &str) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    let bytes = source.as_bytes();
    let mut at = 0;
    while let Some(offset) = source[at..].find('}') {
        let brace = at + offset;
        let mut j = brace + 1;
        while let Some(ch) = source[j..].chars().next() {
            if ch == '\n' || !is_js_space(ch) {
                break;
            }
            j += ch.len_utf8();
        }
        if j < bytes.len() && bytes[j] == b'\n' {
            matches.push((brace, j + 1));
            at = j + 1;
        } else {
            at = brace + 1;
        }
    }
    matches
}

fn strip_brace_trail(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut last = 0;
    for (start, end) in brace_trail_matches(source) {
        out.push_str(&source[last..start]);
        out.push_str("}\n");
        last = end;
    }
    out.push_str(&source[last..]);
    out
}

struct Frame {
    global: bool,
    at: usize,
    id: Option<FlowText>,
    title: Option<FlowText>,
    data: Option<String>,
    data_from: usize,
    data_to: usize,
    list: Vec<DocItem>,
    collect: bool,
}

struct Parser<'a> {
    source: &'a str,
    src: String,
    tokens: Tokens,
    i: usize,
    data_from: usize,
    data_to: usize,
    pending: Option<(String, usize, usize)>,
    db: &'a mut AgentflowDb,
}

impl Parser<'_> {
    fn ty(&self, k: usize) -> usize {
        self.tokens.types[k]
    }

    fn text(&self, k: usize) -> String {
        self.tokens.texts[k].clone()
    }

    fn fail(&self, expected: &str) -> PelersError {
        let t = self.ty(self.i);
        let mut from = self.tokens.starts[self.i];
        if t == EOF || t == END {
            let mut k = self.i;
            while k > 0 && is_trailing(self.ty(k - 1)) {
                k -= 1;
            }
            let prefix = &self.src[..utf16_to_byte(&self.src, self.tokens.starts[k])];
            from = utf16_len(trim_end(prefix));
        }
        let at = if self.src == self.source {
            from
        } else {
            let edits: Vec<Edit> = brace_trail_matches(self.source)
                .into_iter()
                .map(|(start, end)| {
                    Edit::resized(
                        utf16_len(&self.source[..start]),
                        utf16_len(&self.source[..end]),
                        2,
                    )
                })
                .collect();
            input_offset(&[edits], from)
        };
        let at = utf16_to_byte(self.source, at);
        if t == ERROR {
            return lexical_error("agentflow", self.source, at);
        }
        syntax_error(
            "agentflow",
            self.source,
            at,
            format!("Expecting {expected}, got '{}'", TOKEN_NAMES[t]),
        )
    }

    fn span(&self, a: usize, b: usize) -> Loc {
        Loc {
            first_line: self.tokens.locs[4 * a],
            first_column: self.tokens.locs[4 * a + 1],
            last_line: self.tokens.locs[4 * b + 2],
            last_column: self.tokens.locs[4 * b + 3],
        }
    }

    fn expect(&mut self, kind: usize, also: &str) -> Result<String, PelersError> {
        if self.ty(self.i) != kind {
            let suffix = if also.is_empty() {
                String::new()
            } else {
                format!(", {also}")
            };
            return Err(self.fail(&format!("'{}'{suffix}", TOKEN_NAMES[kind])));
        }
        let text = self.text(self.i);
        self.i += 1;
        Ok(text)
    }

    fn separator(&mut self) -> Result<(), PelersError> {
        if !is_separator(self.ty(self.i)) {
            return Err(self.fail(SEPARATORS));
        }
        self.i += 1;
        Ok(())
    }

    fn id_string(&mut self) -> Result<String, PelersError> {
        if !is_id(self.ty(self.i)) {
            return Err(self.fail("'NODE_STRING'"));
        }
        let mut s = self.text(self.i);
        self.i += 1;
        while is_id(self.ty(self.i)) {
            s.push_str(&self.text(self.i));
            self.i += 1;
        }
        Ok(s)
    }

    fn alpha_num(&mut self) -> Result<String, PelersError> {
        if !is_alphanum(self.ty(self.i)) {
            return Err(self.fail("'NODE_STRING'"));
        }
        let mut s = self.text(self.i);
        self.i += 1;
        while is_alphanum(self.ty(self.i)) {
            s.push_str(&self.text(self.i));
            self.i += 1;
        }
        Ok(s)
    }

    fn flow_text(&mut self) -> Result<FlowText, PelersError> {
        let t = self.ty(self.i);
        let kind = if t == STR {
            LabelType::String
        } else if t == MD_STR {
            LabelType::Markdown
        } else if is_text(t) {
            LabelType::Text
        } else {
            return Err(self.fail(&format!("{TEXT_TOKENS}, 'TAGEND', 'STR', 'MD_STR'")));
        };
        let mut s = self.text(self.i);
        self.i += 1;
        while is_text(self.ty(self.i)) {
            s.push_str(&self.text(self.i));
            self.i += 1;
        }
        Ok(FlowText { text: s, kind })
    }

    fn closed(&mut self, kind: usize) -> Result<(), PelersError> {
        self.expect(kind, TEXT_TOKENS)?;
        Ok(())
    }

    fn edge_text(&mut self) -> Result<FlowText, PelersError> {
        let t = self.ty(self.i);
        let kind = if t == STR {
            LabelType::String
        } else if t == MD_STR {
            LabelType::Markdown
        } else if t == EDGE_TEXT || t == UNICODE_TEXT {
            LabelType::Text
        } else {
            return Err(self.fail("'EDGE_TEXT', 'STR', 'MD_STR'"));
        };
        let mut s = self.text(self.i);
        self.i += 1;
        while matches!(self.ty(self.i), EDGE_TEXT | UNICODE_TEXT) {
            s.push_str(&self.text(self.i));
            self.i += 1;
        }
        Ok(FlowText { text: s, kind })
    }

    fn text_no_tags(&mut self) -> Result<FlowText, PelersError> {
        let t = self.ty(self.i);
        let kind = if t == STR {
            LabelType::Text
        } else if t == MD_STR {
            LabelType::Markdown
        } else if is_no_tags(t) {
            LabelType::Text
        } else {
            return Err(self.fail("'NODE_STRING', 'STR', 'MD_STR'"));
        };
        let mut s = self.text(self.i);
        self.i += 1;
        while is_no_tags(self.ty(self.i)) {
            s.push_str(&self.text(self.i));
            self.i += 1;
        }
        Ok(FlowText { text: s, kind })
    }

    fn shape_data(&mut self) -> String {
        self.data_from = self.i;
        let mut s = self.text(self.i);
        self.i += 1;
        while self.ty(self.i) == SHAPE_DATA {
            s.push_str(&self.text(self.i));
            self.i += 1;
        }
        self.data_to = self.i - 1;
        s
    }

    fn attach(&mut self, id: &str, data: &str, from: usize, to: usize) -> Result<(), PelersError> {
        let loc = self.span(from, to);
        self.db
            .add_vertex(id, None, None, None, Some((data, loc)))?;
        self.db.extend_vertex_mapping(id, Some(loc));
        Ok(())
    }

    fn attach_data(&mut self, id: &str, frame: &Frame) -> Result<(), PelersError> {
        let Some(data) = &frame.data else {
            return Ok(());
        };
        let loc = self.span(frame.data_from, frame.data_to);
        self.db.add_vertex(id, None, None, None, Some((data, loc)))
    }

    fn vertex(&mut self) -> Result<String, PelersError> {
        let from = self.i;
        let id = self.id_string()?;
        let mut label: Option<FlowText> = None;
        let mut shape: Option<&str> = None;
        match self.ty(self.i) {
            SQS => {
                self.i += 1;
                label = Some(self.flow_text()?);
                self.closed(SQE)?;
                shape = Some("square");
            }
            DOUBLECIRCLESTART => {
                self.i += 1;
                label = Some(self.flow_text()?);
                self.closed(DOUBLECIRCLEEND)?;
                shape = Some("doublecircle");
            }
            PS => {
                self.i += 1;
                if self.ty(self.i) == PS {
                    self.i += 1;
                    label = Some(self.flow_text()?);
                    self.closed(PE)?;
                    self.expect(PE, "")?;
                    shape = Some("circle");
                } else {
                    label = Some(self.flow_text()?);
                    self.closed(PE)?;
                    shape = Some("round");
                }
            }
            ELLIPSE_START => {
                self.i += 1;
                label = Some(self.flow_text()?);
                self.closed(ELLIPSE_END)?;
                shape = Some("ellipse");
            }
            STADIUMSTART => {
                self.i += 1;
                label = Some(self.flow_text()?);
                self.closed(STADIUMEND)?;
                shape = Some("stadium");
            }
            SUBROUTINESTART => {
                self.i += 1;
                label = Some(self.flow_text()?);
                self.closed(SUBROUTINEEND)?;
                shape = Some("subroutine");
            }
            VERTEX_WITH_PROPS_START => {
                self.i += 1;
                self.expect(NODE_STRING, "")?;
                self.expect(COLON, "")?;
                self.expect(NODE_STRING, "")?;
                self.expect(PIPE, "")?;
                label = Some(self.flow_text()?);
                self.closed(SQE)?;
                shape = Some("rect");
            }
            CYLINDERSTART => {
                self.i += 1;
                label = Some(self.flow_text()?);
                self.closed(CYLINDEREND)?;
                shape = Some("cylinder");
            }
            DIAMOND_START => {
                self.i += 1;
                if self.ty(self.i) == DIAMOND_START {
                    self.i += 1;
                    label = Some(self.flow_text()?);
                    self.closed(DIAMOND_STOP)?;
                    self.expect(DIAMOND_STOP, "")?;
                    shape = Some("hexagon");
                } else {
                    label = Some(self.flow_text()?);
                    self.closed(DIAMOND_STOP)?;
                    shape = Some("diamond");
                }
            }
            TAGEND => {
                self.i += 1;
                label = Some(self.flow_text()?);
                self.closed(SQE)?;
                shape = Some("odd");
            }
            TRAPSTART => {
                self.i += 1;
                label = Some(self.flow_text()?);
                if self.ty(self.i) == INVTRAPEND {
                    self.i += 1;
                    shape = Some("lean_right");
                } else {
                    self.closed(TRAPEND)?;
                    shape = Some("trapezoid");
                }
            }
            INVTRAPSTART => {
                self.i += 1;
                label = Some(self.flow_text()?);
                if self.ty(self.i) == TRAPEND {
                    self.i += 1;
                    shape = Some("lean_left");
                } else {
                    self.closed(INVTRAPEND)?;
                    shape = Some("inv_trapezoid");
                }
            }
            _ => {}
        }
        self.db.add_vertex(&id, label, shape, None, None)?;
        let loc = self.span(from, self.i - 1);
        self.db.add_vertex_mapping(&id, shape, Some(loc));
        Ok(id)
    }

    fn styled_vertex(&mut self) -> Result<String, PelersError> {
        let id = self.vertex()?;
        if self.ty(self.i) == STYLE_SEPARATOR {
            self.i += 1;
            let class = self.id_string()?;
            self.db.set_class(&id, &class);
        }
        Ok(id)
    }

    fn node(&mut self) -> Result<Vec<String>, PelersError> {
        let mut ids = vec![self.styled_vertex()?];
        self.pending = None;
        loop {
            let data = if self.ty(self.i) == SHAPE_DATA {
                Some(self.shape_data())
            } else {
                None
            };
            let from = self.data_from;
            let to = self.data_to;
            if self.ty(self.i) != SPACE {
                self.pending = data.map(|d| (d, from, to));
                return Ok(ids);
            }
            loop {
                self.i += 1;
                if self.ty(self.i) != SPACE {
                    break;
                }
            }
            if self.ty(self.i) != AMP {
                if data.is_some() {
                    return Err(self.fail("'AMP'"));
                }
                return Ok(ids);
            }
            self.i += 1;
            if self.ty(self.i) != SPACE {
                return Err(self.fail("'SPACE'"));
            }
            loop {
                self.i += 1;
                if self.ty(self.i) != SPACE {
                    break;
                }
            }
            let next = self.styled_vertex()?;
            if let Some(data) = data {
                let last = ids[ids.len() - 1].clone();
                self.attach(&last, &data, from, to)?;
            }
            ids.push(next);
        }
    }

    fn link(&mut self) -> Result<LinkInfo, PelersError> {
        let mut id: Option<String> = None;
        if self.ty(self.i) == LINK_ID {
            id = Some(self.text(self.i));
            self.i += 1;
        }
        if self.ty(self.i) == LINK {
            let raw = self.text(self.i);
            self.i += 1;
            let mut info = self.db.destruct_link(&raw, None);
            info.id = id;
            if self.ty(self.i) == PIPE {
                self.i += 1;
                info.text = Some(self.flow_text()?);
                self.closed(PIPE)?;
                if self.ty(self.i) == SPACE {
                    self.i += 1;
                }
            }
            return Ok(info);
        }
        if self.ty(self.i) == START_LINK {
            let start = self.text(self.i);
            self.i += 1;
            let label = self.edge_text()?;
            let end = self.expect(LINK, "'EDGE_TEXT'")?;
            let mut info = self.db.destruct_link(&end, Some(&start));
            info.text = Some(label);
            info.id = id;
            return Ok(info);
        }
        Err(self.fail("'LINK', 'START_LINK'"))
    }

    fn vertex_statement(&mut self) -> Result<Vec<String>, PelersError> {
        let from = self.i;
        let mut stmt = self.node()?;
        if let Some((data, data_from, data_to)) = self.pending.clone() {
            let last = stmt[stmt.len() - 1].clone();
            self.attach(&last, &data, data_from, data_to)?;
        }
        let mut nodes = stmt.clone();
        loop {
            let t = self.ty(self.i);
            if t != LINK && t != START_LINK && t != LINK_ID {
                return Ok(nodes);
            }
            let info = self.link()?;
            let next = self.node()?;
            if let Some((data, data_from, data_to)) = self.pending.clone() {
                let last = next[next.len() - 1].clone();
                self.attach(&last, &data, data_from, data_to)?;
            }
            self.db.add_link(&stmt, &next, &info)?;
            let loc = self.span(from, self.i - 1);
            self.db.add_edge_mapping(&next, Some(loc));
            let mut merged = next.clone();
            merged.extend(nodes);
            nodes = merged;
            stmt = next;
        }
    }

    fn style(&mut self) -> Result<String, PelersError> {
        if !is_style_part(self.ty(self.i)) {
            return Err(self.fail("'NODE_STRING', 'NUM', 'COLON', 'BRKT'"));
        }
        let mut s = self.text(self.i);
        self.i += 1;
        while is_style_part(self.ty(self.i)) {
            s.push_str(&self.text(self.i));
            self.i += 1;
        }
        Ok(s)
    }

    fn styles_opt(&mut self) -> Result<Vec<String>, PelersError> {
        let mut out = vec![self.style()?];
        while self.ty(self.i) == COMMA {
            self.i += 1;
            out.push(self.style()?);
        }
        Ok(out)
    }

    fn link_style_statement(&mut self) -> Result<(), PelersError> {
        self.i += 1;
        self.expect(SPACE, "")?;
        let mut positions: Vec<String>;
        if self.ty(self.i) == DEFAULT {
            positions = vec![self.text(self.i)];
            self.i += 1;
        } else {
            positions = vec![self.expect(NUM, "'DEFAULT'")?];
            while self.ty(self.i) == COMMA {
                self.i += 1;
                positions.push(self.expect(NUM, "")?);
            }
        }
        self.expect(SPACE, "")?;
        if self.ty(self.i) != INTERPOLATE {
            let styles = self.styles_opt()?;
            self.db.update_link(&positions, styles)?;
            return Ok(());
        }
        self.i += 1;
        self.expect(SPACE, "")?;
        let interpolate = self.alpha_num()?;
        if self.ty(self.i) == SPACE {
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
        if self.ty(self.i) != SPACE {
            self.db.set_link(id, url, None);
            return Ok(());
        }
        self.i += 1;
        if self.ty(self.i) == LINK_TARGET {
            let target = self.text(self.i);
            self.i += 1;
            self.db.set_link(id, url, Some(&target));
            return Ok(());
        }
        let tooltip = self.expect(STR, "'LINK_TARGET'")?;
        if self.ty(self.i) == SPACE {
            self.i += 1;
            let target = self.expect(LINK_TARGET, "")?;
            self.db.set_link(id, url, Some(&target));
        } else {
            self.db.set_link(id, url, None);
        }
        self.db.set_tooltip(id, &tooltip);
        Ok(())
    }

    fn click_statement(&mut self) -> Result<(), PelersError> {
        let id = self.text(self.i);
        self.i += 1;
        match self.ty(self.i) {
            CALLBACKNAME => {
                self.i += 1;
                if self.ty(self.i) == CALLBACKARGS {
                    self.i += 1;
                }
                let mut tooltip: Option<String> = None;
                if self.ty(self.i) == SPACE {
                    self.i += 1;
                    tooltip = Some(self.expect(STR, "")?);
                }
                self.db.set_click_event(&id);
                if let Some(tooltip) = tooltip {
                    self.db.set_tooltip(&id, &tooltip);
                }
                Ok(())
            }
            HREF => {
                self.i += 1;
                let url = self.expect(STR, "")?;
                self.link_tail(&id, &url)
            }
            STR => {
                let url = self.text(self.i);
                self.i += 1;
                self.link_tail(&id, &url)
            }
            _ => {
                let _name = self.alpha_num()?;
                if self.ty(self.i) == SPACE {
                    self.i += 1;
                    let tooltip = self.expect(STR, "")?;
                    self.db.set_click_event(&id);
                    self.db.set_tooltip(&id, &tooltip);
                } else {
                    self.db.set_click_event(&id);
                }
                Ok(())
            }
        }
    }

    fn header(&mut self, at: usize) -> Result<Frame, PelersError> {
        self.expect(SPACE, SEPARATORS)?;
        let id = self.text_no_tags()?;
        let mut title = FlowText {
            text: String::new(),
            kind: LabelType::Text,
        };
        if self.ty(self.i) == SQS {
            self.i += 1;
            title = self.flow_text()?;
            self.closed(SQE)?;
        }
        let data = if self.ty(self.i) == SHAPE_DATA {
            Some(self.shape_data())
        } else {
            None
        };
        if !is_separator(self.ty(self.i)) {
            return Err(self.fail(&format!("{SEPARATORS}, 'SHAPE_DATA'")));
        }
        Ok(Frame {
            global: false,
            at,
            id: Some(id),
            title: Some(title),
            data,
            data_from: self.data_from,
            data_to: self.data_to,
            list: Vec::new(),
            collect: false,
        })
    }

    fn statement(&mut self) -> Result<Option<DocItem>, PelersError> {
        let t = self.ty(self.i);
        match t {
            STYLE => {
                self.i += 1;
                self.expect(SPACE, "")?;
                let id = self.id_string()?;
                self.expect(SPACE, "")?;
                let styles = self.styles_opt()?;
                self.db.add_vertex(&id, None, None, Some(styles), None)?;
                self.separator()?;
                Ok(None)
            }
            LINKSTYLE => {
                self.link_style_statement()?;
                self.separator()?;
                Ok(None)
            }
            CLASSDEF => {
                self.i += 1;
                self.expect(SPACE, "")?;
                let id = self.id_string()?;
                self.expect(SPACE, "")?;
                let styles = self.styles_opt()?;
                self.db.add_class(&id, &styles);
                self.separator()?;
                Ok(None)
            }
            CLASS => {
                self.i += 1;
                self.expect(SPACE, "")?;
                let ids = self.id_string()?;
                self.expect(SPACE, "")?;
                let class = self.id_string()?;
                self.db.set_class(&ids, &class);
                self.separator()?;
                Ok(None)
            }
            CLICK => {
                self.click_statement()?;
                self.separator()?;
                Ok(None)
            }
            CONNECTOR => {
                let at = self.i;
                self.i += 1;
                let frame = self.header(at)?;
                let sep = self.i;
                self.i += 1;
                let id = self
                    .db
                    .add_connector(frame.id.as_ref(), frame.title.as_ref());
                self.attach_data(&id, &frame)?;
                let frame_span = self.span(frame.at, frame.at);
                let sep_span = self.span(sep, sep);
                self.db
                    .add_connector_mapping(frame.id.as_ref(), Some(frame_span), Some(sep_span));
                Ok(Some(DocItem::Node(id)))
            }
            DIRECTION_TB | DIRECTION_BT | DIRECTION_RL | DIRECTION_LR | DIRECTION_TD => {
                self.i += 1;
                Ok(Some(DocItem::Dir(
                    DIRECTION_NAMES[t - DIRECTION_TB].to_owned(),
                )))
            }
            ACC_TITLE => {
                self.i += 1;
                let value = trim(&self.expect(ACC_TITLE_VALUE, "")?).to_owned();
                self.db.set_acc_title(&value);
                Ok(Some(DocItem::Node(value)))
            }
            ACC_DESCR => {
                self.i += 1;
                let value = trim(&self.expect(ACC_DESCR_VALUE, "")?).to_owned();
                self.db.set_acc_description(&value);
                Ok(Some(DocItem::Node(value)))
            }
            ACC_DESCR_MULTILINE_VALUE => {
                let value = trim(&self.text(self.i)).to_owned();
                self.i += 1;
                self.db.set_acc_description(&value);
                Ok(Some(DocItem::Node(value)))
            }
            _ => {
                if !is_id(t) {
                    return Err(self.fail(STATEMENT_START));
                }
                let nodes = self.vertex_statement()?;
                self.separator()?;
                Ok(Some(DocItem::Nodes(nodes)))
            }
        }
    }

    fn document(&mut self) -> Result<(), PelersError> {
        let mut open: Vec<Frame> = Vec::new();
        let mut list: Vec<DocItem> = Vec::new();
        let mut collect = false;
        loop {
            let t = self.ty(self.i);
            if t == END {
                break;
            }
            if t == END_KW {
                let Some(frame) = open.pop() else {
                    break;
                };
                let at = self.i;
                self.i += 1;
                if frame.global {
                    self.db.add_global(std::mem::take(&mut list));
                    list = frame.list;
                    collect = frame.collect;
                    continue;
                }
                let id = self.db.add_sub_graph(
                    frame.id.as_ref(),
                    std::mem::take(&mut list),
                    frame.title.as_ref(),
                );
                self.attach_data(&id, &frame)?;
                let frame_span = self.span(frame.at, frame.at);
                let end_span = self.span(at, at);
                let subgraph_text = frame.id.as_ref().map_or(String::new(), |t| t.text.clone());
                self.db
                    .add_subgraph_mapping(&subgraph_text, Some(frame_span), Some(end_span));
                list = frame.list;
                collect = frame.collect;
                if collect {
                    list.push(DocItem::Node(id));
                }
            } else if matches!(t, SEMI | NEWLINE | SPACE | EOF) {
                if collect {
                    list.push(DocItem::Node(self.text(self.i)));
                }
                self.i += 1;
            } else if t == COMMENT {
                self.i += 1;
            } else if t == FLOW || t == GLOBAL {
                let at = self.i;
                self.i += 1;
                let frame = if t == GLOBAL || is_separator(self.ty(self.i)) {
                    let frame = Frame {
                        global: t == GLOBAL,
                        at,
                        id: None,
                        title: None,
                        data: None,
                        data_from: 0,
                        data_to: 0,
                        list: std::mem::take(&mut list),
                        collect,
                    };
                    self.separator()?;
                    frame
                } else {
                    let mut frame = self.header(at)?;
                    frame.list = std::mem::take(&mut list);
                    frame.collect = collect;
                    self.i += 1;
                    frame
                };
                open.push(frame);
                list = Vec::new();
                collect = true;
            } else {
                let item = self.statement()?;
                if let Some(item) = item.filter(|_| collect) {
                    list.push(item);
                }
            }
        }
        if !open.is_empty() {
            return Err(self.fail("'end'"));
        }
        Ok(())
    }

    fn program(&mut self) -> Result<(), PelersError> {
        while matches!(self.ty(self.i), SPACE | NEWLINE | COMMENT) {
            self.i += 1;
        }
        self.expect(GRAPH, "'SPACE', 'NEWLINE', 'COMMENT'")?;
        if self.ty(self.i) == NODIR {
            self.i += 1;
            self.db.set_direction("TB");
        } else {
            let dir = self.expect(DIR, "'NODIR'")?;
            let t = self.ty(self.i);
            if t == SPACE {
                loop {
                    self.i += 1;
                    if self.ty(self.i) != SPACE {
                        break;
                    }
                }
                if self.ty(self.i) != NEWLINE {
                    return Err(self.fail("'NEWLINE', 'SPACE'"));
                }
            } else if t != SEMI && t != NEWLINE && t != COMMENT {
                return Err(self.fail("'COMMENT', 'SEMI', 'NEWLINE', 'SPACE'"));
            }
            self.i += 1;
            self.db.set_direction(&dir);
        }
        self.document()?;
        if self.ty(self.i) != END {
            return Err(self.fail(STATEMENT_START));
        }
        Ok(())
    }
}

pub fn parse_agentflow(source: &str, db: &mut AgentflowDb) -> Result<(), PelersError> {
    let src = if source.contains('}') {
        strip_brace_trail(source)
    } else {
        source.to_owned()
    };
    let tokens = tokenize(&src);
    let mut parser = Parser {
        source,
        src,
        tokens,
        i: 0,
        data_from: 0,
        data_to: 0,
        pending: None,
        db,
    };
    parser.program()
}
