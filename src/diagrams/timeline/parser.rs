use super::db::TimelineModel;
use super::lexer::{
    ACC_DESCR, ACC_DESCR_MULTILINE_VALUE, ACC_TITLE, END, EOF, EVENT, NEWLINE, PERIOD, SECTION,
    TIMELINE, TIMELINE_LR, TIMELINE_TD, TITLE, TOKEN_NAMES, tokenize,
};
use crate::errors::{PelersError, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const STATEMENT: &str = "'EOF', 'SPACE', 'NEWLINE', 'title', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'section', 'period', 'event'";

pub fn parse_timeline(source: &str, db: &mut TimelineModel) -> Result<(), PelersError> {
    let tokens = tokenize(source);
    let (types, texts, starts) = (tokens.types, tokens.texts, tokens.starts);
    let fail = |i: usize, expected: &str| {
        let got = if types[i] == END {
            "1"
        } else {
            TOKEN_NAMES[types[i] as usize]
        };
        syntax_error(
            "timeline",
            source,
            utf16_to_byte(source, starts[i]),
            format!("Expecting {expected}, got '{got}'"),
        )
    };

    let head = types[0];
    if head != TIMELINE && head != TIMELINE_LR && head != TIMELINE_TD {
        return Err(fail(0, "'timeline', 'timeline_lr', 'timeline_td'"));
    }
    if head != TIMELINE {
        db.vertical = head == TIMELINE_TD;
    }

    let mut i = 1usize;
    loop {
        let text = texts[i].as_str();
        match types[i] {
            NEWLINE => {}
            TITLE => db.title = Some(text.chars().skip(6).collect()),
            ACC_TITLE => {
                i += 1;
                db.acc_title = Some(trim(&texts[i]).to_owned());
            }
            ACC_DESCR => {
                i += 1;
                db.acc_descr = Some(trim(&texts[i]).to_owned());
            }
            ACC_DESCR_MULTILINE_VALUE => db.acc_descr = Some(trim(text).to_owned()),
            SECTION => db.add_section(&text.chars().skip(8).collect::<String>()),
            PERIOD => db.add_task(text),
            EVENT => db.add_event(&text.chars().skip(2).collect::<String>())?,
            EOF => return Ok(()),
            _ => return Err(fail(i, STATEMENT)),
        }
        i += 1;
    }
}
