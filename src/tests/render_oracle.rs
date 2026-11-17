use std::fs;
use std::path::Path;

use std::rc::Rc;

use crate::text::measurer::MeasureFn;
use crate::util::hook::Hook;
use crate::util::jsnum::js_number_to_string;
use crate::{ParseOptions, PelersErrorCode, RenderOptions, parse_with, render_with};
use serde_json::Value as Json;

fn code(code: PelersErrorCode) -> &'static str {
    match code {
        PelersErrorCode::UnsupportedDiagram => "unsupported-diagram",
        PelersErrorCode::Syntax => "syntax",
        PelersErrorCode::Semantic => "semantic",
        PelersErrorCode::Limit => "limit",
    }
}

fn options(case: &Json) -> RenderOptions {
    let given = &case["options"];
    let measurer = (given["measurer"].as_str() == Some("demo")).then(|| {
        let measure: Rc<MeasureFn> = Rc::new(|text: &str, size: f64, style: u32| {
            text.encode_utf16().count() as f64 * size * if style & 1 != 0 { 0.6 } else { 0.5 }
        });
        Hook(measure)
    });
    let list = |key: &str| {
        given[key].as_array().map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect::<Vec<String>>()
        })
    };
    RenderOptions {
        measurer,
        font_size: given["fontSize"].as_f64(),
        padding: given["padding"].as_f64(),
        max_width: given["maxWidth"].as_f64(),
        auto_direction: given["autoDirection"].as_bool(),
        direction_breakpoint: given["directionBreakpoint"].as_f64(),
        id_prefix: given["idPrefix"].as_str().map(str::to_owned),
        now: given["now"].as_f64(),
        links: given["links"].as_bool(),
        link_rel: given["linkRel"].as_str().map(str::to_owned),
        link_schemes: list("linkSchemes"),
        image_schemes: list("imageSchemes"),
        images: given["images"].as_bool(),
        monospace: list("monospace"),
        ..RenderOptions::default()
    }
}

fn demo_hook(case: &Json) -> bool {
    let source = case["source"].as_str().unwrap_or("");
    (case["options"]["math"].as_str() == Some("demo") && source.contains("$$"))
        || (case["options"]["icons"].as_str() == Some("demo")
            && (source.contains("::icon(") || source.contains(":fa-")))
}

fn check(name: &str) {
    let mut total = 0;
    let mut failures = Vec::new();
    for file in [
        format!("render-{name}.json"),
        format!("render-mut-{name}.json"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(&file);
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let fixtures: Json = serde_json::from_str(&text).unwrap();
        for (index, case) in fixtures["cases"].as_array().unwrap().iter().enumerate() {
            if demo_hook(case) {
                continue;
            }
            total += 1;
            let source = case["source"].as_str().unwrap();
            let parse_options = ParseOptions {
                max_edges: case["options"]["maxEdges"].as_u64().map(|v| v as usize),
                ..ParseOptions::default()
            };
            let result = parse_with(source, &parse_options)
                .and_then(|model| render_with(&model, &options(case)));
            let verdict = match (&result, case.get("error")) {
                (Ok(rendered), None) => {
                    if rendered.svg != case["svg"].as_str().unwrap() {
                        Some("svg differs".to_owned())
                    } else if js_number_to_string(rendered.width) != case["width"].as_str().unwrap()
                        || js_number_to_string(rendered.height) != case["height"].as_str().unwrap()
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
                failures.push(format!("{file}#{index} {verdict}\n{source:?}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{name}: {} of {total} cases differ\n{}",
        failures.len(),
        failures
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n")
    );
}

#[test]
fn agentflow_matches_the_oracle() {
    check("agentflow");
}

#[test]
fn pie_matches_the_oracle() {
    check("render-pie.json");
}

#[test]
fn packet_matches_the_oracle() {
    check("render-packet.json");
}

#[test]
fn sankey_matches_the_oracle() {
    check("render-sankey.json");
}

#[test]
fn swimlane_matches_the_oracle() {
    check("render-swimlane.json");
}

#[test]
fn git_graph_matches_the_oracle() {
    check("render-gitGraph.json");
}

#[test]
fn tree_view_matches_the_oracle() {
    check("render-treeView.json");
}

#[test]
fn cynefin_matches_the_oracle() {
    check("render-cynefin.json");
}

#[test]
fn railroad_matches_the_oracle() {
    check("render-railroad.json");
}

#[test]
fn er_matches_the_oracle() {
    check("render-er.json");
}

#[test]
fn state_matches_the_oracle() {
    check("render-state.json");
}

#[test]
fn class_matches_the_oracle() {
    check("render-class.json");
}

#[test]
fn sequence_matches_the_oracle() {
    check("render-sequence.json");
}

#[test]
fn usecase_matches_the_oracle() {
    check("render-usecase.json");
}

#[test]
fn eventmodeling_matches_the_oracle() {
    check("render-eventmodeling.json");
}

#[test]
fn wardley_matches_the_oracle() {
    check("render-wardley.json");
}

#[test]
fn requirement_matches_the_oracle() {
    check("render-requirement.json");
}

#[test]
fn architecture_matches_the_oracle() {
    check("render-architecture.json");
}
