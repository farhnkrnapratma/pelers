use super::db::ClassDb;
use super::lexer::tokenize;
use super::tokens::*;
use super::types::{ClassRelation, RelationEnd};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const NAME_TOKENS: &str = "'UNICODE_TEXT', 'NUM', 'ALPHA', 'MINUS', 'BQUOTE_STR'";
const STATEMENT_TOKENS: &str = "'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'NAMESPACE', 'ANNOTATION_START', 'CLASS', 'MEMBER', 'NOTE_FOR', 'NOTE', 'CLASSDEF', 'direction_tb', 'CALLBACK', 'LINK', 'CLICK', 'STYLE', 'CSSCLASS', 'ALPHA', 'MINUS', 'BQUOTE_STR'";

fn is_alphanum(t: usize) -> bool {
    matches!(t, UNICODE_TEXT | ALPHA | MINUS)
}

fn is_style_part(t: usize) -> bool {
    matches!(t, ALPHA | COLON | BRKT | STYLE | PCT | LABEL)
}

fn end_type(t: usize) -> Option<u8> {
    match t {
        AGGREGATION => Some(0),
        EXTENSION => Some(1),
        COMPOSITION => Some(2),
        DEPENDENCY => Some(3),
        LOLLIPOP => Some(4),
        _ => None,
    }
}

struct Frame {
    id: String,
    classes: Vec<String>,
    notes: Vec<String>,
}

struct Parser<'a, 'b> {
    src: &'a str,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
    i: usize,
    db: &'b mut ClassDb,
}

impl Parser<'_, '_> {
    fn ty(&self, at: usize) -> usize {
        self.types.get(at).copied().unwrap_or(usize::MAX)
    }

    fn text(&self, at: usize) -> String {
        self.texts.get(at).cloned().unwrap_or_default()
    }

    fn fail<T>(&self, expected: &str) -> Result<T, PelersError> {
        let t = self.ty(self.i);
        let unit = self
            .starts
            .get(self.i)
            .copied()
            .unwrap_or_else(|| self.src.encode_utf16().count());
        let offset = utf16_to_byte(self.src, unit);
        if t == ERROR {
            return Err(lexical_error("class", self.src, offset));
        }
        let name = TOKEN_NAMES.get(t).copied().unwrap_or("undefined");
        Err(syntax_error(
            "class",
            self.src,
            offset,
            format!("Expecting {expected}, got '{name}'"),
        ))
    }

    fn expect(&mut self, t: usize) -> Result<String, PelersError> {
        if self.ty(self.i) != t {
            return self.fail(&format!("'{}'", TOKEN_NAMES[t]));
        }
        let text = self.text(self.i);
        self.i += 1;
        Ok(text)
    }

    fn alpha_num(&mut self) -> Result<String, PelersError> {
        if !is_alphanum(self.ty(self.i)) {
            return self.fail("'UNICODE_TEXT', 'NUM', 'ALPHA', 'MINUS'");
        }
        let text = self.text(self.i);
        self.i += 1;
        Ok(text)
    }

    fn name(&mut self, generic: bool) -> Result<String, PelersError> {
        let mut out = String::new();
        loop {
            if self.ty(self.i) == BQUOTE_STR {
                out.push_str(&self.text(self.i));
                self.i += 1;
                break;
            }
            if !is_alphanum(self.ty(self.i)) {
                return self.fail(NAME_TOKENS);
            }
            out.push_str(&self.text(self.i));
            self.i += 1;
            let t = self.ty(self.i);
            if t == DOT {
                out.push('.');
                self.i += 1;
            } else if !is_alphanum(t) && t != BQUOTE_STR {
                break;
            }
        }
        if generic && self.ty(self.i) == GENERICTYPE {
            out.push('~');
            out.push_str(&self.text(self.i));
            out.push('~');
            self.i += 1;
        }
        Ok(out)
    }

