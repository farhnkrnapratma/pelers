use super::db::{DataPoint, XyChartDb, XyText};
use super::lexer::{
    ACC_DESCR, ACC_DESCR_MULTILINE_VALUE, ACC_DESCR_VALUE, ACC_TITLE, ACC_TITLE_VALUE, ALPHA, AMP,
    ARROW_DELIMITER, BAR, BRKT, CHART_ORIENTATION, COMMA, DOT, END, EOF, EQUALS, ERROR, LINE,
    MINUS, MULT, NEWLINE, NUM, NUMBER_WITH_DECIMAL, PLUS, SEMI, SQUARE_BRACES_END,
    SQUARE_BRACES_START, STR, TITLE, TOKEN_NAMES, UNDERSCORE, X_AXIS, XYCHART, Y_AXIS, tokenize,
};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};
use crate::util::jsnum::js_number;

fn word_flag(kind: i32) -> bool {
    matches!(
        kind,
        AMP | NUM | ALPHA | PLUS | EQUALS | MULT | DOT | BRKT | MINUS | UNDERSCORE
    )
}

fn eol_flag(kind: i32) -> bool {
    matches!(kind, NEWLINE | SEMI | EOF)
}

pub fn parse_xychart(source: &str, db: &mut XyChartDb) -> Result<(), PelersError> {
    let tokens = tokenize(source);
    let (types, texts, starts) = (tokens.types, tokens.texts, tokens.starts);
    let mut i = 0usize;

    let fail = |i: usize, expected: &str| -> PelersError {
        let t = types[i];
        let offset = utf16_to_byte(source, starts[i]);
        if t == ERROR {
            return lexical_error("xychart", source, offset);
        }
        syntax_error(
            "xychart",
            source,
            offset,
            format!("Expecting {expected}, got '{}'", TOKEN_NAMES[t as usize]),
        )
    };

    macro_rules! expect {
        ($kind:expr) => {{
            if types[i] != $kind {
                return Err(fail(i, &format!("'{}'", TOKEN_NAMES[$kind as usize])));
            }
            i += 1;
            texts[i - 1].clone()
        }};
    }

    macro_rules! skip {
        ($kind:expr) => {{
            if types[i] != $kind {
                return Err(fail(i, &format!("'{}'", TOKEN_NAMES[$kind as usize])));
            }
            i += 1;
        }};
    }

    let is_text = |t: i32| t == STR || word_flag(t);

    macro_rules! text {
        () => {{
            if !is_text(types[i]) {
                return Err(fail(i, "'STR', 'ALPHA', 'NUM'"));
            }
            let mut s = texts[i].clone();
            i += 1;
            if types[i - 1] != STR {
                while word_flag(types[i]) {
                    s.push_str(&texts[i]);
                    i += 1;
                }
            }
            XyText { text: s }
        }};
    }

    loop {
        if eol_flag(types[i]) {
            i += 1;
        } else if types[i] != XYCHART {
            break;
        } else {
            i += 1;
            if types[i] == CHART_ORIENTATION {
                db.set_orientation(&texts[i]);
                i += 1;
            }
        }
    }

    while types[i] != END {
        let t = types[i];
        i += 1;
        match t {
            TITLE => {
                let value = text!();
                db.set_diagram_title(trim(&value.text));
            }
            X_AXIS | Y_AXIS => {
                let title = if is_text(types[i]) {
                    Some(text!())
                } else {
                    None
                };
                if t == X_AXIS && types[i] == SQUARE_BRACES_START {
                    let mut categories: Vec<XyText> = Vec::new();
                    loop {
                        i += 1;
                        categories.push(text!());
                        if types[i] != COMMA {
                            break;
                        }
                    }
                    skip!(SQUARE_BRACES_END);
                    db.set_x_axis_band(categories);
                } else if types[i] == NUMBER_WITH_DECIMAL || title.is_none() {
                    let min = js_number(&expect!(NUMBER_WITH_DECIMAL));
                    skip!(ARROW_DELIMITER);
                    let max = js_number(&expect!(NUMBER_WITH_DECIMAL));
                    if t == X_AXIS {
                        db.set_x_axis_range(min, max);
                    } else {
                        db.set_y_axis_range(min, max);
                    }
                }
                let title = title.unwrap_or(XyText {
                    text: String::new(),
                });
                if t == X_AXIS {
                    db.set_x_axis_title(title);
                } else {
                    db.set_y_axis_title(title);
                }
            }
            LINE | BAR => {
                let title = if types[i] == SQUARE_BRACES_START {
                    XyText {
                        text: String::new(),
                    }
                } else {
                    text!()
                };
                let mut data: Vec<DataPoint> = Vec::new();
                skip!(SQUARE_BRACES_START);
                loop {
                    let value = js_number(&expect!(NUMBER_WITH_DECIMAL));
                    let label = if types[i] == STR {
                        i += 1;
                        texts[i - 1].clone()
                    } else {
                        String::new()
                    };
                    data.push(DataPoint { value, label });
                    if types[i] != COMMA {
                        break;
                    }
                    i += 1;
                }
                skip!(SQUARE_BRACES_END);
                if t == LINE {
                    db.set_line_data(title, data);
                } else {
                    db.set_bar_data(title, data);
                }
            }
            ACC_TITLE => {
                let value = expect!(ACC_TITLE_VALUE);
                db.set_acc_title(trim(&value));
            }
            ACC_DESCR => {
                let value = expect!(ACC_DESCR_VALUE);
                db.set_acc_description(trim(&value));
            }
            ACC_DESCR_MULTILINE_VALUE => {
                db.set_acc_description(trim(&texts[i - 1]));
            }
            _ => {
                i -= 1;
                return Err(fail(
                    i,
                    "'title', 'X_AXIS', 'Y_AXIS', 'LINE', 'BAR', 'acc_title', 'acc_descr', 'acc_descr_multiline_value'",
                ));
            }
        }
        while eol_flag(types[i]) {
            i += 1;
        }
    }
    Ok(())
}
