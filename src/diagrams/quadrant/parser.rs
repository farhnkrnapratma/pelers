use super::db::{QuadrantDb, QuadrantText, trimmed};
use super::lexer::{
    ACC_DESCR, ACC_DESCR_MULTILINE_VALUE, ACC_DESCR_VALUE, ACC_TITLE, ACC_TITLE_VALUE, ALPHA, AMP,
    BRKT, CLASS_NAME, CLASSDEF, COLON, COMMA, DELIMITER, DOT, END, EOF, EQUALS, ERROR, MD_STR,
    MINUS, MULT, NEWLINE, NUM, PLUS, POINT_START, POINT_X, POINT_Y, PUNCTUATION, QUADRANT,
    QUADRANT_1, QUADRANT_4, SEMI, SPACE, STR, TITLE, TITLE_VALUE, TOKEN_NAMES, UNDERSCORE,
    UNICODE_TEXT, X_AXIS, Y_AXIS, tokenize,
};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::{trim, utf16_to_byte};

fn word_flags(kind: i32) -> bool {
    matches!(
        kind,
        PUNCTUATION
            | AMP
            | NUM
            | ALPHA
            | COMMA
            | PLUS
            | EQUALS
            | MULT
            | DOT
            | BRKT
            | UNDERSCORE
            | UNICODE_TEXT
    )
}

fn id_flags(kind: i32) -> bool {
    matches!(
        kind,
        ALPHA | NUM | MINUS | COMMA | COLON | AMP | BRKT | MULT | UNICODE_TEXT
    )
}

fn style_flags(kind: i32) -> bool {
    matches!(kind, ALPHA | NUM | COLON | SPACE | BRKT | MINUS)
}

fn eol_flags(kind: i32) -> bool {
    matches!(kind, NEWLINE | SEMI | EOF)
}

const TEXT_TOKENS: &str = "'STR', 'MD_STR', 'ALPHA', 'NUM', 'UNICODE_TEXT', 'PUNCTUATION'";

