use super::db::{
    REQUIREMENT_TYPE_DESIGN, REQUIREMENT_TYPE_FUNCTIONAL, REQUIREMENT_TYPE_INTERFACE,
    REQUIREMENT_TYPE_PERFORMANCE, REQUIREMENT_TYPE_PHYSICAL, REQUIREMENT_TYPE_REQUIREMENT,
    RequirementDb,
};
use super::lexer::tokenize;
use super::tokens::{T, token_name};
use crate::diagrams::common::spans::{Span, Spans, text as span_text};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const VALUES: [&str; 21] = [
    REQUIREMENT_TYPE_REQUIREMENT,
    REQUIREMENT_TYPE_FUNCTIONAL,
    REQUIREMENT_TYPE_INTERFACE,
    REQUIREMENT_TYPE_PERFORMANCE,
    REQUIREMENT_TYPE_PHYSICAL,
    REQUIREMENT_TYPE_DESIGN,
    "Low",
    "Medium",
    "High",
    "Analysis",
    "Demonstration",
    "Inspection",
    "Test",
    "",
    "contains",
    "copies",
    "derives",
    "satisfies",
    "verifies",
    "refines",
    "traces",
];

fn is_style_part(t: i32) -> bool {
    t == T::Alpha as i32
        || t == T::Colon as i32
        || t == T::Brkt as i32
        || t == T::Minus as i32
        || t == T::Semicolon as i32
}

struct Parser<'a> {
    src: &'a str,
    units: Vec<u16>,
    types: Vec<i32>,
    spans: Vec<Span>,
    starts: Vec<usize>,
    i: usize,
    fields: Vec<i32>,
    values: Vec<String>,
}

impl Parser<'_> {
    fn text(&self, at: usize) -> &str {
        self.spans
            .get(at)
            .map_or("", |span| span_text(self.src, span))
    }
}

