use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, NEWLINE, Reader, SINGLE_LINE_COMMENT, TITLE, TokenType,
    WHITESPACE, YAML, acc_descr_value, acc_title_value, keyword, title_value, tokenize,
};
use crate::errors::PeleError;

const SHOW_INFO: i32 = 0;
const INFO: i32 = 1;
const T_ACC_DESCR: i32 = 2;
const T_ACC_TITLE: i32 = 3;
const T_TITLE: i32 = 4;
const T_NEWLINE: i32 = 5;

pub const INFO_TOKENS: [TokenType; 10] = [
    keyword("showInfo"),
    keyword("info"),
    ACC_DESCR,
    ACC_TITLE,
    TITLE,
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InfoAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
}

fn skip_newlines(r: &mut Reader) {
    while r.kind() == T_NEWLINE {
        r.i += 1;
    }
}

pub fn parse_info(src: &str) -> Result<InfoAst, PeleError> {
    let mut r = Reader::new(tokenize(src, &INFO_TOKENS, "info")?, &INFO_TOKENS, "info");
    let mut ast = InfoAst::default();

    skip_newlines(&mut r);
    r.expect(INFO)?;
    skip_newlines(&mut r);
    if r.accept(SHOW_INFO) {
        skip_newlines(&mut r);
    }
    while r.kind() != -1 {
        match r.kind() {
            T_ACC_DESCR => ast.acc_descr = Some(acc_descr_value(&r.take())),
            T_ACC_TITLE => ast.acc_title = Some(acc_title_value(&r.take())),
            T_TITLE => ast.title = Some(title_value(&r.take())),
            _ => return Err(r.fail("a title or the end of input")),
        }
        if r.kind() == -1 {
            break;
        }
        if r.kind() != T_NEWLINE {
            return Err(r.fail("a line break or the end of input"));
        }
        skip_newlines(&mut r);
    }
    Ok(ast)
}