pub fn parse_quadrant(source: &str, db: &mut QuadrantDb) -> Result<(), PelersError> {
    let tokens = tokenize(source);
    let (types, texts, starts) = (tokens.types, tokens.texts, tokens.starts);
    let mut i = 0usize;

    let fail = |i: usize, expected: &str| -> PelersError {
        let t = types[i];
        let offset = utf16_to_byte(source, starts[i]);
        if t == ERROR {
            return lexical_error("quadrantChart", source, offset);
        }
        syntax_error(
            "quadrantChart",
            source,
            offset,
            format!("Expecting {expected}, got '{}'", TOKEN_NAMES[t as usize]),
        )
    };

    fn expect(
        types: &[i32],
        texts: &[String],
        i: &mut usize,
        kind: i32,
        fail: &dyn Fn(usize, &str) -> PelersError,
    ) -> Result<String, PelersError> {
        if types[*i] != kind {
            return Err(fail(*i, &format!("'{}'", TOKEN_NAMES[kind as usize])));
        }
        *i += 1;
        Ok(texts[*i - 1].clone())
    }

    let is_text = |t: i32| t == STR || t == MD_STR || word_flags(t);

    fn text(
        types: &[i32],
        texts: &[String],
        i: &mut usize,
        fail: &dyn Fn(usize, &str) -> PelersError,
        is_text: &dyn Fn(i32) -> bool,
    ) -> Result<QuadrantText, PelersError> {
        let first = types[*i];
        if !is_text(first) {
            return Err(fail(*i, TEXT_TOKENS));
        }
        let mut s = texts[*i].clone();
        *i += 1;
        while word_flags(types[*i]) || types[*i] == SPACE || types[*i] == MINUS {
            s.push_str(&texts[*i]);
            *i += 1;
        }
        Ok(QuadrantText {
            text: s,
            markdown: first == MD_STR,
        })
    }

    fn styles(
        types: &[i32],
        texts: &[String],
        i: &mut usize,
        fail: &dyn Fn(usize, &str) -> PelersError,
    ) -> Result<Vec<String>, PelersError> {
        let mut out = Vec::new();
        loop {
            if !style_flags(types[*i]) {
                return Err(fail(
                    *i,
                    "'ALPHA', 'NUM', 'COLON', 'SPACE', 'BRKT', 'MINUS'",
                ));
            }
            let mut s = texts[*i].clone();
            *i += 1;
            while style_flags(types[*i]) {
                s.push_str(&texts[*i]);
                *i += 1;
            }
            out.push(trim(&s).to_owned());
            if types[*i] != COMMA {
                return Ok(out);
            }
            *i += 1;
        }
    }

    let fail_dyn: &dyn Fn(usize, &str) -> PelersError = &fail;
    let is_text_dyn: &dyn Fn(i32) -> bool = &is_text;

    while eol_flags(types[i]) || types[i] == SPACE {
        i += 1;
    }
    expect(&types, &texts, &mut i, QUADRANT, fail_dyn)?;

    while types[i] != END {
        while types[i] == SPACE {
            i += 1;
        }
        let t = types[i];
        match t {
            CLASSDEF => {
                i += 1;
                expect(&types, &texts, &mut i, SPACE, fail_dyn)?;
                if !id_flags(types[i]) {
                    return Err(fail(i, "'ALPHA', 'NUM', 'UNICODE_TEXT'"));
                }
                let mut id = texts[i].clone();
                i += 1;
                while id_flags(types[i]) {
                    id.push_str(&texts[i]);
                    i += 1;
                }
                expect(&types, &texts, &mut i, SPACE, fail_dyn)?;
                let declared = styles(&types, &texts, &mut i, fail_dyn)?;
                db.add_class(id, &declared)?;
            }
            X_AXIS | Y_AXIS => {
                i += 1;
                let from = text(&types, &texts, &mut i, fail_dyn, is_text_dyn)?;
                let mut from = from;
                let mut to: Option<QuadrantText> = None;
                if types[i] == DELIMITER {
                    i += 1;
                    if is_text(types[i]) {
                        to = Some(text(&types, &texts, &mut i, fail_dyn, is_text_dyn)?);
                    } else {
                        from.text.push_str(" ⟶ ");
                    }
                }
                if t == X_AXIS {
                    db.x_axis_left = Some(trimmed(from));
                    if let Some(to) = to {
                        db.x_axis_right = Some(trimmed(to));
                    }
                } else {
                    db.y_axis_bottom = Some(trimmed(from));
                    if let Some(to) = to {
                        db.y_axis_top = Some(trimmed(to));
                    }
                }
            }
            QUADRANT_1..=QUADRANT_4 => {
                i += 1;
                let label = text(&types, &texts, &mut i, fail_dyn, is_text_dyn)?;
                db.quadrants[(t - QUADRANT_1) as usize] = Some(trimmed(label));
            }
            TITLE => {
                i += 1;
                let value = expect(&types, &texts, &mut i, TITLE_VALUE, fail_dyn)?;
                db.title = Some(trim(&value).to_owned());
            }
            ACC_TITLE => {
                i += 1;
                let value = expect(&types, &texts, &mut i, ACC_TITLE_VALUE, fail_dyn)?;
                db.acc_title = Some(trim(&value).to_owned());
            }
            ACC_DESCR => {
                i += 1;
                let value = expect(&types, &texts, &mut i, ACC_DESCR_VALUE, fail_dyn)?;
                db.acc_descr = Some(trim(&value).to_owned());
            }
            ACC_DESCR_MULTILINE_VALUE => {
                db.acc_descr = Some(trim(&texts[i]).to_owned());
                i += 1;
            }
            _ => {
                if is_text(t) {
                    let label = text(&types, &texts, &mut i, fail_dyn, is_text_dyn)?;
                    let class_name = if types[i] == CLASS_NAME {
                        i += 1;
                        texts[i - 1].clone()
                    } else {
                        String::new()
                    };
                    expect(&types, &texts, &mut i, POINT_START, fail_dyn)?;
                    let x = expect(&types, &texts, &mut i, POINT_X, fail_dyn)?;
                    let y = expect(&types, &texts, &mut i, POINT_Y, fail_dyn)?;
                    let declared = if style_flags(types[i]) {
                        styles(&types, &texts, &mut i, fail_dyn)?
                    } else {
                        Vec::new()
                    };
                    db.add_point(label, class_name, &x, &y, &declared)?;
                }
            }
        }
        if !eol_flags(types[i]) {
            return Err(fail(i, "'NEWLINE', 'SEMI', 'EOF'"));
        }
        i += 1;
    }
    Ok(())
}
