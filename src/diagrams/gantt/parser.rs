use super::db::GanttDb;
use super::lexer::tokenize;
use super::tokens::*;
use crate::errors::{PelersError, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

const LINE: &str = "'EOF', 'SPACE', 'NL', 'weekday_monday', 'weekday_tuesday', 'weekday_wednesday', 'weekday_thursday', \
'weekday_friday', 'weekday_saturday', 'weekday_sunday', 'weekend_friday', 'weekend_saturday', 'dateFormat', \
'inclusiveEndDates', 'topAxis', 'axisFormat', 'tickInterval', 'excludes', 'includes', 'todayMarker', 'title', \
'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'section', 'taskTxt', 'click'";

fn starts_line(t: usize) -> bool {
    matches!(
        t,
        WEEKDAY_MONDAY
            ..=WEEKEND_SATURDAY
                | EOF
                | NL
                | DATEFORMAT
                | INCLUSIVEENDDATES
                | TOPAXIS
                | AXISFORMAT
                | TICKINTERVAL
                | EXCLUDES
                | INCLUDES
                | TODAYMARKER
                | TITLE
                | ACC_TITLE
                | ACC_DESCR
                | ACC_DESCR_MULTILINE_VALUE
                | SECTION
                | TASKTXT
                | CLICK
    )
}

fn after(text: &str, n: usize) -> String {
    text.chars().skip(n).collect()
}

struct Parser<'a, 'b> {
    src: &'a str,
    types: Vec<usize>,
    texts: Vec<String>,
    starts: Vec<usize>,
    i: usize,
    db: &'b mut GanttDb,
}

impl Parser<'_, '_> {
    fn ty(&self, at: usize) -> usize {
        self.types.get(at).copied().unwrap_or(usize::MAX)
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
        let t = self.ty(self.i);
        let got = if t == END {
            "1".to_owned()
        } else {
            TOKEN_NAMES
                .get(t)
                .map_or_else(|| "undefined".to_owned(), |name| (*name).to_owned())
        };
        Err(syntax_error(
            "gantt",
            self.src,
            offset,
            format!("Expecting {expected}, got '{got}'"),
        ))
    }

    fn end(&self, also: &str) -> Result<(), PelersError> {
        if !starts_line(self.ty(self.i)) {
            return self.fail(&format!("{LINE}{also}"));
        }
        Ok(())
    }
}

pub fn parse_gantt(src: &str, db: &mut GanttDb) -> Result<(), PelersError> {
    let tokens = tokenize(src);
    let mut p = Parser {
        src,
        types: tokens.types,
        texts: tokens.texts,
        starts: tokens.starts,
        i: 0,
        db,
    };
    if p.ty(p.i) != GANTT {
        return p.fail("'gantt'");
    }
    p.i += 1;

    loop {
        let t = p.ty(p.i);
        let text = p.text(p.i);
        if !starts_line(t) {
            return p.fail(LINE);
        }
        p.i += 1;
        match t {
            EOF => {
                if p.ty(p.i) == END {
                    return Ok(());
                }
                p.end("")?;
            }
            NL => {}
            DATEFORMAT => {
                p.end("")?;
                p.db.set_date_format(&after(&text, 11));
            }
            INCLUSIVEENDDATES => {
                p.end("")?;
                p.db.enable_inclusive_end_dates();
            }
            TOPAXIS => {
                p.end("")?;
                p.db.top_axis();
            }
            AXISFORMAT => {
                p.end("")?;
                p.db.set_axis_format(&after(&text, 11));
            }
            TICKINTERVAL => {
                p.end("")?;
                p.db.set_tick_interval(&after(&text, 13));
            }
            EXCLUDES => {
                p.end("")?;
                p.db.set_excludes(&after(&text, 9));
            }
            INCLUDES => {
                p.end("")?;
                p.db.set_includes(&after(&text, 9));
            }
            TODAYMARKER => {
                p.end("")?;
                p.db.set_today_marker(&after(&text, 12));
            }
            WEEKEND_FRIDAY => {
                p.end("")?;
                p.db.set_weekend("friday");
            }
            WEEKEND_SATURDAY => {
                p.end("")?;
                p.db.set_weekend("saturday");
            }
            TITLE => {
                p.end("")?;
                p.db.set_diagram_title(&after(&text, 6));
            }
            ACC_TITLE => {
                if p.ty(p.i) != ACC_TITLE_VALUE {
                    return p.fail("'acc_title_value'");
                }
                let value = p.text(p.i);
                p.i += 1;
                p.end("")?;
                p.db.set_acc_title(trim(&value));
            }
            ACC_DESCR => {
                if p.ty(p.i) != ACC_DESCR_VALUE {
                    return p.fail("'acc_descr_value'");
                }
                let value = p.text(p.i);
                p.i += 1;
                p.end("")?;
                p.db.set_acc_description(trim(&value));
            }
            ACC_DESCR_MULTILINE_VALUE => {
                p.end("")?;
                p.db.set_acc_description(trim(&text));
            }
            SECTION => {
                p.end("")?;
                p.db.add_section(&after(&text, 8));
            }
            TASKTXT => {
                if p.ty(p.i) != TASKDATA {
                    return p.fail("'taskData'");
                }
                let data = p.text(p.i);
                p.i += 1;
                p.end("")?;
                p.db.add_task(&text, &data)?;
            }
            CLICK => {
                let mut name: Option<String> = None;
                let mut args: Option<String> = None;
                let mut href: Option<String> = None;
                if p.ty(p.i) == CALLBACKNAME {
                    name = Some(p.text(p.i));
                    p.i += 1;
                    if p.ty(p.i) == CALLBACKARGS {
                        args = Some(p.text(p.i));
                        p.i += 1;
                    }
                    if p.ty(p.i) == HREF {
                        href = Some(p.text(p.i));
                        p.i += 1;
                        p.end("")?;
                    } else {
                        p.end(if args.is_none() {
                            ", 'callbackargs', 'href'"
                        } else {
                            ", 'href'"
                        })?;
                    }
                } else if p.ty(p.i) == HREF {
                    href = Some(p.text(p.i));
                    p.i += 1;
                    if p.ty(p.i) == CALLBACKNAME {
                        name = Some(p.text(p.i));
                        p.i += 1;
                        if p.ty(p.i) == CALLBACKARGS {
                            args = Some(p.text(p.i));
                            p.i += 1;
                            p.end("")?;
                        } else {
                            p.end(", 'callbackargs'")?;
                        }
                    } else {
                        p.end(", 'callbackname'")?;
                    }
                } else {
                    return p.fail("'callbackname', 'href'");
                }
                if let Some(name) = name {
                    p.db.set_click_event(&text, &name, args.as_deref());
                }
                if let Some(href) = href {
                    p.db.set_link(&text, &href);
                }
            }
            _ => {
                p.end("")?;
                p.db.set_weekday(WEEKDAYS[t - WEEKDAY_MONDAY]);
            }
        }
    }
}
