mod parser;
mod render;

use crate::errors::PeleError;

pub use parser::{InfoAst, parse_info};
pub use render::render_info;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
}

fn dedent_descr(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for c in text.chars() {
        if skipping && crate::util::chars::is_js_space(c) {
            continue;
        }
        skipping = c == '\n';
        out.push(c);
    }
    out
}

pub fn build_info(ast: InfoAst, title: Option<String>) -> InfoModel {
    InfoModel {
        title: ast.title.filter(|t| !t.is_empty()).or(title),
        acc_title: ast
            .acc_title
            .filter(|t| !t.is_empty())
            .map(|t| crate::util::chars::trim_start(&t).to_owned()),
        acc_descr: ast
            .acc_descr
            .filter(|d| !d.is_empty())
            .map(|d| dedent_descr(&d)),
    }
}

pub fn parse(source: &str, title: Option<String>) -> Result<InfoModel, PeleError> {
    Ok(build_info(parse_info(source)?, title))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_line_wins_over_front_matter() {
        assert_eq!(
            parse("info\ntitle Hi\n", Some("Front".into()))
                .unwrap()
                .title
                .as_deref(),
            Some("Hi")
        );
        assert_eq!(
            parse("info\n", Some("Front".into()))
                .unwrap()
                .title
                .as_deref(),
            Some("Front")
        );
        assert_eq!(parse("info showInfo\n", None).unwrap().title, None);
    }

    #[test]
    fn collapses_indented_description_lines() {
        let model = parse("info\naccDescr {\n  a\n   b\n}\n", None).unwrap();
        assert_eq!(model.acc_descr.as_deref(), Some("a\nb"));
    }

    #[test]
    fn rejects_text_after_the_header() {
        let error = parse("info\nnope\n", None).unwrap_err();
        assert!(error.message.starts_with("Lexical error on line 2."));
    }
}
