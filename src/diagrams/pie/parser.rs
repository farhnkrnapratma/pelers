use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, NEWLINE, Pattern, Reader, SINGLE_LINE_COMMENT, STRING, TITLE,
    TokenType, WHITESPACE, YAML, acc_descr_value, acc_title_value, keyword, string_value,
    title_value, tokenize,
};
use crate::errors::PeleError;

const SHOW_DATA: i32 = 0;
const PIE: i32 = 1;
const COLON: i32 = 2;
const NUMBER: i32 = 3;
const T_ACC_DESCR: i32 = 4;
const T_ACC_TITLE: i32 = 5;
const T_TITLE: i32 = 6;
const T_STRING: i32 = 7;
const T_NEWLINE: i32 = 8;

fn digits(bytes: &[u8], from: usize) -> usize {
    from + bytes[from..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count()
}

fn number(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let at = if bytes.get(at) == Some(&b'-') {
        at + 1
    } else {
        at
    };
    let end = digits(bytes, at);
    if end == at {
        return None;
    }
    if bytes.get(end) == Some(&b'.') {
        let stop = digits(bytes, end + 1);
        let length = stop - end - 1;
        if length > 0 && bytes.get(stop) != Some(&b'.') {
            return Some(stop);
        }
        if length > 1 {
            return Some(stop - 1);
        }
    }
    if bytes[at] == b'0' {
        return (bytes.get(at + 1) != Some(&b'.')).then_some(at + 1);
    }
    if bytes.get(end) != Some(&b'.') {
        return Some(end);
    }
    (end - 1 > at).then_some(end - 1)
}

pub const PIE_TOKENS: [TokenType; 13] = [
    keyword("showData"),
    keyword("pie"),
    TokenType::new(":", Pattern::Literal(":"), None),
    TokenType::new("NUMBER_PIE", Pattern::Match(number), Some("-0123456789")),
    ACC_DESCR,
    ACC_TITLE,
    TITLE,
    STRING,
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PieAst {
    pub show_data: bool,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub sections: Vec<(String, f64)>,
}

fn skip_newlines(r: &mut Reader) {
    while r.kind() == T_NEWLINE {
        r.i += 1;
    }
}

fn end_of_line(r: &mut Reader) -> Result<(), PeleError> {
    if r.kind() == -1 {
        return Ok(());
    }
    if r.kind() != T_NEWLINE {
        return Err(r.fail("a line break or the end of input"));
    }
    skip_newlines(r);
    Ok(())
}

pub fn parse_pie(src: &str) -> Result<PieAst, PeleError> {
    let mut r = Reader::new(tokenize(src, &PIE_TOKENS, "pie")?, &PIE_TOKENS, "pie");
    let mut ast = PieAst::default();

    skip_newlines(&mut r);
    r.expect(PIE)?;
    ast.show_data = r.accept(SHOW_DATA);
    while r.kind() != -1 {
        match r.kind() {
            T_NEWLINE => r.i += 1,
            T_ACC_DESCR => {
                ast.acc_descr = Some(acc_descr_value(&r.take()));
                end_of_line(&mut r)?;
            }
            T_ACC_TITLE => {
                ast.acc_title = Some(acc_title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            T_TITLE => {
                ast.title = Some(title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            T_STRING => {
                let label = string_value(&r.take());
                r.expect(COLON)?;
                let value = r
                    .expect(NUMBER)?
                    .parse::<f64>()
                    .expect("a NUMBER_PIE token is a decimal number");
                ast.sections.push((label, value));
                end_of_line(&mut r)?;
            }
            _ => return Err(r.fail("a section, a title, or a line break")),
        }
    }
    Ok(ast)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_numbers_as_the_grammar_does() {
        let cases: [(&str, Option<usize>); 11] = [
            ("1.5", Some(3)),
            ("-0.25", Some(5)),
            ("12", Some(2)),
            ("007", Some(1)),
            ("0.", None),
            ("1.5.5", None),
            ("3.45.6", Some(3)),
            ("10.", Some(1)),
            ("1.", None),
            ("-", None),
            ("x", None),
        ];
        for (src, expected) in cases {
            assert_eq!(number(src, 0), expected, "{src}");
        }
    }

    #[test]
    fn parses_sections_titles_and_options() {
        let ast =
            parse_pie("pie showData\ntitle Pets\n\"Dogs\" : 386\n'Cats':85.5\n\"a\\\"b\" : 0\n")
                .unwrap();
        assert!(ast.show_data);
        assert_eq!(ast.title.as_deref(), Some("Pets"));
        assert_eq!(
            ast.sections,
            vec![
                ("Dogs".to_owned(), 386.0),
                ("Cats".to_owned(), 85.5),
                ("a\"b".to_owned(), 0.0)
            ]
        );
    }

    #[test]
    fn reports_a_missing_value() {
        let error = parse_pie("pie\n\"a\" :\n").unwrap_err();
        assert!(
            error
                .message
                .contains("Expecting token of type 'NUMBER_PIE'")
        );
    }

    #[test]
    fn requires_the_header() {
        assert!(parse_pie("\"a\" : 1").is_err());
    }
}