    fn class_label(&mut self) -> Result<String, PelersError> {
        self.i += 1;
        let label = self.expect(STR)?;
        self.expect(SQE)?;
        Ok(label)
    }

    fn members(&mut self) -> Result<Vec<String>, PelersError> {
        if self.ty(self.i) != MEMBER {
            return self.fail("'STRUCT_STOP', 'MEMBER'");
        }
        let mut list = Vec::new();
        while self.ty(self.i) == MEMBER {
            list.push(self.text(self.i));
            self.i += 1;
        }
        self.expect(STRUCT_STOP)?;
        list.reverse();
        Ok(list)
    }

    fn class_statement(&mut self) -> Result<String, PelersError> {
        self.i += 1;
        let id = self.name(true)?;
        self.db.add_class(&id);
        if self.ty(self.i) == SQS {
            let label = self.class_label()?;
            self.db.set_class_label(&id, &label);
        }
        match self.ty(self.i) {
            STYLE_SEPARATOR => {
                self.i += 1;
                let css = self.alpha_num()?;
                self.db.set_css_class(&id, &css);
                if self.ty(self.i) == STRUCT_START {
                    self.i += 1;
                    let list = self.members()?;
                    self.db.add_members(&id, &list);
                }
            }
            STRUCT_START => {
                self.i += 1;
                if self.ty(self.i) == STRUCT_STOP {
                    self.i += 1;
                } else {
                    let list = self.members()?;
                    self.db.add_members(&id, &list);
                }
            }
            ANNOTATION_START => {
                self.i += 1;
                let annotation = self.alpha_num()?;
                self.expect(ANNOTATION_END)?;
                self.db.add_annotation(&id, &annotation);
                if self.ty(self.i) == STRUCT_START {
                    self.i += 1;
                    if self.ty(self.i) == STRUCT_STOP {
                        self.i += 1;
                    } else {
                        let list = self.members()?;
                        self.db.add_members(&id, &list);
                    }
                }
            }
            _ => {}
        }
        Ok(id)
    }

    fn note_statement(&mut self) -> Result<String, PelersError> {
        let first = self.ty(self.i);
        self.i += 1;
        if first == NOTE {
            let text = self.expect(STR)?;
            return Ok(self.db.add_note(&text, None));
        }
        let id = self.name(true)?;
        let text = self.expect(STR)?;
        Ok(self.db.add_note(&text, Some(&id)))
    }

    fn namespace_statement(&mut self) -> Result<(), PelersError> {
        let mut open: Vec<Frame> = Vec::new();
        loop {
            self.i += 1;
            let ns = self.name(false)?;
            let id = if self.ty(self.i) == SQS {
                let label = self.class_label()?;
                self.db.add_namespace(&ns, Some(&label))
            } else {
                self.db.add_namespace(&ns, None)
            };
            self.expect(STRUCT_START)?;
            if self.ty(self.i) == NEWLINE {
                self.i += 1;
            }
            open.push(Frame {
                id,
                classes: Vec::new(),
                notes: Vec::new(),
            });
            let mut item = true;
            while item {
                let t = self.ty(self.i);
                if t == NAMESPACE {
                    break;
                }
                if t == CLASS {
                    let class = self.class_statement()?;
                    if let Some(frame) = open.last_mut() {
                        frame.classes.push(class);
                    }
                } else if t == NOTE || t == NOTE_FOR {
                    let note = self.note_statement()?;
                    if let Some(frame) = open.last_mut() {
                        frame.notes.push(note);
                    }
                } else {
                    return self.fail("'NAMESPACE', 'CLASS', 'NOTE_FOR', 'NOTE'");
                }
                loop {
                    if self.ty(self.i) == NEWLINE {
                        let next = self.ty(self.i + 1);
                        if matches!(next, NAMESPACE | CLASS | NOTE | NOTE_FOR) {
                            self.i += 1;
                            break;
                        }
                        self.i += 1;
                    }
                    self.expect(STRUCT_STOP)?;
                    let Some(done) = open.pop() else {
                        break;
                    };
                    self.db
                        .add_classes_to_namespace(&done.id, &done.classes, &done.notes);
                    self.db.pop_namespace();
                    if open.is_empty() {
                        item = false;
                        break;
                    }
                }
            }
            if open.is_empty() {
                return Ok(());
            }
        }
    }

