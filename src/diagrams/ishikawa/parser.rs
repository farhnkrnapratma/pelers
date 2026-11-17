use super::db::IshikawaDb;
use super::lexer::{END, EOF, ISHIKAWA, NL, SPACELINE, SPACELIST, TEXT, TOKEN_NAMES, tokenize};
use crate::errors::{PelersError, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

const STOP: &str = "'SPACELINE', 'NL', 'EOF'";

pub fn parse_ishikawa(source: &str, db: &mut IshikawaDb) -> Result<(), PelersError> {
    let tokens = tokenize(source);
    let (types, starts, ends) = (tokens.types, tokens.starts, tokens.ends);
    let fail = |i: usize, expected: &str| {
        syntax_error(
            "ishikawa",
            source,
            utf16_to_byte(source, starts[i]),
            format!(
                "Expecting {expected}, got '{}'",
                TOKEN_NAMES[types[i] as usize]
            ),
        )
    };
    let stop = |kind: i32| kind == SPACELINE || kind == NL || kind == EOF;
    let units: Vec<u16> = source.encode_utf16().collect();
    let text_of =
        |from: usize, to: usize| trim(&String::from_utf16_lossy(&units[from..to])).to_owned();

    let mut i = 0usize;
    if types[i] == SPACELINE {
        i += 1;
        while types[i] == SPACELINE || types[i] == NL {
            i += 1;
        }
        if types[i] != ISHIKAWA {
            return Err(fail(i, "'SPACELINE', 'NL', 'ISHIKAWA'"));
        }
    } else if types[i] != ISHIKAWA {
        return Err(fail(i, "'SPACELINE', 'ISHIKAWA'"));
    }
    i += 1;
    if types[i] == NL {
        i += 1;
    }

    loop {
        let kind = types[i];
        if kind == TEXT || (kind == SPACELIST && types[i + 1] == TEXT) {
            let text = if kind == TEXT { i } else { i + 1 };
            i = text + 1;
            if !stop(types[i]) {
                return Err(fail(i, STOP));
            }
            let level = if kind == TEXT {
                0
            } else {
                ends[text - 1] - starts[text - 1]
            };
            db.add_node(level as i64, &text_of(starts[text], ends[text]));
        } else if kind == SPACELINE || kind == SPACELIST {
            i += 1;
            if !stop(types[i]) {
                let expected = if kind == SPACELIST {
                    format!("{STOP}, 'TEXT'")
                } else {
                    STOP.to_owned()
                };
                return Err(fail(i, &expected));
            }
        } else {
            return Err(fail(i, "'SPACELINE', 'SPACELIST', 'TEXT'"));
        }
        i += 1;
        while types[i] == NL || types[i] == EOF {
            i += 1;
        }
        if types[i] == END {
            return Ok(());
        }
    }
}
