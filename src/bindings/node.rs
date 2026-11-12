use crate::{
    DiagramType, PelersError, PelersErrorCode, detect_diagram, parse, render as render_model,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Debug)]
#[napi(object)]
pub struct RenderResult {
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
}

fn code(error: &PelersError) -> &'static str {
    match error.code {
        PelersErrorCode::UnsupportedDiagram => "unsupported-diagram",
        PelersErrorCode::Syntax => "syntax",
        PelersErrorCode::Semantic => "semantic",
        PelersErrorCode::Limit => "limit",
    }
}

fn to_napi(error: PelersError) -> Error<String> {
    Error::new(code(&error).to_owned(), error.message)
}

fn render_catching_panics(source: &str) -> std::result::Result<crate::RenderResult, PelersError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parse(source).and_then(|model| render_model(&model))
    }))
    .unwrap_or_else(|panic_info| {
        let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = panic_info.downcast_ref::<&str>() {
            (*s).to_owned()
        } else {
            "unknown panic".to_owned()
        };
        Err(PelersError::new(
            format!("render panicked: {msg}"),
            PelersErrorCode::Semantic,
        ))
    })
}

#[napi]
pub fn render(source: String) -> std::result::Result<String, Error<String>> {
    Ok(render_catching_panics(&source).map_err(to_napi)?.svg)
}

#[napi(js_name = "renderFull")]
pub fn render_full(source: String) -> std::result::Result<RenderResult, Error<String>> {
    let rendered = render_catching_panics(&source).map_err(to_napi)?;
    Ok(RenderResult {
        svg: rendered.svg,
        width: rendered.width,
        height: rendered.height,
        title: rendered.title,
    })
}

#[napi(js_name = "detectType")]
pub fn detect_type(source: String) -> Option<String> {
    detect_diagram(&source).map(|found| found.diagram_type.as_str().to_owned())
}

#[napi]
pub fn supports(source: String) -> bool {
    detect_diagram(&source).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_flowchart() {
        let svg = render("flowchart LR\n  A --> B".to_owned()).unwrap();
        assert!(svg.starts_with("<svg"));
        let full = render_full("flowchart LR\n  A --> B".to_owned()).unwrap();
        assert!(full.svg.starts_with("<svg") && full.width > 0.0 && full.height > 0.0);
    }

    #[test]
    fn rejects_unknown_with_a_code() {
        let error = render("hello".to_owned()).unwrap_err();
        assert_eq!(error.status, "unsupported-diagram");
    }

    #[test]
    fn detects_and_supports() {
        assert_eq!(
            detect_type("pie\n \"a\": 1".to_owned()).as_deref(),
            Some("pie")
        );
        assert!(supports("graph TD\n A-->B".to_owned()));
        assert!(supports("pie\n \"a\": 1".to_owned()));
        assert!(supports("sequenceDiagram\n  A->>B: hi".to_owned()));
        assert!(!supports("hello".to_owned()));
    }
}
