#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PelersErrorCode {
    UnsupportedDiagram,
    Syntax,
    Semantic,
    Limit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PelersError {
    pub message: String,
    pub code: PelersErrorCode,
    pub diagram_type: Option<String>,
    pub mermaid_type: Option<String>,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
}

impl PelersError {
    pub fn new(message: impl Into<String>, code: PelersErrorCode) -> Self {
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

impl std::fmt::Display for PelersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PelersError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub from: usize,
    pub to: usize,
    pub length: usize,
    pub text: Option<String>,
}

impl Edit {
    pub fn cut(from: usize, to: usize) -> Self {
        Self {
            from,
            to,
            length: 0,
            text: None,
        }
    }

    pub fn swap(from: usize, to: usize, text: String) -> Self {
        Self {
            from,
            to,
            length: text.len(),
            text: Some(text),
        }
    }

    pub fn resized(from: usize, to: usize, length: usize) -> Self {
        Self {
            from,
            to,
            length,
            text: None,
        }
    }
}

fn step_input(edits: &[Edit], at: usize) -> usize {
    let mut shift: i64 = 0;
    for edit in edits {
        let start = edit.from as i64 - shift;
        let into = at as i64 - start;
        if into < 0 {
            break;
        }
        if into < edit.length as i64 {
            let inside = match &edit.text {
                Some(text) => {
                    text.char_indices()
                        .take_while(|&(offset, _)| (offset as i64) <= into)
                        .count() as i64
                        - 1
                }
                None => into,
            };
            return edit.from + inside.min(edit.to as i64 - edit.from as i64 - 1).max(0) as usize;
        }
        shift += edit.to as i64 - edit.from as i64 - edit.length as i64;
    }
    (at as i64 + shift) as usize
}

pub fn input_offset(steps: &[Vec<Edit>], mut at: usize) -> usize {
    for edits in steps.iter().rev() {
        at = step_input(edits, at);
    }
    at
}

struct Origin {
    parsed: String,
    find: Box<dyn Fn(usize) -> (String, usize)>,
}

thread_local! {
    static ORIGIN: std::cell::RefCell<Option<Origin>> = const { std::cell::RefCell::new(None) };
}

struct Restore(Option<Origin>);

impl Drop for Restore {
    fn drop(&mut self) {
        ORIGIN.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}

pub fn locating_in<T>(
    parsed: String,
    find: impl Fn(usize) -> (String, usize) + 'static,
    run: impl FnOnce() -> T,
) -> T {
    let outer = ORIGIN.with(|slot| {
        slot.borrow_mut().replace(Origin {
            parsed,
            find: Box::new(find),
        })
    });
    let _restore = Restore(outer);
    run()
}

pub fn written(source: &str, offset: usize) -> (String, usize) {
    ORIGIN.with(|slot| match slot.borrow().as_ref() {
        Some(origin) if origin.parsed == source => (origin.find)(offset),
        _ => (source.to_owned(), offset),
    })
}

pub fn locate(source: &str, offset: usize) -> (usize, usize, String) {
    let end = crate::util::chars::trim_end(source).len();
    let (source, offset) = match source[..end].chars().next_back() {
        Some(last) if offset >= end => {
            let (mapped, at) = written(source, end - last.len_utf8());
            let step = mapped[at..].chars().next().map_or(1, char::len_utf8);
            (mapped, at + step)
        }
        _ => written(source, offset.min(source.len())),
    };
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
    let column = source[line_start..offset].encode_utf16().count() + 1;
    let text = &source[line_start..line_end];
    (line, column, format!("{text}\n{}^", "-".repeat(column - 1)))
}

fn located(
    diagram_type: String,
    source: &str,
    offset: usize,
    head: fn(usize) -> String,
    tail: &str,
) -> PelersError {
    let (line, column, snippet) = locate(source, offset);
    let mut message = format!("{}\n{snippet}", head(line));
    if !tail.is_empty() {
        message.push('\n');
        message.push_str(tail);
    }
    PelersError {
        message,
        code: PelersErrorCode::Syntax,
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
) -> PelersError {
    located(
        diagram_type.into(),
        source,
        offset,
        |line| format!("Parse error on line {line}:"),
        detail.as_ref(),
    )
}

pub fn lexical_error(diagram_type: impl Into<String>, source: &str, offset: usize) -> PelersError {
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
    use super::{
        Edit, PelersErrorCode, input_offset, lexical_error, locate, locating_in, syntax_error,
    };

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
    fn maps_offsets_back_through_recorded_edits() {
        let steps = vec![vec![Edit::cut(0, 5)], vec![Edit::resized(2, 3, 4)]];
        assert_eq!(input_offset(&steps, 0), 5);
        assert_eq!(input_offset(&steps, 2), 7);
        assert_eq!(input_offset(&steps, 4), 7);
        assert_eq!(input_offset(&steps, 7), 9);
    }

    #[test]
    fn reports_positions_in_the_original_source() {
        let source = "%% hi\ninfo\nx";
        let found = locating_in(
            "info\nx".to_owned(),
            {
                let source = source.to_owned();
                move |at| (source.clone(), at + 6)
            },
            || locate("info\nx", 5),
        );
        assert_eq!(found, (3, 1, "x\n^".to_owned()));
        assert_eq!(locate("info\nx", 5), (2, 1, "x\n^".to_owned()));
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
        assert_eq!(error.code, PelersErrorCode::Syntax);
        assert_eq!(error.line, 2);
        assert_eq!(error.column, 1);
    }

    #[test]
    fn displays_the_message() {
        let error = super::PelersError::new("boom", PelersErrorCode::Syntax);
        assert_eq!(error.to_string(), "boom");
        let boxed: &dyn std::error::Error = &error;
        assert_eq!(boxed.to_string(), "boom");
    }
}