    fn relation_end(&mut self) -> RelationEnd {
        let kind = end_type(self.ty(self.i));
        kind?;
        self.i += 1;
        kind
    }

    fn class_or_relation(&mut self) -> Result<(), PelersError> {
        let id1 = self.name(true)?;
        let mut t = self.ty(self.i);
        if t == LABEL {
            let label = self.db.cleanup_label(&self.text(self.i));
            self.i += 1;
            self.db.add_member(&id1, &label);
            return Ok(());
        }
        let mut title1 = "none".to_owned();
        if t == STR {
            title1 = self.text(self.i);
            self.i += 1;
        } else if end_type(t).is_none() && t != LINE && t != DOTTED_LINE {
            return Ok(());
        }
        let type1 = self.relation_end();
        t = self.ty(self.i);
        if t != LINE && t != DOTTED_LINE {
            return self.fail(
                "'AGGREGATION', 'EXTENSION', 'COMPOSITION', 'DEPENDENCY', 'LOLLIPOP', 'LINE', 'DOTTED_LINE'",
            );
        }
        self.i += 1;
        let type2 = self.relation_end();
        let title2 = if self.ty(self.i) == STR {
            let text = self.text(self.i);
            self.i += 1;
            text
        } else {
            "none".to_owned()
        };
        let id2 = self.name(true)?;
        let mut relation = ClassRelation {
            id1,
            id2,
            relation_title1: title1,
            relation_title2: title2,
            title: None,
            type1,
            type2,
            line_type: u8::from(t != LINE),
        };
        if self.ty(self.i) == LABEL {
            relation.title = Some(self.db.cleanup_label(&self.text(self.i)));
            self.i += 1;
        }
        self.db.add_relation(relation);
        Ok(())
    }

    fn styles(&mut self) -> Result<Vec<String>, PelersError> {
        let mut out = Vec::new();
        loop {
            if !is_style_part(self.ty(self.i)) {
                return self.fail(
                    "'ALPHA', 'NUM', 'COLON', 'UNIT', 'SPACE', 'BRKT', 'STYLE', 'PCT', 'LABEL'",
                );
            }
            let mut s = self.text(self.i);
            self.i += 1;
            while is_style_part(self.ty(self.i)) {
                s.push_str(&self.text(self.i));
                self.i += 1;
            }
            out.push(s);
            if self.ty(self.i) != COMMA {
                return Ok(out);
            }
            self.i += 1;
        }
    }

    fn tooltip(&mut self, id: &str) {
        if self.ty(self.i) == STR {
            let text = self.text(self.i);
            self.i += 1;
            self.db.set_tooltip(id, Some(&text));
        }
    }

    fn link_statement(&mut self, id: &str) -> Result<(), PelersError> {
        let href = self.expect(STR)?;
        let tip = if self.ty(self.i) == STR {
            let text = self.text(self.i);
            self.i += 1;
            Some(text)
        } else {
            None
        };
        if self.ty(self.i) == LINK_TARGET {
            let target = self.text(self.i);
            self.i += 1;
            self.db.set_link(id, &href, Some(&target));
        } else {
            self.db.set_link(id, &href, None);
        }
        if let Some(tip) = tip {
            self.db.set_tooltip(id, Some(&tip));
        }
        Ok(())
    }

