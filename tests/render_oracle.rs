use std::fs;
use std::path::Path;

use pelers::{PeleErrorCode, RenderOptions, parse, render_with};
use serde_json::Value as Json;

fn load(name: &str) -> Json {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn code(code: PeleErrorCode) -> &'static str {
    match code {
        PeleErrorCode::UnsupportedDiagram => "unsupported-diagram",
        PeleErrorCode::Syntax => "syntax",
        PeleErrorCode::Semantic => "semantic",
        PeleErrorCode::Limit => "limit",
    }
}

fn options(case: &Json) -> RenderOptions {
    let given = &case["options"];
    RenderOptions {
        font_size: given["fontSize"].as_f64(),
        padding: given["padding"].as_f64(),
        max_width: given["maxWidth"].as_f64(),
        auto_direction: given["autoDirection"].as_bool(),
        direction_breakpoint: given["directionBreakpoint"].as_f64(),
        ..RenderOptions::default()
    }
}

fn check(name: &str) {
    let fixtures = load(name);
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let source = case["source"].as_str().unwrap();
        let result = parse(source).and_then(|model| render_with(&model, &options(case)));
        let verdict = match (&result, case.get("error")) {
            (Ok(rendered), None) => {
                if rendered.svg != case["svg"].as_str().unwrap() {
                    Some("svg differs".to_owned())
                } else if rendered.width.to_string() != case["width"].as_str().unwrap()
                    || rendered.height.to_string() != case["height"].as_str().unwrap()
                {
                    Some("size differs".to_owned())
                } else {
                    None
                }
            }
            (Err(error), Some(expected)) => {
                let same = code(error.code) == expected["code"].as_str().unwrap()
                    && error.message == expected["message"].as_str().unwrap()
                    && expected["line"]
                        .as_u64()
                        .is_none_or(|line| line as usize == error.line)
                    && expected["column"]
                        .as_u64()
                        .is_none_or(|column| column as usize == error.column);
                (!same).then(|| format!("error differs: {error:?} vs {expected}"))
            }
            (Ok(_), Some(expected)) => Some(format!("rendered, oracle failed: {expected}")),
            (Err(error), None) => Some(format!("failed, oracle rendered: {error:?}")),
        };
        if let Some(verdict) = verdict {
            failures.push(format!("#{index} {verdict}\n{source:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{name}: {} of {} cases differ\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n")
    );
}

#[test]
fn pie_matches_the_oracle() {
    check("render-pie.json");
}

#[test]
fn packet_matches_the_oracle() {
    check("render-packet.json");
}
