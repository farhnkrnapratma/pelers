use super::db::{
    BlockDb, edge_str_to_edge_data, edge_str_to_edge_start_data, edge_str_to_pattern,
    edge_str_to_thickness, type_str_2_type,
};
use super::lexer::tokenize;
use super::tokens::*;
use super::types::{Block, Statement};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};
use crate::util::jsnum::js_parse_int;

const STATEMENT_START: &str = "'NODE_ID', 'COLUMNS', 'SPACE_BLOCK', 'BLOCK_DIAGRAM_KEY', 'id-block', 'classDef', 'class', 'style'";

struct NodeInfo {
    id: String,
    label: Option<String>,
    type_str: Option<String>,
    directions: Option<Vec<String>>,
}

struct Frame {
    list: Vec<Statement>,
    header: Option<Statement>,
}

fn close_document(mut list: Vec<Statement>) -> Vec<Statement> {
    if let Some(Statement::Many(blocks)) = list.last().cloned() {
        list.pop();
        list.extend(blocks.into_iter().map(Statement::One));
    }
    list
}

struct Parser<'a, 'b> {
    src: &'a str,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
    i: usize,
    db: &'b mut BlockDb,
}

impl Parser<'_, '_> {
    fn ty(&self) -> usize {
        self.types.get(self.i).copied().unwrap_or(usize::MAX)
    }

    fn text(&self, at: usize) -> String {
        self.texts.get(at).cloned().unwrap_or_default()
    }

    fn fail<T>(&self, expected: &str) -> Result<T, PelersError> {
        let unit = self
            .starts
            .get(self.i)
            .copied()
            .unwrap_or_else(|| self.src.encode_utf16().count());
        let offset = utf16_to_byte(self.src, unit);
        if self.ty() == ERROR {
            return Err(lexical_error("block", self.src, offset));
        }
        let got = TOKEN_NAMES.get(self.ty()).copied().unwrap_or("undefined");
        Err(syntax_error(
            "block",
            self.src,
            offset,
            format!("Expecting {expected}, got '{got}'"),
        ))
    }

    fn expect(&mut self, t: usize) -> Result<String, PelersError> {
        if self.ty() != t {
            return self.fail(&format!("'{}'", TOKEN_NAMES[t]));
        }
        let text = self.text(self.i);
        self.i += 1;
        Ok(text)
    }

    fn node(&mut self) -> Result<NodeInfo, PelersError> {
        let id = self.expect(NODE_ID)?;
        if self.ty() == NODE_DSTART {
            let open = self.text(self.i);
            self.i += 1;
            let label = self.expect(STR)?;
            let close = self.expect(NODE_DEND)?;
            return Ok(NodeInfo {
                id,
                label: Some(label),
                type_str: Some(open + &close),
                directions: None,
            });
        }
        if self.ty() == BLOCK_ARROW_START {
            let open = self.text(self.i);
            self.i += 1;
            let label = self.expect(STR)?;
            let mut directions = vec![self.expect(DIR)?];
            while self.ty() == DIR {
                directions.push(self.text(self.i));
                self.i += 1;
            }
            let close = self.expect(BLOCK_ARROW_END)?;
            return Ok(NodeInfo {
                id,
                label: Some(label),
                type_str: Some(open + &close),
                directions: Some(directions),
            });
        }
        Ok(NodeInfo {
            id,
            label: None,
            type_str: None,
            directions: None,
        })
    }

    fn node_statement(&mut self) -> Result<Statement, PelersError> {
        let first = self.node()?;
        let size = if self.ty() == SIZE {
            let text = self.text(self.i);
            self.i += 1;
            js_parse_int(&text)
        } else {
            1.0
        };
        let mut from = Block {
            id: Some(first.id),
            label: first.label,
            kind: Some(type_str_2_type(first.type_str.as_deref()).to_owned()),
            directions: first.directions,
            width_in_columns: Some(size),
            ..Block::default()
        };
        let mut chain: Option<Vec<usize>> = None;
        while self.ty() == LINK || self.ty() == START_LINK {
            let mut label = String::new();
            let t = self.ty();
            self.i += 1;
            if t == START_LINK {
                self.expect(LINK_LABEL)?;
                label = self.expect(STR)?;
                self.expect(LINK)?;
            }
            let link = self.text(self.i - 1);
            let to = self.node()?;
            let arrow_type_end = edge_str_to_edge_data(&link);
            let arrow_type_start = edge_str_to_edge_start_data(&link);
            let thickness = edge_str_to_thickness(&link);
            let pattern = edge_str_to_pattern(&link);
            if chain.is_none() {
                let first_block = Block {
                    id: from.id.clone(),
                    label: from.label.clone(),
                    kind: from.kind.clone(),
                    directions: from.directions.clone(),
                    ..Block::default()
                };
                let at = self.db.add_block(first_block);
                chain = Some(vec![at]);
            }
            let list = chain.as_mut().expect("the chain was started above");
            let last_id = self.db.arena[*list.last().expect("a chain has a block")]
                .id
                .clone()
                .unwrap_or_default();
            let edge = Block {
                id: Some(format!("{}-{}", last_id, to.id)),
                start: Some(last_id.clone()),
                end: Some(to.id.clone()),
                label: Some(label),
                kind: Some("edge".to_owned()),
                thickness: Some(thickness.to_owned()),
                pattern: Some(pattern.to_owned()),
                directions: to.directions.clone(),
                arrow_type_end: Some(arrow_type_end.to_owned()),
                arrow_type_start: Some(arrow_type_start.to_owned()),
                ..Block::default()
            };
            let edge_at = self.db.add_block(edge);
            list.push(edge_at);
            from = Block {
                id: Some(to.id),
                label: to.label,
                kind: Some(type_str_2_type(to.type_str.as_deref()).to_owned()),
                directions: to.directions,
                ..Block::default()
            };
            let from_at = self.db.add_block(from.clone());
            list.push(from_at);
        }
        Ok(match chain {
            Some(list) => Statement::Many(list),
            None => Statement::One(self.db.add_block(from)),
        })
    }
}

