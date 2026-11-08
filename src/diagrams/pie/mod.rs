mod parser;
mod render;

use std::collections::HashSet;

use crate::diagrams::common::{acc_descr, acc_title};
use crate::errors::{PeleError, PeleErrorCode};
use crate::util::jsnum::js_number_to_string;

pub use parser::{PieAst, parse_pie};
pub use render::render_pie;

#[derive(Debug, Clone, PartialEq)]
pub struct PieModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub show_data: bool,
    pub sections: Vec<(String, f64)>,
}

pub fn build_pie(ast: PieAst, title: Option<String>) -> Result<PieModel, PeleError> {
    let mut seen = HashSet::new();
    let mut sections = Vec::new();
    for (label, value) in ast.sections {
        if value < 0.0 {
            let mut error = PeleError::new(
                format!(
                    "\"{label}\" has invalid value: {}. Negative values are not allowed in pie charts. All slice values must be >= 0.",
                    js_number_to_string(value)
                ),
                PeleErrorCode::Semantic,
            );
            error.diagram_type = Some("pie".to_owned());
            return Err(error);
        }
        if seen.insert(label.clone()) {
            sections.push((label, value));
        }
    }
    Ok(PieModel {
        title: ast.title.filter(|t| !t.is_empty()).or(title),
        acc_title: acc_title(ast.acc_title),
        acc_descr: acc_descr(ast.acc_descr),
        show_data: ast.show_data,
        sections,
    })
}

pub fn parse(source: &str, title: Option<String>) -> Result<PieModel, PeleError> {
    build_pie(parse_pie(source)?, title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_first_of_duplicate_labels() {
        let model = parse("pie\n\"a\" : 1\n\"b\" : 2\n\"a\" : 3\n", None).unwrap();
        assert_eq!(
            model.sections,
            vec![("a".to_owned(), 1.0), ("b".to_owned(), 2.0)]
        );
    }

    #[test]
    fn rejects_negative_values() {
        let error = parse("pie\n\"a\" : -2\n", None).unwrap_err();
        assert_eq!(error.code, PeleErrorCode::Semantic);
        assert_eq!(
            error.message,
            "\"a\" has invalid value: -2. Negative values are not allowed in pie charts. All slice values must be >= 0."
        );
    }

    #[test]
    fn the_title_line_wins_over_front_matter() {
        assert_eq!(
            parse("pie title T\n", Some("F".into()))
                .unwrap()
                .title
                .as_deref(),
            Some("T")
        );
        assert_eq!(
            parse("pie\n", Some("F".into())).unwrap().title.as_deref(),
            Some("F")
        );
    }
}
