use super::db::VennDb;
use super::lexer::{
    BRACKET_LABEL, COLON, COMMA, EOF, ERROR, IDENTIFIER, INDENT_TEXT, Lexer, NEWLINE, NUMERIC,
    RGBACOLOR, SET, STRING, STYLE, TEXT, TITLE, TOKEN_NAMES, UNION, VENN,
};
use crate::errors::{PelersError, PelersErrorCode, lexical_error, syntax_error};
use crate::util::chars::utf16_to_byte;
use crate::util::jsnum::js_parse_float;

const STATEMENT: &str = "'EOF', 'NEWLINE', 'TITLE', 'SET', 'UNION', 'TEXT', 'INDENT_TEXT', 'STYLE'";
const VALUE: &str = "'NUMERIC', 'IDENTIFIER', 'STRING', 'HEXCOLOR', 'RGBCOLOR', 'RGBACOLOR'";

fn semantic(message: &str) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("venn".to_owned());
    error
}

struct Parser<'a> {
    source: &'a str,
    lexer: Lexer,
    kind: i32,
    db: &'a mut VennDb,
}

impl Parser<'_> {
    fn peek(&mut self) -> i32 {
        if self.kind == -1 {
            self.kind = self.lexer.next();
        }
        self.kind
    }

    fn fail(&mut self, expected: &str) -> PelersError {
        let kind = self.peek();
        let offset = utf16_to_byte(self.source, self.lexer.start);
        if kind == ERROR {
            return lexical_error("venn", self.source, offset);
        }
        syntax_error(
            "venn",
            self.source,
            offset,
            format!(
                "Expecting {expected}, got '{}'",
                TOKEN_NAMES[self.kind as usize]
            ),
        )
    }

    fn take(&mut self) -> String {
        self.kind = -1;
        self.lexer.text.clone()
    }

    fn expect(&mut self, wanted: i32) -> Result<String, PelersError> {
        if self.peek() != wanted {
            return Err(self.fail(&format!("'{}'", TOKEN_NAMES[wanted as usize])));
        }
        Ok(self.take())
    }

    fn identifier(&mut self) -> Result<String, PelersError> {
        let kind = self.peek();
        if kind != IDENTIFIER && kind != STRING {
            return Err(self.fail("'IDENTIFIER', 'STRING'"));
        }
        Ok(self.take())
    }

    fn identifiers(&mut self) -> Result<Vec<String>, PelersError> {
        let mut list = vec![self.identifier()?];
        while self.peek() == COMMA {
            self.take();
            list.push(self.identifier()?);
        }
        Ok(list)
    }

    fn subset(&mut self, ids: Vec<String>, union: bool) -> Result<(), PelersError> {
        let label = if self.peek() == BRACKET_LABEL {
            Some(self.take())
        } else {
            None
        };
        let mut size = None;
        if self.peek() == COLON {
            self.take();
            size = Some(js_parse_float(&self.expect(NUMERIC)?));
        }
        if union {
            if ids.len() < 2 {
                return Err(semantic("union requires multiple identifiers"));
            }
            self.db.validate_union(&ids)?;
        }
        self.db.add_subset(&ids, label, size);
        self.lexer.set_indent(true);
        Ok(())
    }

    fn text(&mut self, sets: Option<Vec<String>>) -> Result<(), PelersError> {
        let kind = self.peek();
        if kind != IDENTIFIER && kind != STRING && kind != NUMERIC {
            return Err(self.fail("'NUMERIC', 'IDENTIFIER', 'STRING'"));
        }
        let id = self.take();
        let label = if kind != NUMERIC && self.peek() == BRACKET_LABEL {
            Some(self.take())
        } else {
            None
        };
        let ids = match sets {
            Some(ids) => ids,
            None => match self.db.current_sets() {
                Some(current) => current.clone(),
                None => return Err(semantic("text requires set")),
            },
        };
        self.db.add_text(ids, &id, label);
        Ok(())
    }

    fn style(&mut self) -> Result<(), PelersError> {
        let ids = self.identifiers()?;
        let mut styles: Vec<(String, String)> = Vec::new();
        loop {
            let key = self.expect(IDENTIFIER)?;
            self.expect(COLON)?;
            let value = if self.peek() == STRING {
                self.take()
            } else {
                let mut parts: Vec<String> = Vec::new();
                loop {
                    let kind = self.peek();
                    if kind != IDENTIFIER && !(NUMERIC..=RGBACOLOR).contains(&kind) {
                        break;
                    }
                    parts.push(self.take());
                }
                if parts.is_empty() {
                    return Err(self.fail(VALUE));
                }
                parts.join(" ")
            };
            styles.push((key, value));
            if self.peek() != COMMA {
                break;
            }
            self.take();
        }
        self.db.add_style(&ids, styles.into_iter().collect());
        Ok(())
    }
}

pub fn parse_venn(source: &str, db: &mut VennDb) -> Result<(), PelersError> {
    let mut parser = Parser {
        source,
        lexer: Lexer::new(source),
        kind: -1,
        db,
    };
    while parser.peek() == NEWLINE {
        parser.take();
    }
    parser.expect(VENN)?;
    loop {
        let statement = parser.peek();
        if statement == EOF {
            return Ok(());
        }
        if !(NEWLINE..=STYLE).contains(&statement) || statement == VENN {
            return Err(parser.fail(STATEMENT));
        }
        parser.take();
        if statement == TITLE {
            let text = parser.lexer.text.clone();
            parser.db.title = Some(text.chars().skip(6).collect());
        } else if statement == SET {
            let id = parser.identifier()?;
            parser.subset(vec![id], false)?;
        } else if statement == UNION {
            let ids = parser.identifiers()?;
            parser.subset(ids, true)?;
        } else if statement == TEXT {
            let ids = parser.identifiers()?;
            parser.text(Some(ids))?;
        } else if statement == INDENT_TEXT {
            parser.text(None)?;
        } else if statement == STYLE {
            parser.style()?;
        }
    }
}
