#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeleErrorCode {
    UnsupportedDiagram,
    Syntax,
    Semantic,
    Limit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeleError {
    pub message: String,
    pub code: PeleErrorCode,
    pub diagram_type: Option<String>,
    pub mermaid_type: Option<String>,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
}

impl PeleError {
    pub fn new(message: impl Into<String>, code: PeleErrorCode) -> Self {
        Self {
            message: message.into(),
            code,
            diagram_type: None,
            mermaid_type: None,
            line: 0,
            column: 0,
            snippet: String::new(),
        }
    }
}

impl std::fmt::Display for PeleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PeleError {}

pub fn locate(source: &str, offset: usize) -> (usize, usize, String) {
    let offset = offset.min(source.len());
    let line = source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[offset..]
        .find('\n')
        .map_or(source.len(), |index| offset + index);
    let column = offset - line_start + 1;
    let text = &source[line_start..line_end];
    (
        line,
        column,
        format!("{text}\n{}^", "-".repeat(column.saturating_sub(1))),
    )
}

pub fn syntax_error(
    diagram_type: impl Into<String>,
    source: &str,
    offset: usize,
    detail: impl Into<String>,
) -> PeleError {
    let (line, column, snippet) = locate(source, offset);
    let message = format!("Parse error on line {line}:\n{snippet}\n{}", detail.into());
    PeleError {
        message,
        code: PeleErrorCode::Syntax,
        diagram_type: Some(diagram_type.into()),
        mermaid_type: None,
        line,
        column,
        snippet,
    }
}

#[cfg(test)]
mod tests {
    use super::{PeleErrorCode, locate, syntax_error};

    #[test]
    fn locates_line_and_column() {
        assert_eq!(locate("a\nbc", 3), (2, 2, "bc\n-^".to_string()));
    }

    #[test]
    fn builds_structured_syntax_error() {
        let error = syntax_error("flowchart", "A\nB", 2, "expected edge");
        assert_eq!(error.code, PeleErrorCode::Syntax);
        assert_eq!(error.line, 2);
        assert_eq!(error.column, 1);
    }
}
