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
    let end = crate::util::chars::trim_end(source).len();
    let offset = if end > 0 && offset >= end {
        end
    } else {
        offset.min(source.len())
    };
    let line = source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[offset..]
        .find('\n')
        .map_or(source.len(), |index| offset + index);
    let column = source[line_start..offset].encode_utf16().count() + 1;
    let text = &source[line_start..line_end];
    (line, column, format!("{text}\n{}^", "-".repeat(column - 1)))
}

fn located(
    diagram_type: String,
    source: &str,
    offset: usize,
    head: impl FnOnce(usize) -> String,
    tail: &str,
) -> PeleError {
    let (line, column, snippet) = locate(source, offset);
    let mut message = format!("{}\n{snippet}", head(line));
    if !tail.is_empty() {
        message.push('\n');
        message.push_str(tail);
    }
    PeleError {
        message,
        code: PeleErrorCode::Syntax,
        diagram_type: Some(diagram_type),
        mermaid_type: None,
        line,
        column,
        snippet,
    }
}

pub fn syntax_error(
    diagram_type: impl Into<String>,
    source: &str,
    offset: usize,
    detail: impl AsRef<str>,
) -> PeleError {
    located(
        diagram_type.into(),
        source,
        offset,
        |line| format!("Parse error on line {line}:"),
        detail.as_ref(),
    )
}

pub fn lexical_error(diagram_type: impl Into<String>, source: &str, offset: usize) -> PeleError {
    located(
        diagram_type.into(),
        source,
        offset,
        |line| format!("Lexical error on line {line}. Unrecognized text."),
        "",
    )
}

#[cfg(test)]
mod tests {
    use super::{PeleErrorCode, lexical_error, locate, syntax_error};

    #[test]
    fn locates_line_and_column() {
        assert_eq!(locate("a\nbc", 3), (2, 2, "bc\n-^".to_string()));
    }

    #[test]
    fn keeps_eof_errors_on_the_last_content_line() {
        assert_eq!(locate("ab\n\n", 5), (1, 3, "ab\n--^".to_string()));
    }

    #[test]
    fn counts_columns_in_utf16_units() {
        assert_eq!(locate("\u{1F600}x", 4).1, 3);
    }

    #[test]
    fn words_a_lexical_error() {
        let error = lexical_error("info", "info\n?", 5);
        assert_eq!(
            error.message,
            "Lexical error on line 2. Unrecognized text.\n?\n^"
        );
    }

    #[test]
    fn builds_structured_syntax_error() {
        let error = syntax_error("flowchart", "A\nB", 2, "expected edge");
        assert_eq!(error.code, PeleErrorCode::Syntax);
        assert_eq!(error.line, 2);
        assert_eq!(error.column, 1);
    }
}
