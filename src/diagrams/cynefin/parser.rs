use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, NEWLINE, Pattern, Reader, SINGLE_LINE_COMMENT, STRING, TITLE,
    TokenType, WHITESPACE, YAML, acc_descr_value, acc_title_value, keyword, string_value,
    title_value, tokenize,
};
use crate::errors::PelersError;

pub mod kind {
    pub const KEYWORD_COLON: i32 = 0;
    pub const KEYWORD: i32 = 1;
    pub const ARROW: i32 = 2;
    pub const COLON: i32 = 3;
    pub const DOMAIN: i32 = 4;
    pub const ACC_DESCR: i32 = 5;
    pub const ACC_TITLE: i32 = 6;
    pub const TITLE: i32 = 7;
    pub const STRING: i32 = 8;
    pub const NEWLINE: i32 = 9;
}

const DOMAIN_NAMES: [&str; 5] = ["complex", "complicated", "clear", "chaotic", "confusion"];

fn domain(src: &str, at: usize) -> Option<usize> {
    DOMAIN_NAMES
        .iter()
        .find(|name| src[at..].starts_with(*name))
        .map(|name| at + name.len())
}

const fn literal(name: &'static str) -> TokenType {
    TokenType::new(name, Pattern::Literal(name), None)
}

pub const CYNEFIN_TOKENS: &[TokenType] = &[
    literal("cynefin-beta:"),
    keyword("cynefin-beta"),
    literal("-->"),
    literal(":"),
    TokenType::new("DOMAIN_NAME", Pattern::Match(domain), None),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DomainName {
    #[default]
    Complex,
    Complicated,
    Clear,
    Chaotic,
    Confusion,
}

impl DomainName {
    pub fn as_str(self) -> &'static str {
        match self {
            DomainName::Complex => "complex",
            DomainName::Complicated => "complicated",
            DomainName::Clear => "clear",
            DomainName::Chaotic => "chaotic",
            DomainName::Confusion => "confusion",
        }
    }

    fn parse(text: &str) -> DomainName {
        match text {
            "complex" => DomainName::Complex,
            "complicated" => DomainName::Complicated,
            "clear" => DomainName::Clear,
            "chaotic" => DomainName::Chaotic,
            _ => DomainName::Confusion,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DomainBlockAst {
    pub name: DomainName,
    pub items: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransitionAst {
    pub from: DomainName,
    pub to: DomainName,
    pub label: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CynefinAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub domains: Vec<DomainBlockAst>,
    pub transitions: Vec<TransitionAst>,
}

fn skip_lines(r: &mut Reader<'_>) {
    while r.kind_at(r.i) == kind::NEWLINE {
        r.i += 1;
    }
}

fn end_of_line(r: &mut Reader<'_>) -> Result<(), PelersError> {
    if r.kind() == -1 {
        return Ok(());
    }
    if r.kind() != kind::NEWLINE {
        return Err(r.fail("a line break or the end of input"));
    }
    skip_lines(r);
    Ok(())
}

pub fn parse_cynefin(src: &str) -> Result<CynefinAst, PelersError> {
    let tokens = tokenize(src, CYNEFIN_TOKENS, "cynefin")?;
    let mut r = Reader::new(tokens, CYNEFIN_TOKENS, "cynefin");
    let mut ast = CynefinAst::default();

    skip_lines(&mut r);
    if r.kind() != kind::KEYWORD && r.kind() != kind::KEYWORD_COLON {
        return Err(r.fail("'cynefin-beta'"));
    }
    r.i += 1;
    while r.kind() != -1 {
        match r.kind() {
            kind::NEWLINE => r.i += 1,
            kind::ACC_DESCR => {
                ast.acc_descr = Some(acc_descr_value(&r.take()));
                end_of_line(&mut r)?;
            }
            kind::ACC_TITLE => {
                ast.acc_title = Some(acc_title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            kind::TITLE => {
                ast.title = Some(title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            kind::DOMAIN => {
                let name = DomainName::parse(&r.take());
                if r.accept(kind::ARROW) {
                    let to = DomainName::parse(&r.expect(kind::DOMAIN)?);
                    let mut label = None;
                    if r.accept(kind::COLON) {
                        label = Some(string_value(&r.expect(kind::STRING)?));
                    }
                    end_of_line(&mut r)?;
                    ast.transitions.push(TransitionAst {
                        from: name,
                        to,
                        label,
                    });
                } else {
                    let mut items = Vec::new();
                    skip_lines(&mut r);
                    while r.kind_at(r.i) == kind::STRING {
                        items.push(string_value(&r.take()));
                        skip_lines(&mut r);
                    }
                    ast.domains.push(DomainBlockAst { name, items });
                }
            }
            _ => {
                return Err(r.fail("a domain, a transition, a title, or a line break"));
            }
        }
    }
    Ok(ast)
}
