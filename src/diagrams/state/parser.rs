use super::db::StateDb;
use super::lexer::tokenize;
use super::tokens::*;
use super::types::{Desc, RelationStmt, StateNote, StateStmt, Stmt};
use crate::errors::{PelersError, PelersErrorCode, lexical_error, locate, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const DIRECTIONS: [&str; 4] = ["TB", "BT", "RL", "LR"];

const STATEMENT: &str = "'NL', 'ID', 'HIDE_EMPTY', 'scale', 'COMPOSIT_STATE', 'STATE_DESCR', 'FORK', 'JOIN', 'CHOICE', 'CONCURRENT', 'note', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'CLICK', 'classDef', 'style', 'class', 'direction_tb', 'direction_bt', 'direction_rl', 'direction_lr', 'EDGE_STATE'";

struct Parser<'a, 'b> {
    src: &'a str,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
    error: Option<String>,
    i: usize,
    db: &'b mut StateDb,
    doc: Vec<Stmt>,
    outer: Vec<(Vec<Stmt>, StateStmt)>,
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
            return Err(match &self.error {
                Some(message) => {
                    let (line, column, snippet) = locate(self.src, offset);
                    PelersError {
                        message: message.clone(),
                        code: PelersErrorCode::Syntax,
                        diagram_type: Some("state".to_owned()),
                        mermaid_type: None,
                        line,
                        column,
                        snippet,
                    }
                }
                None => lexical_error("state", self.src, offset),
            });
        }
        let name = TOKEN_NAMES.get(t).copied().unwrap_or("undefined");
        Err(syntax_error(
            "state",
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

    fn id_statement(&mut self) -> Result<StateStmt, PelersError> {
        let t = self.ty(self.i);
        if t != ID && t != EDGE_STATE {
            return self.fail("'ID', 'EDGE_STATE'");
        }
        let id = trim(&self.text(self.i)).to_owned();
        self.i += 1;
        let mut stmt = StateStmt {
            id,
            type_: Some("default".to_owned()),
            description: Some(Desc::Text(String::new())),
            ..StateStmt::default()
        };
        if self.ty(self.i) == STYLE_SEPARATOR {
            self.i += 1;
            let class = trim(&self.expect(ID)?).to_owned();
            stmt.classes = Some(vec![class]);
        }
        Ok(stmt)
    }

    fn add(&mut self, value: Stmt) {
        self.doc.push(value);
    }

    fn add_text(&mut self, value: String) {
        if value != "nl" {
            self.doc.push(Stmt::Text(value));
        }
    }

    fn open(&mut self, id: String, description: String) {
        self.i += 1;
        let header = StateStmt {
            id,
            type_: Some("default".to_owned()),
            description: Some(Desc::Text(description)),
            ..StateStmt::default()
        };
        let saved = std::mem::take(&mut self.doc);
        self.outer.push((saved, header));
    }

    fn close(&mut self) {
        if let Some((mut parent, mut header)) = self.outer.pop() {
            header.doc = Some(std::mem::take(&mut self.doc));
            parent.push(Stmt::State(header));
            self.doc = parent;
        }
    }

    fn run(&mut self) -> Result<(), PelersError> {
        while self.ty(self.i) == NL {
            self.i += 1;
        }
        if self.ty(self.i) != SD {
            return self.fail("'NL', 'SD'");
        }
        self.i += 1;

        loop {
            let t = self.ty(self.i);
            match t {
                NL => self.i += 1,
                ID | EDGE_STATE => {
                    let mut first = self.id_statement()?;
                    if self.ty(self.i) == DESCR {
                        let text = self.text(self.i);
                        self.i += 1;
                        first.description = Some(Desc::Text(self.db.trim_colon(&text)));
                        self.add(Stmt::State(first));
                    } else if self.ty(self.i) == ARROW {
                        self.i += 1;
                        let second = self.id_statement()?;
                        let description = if self.ty(self.i) == DESCR {
                            let text = self.text(self.i);
                            self.i += 1;
                            Some(self.db.trim_colon(&text))
                        } else {
                            None
                        };
                        self.add(Stmt::Relation(Box::new(RelationStmt {
                            state1: first,
                            state2: second,
                            description,
                        })));
                    } else {
                        self.add(Stmt::State(first));
                    }
                }
                HIDE_EMPTY => {
                    let text = self.text(self.i);
                    self.i += 1;
                    self.add_text(text);
                }
                SCALE => {
                    self.i += 1;
                    self.expect(WIDTH)?;
                    let text = self.text(self.i - 2);
                    self.add_text(text);
                }
                COMPOSIT_STATE => {
                    let text = self.text(self.i);
                    self.i += 1;
                    if self.ty(self.i) == STRUCT_START {
                        self.open(text, String::new());
                    } else {
                        self.add_text(text);
                    }
                }
                STATE_DESCR => {
                    let description = self.text(self.i);
                    self.i += 1;
                    self.expect(AS)?;
                    let id = self.expect(ID)?;
                    if self.ty(self.i) == STRUCT_START {
                        self.open(id, description);
                    } else if let Some((name, rest)) = id.split_once(':') {
                        let part = rest.split(':').next().unwrap_or("").to_owned();
                        self.add(Stmt::State(StateStmt {
                            id: name.to_owned(),
                            type_: Some("default".to_owned()),
                            description: Some(Desc::List(vec![
                                trim(&description).to_owned(),
                                part,
                            ])),
                            ..StateStmt::default()
                        }));
                    } else {
                        self.add(Stmt::State(StateStmt {
                            id,
                            type_: Some("default".to_owned()),
                            description: Some(Desc::Text(trim(&description).to_owned())),
                            ..StateStmt::default()
                        }));
                    }
                }
                FORK | JOIN | CHOICE => {
                    let id = self.text(self.i);
                    self.i += 1;
                    let kind = match t {
                        FORK => "fork",
                        JOIN => "join",
                        _ => "choice",
                    };
                    self.add(Stmt::State(StateStmt {
                        id,
                        type_: Some(kind.to_owned()),
                        ..StateStmt::default()
                    }));
                }
                CONCURRENT => {
                    self.i += 1;
                    let id = self.db.get_divider_id();
                    self.add(Stmt::State(StateStmt {
                        id,
                        type_: Some("divider".to_owned()),
                        ..StateStmt::default()
                    }));
                }
                NOTE => {
                    let keyword = self.text(self.i);
                    self.i += 1;
                    let next = self.ty(self.i);
                    if next == LEFT_OF || next == RIGHT_OF {
                        let position = trim(&self.text(self.i)).to_owned();
                        self.i += 1;
                        let id = trim(&self.expect(ID)?).to_owned();
                        let text = trim(&self.expect(NOTE_TEXT)?).to_owned();
                        self.add(Stmt::State(StateStmt {
                            id,
                            note: Some(StateNote { position, text }),
                            ..StateStmt::default()
                        }));
                    } else {
                        if next != NOTE_TEXT {
                            return self.fail("'NOTE_TEXT', 'left_of', 'right_of'");
                        }
                        self.i += 1;
                        self.expect(AS)?;
                        self.expect(ID)?;
                        self.add_text(keyword);
                    }
                }
                DIRECTION_TB..=DIRECTION_LR => {
                    let value = DIRECTIONS[t - DIRECTION_TB];
                    self.i += 1;
                    self.db.set_direction(value);
                    self.add(Stmt::Dir(value.to_owned()));
                }
                ACC_TITLE => {
                    self.i += 1;
                    let value = trim(&self.expect(ACC_TITLE_VALUE)?).to_owned();
                    self.db.set_acc_title(&value);
                    self.add_text(value);
                }
                ACC_DESCR => {
                    self.i += 1;
                    let value = trim(&self.expect(ACC_DESCR_VALUE)?).to_owned();
                    self.db.set_acc_description(&value);
                    self.add_text(value);
                }
                ACC_DESCR_MULTILINE_VALUE => {
                    let value = trim(&self.text(self.i)).to_owned();
                    self.i += 1;
                    self.db.set_acc_description(&value);
                    self.add_text(value);
                }
                CLICK => {
                    self.i += 1;
                    let id = self.id_statement()?;
                    let (url, tooltip) = if self.ty(self.i) == HREF {
                        self.i += 1;
                        (self.expect(STRING)?, String::new())
                    } else {
                        if self.ty(self.i) != STRING {
                            return self.fail("'STRING', 'HREF'");
                        }
                        let url = self.text(self.i);
                        self.i += 1;
                        let tooltip = self.expect(STRING)?;
                        (url, tooltip)
                    };
                    self.expect(NL)?;
                    self.add(Stmt::Click {
                        id: id.id,
                        url,
                        tooltip,
                    });
                }
                CLASSDEF => {
                    self.i += 1;
                    let id = trim(&self.expect(CLASSDEF_ID)?).to_owned();
                    let classes = trim(&self.expect(CLASSDEF_STYLEOPTS)?).to_owned();
                    self.add(Stmt::ClassDef { id, classes });
                }
                STYLE => {
                    self.i += 1;
                    let id = trim(&self.expect(STYLE_IDS)?).to_owned();
                    let style_class = trim(&self.expect(STYLEDEF_STYLEOPTS)?).to_owned();
                    self.add(Stmt::Style { id, style_class });
                }
                CLASS => {
                    self.i += 1;
                    let id = trim(&self.expect(CLASSENTITY_IDS)?).to_owned();
                    let style_class = trim(&self.expect(STYLECLASS)?).to_owned();
                    self.add(Stmt::ApplyClass { id, style_class });
                }
                STRUCT_STOP => {
                    if self.outer.is_empty() {
                        return self.fail(&format!("'$end', {STATEMENT}"));
                    }
                    self.i += 1;
                    self.close();
                }
                END => {
                    if !self.outer.is_empty() {
                        return self.fail(&format!("'STRUCT_STOP', {STATEMENT}"));
                    }
                    let doc = std::mem::take(&mut self.doc);
                    return self.db.set_root_doc(doc);
                }
                _ => {
                    return self.fail(&if self.outer.is_empty() {
                        format!("'$end', {STATEMENT}")
                    } else {
                        format!("'STRUCT_STOP', {STATEMENT}")
                    });
                }
            }
        }
    }
}

pub fn parse_state(src: &str, db: &mut StateDb) -> Result<(), PelersError> {
    let tokens = tokenize(src);
    let mut parser = Parser {
        src,
        types: tokens.types,
        texts: tokens.texts,
        starts: tokens.starts,
        error: tokens.error,
        i: 0,
        db,
        doc: Vec::new(),
        outer: Vec::new(),
    };
    parser.run()
}
