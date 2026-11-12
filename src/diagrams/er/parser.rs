use super::lexer::tokenize;
use super::model::{Cardinality, DocItem, ErAttribute, ErDb, Identification, RelSpec};
use super::tokens::*;
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const DIRECTIONS: [&str; 4] = ["TB", "BT", "RL", "LR"];

const NAME: &str = "'UNICODE_TEXT', 'NUM', 'ENTITY_NAME', 'DECIMAL_NUM', 'ENTITY_ONE'";
const CARDS: &str = "'ZERO_OR_ONE', 'ZERO_OR_MORE', 'ONE_OR_MORE', 'ONLY_ONE', 'MD_PARENT'";
const LINE: &str = concat!(
    "'EOF', 'NEWLINE', 'title', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'SUBGRAPH', ",
    "'direction_tb', 'direction_bt', 'direction_rl', 'direction_lr', 'CLASSDEF', 'CLASS', 'STYLE', ",
    "'UNICODE_TEXT', 'NUM', 'ENTITY_NAME', 'DECIMAL_NUM', 'ENTITY_ONE'"
);

fn cardinality(t: i32) -> Option<Cardinality> {
    match t {
        ZERO_OR_ONE => Some(Cardinality::ZeroOrOne),
        ZERO_OR_MORE => Some(Cardinality::ZeroOrMore),
        ONE_OR_MORE => Some(Cardinality::OneOrMore),
        ONLY_ONE => Some(Cardinality::OnlyOne),
        MD_PARENT => Some(Cardinality::MdParent),
        _ => None,
    }
}

fn is_name(t: i32) -> bool {
    matches!(
        t,
        UNICODE_TEXT | ENTITY_NAME | NUM | DECIMAL_NUM | ENTITY_ONE
    )
}

fn is_style_component(t: i32) -> bool {
    matches!(t, STYLE_TEXT | NUM | COLON | BRKT)
}

fn unquote(text: &str) -> String {
    text[1..text.len() - 1].to_owned()
}

struct Frame {
    id: String,
    title: String,
    list: Vec<DocItem>,
}

struct Parser<'a, 'b> {
    src: &'a str,
    types: Vec<i32>,
    texts: Vec<String>,
    starts: Vec<usize>,
    i: usize,
    db: &'b mut ErDb,
}

impl Parser<'_, '_> {
    fn ty(&self) -> i32 {
        self.types.get(self.i).copied().unwrap_or(-1)
    }

    fn text(&self) -> String {
        self.texts.get(self.i).cloned().unwrap_or_default()
    }

    fn is_char(&self, ch: &str) -> bool {
        self.ty() == CHAR && self.texts.get(self.i).is_some_and(|t| t == ch)
    }

    fn fail<T>(&self, expected: &str) -> Result<T, PelersError> {
        let unit = self
            .starts
            .get(self.i)
            .copied()
            .unwrap_or_else(|| self.src.encode_utf16().count());
        let offset = utf16_to_byte(self.src, unit);
        let t = self.ty();
        if t == ERROR {
            return Err(lexical_error("er", self.src, offset));
        }
        let got = if t == CHAR {
            self.text()
        } else {
            usize::try_from(t)
                .ok()
                .and_then(|t| TOKEN_NAMES.get(t))
                .map_or_else(|| "undefined".to_owned(), |name| (*name).to_owned())
        };
        Err(syntax_error(
            "er",
            self.src,
            offset,
            format!("Expecting {expected}, got '{got}'"),
        ))
    }

    fn expect(&mut self, t: i32) -> Result<String, PelersError> {
        if self.ty() != t {
            return self.fail(&format!("'{}'", TOKEN_NAMES[t as usize]));
        }
        let text = self.text();
        self.i += 1;
        Ok(text)
    }

    fn separator(&mut self) -> Result<(), PelersError> {
        let t = self.ty();
        if t != NEWLINE && t != SEMI && t != EOF {
            return self.fail("'EOF', 'NEWLINE', 'SEMI'");
        }
        self.i += 1;
        Ok(())
    }

    fn entity_name(&mut self) -> Result<String, PelersError> {
        let t = self.ty();
        if !is_name(t) {
            return self.fail(NAME);
        }
        let text = self.text();
        self.i += 1;
        Ok(if t == ENTITY_NAME {
            unquote(&text)
        } else {
            text
        })
    }

    fn id_list(&mut self) -> Result<Vec<String>, PelersError> {
        let mut ids = Vec::new();
        loop {
            let t = self.ty();
            if t != UNICODE_TEXT && t != STYLE_TEXT {
                return self.fail("'UNICODE_TEXT', 'STYLE_TEXT'");
            }
            ids.push(self.text());
            self.i += 1;
            if self.ty() != COMMA {
                return Ok(ids);
            }
            self.i += 1;
        }
    }