pub fn parse_block(src: &str, db: &mut BlockDb) -> Result<(), PelersError> {
    let tokens = tokenize(src);
    let mut p = Parser {
        src,
        types: tokens.types,
        texts: tokens.texts,
        starts: tokens.starts,
        i: 0,
        db,
    };
    p.expect(KEY)?;
    let mut open: Vec<Frame> = Vec::new();
    let mut list: Vec<Statement> = Vec::new();
    loop {
        match p.ty() {
            NODE_ID => {
                let statement = p.node_statement()?;
                list.push(statement);
            }
            COLUMNS => {
                let columns = js_parse_int(&p.text(p.i));
                p.i += 1;
                let at = p.db.add_block(Block {
                    kind: Some("column-setting".to_owned()),
                    columns: Some(columns),
                    ..Block::default()
                });
                list.push(Statement::One(at));
            }
            SPACE_BLOCK => {
                let width = js_parse_int(&p.text(p.i));
                p.i += 1;
                let id = p.db.generate_id();
                let at = p.db.add_block(Block {
                    id: Some(id),
                    kind: Some("space".to_owned()),
                    label: Some(String::new()),
                    width: Some(width),
                    inner: Some(Vec::new()),
                    ..Block::default()
                });
                list.push(Statement::One(at));
            }
            KEY => {
                p.i += 1;
                open.push(Frame {
                    list: std::mem::take(&mut list),
                    header: None,
                });
            }
            ID_BLOCK => {
                p.i += 1;
                if p.ty() != NODE_ID {
                    return p.fail("'NODE_ID'");
                }
                let header = p.node_statement()?;
                open.push(Frame {
                    list: std::mem::take(&mut list),
                    header: Some(header),
                });
            }
            END_KW => {
                let frame = open.pop();
                if frame.is_none() || list.is_empty() {
                    return p.fail(STATEMENT_START);
                }
                let frame = frame.expect("checked above");
                p.i += 1;
                let id = p.db.generate_id();
                let children = close_document(std::mem::take(&mut list));
                list = frame.list;
                let block = match frame.header {
                    None => Block {
                        id: Some(id),
                        kind: Some("composite".to_owned()),
                        label: Some(String::new()),
                        inner: Some(children),
                        ..Block::default()
                    },
                    Some(Statement::One(header)) => Block {
                        kind: Some("composite".to_owned()),
                        inner: Some(children),
                        ..p.db.arena[header].clone()
                    },
                    Some(Statement::Many(_)) => Block {
                        kind: Some("composite".to_owned()),
                        inner: Some(children),
                        ..Block::default()
                    },
                };
                let at = p.db.add_block(block);
                list.push(Statement::One(at));
            }
            CLASSDEF => {
                p.i += 1;
                let id = trim(&p.expect(CLASSDEF_ID)?).to_owned();
                let css = trim(&p.expect(CLASSDEF_STYLEOPTS)?).to_owned();
                let at = p.db.add_block(Block {
                    kind: Some("classDef".to_owned()),
                    id: Some(id),
                    css: Some(css),
                    ..Block::default()
                });
                list.push(Statement::One(at));
            }
            CLASS => {
                p.i += 1;
                let id = trim(&p.expect(CLASSENTITY_IDS)?).to_owned();
                let style_class = trim(&p.expect(STYLECLASS)?).to_owned();
                let at = p.db.add_block(Block {
                    kind: Some("applyClass".to_owned()),
                    id: Some(id),
                    style_class: Some(style_class),
                    ..Block::default()
                });
                list.push(Statement::One(at));
            }
            STYLE => {
                p.i += 1;
                let id = trim(&p.expect(STYLE_ENTITY_IDS)?).to_owned();
                let styles_str = trim(&p.expect(STYLE_DEFINITION_DATA)?).to_owned();
                let at = p.db.add_block(Block {
                    kind: Some("applyStyles".to_owned()),
                    id: Some(id),
                    styles_str: Some(styles_str),
                    ..Block::default()
                });
                list.push(Statement::One(at));
            }
            EOF => {
                if !open.is_empty() {
                    return p.fail(&format!("{STATEMENT_START}, 'end'"));
                }
                if list.is_empty() {
                    return p.fail(STATEMENT_START);
                }
                let document = close_document(list);
                p.db.set_hierarchy(&document)?;
                return Ok(());
            }
            _ => {
                let expected = if list.is_empty() {
                    STATEMENT_START.to_owned()
                } else if !open.is_empty() {
                    format!("{STATEMENT_START}, 'end'")
                } else {
                    format!("{STATEMENT_START}, 'EOF'")
                };
                return p.fail(&expected);
            }
        }
    }
}