    fn statement(&mut self) -> Result<(), PelersError> {
        let t = self.ty(self.i);
        match t {
            UNICODE_TEXT | ALPHA | MINUS | BQUOTE_STR => return self.class_or_relation(),
            NAMESPACE => return self.namespace_statement(),
            CLASS => {
                self.class_statement()?;
                return Ok(());
            }
            MEMBER => {
                self.i += 1;
                return Ok(());
            }
            ANNOTATION_START => {
                self.i += 1;
                let annotation = self.alpha_num()?;
                self.expect(ANNOTATION_END)?;
                let id = self.name(true)?;
                self.db.add_annotation(&id, &annotation);
                return Ok(());
            }
            CALLBACK => {
                self.i += 1;
                let id = self.name(true)?;
                self.expect(STR)?;
                self.db.set_click_event(&id);
                self.tooltip(&id);
                return Ok(());
            }
            LINK => {
                self.i += 1;
                let id = self.name(true)?;
                return self.link_statement(&id);
            }
            CLICK => {
                self.i += 1;
                let id = self.name(true)?;
                if self.ty(self.i) == HREF {
                    self.i += 1;
                    return self.link_statement(&id);
                }
                self.expect(CALLBACK_NAME)?;
                if self.ty(self.i) == CALLBACK_ARGS {
                    self.i += 1;
                }
                self.db.set_click_event(&id);
                self.tooltip(&id);
                return Ok(());
            }
            STYLE => {
                self.i += 1;
                let id = self.expect(ALPHA)?;
                let styles = self.styles()?;
                self.db.set_css_style(&id, &styles);
                return Ok(());
            }
            CSSCLASS => {
                self.i += 1;
                let ids = self.expect(STR)?;
                let css = self.expect(ALPHA)?;
                self.db.set_css_class(&ids, &css);
                return Ok(());
            }
            NOTE | NOTE_FOR => {
                self.note_statement()?;
                return Ok(());
            }
            CLASSDEF => {
                self.i += 1;
                let mut ids = vec![self.expect(ALPHA)?];
                while self.ty(self.i) == COMMA {
                    self.i += 1;
                    ids.push(self.expect(ALPHA)?);
                }
                let styles = self.styles()?;
                self.db.define_class(&ids, &styles);
                return Ok(());
            }
            DIRECTION_TB | DIRECTION_BT | DIRECTION_RL | DIRECTION_LR => {
                self.i += 1;
                let dir = match t {
                    DIRECTION_TB => "TB",
                    DIRECTION_BT => "BT",
                    DIRECTION_RL => "RL",
                    _ => "LR",
                };
                self.db.set_direction(dir);
                return Ok(());
            }
            ACC_TITLE => {
                self.i += 1;
                let value = self.expect(ACC_TITLE_VALUE)?;
                self.db.set_acc_title(trim(&value));
                return Ok(());
            }
            ACC_DESCR => {
                self.i += 1;
                let value = self.expect(ACC_DESCR_VALUE)?;
                self.db.set_acc_description(trim(&value));
                return Ok(());
            }
            ACC_DESCR_MULTILINE_VALUE => {
                let value = self.text(self.i);
                self.i += 1;
                self.db.set_acc_description(trim(&value));
                return Ok(());
            }
            _ => {}
        }
        self.fail(STATEMENT_TOKENS)
    }

    fn run(&mut self) -> Result<(), PelersError> {
        let header = self.ty(self.i) == CLASS_DIAGRAM;
        if header {
            self.i += 1;
            self.expect(NEWLINE)?;
        }
        loop {
            self.statement()?;
            if self.ty(self.i) != NEWLINE {
                break;
            }
            self.i += 1;
            if matches!(self.ty(self.i), EOF | END) {
                break;
            }
        }
        if header {
            self.expect(EOF)?;
        }
        self.expect(END)?;
        Ok(())
    }
}

pub fn parse_class_diagram(src: &str, db: &mut ClassDb) -> Result<(), PelersError> {
    let tokens = tokenize(src);
    let mut parser = Parser {
        src,
        types: tokens.types,
        texts: tokens.texts,
        starts: tokens.starts,
        i: 0,
        db,
    };
    parser.run()
}
