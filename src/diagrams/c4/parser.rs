use super::lexer::tokenize;
use super::model::{Attr, C4Db, Named};
use super::tokens::*;
use crate::diagrams::common::spans::{Span, text as span_text};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const KINDS: [&str; 20] = [
    "person",
    "external_person",
    "system",
    "system_db",
    "system_queue",
    "external_system",
    "external_system_db",
    "external_system_queue",
    "container",
    "container_db",
    "container_queue",
    "external_container",
    "external_container_db",
    "external_container_queue",
    "component",
    "component_db",
    "component_queue",
    "external_component",
    "external_component_db",
    "external_component_queue",
];
const REL_TYPES: [&str; 8] = [
    "rel", "birel", "rel_u", "rel_d", "rel_l", "rel_r", "rel_b", "rel",
];
const NODE_TYPES: [&str; 3] = ["node", "nodeL", "nodeR"];

const HEADERS: &str = "'direction_tb', 'direction_bt', 'direction_rl', 'direction_lr', 'C4_CONTEXT', 'C4_CONTAINER', 'C4_COMPONENT', 'C4_DYNAMIC', 'C4_DEPLOYMENT'";
const ATTRIBUTES: &str = "'STR', 'STR_KEY', 'ATTRIBUTE', 'ATTRIBUTE_EMPTY'";

const NONE: usize = usize::MAX;

#[derive(Clone, Copy, PartialEq)]
enum P {
    Item,
    Diagram,
    ItemOrEnd,
    AfterOther,
    AfterDiagram,
}

fn is_other(t: usize) -> bool {
    matches!(
        t,
        TITLE | ACC_DESCRIPTION | ACC_TITLE | ACC_DESCR | ACC_DESCR_MULTILINE_VALUE
    )
}

fn is_diagram(t: usize) -> bool {
    (PERSON..=UPDATE_LAYOUT_CONFIG).contains(&t)
}

struct Parser<'a, 'b> {
    src: &'a str,
    types: Vec<usize>,
    spans: Vec<Span>,
    starts: Vec<usize>,
    i: usize,
    db: &'b mut C4Db,
}

impl Parser<'_, '_> {
    fn ty(&self, at: usize) -> usize {
        self.types.get(at).copied().unwrap_or(NONE)
    }

    fn text(&self, at: usize) -> &str {
        self.spans
            .get(at)
            .map_or("", |span| span_text(self.src, span))
    }

    fn fail<T>(&self, expected: &str) -> Result<T, PelersError> {
        let unit = self
            .starts
            .get(self.i)
            .copied()
            .unwrap_or_else(|| self.src.encode_utf16().count());
        let offset = utf16_to_byte(self.src, unit);
        let t = self.ty(self.i);
        if t == ERROR {
            return Err(lexical_error("c4", self.src, offset));
        }
        let got = TOKEN_NAMES.get(t).copied().unwrap_or("undefined");
        Err(syntax_error(
            "c4",
            self.src,
            offset,
            format!("Expecting {expected}, got '{got}'"),
        ))
    }

    fn expect(&mut self, t: usize) -> Result<(), PelersError> {
        if self.ty(self.i) != t {
            return self.fail(&format!("'{}'", TOKEN_NAMES[t]));
        }
        self.i += 1;
        Ok(())
    }

    fn attributes(&mut self) -> Result<Vec<Attr>, PelersError> {
        let mut attrs = Vec::new();
        loop {
            let t = self.ty(self.i);
            if t == STR {
                attrs.push(Attr::Str(trim(self.text(self.i)).to_owned()));
                self.i += 1;
            } else if t == ATTRIBUTE_EMPTY {
                attrs.push(Attr::Str(String::new()));
                self.i += 1;
            } else if t == STR_KEY {
                self.i += 1;
                if self.ty(self.i) != STR_VALUE {
                    return self.fail("'STR_VALUE'");
                }
                attrs.push(Attr::Named(Named {
                    key: trim(self.text(self.i - 1)).to_owned(),
                    value: trim(self.text(self.i)).to_owned(),
                }));
                self.i += 1;
            } else {
                break;
            }
        }
        if attrs.is_empty() {
            return self.fail(ATTRIBUTES);
        }
        Ok(attrs)
    }

