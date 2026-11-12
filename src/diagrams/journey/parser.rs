use super::JourneyModel;
use super::JourneyTask;
use super::lexer::{
    ACC_DESCR, ACC_DESCR_MULTILINE_VALUE, ACC_TITLE, END, EOF, JOURNEY, NEWLINE, SECTION,
    TASK_DATA, TASK_NAME, TITLE, TOKEN_NAMES, tokenize,
};
use crate::errors::{PelersError, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};
use crate::util::jsnum::js_number;

const STATEMENT: &str = "'EOF', 'SPACE', 'NEWLINE', 'title', 'acc_title', 'acc_descr', 'acc_descr_multiline_value', 'section', 'taskName'";

fn task(name: &str, data: &str, section: &str) -> JourneyTask {
    let pieces: Vec<&str> = data[1..].split(':').collect();
    let people = if pieces.len() == 1 {
        Vec::new()
    } else {
        pieces[1].split(',').map(|n| trim(n).to_owned()).collect()
    };
    JourneyTask {
        task: name.to_owned(),
        score: js_number(pieces[0]),
        people,
        section: section.to_owned(),
    }
}

pub fn parse_journey(source: &str, title: Option<String>) -> Result<JourneyModel, PelersError> {
    let tokens = tokenize(source);
    let (types, texts, starts) = (tokens.types, tokens.texts, tokens.starts);
    let fail = |i: usize, expected: &str| {
        let got = if types[i] == END {
            "1"
        } else {
            TOKEN_NAMES[types[i] as usize]
        };
        syntax_error(
            "journey",
            source,
            utf16_to_byte(source, starts[i]),
            format!("Expecting {expected}, got '{got}'"),
        )
    };

    if types[0] != JOURNEY {
        return Err(fail(0, "'journey'"));
    }
    let mut model = JourneyModel {
        title,
        acc_title: None,
        acc_descr: None,
        tasks: Vec::new(),
    };
    let mut section = String::new();
    let mut i = 1;
    loop {
        let text = texts[i].as_str();
        match types[i] {
            NEWLINE => {}
            TITLE => model.title = Some(text.chars().skip(6).collect()),
            ACC_TITLE => {
                i += 1;
                model.acc_title = Some(trim(&texts[i]).to_owned());
            }
            ACC_DESCR => {
                i += 1;
                model.acc_descr = Some(trim(&texts[i]).to_owned());
            }
            ACC_DESCR_MULTILINE_VALUE => model.acc_descr = Some(trim(text).to_owned()),
            SECTION => section = text.chars().skip(8).collect(),
            TASK_NAME => {
                i += 1;
                if types[i] != TASK_DATA {
                    return Err(fail(i, "'taskData'"));
                }
                model.tasks.push(task(text, &texts[i], &section));
            }
            EOF => return Ok(model),
            _ => return Err(fail(i, STATEMENT)),
        }
        i += 1;
    }
}