impl Parser<'_> {
    fn kind(&self) -> i32 {
        self.types[self.i]
    }

    fn error_at(&self, offset: usize, detail: Option<String>) -> PelersError {
        let byte = utf16_to_byte(self.src, offset.min(self.units.len()));
        match detail {
            None => lexical_error("requirement", self.src, byte),
            Some(detail) => syntax_error("requirement", self.src, byte, detail),
        }
    }

    fn fail(&self, from: i32, to: i32, others: &[i32]) -> PelersError {
        let t = self.kind();
        if t == T::Error as i32 {
            return self.error_at(self.starts[self.i], None);
        }
        let mut expected: Vec<i32> = (from..=to).collect();
        expected.extend_from_slice(others);
        let list = expected
            .iter()
            .map(|&k| format!("'{}'", token_name(k)))
            .collect::<Vec<_>>()
            .join(", ");
        self.error_at(
            self.starts[self.i],
            Some(format!("Expecting {list}, got '{}'", token_name(t))),
        )
    }

    fn expect(&mut self, kind: i32) -> Result<String, PelersError> {
        if self.kind() != kind {
            return Err(self.fail(kind, kind, &[]));
        }
        let text = self.text(self.i).to_owned();
        self.i += 1;
        Ok(text)
    }

    fn keyword(&mut self, from: i32, to: i32) -> Result<&'static str, PelersError> {
        let t = self.kind();
        if t < from || t > to {
            return Err(self.fail(from, to, &[]));
        }
        self.i += 1;
        Ok(VALUES[(t - T::Requirement as i32) as usize])
    }

    fn string(&mut self) -> Result<String, PelersError> {
        let t = self.kind();
        if t != T::UnqString as i32 && t != T::QString as i32 {
            return Err(self.fail(T::QString as i32, T::UnqString as i32, &[]));
        }
        let text = self.text(self.i).to_owned();
        self.i += 1;
        Ok(text)
    }

    fn directive(&mut self, db: &mut RequirementDb) -> Result<bool, PelersError> {
        let t = self.kind();
        if t == T::AccTitle as i32 {
            self.i += 1;
            let text = self.expect(T::AccTitleValue as i32)?;
            db.set_acc_title(trim(&text));
        } else if t == T::AccDescr as i32 {
            self.i += 1;
            let text = self.expect(T::AccDescrValue as i32)?;
            db.set_acc_description(trim(&text));
        } else if t == T::AccDescrMultilineValue as i32 {
            let text = self.text(self.i).to_owned();
            self.i += 1;
            db.set_acc_description(trim(&text));
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    fn id_list(&mut self) -> Result<Vec<String>, PelersError> {
        let mut list = Vec::new();
        loop {
            let t = self.kind();
            if t != T::Alpha as i32 && t != T::UnqString as i32 && t != T::QString as i32 {
                return Err(self.fail(T::QString as i32, T::UnqString as i32, &[T::Alpha as i32]));
            }
            list.push(self.text(self.i).to_owned());
            self.i += 1;
            if self.kind() != T::Comma as i32 {
                return Ok(list);
            }
            self.i += 1;
        }
    }

    fn styles_opt(&mut self) -> Result<Vec<String>, PelersError> {
        let mut styles = Vec::new();
        loop {
            if !is_style_part(self.kind()) {
                return Err(self.fail(
                    T::Alpha as i32,
                    T::Semicolon as i32,
                    &[T::Minus as i32, T::Brkt as i32],
                ));
            }
            let mut style = self.text(self.i).to_owned();
            self.i += 1;
            while is_style_part(self.kind()) {
                style.push_str(self.text(self.i));
                self.i += 1;
            }
            styles.push(style);
            if self.kind() != T::Comma as i32 {
                return Ok(styles);
            }
            self.i += 1;
        }
    }

    fn class_list(&mut self) -> Result<Option<Vec<String>>, PelersError> {
        let mut classes = None;
        if self.kind() == T::StyleSeparator as i32 {
            self.i += 1;
            classes = Some(self.id_list()?);
            self.expect(T::StructStart as i32)?;
        } else if self.kind() == T::StructStart as i32 {
            self.i += 1;
        } else {
            return Err(self.fail(
                T::StructStart as i32,
                T::StructStart as i32,
                &[T::StyleSeparator as i32],
            ));
        }
        self.expect(T::Newline as i32)?;
        Ok(classes)
    }

    fn body(&mut self, db: &mut RequirementDb, requirement: bool) -> Result<(), PelersError> {
        self.fields.clear();
        self.values.clear();
        loop {
            let t = self.kind();
            if t == T::StructStop as i32 {
                self.i += 1;
                break;
            }
            if t == T::Newline as i32 {
                self.i += 1;
                continue;
            }
            let allowed = if requirement {
                t >= T::Id as i32 && t <= T::Verifymthd as i32
            } else {
                t == T::Type as i32 || t == T::Docref as i32
            };
            if !allowed {
                if requirement {
                    return Err(self.fail(
                        T::Id as i32,
                        T::Verifymthd as i32,
                        &[T::Newline as i32, T::StructStop as i32],
                    ));
                }
                return Err(self.fail(
                    T::Type as i32,
                    T::Docref as i32,
                    &[T::Newline as i32, T::StructStop as i32],
                ));
            }
            self.i += 1;
            self.expect(T::Colonsep as i32)?;
            let value = if t == T::Risk as i32 {
                self.keyword(T::LowRisk as i32, T::HighRisk as i32)?
                    .to_owned()
            } else if t == T::Verifymthd as i32 {
                self.keyword(T::VerifyAnalysis as i32, T::VerifyTest as i32)?
                    .to_owned()
            } else {
                self.string()?
            };
            self.values.push(value);
            self.fields.push(t);
            self.expect(T::Newline as i32)?;
        }
        for k in (0..self.fields.len()).rev() {
            let value = self.values[k].clone();
            let field = self.fields[k];
            if field == T::Id as i32 {
                db.set_new_req_id(&value);
            } else if field == T::Text as i32 {
                db.set_new_req_text(&value);
            } else if field == T::Risk as i32 {
                db.set_new_req_risk(&value);
            } else if field == T::Verifymthd as i32 {
                db.set_new_req_verify_method(&value);
            } else if field == T::Type as i32 {
                db.set_new_element_type(&value);
            } else {
                db.set_new_element_doc_ref(&value);
            }
        }
        Ok(())
    }
}

pub fn parse_requirement(src: &str, db: &mut RequirementDb) -> Result<(), PelersError> {
    let units: Vec<u16> = src.encode_utf16().collect();
    let tokens = tokenize(&units, Spans::new(src));
    let mut p = Parser {
        src,
        units,
        types: tokens.types,
        spans: tokens.spans,
        starts: tokens.starts,
        i: 0,
        fields: Vec::new(),
        values: Vec::new(),
    };

    while p.directive(db)? {
        if p.kind() == T::Newline as i32 {
            p.i += 1;
        }
    }
    if p.kind() != T::Rd as i32 {
        return Err(p.fail(
            T::Rd as i32,
            T::Rd as i32,
            &[
                T::AccTitle as i32,
                T::AccDescr as i32,
                T::AccDescrMultilineValue as i32,
            ],
        ));
    }
    p.i += 1;
    p.expect(T::Newline as i32)?;

    loop {
        let t = p.kind();
        if t == T::Eof as i32 {
            return Ok(());
        }
        if t == T::Newline as i32 {
            p.i += 1;
        } else if p.directive(db)? {
            continue;
        } else if t >= T::DirectionTb as i32 && t <= T::DirectionLr as i32 {
            p.i += 1;
            let dir = if t == T::DirectionTb as i32 {
                "TB"
            } else if t == T::DirectionBt as i32 {
                "BT"
            } else if t == T::DirectionRl as i32 {
                "RL"
            } else {
                "LR"
            };
            db.set_direction(dir);
        } else if t >= T::Requirement as i32 && t <= T::DesignConstraint as i32 {
            let type_ = p.keyword(T::Requirement as i32, T::DesignConstraint as i32)?;
            let name = p.string()?;
            let classes = p.class_list()?;
            p.body(db, true)?;
            db.add_requirement(&name, type_);
            if let Some(classes) = classes {
                db.set_class(std::slice::from_ref(&name), &classes)?;
            }
        } else if t == T::Element as i32 {
            p.i += 1;
            let name = p.string()?;
            let classes = p.class_list()?;
            p.body(db, false)?;
            db.add_element(&name);
            if let Some(classes) = classes {
                db.set_class(std::slice::from_ref(&name), &classes)?;
            }
        } else if t == T::Style as i32 {
            p.i += 1;
            let ids = p.id_list()?;
            let styles = p.styles_opt()?;
            db.set_css_style(&ids, &styles)?;
        } else if t == T::Classdef as i32 {
            p.i += 1;
            let ids = p.id_list()?;
            let styles = p.styles_opt()?;
            db.define_class(&ids, &styles)?;
        } else if t == T::Class as i32 {
            p.i += 1;
            let ids = p.id_list()?;
            let classes = p.id_list()?;
            db.set_class(&ids, &classes)?;
        } else if t == T::UnqString as i32 || t == T::QString as i32 {
            let id = p.text(p.i).to_owned();
            p.i += 1;
            let next = p.kind();
            if next == T::StyleSeparator as i32 {
                p.i += 1;
                let classes = p.id_list()?;
                db.set_class(&[id], &classes)?;
            } else if next == T::EndArrowL as i32 || next == T::Line as i32 {
                p.i += 1;
                let relationship = p.keyword(T::Contains as i32, T::Traces as i32)?;
                p.expect(if next == T::Line as i32 {
                    T::EndArrowR as i32
                } else {
                    T::Line as i32
                })?;
                let other = p.string()?;
                if next == T::Line as i32 {
                    db.add_relationship(relationship, &id, &other);
                } else {
                    db.add_relationship(relationship, &other, &id);
                }
            } else {
                return Err(p.fail(
                    T::StyleSeparator as i32,
                    T::StyleSeparator as i32,
                    &[T::EndArrowL as i32, T::Line as i32],
                ));
            }
        } else {
            let expected = [
                T::Newline as i32,
                T::Eof as i32,
                T::AccTitle as i32,
                T::AccDescr as i32,
                T::AccDescrMultilineValue as i32,
                T::DirectionTb as i32,
                T::DirectionBt as i32,
                T::DirectionRl as i32,
                T::DirectionLr as i32,
                T::Requirement as i32,
                T::FunctionalRequirement as i32,
                T::InterfaceRequirement as i32,
                T::PerformanceRequirement as i32,
                T::PhysicalRequirement as i32,
                T::DesignConstraint as i32,
                T::Element as i32,
                T::Classdef as i32,
                T::Class as i32,
                T::Style as i32,
                T::UnqString as i32,
                T::QString as i32,
            ];
            return Err(p.fail(T::Newline as i32, T::Eof as i32, &expected[2..]));
        }
    }
}