    fn other(&mut self, t: usize) -> Result<(), PelersError> {
        if t == TITLE {
            let text = self.text(self.i).to_owned();
            self.i += 1;
            self.db.set_title(&text.chars().skip(6).collect::<String>());
        } else if t == ACC_DESCRIPTION {
            let text = self.text(self.i).to_owned();
            self.i += 1;
            self.db
                .set_acc_description(&text.chars().skip(15).collect::<String>());
        } else if t == ACC_DESCR_MULTILINE_VALUE {
            let text = trim(self.text(self.i)).to_owned();
            self.i += 1;
            self.db.set_acc_description(&text);
        } else {
            self.i += 1;
            if t == ACC_TITLE {
                if self.ty(self.i) != ACC_TITLE_VALUE {
                    return self.fail("'acc_title_value'");
                }
                let text = trim(self.text(self.i)).to_owned();
                self.i += 1;
                self.db.set_title(&text);
            } else {
                if self.ty(self.i) != ACC_DESCR_VALUE {
                    return self.fail("'acc_descr_value'");
                }
                let text = trim(self.text(self.i)).to_owned();
                self.i += 1;
                self.db.set_acc_description(&text);
            }
        }
        Ok(())
    }

    fn diagram(&mut self, t: usize) -> Result<bool, PelersError> {
        self.i += 1;
        let mut attrs = self.attributes()?;
        if t <= SYSTEM_EXT_QUEUE {
            self.db.add_person_or_system(KINDS[t - PERSON], &attrs);
        } else if t <= CONTAINER_EXT_QUEUE {
            self.db.add_container(KINDS[t - PERSON], &attrs);
        } else if t <= COMPONENT_EXT_QUEUE {
            self.db.add_component(KINDS[t - PERSON], &attrs);
        } else if t <= NODE_R {
            let sort = match t {
                ENTERPRISE_BOUNDARY => Some("ENTERPRISE"),
                SYSTEM_BOUNDARY => Some("SYSTEM"),
                CONTAINER_BOUNDARY => Some("CONTAINER"),
                _ => None,
            };
            if let Some(sort) = sort {
                let at = attrs.len().min(2);
                attrs.insert(at, Attr::Str(sort.to_owned()));
            }
            if t >= NODE {
                self.db.add_deployment_node(NODE_TYPES[t - NODE], &attrs);
            } else if t == CONTAINER_BOUNDARY {
                self.db.add_container_boundary(&attrs);
            } else {
                self.db.add_person_or_system_boundary(&attrs, "system");
            }
            if self.ty(self.i) == LBRACE {
                self.i += 1;
                self.expect(NEWLINE)?;
            } else if self.ty(self.i) == NEWLINE {
                self.i += 1;
                self.expect(LBRACE)?;
                if self.ty(self.i) == NEWLINE {
                    self.i += 1;
                }
            } else {
                return self.fail("'NEWLINE', 'LBRACE'");
            }
            return Ok(true);
        } else if t <= REL_INDEX {
            if t == REL_INDEX && !attrs.is_empty() {
                attrs.remove(0);
            }
            self.db.add_rel(REL_TYPES[t - REL], &attrs);
        } else if t == UPDATE_EL_STYLE {
            self.db.update_el_style(&attrs);
        } else if t == UPDATE_REL_STYLE {
            self.db.update_rel_style(&attrs);
        } else {
            self.db.update_layout_config(&attrs);
        }
        Ok(false)
    }
}

pub fn parse_c4(src: &str, db: &mut C4Db) -> Result<(), PelersError> {
    let tokens = tokenize(src);
    let mut p = Parser {
        src,
        types: tokens.types,
        spans: tokens.spans,
        starts: tokens.starts,
        i: 0,
        db,
    };
    let header = p.ty(p.i);
    if !(C4_CONTEXT..=C4_DEPLOYMENT).contains(&header) {
        return p.fail(HEADERS);
    }
    p.i += 1;
    p.expect(NEWLINE)?;

    let mut depth = 0usize;
    let mut state = P::Item;
    loop {
        let t = p.ty(p.i);
        if state == P::AfterOther || state == P::AfterDiagram {
            if t == NEWLINE {
                p.i += 1;
                state = P::ItemOrEnd;
                continue;
            }
            if state == P::AfterOther && is_diagram(t) {
                state = P::Diagram;
                continue;
            }
        } else if is_diagram(t) {
            if p.diagram(t)? {
                depth += 1;
                state = P::Diagram;
            } else {
                state = P::AfterDiagram;
            }
            continue;
        } else if is_other(t) && state != P::Diagram {
            p.other(t)?;
            state = P::AfterOther;
            continue;
        } else if state != P::ItemOrEnd {
            return p.fail(
                "'title', 'accDescription', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'PERSON', 'REL'",
            );
        }

        if depth > 0 {
            if t != RBRACE {
                return p.fail("'NEWLINE', 'RBRACE'");
            }
            p.i += 1;
            p.db.pop_boundary_parse_stack();
            depth -= 1;
            state = P::AfterDiagram;
        } else {
            if t != EOF {
                return p.fail("'NEWLINE', 'EOF'");
            }
            p.i += 1;
            p.expect(END)?;
            let header = p.text(0).to_owned();
            p.db.set_c4_type(&header);
            return Ok(());
        }
    }
}
