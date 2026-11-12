use super::db::SeqDb;
use super::lexer::tokenize;
use super::tokens::*;
use super::types::{Item, SeqText, Statement, Who, linetype, placement};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::trim;
use crate::util::chars::utf16_to_byte;

const SIGNAL_TOKENS: &str = "'SOLID_OPEN_ARROW', 'DOTTED_OPEN_ARROW', 'SOLID_ARROW', 'DOTTED_ARROW', 'SOLID_CROSS', 'DOTTED_CROSS', 'SOLID_POINT', 'DOTTED_POINT'";

fn signal_type(t: usize) -> Option<i32> {
    Some(match t {
        SOLID_OPEN_ARROW => linetype::SOLID_OPEN,
        DOTTED_OPEN_ARROW => linetype::DOTTED_OPEN,
        SOLID_ARROW => linetype::SOLID,
        DOTTED_ARROW => linetype::DOTTED,
        BIDIRECTIONAL_SOLID_ARROW => linetype::BIDIRECTIONAL_SOLID,
        BIDIRECTIONAL_DOTTED_ARROW => linetype::BIDIRECTIONAL_DOTTED,
        SOLID_CROSS => linetype::SOLID_CROSS,
        DOTTED_CROSS => linetype::DOTTED_CROSS,
        SOLID_POINT => linetype::SOLID_POINT,
        DOTTED_POINT => linetype::DOTTED_POINT,
        SOLID_ARROW_TOP => linetype::SOLID_TOP,
        SOLID_ARROW_BOTTOM => linetype::SOLID_BOTTOM,
        STICK_ARROW_TOP => linetype::STICK_TOP,
        STICK_ARROW_BOTTOM => linetype::STICK_BOTTOM,
        SOLID_ARROW_TOP_DOTTED => linetype::SOLID_TOP_DOTTED,
        SOLID_ARROW_BOTTOM_DOTTED => linetype::SOLID_BOTTOM_DOTTED,
        STICK_ARROW_TOP_DOTTED => linetype::STICK_TOP_DOTTED,
        STICK_ARROW_BOTTOM_DOTTED => linetype::STICK_BOTTOM_DOTTED,
        SOLID_ARROW_TOP_REVERSE => linetype::SOLID_ARROW_TOP_REVERSE,
        SOLID_ARROW_BOTTOM_REVERSE => linetype::SOLID_ARROW_BOTTOM_REVERSE,
        STICK_ARROW_TOP_REVERSE => linetype::STICK_ARROW_TOP_REVERSE,
        STICK_ARROW_BOTTOM_REVERSE => linetype::STICK_ARROW_BOTTOM_REVERSE,
        SOLID_ARROW_TOP_REVERSE_DOTTED => linetype::SOLID_ARROW_TOP_REVERSE_DOTTED,
        SOLID_ARROW_BOTTOM_REVERSE_DOTTED => linetype::SOLID_ARROW_BOTTOM_REVERSE_DOTTED,
        STICK_ARROW_TOP_REVERSE_DOTTED => linetype::STICK_ARROW_TOP_REVERSE_DOTTED,
        STICK_ARROW_BOTTOM_REVERSE_DOTTED => linetype::STICK_ARROW_BOTTOM_REVERSE_DOTTED,
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum K {
    Top,
    Box,
    Loop,
    Rect,
    Opt,
    Break,
    Alt,
    Par,
    Critical,
}

#[derive(Clone, Debug)]
struct Frame {
    kind: K,
    text: String,
    start_type: i32,
    list: Vec<Item>,
    sections: Option<Vec<(Vec<Item>, String)>>,
}

fn block_kind(t: usize) -> Option<K> {
    Some(match t {
        LOOP => K::Loop,
        RECT => K::Rect,
        OPT => K::Opt,
        BREAK => K::Break,
        ALT => K::Alt,
        PAR | PAR_OVER => K::Par,
        CRITICAL => K::Critical,
        BOX => K::Box,
        _ => return None,
    })
}

fn after_first(text: &str, n: usize) -> String {
    text.chars().skip(n).collect()
}

struct Parser<'a, 'b> {
    src: &'a str,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
    i: usize,
    db: &'b mut SeqDb,
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
            return Err(lexical_error("sequence", self.src, offset));
        }
        let name = TOKEN_NAMES.get(t).copied().unwrap_or("undefined");
        Err(syntax_error(
            "sequence",
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

    fn actor(&mut self) -> Result<Statement, PelersError> {
        let t = self.ty(self.i);
        if t != ACTOR && t != LINK {
            return self.fail("'ACTOR'");
        }
        let name = self.text(self.i);
        self.i += 1;
        Ok(Statement {
            type_: "addParticipant".to_owned(),
            actor: Some(Who::One(name)),
            ..Statement::default()
        })
    }

    fn text2(&mut self) -> Result<SeqText, PelersError> {
        let raw = self.expect(TXT)?;
        Ok(self.db.parse_message(&after_first(trim(&raw), 1)))
    }

    fn participant(&mut self) -> Result<Statement, PelersError> {
        let t = self.ty(self.i);
        self.i += 1;
        if t == DESTROY {
            let mut node = self.actor()?;
            self.expect(NEWLINE)?;
            node.type_ = "destroyParticipant".to_owned();
            return Ok(node);
        }
        let mut node = self.actor()?;
        if self.ty(self.i) == CONFIG_START && self.ty(self.i - 1) == ACTOR {
            self.i += 1;
            let content = self.expect(CONFIG_CONTENT)?;
            self.expect(CONFIG_END)?;
            node.config = Some(trim(&content).to_owned());
        }
        node.draw = Some(
            if t == PARTICIPANT {
                "participant"
            } else {
                "actor"
            }
            .to_owned(),
        );
        if self.ty(self.i) == AS {
            self.i += 1;
            let alias = self.expect(RESTOFLINE)?;
            self.expect(NEWLINE)?;
            node.description = Some(self.db.parse_message(&alias));
        } else if self.ty(self.i) == NEWLINE {
            self.i += 1;
        } else {
            return self.fail("'AS', 'NEWLINE'");
        }
        Ok(node)
    }

    fn signal(&mut self, from: Statement) -> Result<Item, PelersError> {
        let mut central_from = false;
        if self.ty(self.i) == CENTRAL {
            central_from = true;
            self.i += 1;
        }
        let Some(signal) = signal_type(self.ty(self.i)) else {
            return self.fail(SIGNAL_TOKENS);
        };
        self.i += 1;
        let mark = self.ty(self.i);
        if mark == CENTRAL || (!central_from && (mark == PLUS || mark == MINUS)) {
            self.i += 1;
        }
        let to = self.actor()?;
        let msg = self.text2()?;
        let mut message = Statement {
            type_: "addMessage".to_owned(),
            from: from.actor.as_ref().and_then(who_one),
            to: to.actor.as_ref().and_then(who_one),
            signal_type: Some(signal),
            msg: Some(msg),
            ..Statement::default()
        };
        let mut out = vec![
            Item::Statement(Box::new(from.clone())),
            Item::Statement(Box::new(to.clone())),
        ];
        if central_from {
            let dual = mark == CENTRAL;
            message.activate = Some(dual);
            message.central_connection = Some(if dual {
                linetype::CENTRAL_CONNECTION_DUAL
            } else {
                linetype::CENTRAL_CONNECTION_REVERSE
            });
            if dual {
                out.push(Item::Statement(Box::new(Statement {
                    type_: "centralConnection".to_owned(),
                    signal_type: Some(linetype::CENTRAL_CONNECTION),
                    actor: to.actor.clone(),
                    ..Statement::default()
                })));
            }
            out.push(Item::Statement(Box::new(Statement {
                type_: "centralConnectionReverse".to_owned(),
                signal_type: Some(linetype::CENTRAL_CONNECTION_REVERSE),
                actor: from.actor.clone(),
                ..Statement::default()
            })));
        } else if mark == PLUS {
            message.activate = Some(true);
            out.push(Item::Statement(Box::new(Statement {
                type_: "activeStart".to_owned(),
                signal_type: Some(linetype::ACTIVE_START),
                actor: to.actor.clone(),
                ..Statement::default()
            })));
        } else if mark == MINUS {
            out.push(Item::Statement(Box::new(Statement {
                type_: "activeEnd".to_owned(),
                signal_type: Some(linetype::ACTIVE_END),
                actor: from.actor.clone(),
                ..Statement::default()
            })));
        } else if mark == CENTRAL {
            message.activate = Some(true);
            message.central_connection = Some(linetype::CENTRAL_CONNECTION);
            out.push(Item::Statement(Box::new(Statement {
                type_: "centralConnection".to_owned(),
                signal_type: Some(linetype::CENTRAL_CONNECTION),
                actor: to.actor.clone(),
                ..Statement::default()
            })));
        }
        out.insert(2, Item::Statement(Box::new(message)));
        Ok(Item::List(out))
    }

    fn menu(&mut self, type_: &str) -> Result<Item, PelersError> {
        self.i += 1;
        let who = self.actor()?;
        let text = self.text2()?;
        Ok(Item::List(vec![
            Item::Statement(Box::new(who.clone())),
            Item::Statement(Box::new(Statement {
                type_: type_.to_owned(),
                actor: who.actor,
                text: Some(text),
                ..Statement::default()
            })),
        ]))
    }

    fn note(&mut self) -> Result<Item, PelersError> {
        self.i += 1;
        let t = self.ty(self.i);
        if t == OVER {
            self.i += 1;
            let first = self.actor()?;
            let first_name = first.actor.as_ref().and_then(who_one).unwrap_or_default();
            let (who, ids) = if self.ty(self.i) == COMMA {
                self.i += 1;
                let second = self.actor()?;
                let second_name = second.actor.as_ref().and_then(who_one).unwrap_or_default();
                (
                    Item::List(vec![
                        Item::Statement(Box::new(first)),
                        Item::Statement(Box::new(second)),
                    ]),
                    vec![first_name, second_name],
                )
            } else {
                (
                    Item::Statement(Box::new(first)),
                    vec![first_name.clone(), first_name],
                )
            };
            let text = self.text2()?;
            return Ok(Item::List(vec![
                who,
                Item::Statement(Box::new(Statement {
                    type_: "addNote".to_owned(),
                    placement: Some(placement::OVER),
                    actor: Some(Who::Many(ids)),
                    text: Some(text),
                    ..Statement::default()
                })),
            ]));
        }
        if t != LEFT_OF && t != RIGHT_OF {
            return self.fail("'over', 'left_of', 'right_of'");
        }
        self.i += 1;
        let who = self.actor()?;
        let place = if t == LEFT_OF {
            placement::LEFTOF
        } else {
            placement::RIGHTOF
        };
        let name = who.actor.clone();
        let text = self.text2()?;
        Ok(Item::List(vec![
            Item::Statement(Box::new(who)),
            Item::Statement(Box::new(Statement {
                type_: "addNote".to_owned(),
                placement: Some(place),
                actor: name,
                text: Some(text),
                ..Statement::default()
            })),
        ]))
    }

    fn autonumber(&mut self) -> Result<Item, PelersError> {
        self.i += 1;
        let mut node = Statement {
            type_: "sequenceIndex".to_owned(),
            sequence_visible: Some(true),
            signal_type: Some(linetype::AUTONUMBER),
            ..Statement::default()
        };
        if self.ty(self.i) == NUM {
            let start = self.text(self.i);
            self.i += 1;
            node.sequence_index = start.parse::<f64>().ok();
            if self.ty(self.i) == NUM {
                let step = self.text(self.i);
                self.i += 1;
                node.sequence_index_step = step.parse::<f64>().ok();
            } else {
                node.sequence_index_step = Some(1.0);
            }
        } else if self.ty(self.i) == OFF {
            self.i += 1;
            node.sequence_visible = Some(false);
        }
        if self.ty(self.i) != NEWLINE {
            return self.fail("'NEWLINE', 'NUM', 'off'");
        }
        self.i += 1;
        Ok(Item::Statement(Box::new(node)))
    }

    fn fold(&self, frame: &Frame, divider: &str, key: &str, signal: i32) -> Vec<Item> {
        let mut list = frame.list.clone();
        if let Some(sections) = &frame.sections {
            for (section_list, section_text) in sections.iter().rev() {
                let mut head = Statement {
                    type_: divider.to_owned(),
                    signal_type: Some(signal),
                    ..Statement::default()
                };
                let parsed = Some(self.db.parse_message(section_text));
                match key {
                    "altText" => head.alt_text = parsed,
                    "parText" => head.par_text = parsed,
                    _ => head.option_text = parsed,
                }
                let mut next = section_list.clone();
                next.push(Item::Statement(Box::new(head)));
                next.push(Item::List(list));
                list = next;
            }
        }
        list
    }

    fn close(&self, frame: Frame) -> Vec<Item> {
        let text = frame.text.clone();
        let mut list = match frame.kind {
            K::Alt => self.fold(&frame, "else", "altText", linetype::ALT_ELSE),
            K::Par => self.fold(&frame, "and", "parText", linetype::PAR_AND),
            K::Critical => self.fold(&frame, "option", "optionText", linetype::CRITICAL_OPTION),
            _ => frame.list.clone(),
        };
        let (head, tail) = match frame.kind {
            K::Box => (
                Statement {
                    type_: "boxStart".to_owned(),
                    box_data: Some(self.db.parse_box_data(&text)),
                    ..Statement::default()
                },
                Statement {
                    type_: "boxEnd".to_owned(),
                    ..Statement::default()
                },
            ),
            K::Loop => (
                Statement {
                    type_: "loopStart".to_owned(),
                    loop_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::LOOP_START),
                    ..Statement::default()
                },
                Statement {
                    type_: "loopEnd".to_owned(),
                    signal_type: Some(linetype::LOOP_END),
                    ..Statement::default()
                },
            ),
            K::Rect => (
                Statement {
                    type_: "rectStart".to_owned(),
                    color: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::RECT_START),
                    ..Statement::default()
                },
                Statement {
                    type_: "rectEnd".to_owned(),
                    color: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::RECT_END),
                    ..Statement::default()
                },
            ),
            K::Opt => (
                Statement {
                    type_: "optStart".to_owned(),
                    opt_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::OPT_START),
                    ..Statement::default()
                },
                Statement {
                    type_: "optEnd".to_owned(),
                    opt_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::OPT_END),
                    ..Statement::default()
                },
            ),
            K::Break => (
                Statement {
                    type_: "breakStart".to_owned(),
                    break_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::BREAK_START),
                    ..Statement::default()
                },
                Statement {
                    type_: "breakEnd".to_owned(),
                    opt_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::BREAK_END),
                    ..Statement::default()
                },
            ),
            K::Alt => (
                Statement {
                    type_: "altStart".to_owned(),
                    alt_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::ALT_START),
                    ..Statement::default()
                },
                Statement {
                    type_: "altEnd".to_owned(),
                    signal_type: Some(linetype::ALT_END),
                    ..Statement::default()
                },
            ),
            K::Par => (
                Statement {
                    type_: "parStart".to_owned(),
                    par_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(frame.start_type),
                    ..Statement::default()
                },
                Statement {
                    type_: "parEnd".to_owned(),
                    signal_type: Some(linetype::PAR_END),
                    ..Statement::default()
                },
            ),
            K::Critical | K::Top => (
                Statement {
                    type_: "criticalStart".to_owned(),
                    critical_text: Some(self.db.parse_message(&text)),
                    signal_type: Some(linetype::CRITICAL_START),
                    ..Statement::default()
                },
                Statement {
                    type_: "criticalEnd".to_owned(),
                    signal_type: Some(linetype::CRITICAL_END),
                    ..Statement::default()
                },
            ),
        };
        if list.is_empty() {
            list.push(Item::Statement(Box::new(head)));
        } else {
            list[0] = Item::Statement(Box::new(head));
        }
        list.push(Item::Statement(Box::new(tail)));
        list
    }

    fn run(&mut self) -> Result<(), PelersError> {
        while self.ty(self.i) == NEWLINE {
            self.i += 1;
        }
        self.expect(SD)?;

        let mut stack: Vec<Frame> = Vec::new();
        let mut frame = Frame {
            kind: K::Top,
            text: String::new(),
            start_type: 0,
            list: Vec::new(),
            sections: None,
        };

        loop {
            let t = self.ty(self.i);
            if frame.kind == K::Box {
                if t == PARTICIPANT || t == PARTICIPANT_ACTOR || t == DESTROY {
                    let node = self.participant()?;
                    frame.list.push(Item::Statement(Box::new(node)));
                } else if t == NEWLINE {
                    frame.list.push(Item::List(Vec::new()));
                    self.i += 1;
                } else if t == END_KW {
                    self.i += 1;
                    let done = self.close(frame);
                    frame = stack.pop().unwrap_or(Frame {
                        kind: K::Top,
                        text: String::new(),
                        start_type: 0,
                        list: Vec::new(),
                        sections: None,
                    });
                    frame.list.push(Item::List(done));
                } else {
                    return self
                        .fail("'end', 'NEWLINE', 'participant', 'participant_actor', 'destroy'");
                }
                continue;
            }
            match t {
                NEWLINE | INVALID => {
                    frame.list.push(Item::List(Vec::new()));
                    self.i += 1;
                }
                PARTICIPANT | PARTICIPANT_ACTOR | DESTROY => {
                    let node = self.participant()?;
                    frame.list.push(Item::Statement(Box::new(node)));
                }
                CREATE => {
                    self.i += 1;
                    let next = self.ty(self.i);
                    if next != PARTICIPANT && next != PARTICIPANT_ACTOR && next != DESTROY {
                        return self.fail("'participant', 'participant_actor', 'destroy'");
                    }
                    let mut node = self.participant()?;
                    node.type_ = "createParticipant".to_owned();
                    frame.list.push(Item::Statement(Box::new(node)));
                }
                ACTOR => {
                    let from = self.actor()?;
                    let item = self.signal(from)?;
                    frame.list.push(item);
                    self.expect(NEWLINE)?;
                }
                LINK => {
                    let next = self.ty(self.i + 1);
                    let item = if next == ACTOR || next == LINK {
                        self.menu("addALink")?
                    } else {
                        let from = self.actor()?;
                        self.signal(from)?
                    };
                    frame.list.push(item);
                    self.expect(NEWLINE)?;
                }
                LINKS => {
                    let item = self.menu("addLinks")?;
                    frame.list.push(item);
                    self.expect(NEWLINE)?;
                }
                PROPERTIES => {
                    let item = self.menu("addProperties")?;
                    frame.list.push(item);
                    self.expect(NEWLINE)?;
                }
                DETAILS => {
                    let item = self.menu("addDetails")?;
                    frame.list.push(item);
                    self.expect(NEWLINE)?;
                }
                NOTE => {
                    let item = self.note()?;
                    frame.list.push(item);
                    self.expect(NEWLINE)?;
                }
                AUTONUMBER => {
                    let item = self.autonumber()?;
                    frame.list.push(item);
                }
                ACTIVATE | DEACTIVATE => {
                    self.i += 1;
                    let who = self.actor()?;
                    self.expect(NEWLINE)?;
                    let stmt = if t == ACTIVATE {
                        Statement {
                            type_: "activeStart".to_owned(),
                            signal_type: Some(linetype::ACTIVE_START),
                            actor: who.actor,
                            ..Statement::default()
                        }
                    } else {
                        Statement {
                            type_: "activeEnd".to_owned(),
                            signal_type: Some(linetype::ACTIVE_END),
                            actor: who.actor,
                            ..Statement::default()
                        }
                    };
                    frame.list.push(Item::Statement(Box::new(stmt)));
                }
                TITLE | LEGACY_TITLE => {
                    let raw = self.text(self.i);
                    self.i += 1;
                    let title = after_first(&raw, if t == TITLE { 6 } else { 7 });
                    self.db.set_diagram_title(&title);
                    frame.list.push(Item::Text(title));
                }
                ACC_TITLE => {
                    self.i += 1;
                    let value = trim(&self.expect(ACC_TITLE_VALUE)?).to_owned();
                    self.db.set_acc_title(&value);
                    frame.list.push(Item::Text(value));
                }
                ACC_DESCR => {
                    self.i += 1;
                    let value = trim(&self.expect(ACC_DESCR_VALUE)?).to_owned();
                    self.db.set_acc_description(&value);
                    frame.list.push(Item::Text(value));
                }
                ACC_DESCR_MULTILINE_VALUE => {
                    let value = trim(&self.text(self.i)).to_owned();
                    self.i += 1;
                    self.db.set_acc_description(&value);
                    frame.list.push(Item::Text(value));
                }
                LOOP | RECT | OPT | BREAK | ALT | PAR | PAR_OVER | CRITICAL | BOX => {
                    self.i += 1;
                    let text = self.expect(RESTOFLINE)?;
                    let kind = block_kind(t).unwrap_or(K::Top);
                    let next = Frame {
                        kind,
                        text,
                        start_type: if t == PAR_OVER {
                            linetype::PAR_OVER_START
                        } else {
                            linetype::PAR_START
                        },
                        list: vec![Item::List(Vec::new())],
                        sections: None,
                    };
                    stack.push(std::mem::replace(&mut frame, next));
                }
                ELSE | AND | OPTION => {
                    let expected = match t {
                        ELSE => K::Alt,
                        AND => K::Par,
                        _ => K::Critical,
                    };
                    if frame.kind != expected {
                        return self.fail("'end'");
                    }
                    self.i += 1;
                    let text = self.expect(RESTOFLINE)?;
                    let list = std::mem::take(&mut frame.list);
                    frame
                        .sections
                        .get_or_insert_with(Vec::new)
                        .push((list, text));
                }
                END_KW => {
                    if frame.kind == K::Top {
                        return self.fail("'NEWLINE', 'EOF'");
                    }
                    self.i += 1;
                    let done = self.close(frame);
                    frame = stack.pop().unwrap_or(Frame {
                        kind: K::Top,
                        text: String::new(),
                        start_type: 0,
                        list: Vec::new(),
                        sections: None,
                    });
                    frame.list.push(Item::List(done));
                }
                END => {
                    if frame.kind != K::Top {
                        return self.fail("'end'");
                    }
                    let root = Item::List(std::mem::take(&mut frame.list));
                    return self.db.apply(&root);
                }
                _ => return self.fail("'NEWLINE', 'ACTOR', 'participant', 'note', 'end'"),
            }
        }
    }
}

fn who_one(who: &Who) -> Option<String> {
    match who {
        Who::One(name) => Some(name.clone()),
        Who::Many(_) => None,
    }
}

pub fn parse_sequence(src: &str, db: &mut SeqDb) -> Result<(), PelersError> {
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
