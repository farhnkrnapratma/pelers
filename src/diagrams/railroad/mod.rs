mod abnf;
mod common;
mod ebnf;
mod ir;
mod peg;
mod render;
mod types;

pub use render::render_railroad;
pub use types::{RailroadModel, RailroadNode, RailroadRule};

use crate::diagrams::common::dedent_descr;
use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::is_js_space;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Notation {
    Railroad,
    Ebnf,
    Abnf,
    Peg,
}

fn strip_ci<'a>(source: &'a str, prefix: &str) -> Option<&'a str> {
    let head = source.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &source[prefix.len()..])
}

fn notation_of(source: &str) -> Option<Notation> {
    let rest = strip_ci(source.trim_start_matches(is_js_space), "railroad-")?;
    let (notation, rest) = [
        ("ebnf-", Notation::Ebnf),
        ("abnf-", Notation::Abnf),
        ("peg-", Notation::Peg),
    ]
    .into_iter()
    .find_map(|(infix, notation)| strip_ci(rest, infix).map(|tail| (notation, tail)))
    .unwrap_or((Notation::Railroad, rest));
    strip_ci(rest, "beta").map(|_| notation)
}

type Parts = (
    Option<String>,
    Option<String>,
    Option<String>,
    Vec<RailroadRule>,
);

fn grammar(source: &str, notation: Notation) -> Result<Parts, PelersError> {
    Ok(match notation {
        Notation::Ebnf => {
            let ast = ebnf::parse_ebnf(source)?;
            let rules = ebnf::ebnf_rules(&ast);
            (ast.title, ast.acc_title, ast.acc_descr, rules)
        }
        Notation::Abnf => {
            let ast = abnf::parse_abnf(source)?;
            let rules = abnf::abnf_rules(&ast);
            (ast.title, ast.acc_title, ast.acc_descr, rules)
        }
        Notation::Peg => {
            let ast = peg::parse_peg(source)?;
            let rules = peg::peg_rules(&ast);
            (ast.title, ast.acc_title, ast.acc_descr, rules)
        }
        Notation::Railroad => {
            let ast = ir::parse_ir(source)?;
            let rules = ir::ir_rules(&ast);
            (ast.title, ast.acc_title, ast.acc_descr, rules)
        }
    })
}

fn non_empty(text: Option<String>) -> Option<String> {
    text.filter(|t| !t.is_empty())
}

pub fn parse(source: &str, title: Option<String>) -> Result<RailroadModel, PelersError> {
    let Some(notation) = notation_of(source) else {
        let mut error = PelersError::new(
            "No railroad diagram keyword found.",
            PelersErrorCode::Syntax,
        );
        error.diagram_type = Some("railroad".to_owned());
        return Err(error);
    };
    let (ast_title, ast_acc_title, ast_acc_descr, rules) = grammar(source, notation)?;
    let mut model = RailroadModel {
        title: non_empty(title),
        acc_title: None,
        acc_descr: None,
        rules,
    };
    if let Some(descr) = non_empty(ast_acc_descr) {
        model.acc_descr = Some(dedent_descr(&descr));
    }
    if let Some(acc_title) = non_empty(ast_acc_title) {
        model.acc_title = Some(acc_title.trim_start_matches(is_js_space).to_owned());
    }
    if let Some(ast) = non_empty(ast_title) {
        model.title = Some(ast);
    }
    Ok(model)
}