    fn styles_opt(&mut self) -> Result<Vec<String>, PelersError> {
        let mut styles = Vec::new();
        loop {
            if !is_style_component(self.ty()) {
                return self.fail("'STYLE_TEXT', 'COLON', 'NUM', 'BRKT'");
            }
            let mut style = self.text();
            self.i += 1;
            while is_style_component(self.ty()) {
                style.push_str(&self.text());
                self.i += 1;
            }
            styles.push(style);
            if self.ty() != COMMA {
                return Ok(styles);
            }
            self.i += 1;
        }
    }

    fn rel_spec(&mut self) -> Result<RelSpec, PelersError> {
        let card_b = cardinality(self.ty()).expect("a cardinality starts a relationship");
        self.i += 1;
        let rel_type = match self.ty() {
            IDENTIFYING => Identification::Identifying,
            NON_IDENTIFYING => Identification::NonIdentifying,
            _ => return self.fail("'NON_IDENTIFYING', 'IDENTIFYING'"),
        };
        self.i += 1;
        let Some(card_a) = cardinality(self.ty()) else {
            return self.fail(CARDS);
        };
        self.i += 1;
        Ok(RelSpec {
            card_a,
            rel_type,
            card_b,
        })
    }

    fn role(&mut self) -> Result<String, PelersError> {
        let t = self.ty();
        if t != WORD && t != ENTITY_NAME && t != UNICODE_TEXT {
            return self.fail("'UNICODE_TEXT', 'ENTITY_NAME', 'WORD'");
        }
        let text = self.text();
        self.i += 1;
        Ok(if t == UNICODE_TEXT {
            text
        } else {
            unquote(&text)
        })
    }

    fn attributes(&mut self) -> Result<Vec<ErAttribute>, PelersError> {
        let mut list = Vec::new();
        loop {
            let mut attr_type = self.expect(ATTRIBUTE_WORD)?;
            if self.is_char("?") {
                attr_type.push_str(&self.text());
                self.i += 1;
            }
            if self.ty() != ATTRIBUTE_WORD {
                let expected = if attr_type.ends_with('?') {
                    "'ATTRIBUTE_WORD'"
                } else {
                    "'ATTRIBUTE_WORD', '?'"
                };
                return self.fail(expected);
            }
            let name = self.text();
            self.i += 1;
            let mut keys = Vec::new();
            if self.ty() == ATTRIBUTE_KEY {
                keys.push(self.text());
                self.i += 1;
                while self.is_char(",") {
                    self.i += 1;
                    keys.push(self.expect(ATTRIBUTE_KEY)?);
                }
            }
            let mut comment = String::new();
            if self.ty() == COMMENT {
                comment = unquote(&self.text());
                self.i += 1;
            }
            list.push(ErAttribute {
                attr_type,
                name,
                keys,
                comment,
            });
            if self.ty() == BLOCK_STOP {
                break;
            }
        }
        list.reverse();
        Ok(list)
    }

    fn block(&mut self) -> Result<Option<Vec<ErAttribute>>, PelersError> {
        if self.ty() != BLOCK_START {
            return Ok(None);
        }
        self.i += 1;
        if self.ty() == BLOCK_STOP {
            self.i += 1;
            return Ok(None);
        }
        let list = self.attributes()?;
        self.i += 1;
        Ok(Some(list))
    }

    fn entity_statement(&mut self) -> Result<Vec<String>, PelersError> {
        let a = self.entity_name()?;
        let mut t = self.ty();
        if t == SQS {
            self.i += 1;
            let alias = self.entity_name()?;
            self.expect(SQE)?;
            let mut classes = None;
            if self.ty() == STYLE_SEPARATOR {
                self.i += 1;
                classes = Some(self.id_list()?);
            }
            let attrs = self.block()?;
            self.db.add_entity(&a, &alias);
            if let Some(attrs) = attrs {
                self.db.add_attributes(&a, attrs);
            }
            if let Some(classes) = classes {
                self.db.set_class(std::slice::from_ref(&a), &classes);
            }
            return Ok(vec![a]);
        }
        let mut classes = None;
        if t == STYLE_SEPARATOR {
            self.i += 1;
            classes = Some(self.id_list()?);
            t = self.ty();
        }
        if cardinality(t).is_some() {
            let spec = self.rel_spec()?;
            let b = self.entity_name()?;
            let mut classes_b = None;
            if self.ty() == STYLE_SEPARATOR {
                self.i += 1;
                classes_b = Some(self.id_list()?);
            }
            self.expect(COLON)?;
            let label = self.role()?;
            self.db.add_entity(&a, "");
            self.db.add_entity(&b, "");
            self.db.add_relationship(&a, &label, &b, spec);
            if let Some(classes) = classes {
                self.db.set_class(std::slice::from_ref(&a), &classes);
            }
            if let Some(classes_b) = classes_b {
                self.db.set_class(std::slice::from_ref(&b), &classes_b);
            }
            return Ok(vec![a, b]);
        }
        let attrs = self.block()?;
        self.db.add_entity(&a, "");
        if let Some(attrs) = attrs {
            self.db.add_attributes(&a, attrs);
        }
        if let Some(classes) = classes {
            self.db.set_class(std::slice::from_ref(&a), &classes);
        }
        Ok(vec![a])
    }
}

