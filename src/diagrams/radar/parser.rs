use super::lexer::{
    ACC_DESCR_KIND, ACC_TITLE_KIND, AXIS, BOOLEAN, CLOSE, CLOSE_BRACE, COLON, COMMA, CURVE,
    GRATICULE, GRATICULE_VALUE, ID, MAX, MIN, NEWLINE_KIND, NUMBER, OPEN, OPEN_BRACE, RADAR,
    RADAR_TOKENS, RADARCOLON, SHOW_LEGEND, STRING_KIND, TICKS, TITLE,
};
use crate::diagrams::common::tokens::{
    Reader, acc_descr_value, acc_title_value, string_value, title_value, tokenize,
};
use crate::errors::PelersError;
use crate::util::jsnum::js_number;

#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    pub name: String,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub axis: Option<String>,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub name: String,
    pub label: Option<String>,
    pub entries: Vec<Entry>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionValue {
    Bool(bool),
    Text(String),
    Number(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct OptionAst {
    pub name: String,
    pub value: OptionValue,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RadarAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub axes: Vec<Axis>,
    pub curves: Vec<Curve>,
    pub options: Vec<OptionAst>,
}

fn is_option(kind: i32) -> bool {
    matches!(kind, SHOW_LEGEND | TICKS | MAX | MIN | GRATICULE)
}

pub fn parse_radar(src: &str) -> Result<RadarAst, PelersError> {
    let mut r = Reader::new(
        tokenize(src, &RADAR_TOKENS, "radar")?,
        &RADAR_TOKENS,
        "radar",
    );
    let mut ast = RadarAst::default();

    fn newlines(r: &mut Reader) {
        while r.kind() == NEWLINE_KIND {
            r.i += 1;
        }
    }
    fn end_of_line(r: &mut Reader) -> Result<(), PelersError> {
        if r.kind() == -1 {
            return Ok(());
        }
        if r.kind() != NEWLINE_KIND {
            return Err(r.fail("a line break or the end of input"));
        }
        newlines(r);
        Ok(())
    }
    fn label(r: &mut Reader) -> Result<Option<String>, PelersError> {
        if !r.accept(OPEN) {
            return Ok(None);
        }
        let text = string_value(&r.expect(STRING_KIND)?);
        r.expect(CLOSE)?;
        Ok(Some(text))
    }
    fn entry(r: &mut Reader, detailed: bool) -> Result<Entry, PelersError> {
        if !detailed {
            return Ok(Entry {
                axis: None,
                value: js_number(&r.expect(NUMBER)?),
            });
        }
        let axis = r.expect(ID)?;
        r.accept(COLON);
        Ok(Entry {
            axis: Some(axis),
            value: js_number(&r.expect(NUMBER)?),
        })
    }
    fn curve(r: &mut Reader) -> Result<Curve, PelersError> {
        let name = r.expect(ID)?;
        let text = label(r)?;
        r.expect(OPEN_BRACE)?;
        newlines(r);
        if r.kind() != NUMBER && r.kind() != ID {
            return Err(r.fail("a number or an axis name"));
        }
        let detailed = r.kind() == ID;
        let mut node = Curve {
            name,
            label: text,
            entries: Vec::new(),
        };
        node.entries.push(entry(r, detailed)?);
        while r.accept(COMMA) {
            newlines(r);
            node.entries.push(entry(r, detailed)?);
        }
        newlines(r);
        r.expect(CLOSE_BRACE)?;
        Ok(node)
    }
    fn axis(r: &mut Reader) -> Result<Axis, PelersError> {
        let name = r.expect(ID)?;
        let text = label(r)?;
        Ok(Axis { name, label: text })
    }
    fn option(r: &mut Reader) -> Result<OptionAst, PelersError> {
        let kind = r.kind();
        let name = r.take();
        let value = if kind == SHOW_LEGEND {
            OptionValue::Bool(r.expect(BOOLEAN)? == "true")
        } else if kind == GRATICULE {
            OptionValue::Text(r.expect(GRATICULE_VALUE)?)
        } else {
            OptionValue::Number(js_number(&r.expect(NUMBER)?))
        };
        Ok(OptionAst { name, value })
    }

    newlines(&mut r);
    if !r.accept(RADARCOLON) {
        r.expect(RADAR)?;
        r.accept(COLON);
    }
    newlines(&mut r);
    while r.kind() != -1 {
        match r.kind() {
            NEWLINE_KIND => r.i += 1,
            ACC_DESCR_KIND => {
                ast.acc_descr = Some(acc_descr_value(&r.take()));
                end_of_line(&mut r)?;
            }
            ACC_TITLE_KIND => {
                ast.acc_title = Some(acc_title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            TITLE => {
                ast.title = Some(title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            AXIS => {
                r.i += 1;
                loop {
                    ast.axes.push(axis(&mut r)?);
                    if !r.accept(COMMA) {
                        break;
                    }
                }
            }
            CURVE => {
                r.i += 1;
                loop {
                    ast.curves.push(curve(&mut r)?);
                    if !r.accept(COMMA) {
                        break;
                    }
                }
            }
            _ => {
                if !is_option(r.kind()) {
                    return Err(r.fail("an axis, a curve, an option, a title, or a line break"));
                }
                ast.options.push(option(&mut r)?);
                while r.accept(COMMA) {
                    if !is_option(r.kind()) {
                        return Err(r.fail("an option"));
                    }
                    ast.options.push(option(&mut r)?);
                }
            }
        }
    }
    Ok(ast)
}
