use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, NEWLINE, Pattern, Reader, SINGLE_LINE_COMMENT, STRING, TITLE,
    TokenType, WHITESPACE, YAML, acc_descr_value, acc_title_value, keyword, string_value,
    title_value, tokenize,
};
use crate::errors::PeleError;

const PACKET_BETA: i32 = 0;
const PACKET: i32 = 1;
const DASH: i32 = 2;
const PLUS: i32 = 3;
const COLON: i32 = 4;
const T_ACC_DESCR: i32 = 5;
const T_ACC_TITLE: i32 = 6;
const T_TITLE: i32 = 7;
const INT: i32 = 8;
const T_STRING: i32 = 9;
const T_NEWLINE: i32 = 10;

fn int(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    match bytes.get(at) {
        Some(b'0') => Some(at + 1),
        Some(b'1'..=b'9') => {
            let end = at
                + bytes[at..]
                    .iter()
                    .take_while(|b| b.is_ascii_digit())
                    .count();
            if bytes.get(end) != Some(&b'.') {
                Some(end)
            } else {
                (end - 1 > at).then_some(end - 1)
            }
        }
        _ => None,
    }
}

pub const PACKET_TOKENS: [TokenType; 15] = [
    TokenType::new("packet-beta", Pattern::Literal("packet-beta"), None),
    keyword("packet"),
    TokenType {
        longer: &["YAML"],
        ..TokenType::new("-", Pattern::Literal("-"), None)
    },
    TokenType::new("+", Pattern::Literal("+"), None),
    TokenType::new(":", Pattern::Literal(":"), None),
    ACC_DESCR,
    ACC_TITLE,
    TITLE,
    TokenType::new("INT", Pattern::Match(int), None),
    STRING,
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PacketBlockAst {
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub bits: Option<f64>,
    pub label: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PacketAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub blocks: Vec<PacketBlockAst>,
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

fn number(text: &str) -> f64 {
    text.parse::<f64>()
        .expect("an INT token is a decimal number")
}

pub fn parse_packet(src: &str) -> Result<PacketAst, PeleError> {
    let mut r = Reader::new(
        tokenize(src, &PACKET_TOKENS, "packet")?,
        &PACKET_TOKENS,
        "packet",
    );
    let mut ast = PacketAst::default();

    skip_newlines(&mut r);
    if !r.accept(PACKET_BETA) {
        r.expect(PACKET)?;
    }
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
            INT | PLUS => {
                let mut block = PacketBlockAst::default();
                if r.accept(PLUS) {
                    block.bits = Some(number(&r.expect(INT)?));
                } else {
                    block.start = Some(number(&r.take()));
                    if r.accept(DASH) {
                        block.end = Some(number(&r.expect(INT)?));
                    }
                }
                r.expect(COLON)?;
                block.label = string_value(&r.expect(T_STRING)?);
                ast.blocks.push(block);
                end_of_line(&mut r)?;
            }
            _ => return Err(r.fail("a field, a title, or a line break")),
        }
    }
    Ok(ast)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_ints_as_the_grammar_does() {
        let cases: [(&str, Option<usize>); 8] = [
            ("0", Some(1)),
            ("0.5", Some(1)),
            ("012", Some(1)),
            ("15", Some(2)),
            ("15.", Some(1)),
            ("5.", None),
            ("-1", None),
            ("x", None),
        ];
        for (src, expected) in cases {
            assert_eq!(int(src, 0), expected, "{src}");
        }
    }

    #[test]
    fn parses_all_three_field_forms() {
        let ast =
            parse_packet("packet-beta\ntitle T\n0-15: \"a\"\n16: \"b\"\n+8: \"c\"\n").unwrap();
        assert_eq!(ast.title.as_deref(), Some("T"));
        assert_eq!(
            ast.blocks,
            vec![
                PacketBlockAst {
                    start: Some(0.0),
                    end: Some(15.0),
                    bits: None,
                    label: "a".into()
                },
                PacketBlockAst {
                    start: Some(16.0),
                    end: None,
                    bits: None,
                    label: "b".into()
                },
                PacketBlockAst {
                    start: None,
                    end: None,
                    bits: Some(8.0),
                    label: "c".into()
                },
            ]
        );
    }

    #[test]
    fn front_matter_wins_over_a_dash() {
        assert!(parse_packet("---\ntitle: x\n---\npacket\n0: \"a\"\n").is_ok());
        assert!(parse_packet("packet\n0 - 1: \"a\"\n").is_ok());
        assert!(parse_packet("packet\n0 - : \"a\"\n").is_err());
    }
}