pub fn parse_er(src: &str, db: &mut ErDb) -> Result<(), PelersError> {
    let tokens = tokenize(src);
    let mut p = Parser {
        src,
        types: tokens.types,
        texts: tokens.texts,
        starts: tokens.starts,
        i: 0,
        db,
    };
    p.expect(ER_DIAGRAM)?;
    let mut frames = vec![Frame {
        id: String::new(),
        title: String::new(),
        list: Vec::new(),
    }];
    let mut nested = false;

    loop {
        let t = p.ty();
        match t {
            NEWLINE => p.i += 1,
            EOF => {
                p.i += 1;
                if nested {
                    return p.fail(&format!("'END', {LINE}"));
                }
                return Ok(());
            }
            END_KW => {
                if !nested {
                    return p.fail(LINE);
                }
                p.i += 1;
                let frame = frames.pop().expect("a nested subgraph frame is open");
                let id = p.db.add_sub_graph(&frame.id, &frame.list, &frame.title);
                nested = frames.len() > 1;
                if nested {
                    let parent = frames.last_mut().expect("the parent frame exists");
                    parent.list.push(DocItem::Text(id));
                }
            }
            SUBGRAPH => {
                p.i += 1;
                let id = p.entity_name()?;
                let mut title = id.clone();
                if p.ty() == SQS {
                    p.i += 1;
                    title = p.entity_name()?;
                    while is_name(p.ty()) {
                        title.push(' ');
                        title.push_str(&p.entity_name()?);
                    }
                    p.expect(SQE)?;
                }
                p.separator()?;
                frames.push(Frame {
                    id,
                    title,
                    list: Vec::new(),
                });
                nested = true;
            }
            ACC_TITLE => {
                p.i += 1;
                let value = trim(&p.expect(ACC_TITLE_VALUE)?).to_owned();
                p.db.set_acc_title(&value);
                if nested {
                    push_text(&mut frames, value);
                }
            }
            ACC_DESCR => {
                p.i += 1;
                let value = trim(&p.expect(ACC_DESCR_VALUE)?).to_owned();
                p.db.set_acc_description(&value);
                if nested {
                    push_text(&mut frames, value);
                }
            }
            ACC_DESCR_MULTILINE_VALUE => {
                let value = trim(&p.text()).to_owned();
                p.i += 1;
                p.db.set_acc_description(&value);
                if nested {
                    push_text(&mut frames, value);
                }
            }
            DIRECTION_TB | DIRECTION_BT | DIRECTION_RL | DIRECTION_LR => {
                p.i += 1;
                let value = DIRECTIONS[(t - DIRECTION_TB) as usize].to_owned();
                if nested {
                    let frame = frames.last_mut().expect("a nested frame is open");
                    frame.list.push(DocItem::Dir(value));
                } else {
                    p.db.set_direction(&value);
                }
            }
            CLASSDEF => {
                let keyword = p.text();
                p.i += 1;
                let ids = p.id_list()?;
                let styles = p.styles_opt()?;
                p.separator()?;
                p.db.add_class(&ids, &styles);
                if nested {
                    push_text(&mut frames, keyword);
                }
            }
            CLASS => {
                let keyword = p.text();
                p.i += 1;
                let ids = p.id_list()?;
                let names = p.id_list()?;
                p.db.set_class(&ids, &names);
                if nested {
                    push_text(&mut frames, keyword);
                }
            }
            STYLE => {
                let keyword = p.text();
                p.i += 1;
                let ids = p.id_list()?;
                let styles = p.styles_opt()?;
                p.separator()?;
                p.db.add_css_styles(&ids, &styles);
                if nested {
                    push_text(&mut frames, keyword);
                }
            }
            _ => {
                if !is_name(t) {
                    return p.fail(&if nested {
                        format!("'END', {LINE}")
                    } else {
                        LINE.to_owned()
                    });
                }
                let names = p.entity_statement()?;
                if nested {
                    for name in names {
                        push_text(&mut frames, name);
                    }
                }
            }
        }
    }
}

fn push_text(frames: &mut [Frame], text: String) {
    if let Some(frame) = frames.last_mut() {
        frame.list.push(DocItem::Text(text));
    }
}
