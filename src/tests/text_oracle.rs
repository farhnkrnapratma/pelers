use std::fs;
use std::path::Path;

use crate::svg::builder::{esc, label_svg, num};
use crate::text::label::layout_label;
use crate::util::jsnum::js_number_to_string as js;
use serde_json::Value;

fn fixtures() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/text.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn labels_lay_out_and_draw_as_the_oracle_does() {
    let mut failures = Vec::new();
    for case in fixtures()["labels"].as_array().unwrap() {
        let raw = case["raw"].as_str().unwrap();
        let (markdown, size) = (
            case["markdown"].as_bool().unwrap(),
            case["size"].as_f64().unwrap(),
        );
        let (max, base) = (
            case["max"].as_f64().unwrap(),
            case["base"].as_u64().unwrap() as u32,
        );
        let label = layout_label(Some(raw), markdown, size, max, base);

        let lines: Vec<Vec<(String, u64, String)>> = label
            .lines
            .iter()
            .map(|line| {
                line.iter()
                    .map(|s| (s.text.clone(), u64::from(s.style), js(s.width)))
                    .collect()
            })
            .collect();
        let expected_lines: Vec<Vec<(String, u64, String)>> = case["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| {
                line.as_array()
                    .unwrap()
                    .iter()
                    .map(|s| {
                        (
                            s[0].as_str().unwrap().to_owned(),
                            s[1].as_u64().unwrap(),
                            s[2].as_str().unwrap().to_owned(),
                        )
                    })
                    .collect()
            })
            .collect();
        let widths: Vec<String> = label.widths.iter().map(|w| js(*w)).collect();
        let expected_widths: Vec<String> = case["widths"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_str().unwrap().to_owned())
            .collect();
        let same = lines == expected_lines
            && widths == expected_widths
            && js(label.width) == case["width"].as_str().unwrap()
            && js(label.height) == case["height"].as_str().unwrap()
            && js(label.line_height) == case["lineHeight"].as_str().unwrap()
            && label_svg(&label, 100.0, 50.0, " class=\"t\"") == case["svg"].as_str().unwrap();
        if !same {
            failures.push(format!(
                "{raw:?} markdown={markdown} size={size} max={max} base={base}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} labels differ, first: {:#?}",
        failures.len(),
        &failures[..failures.len().min(5)]
    );
}

#[test]
fn numbers_format_as_the_oracle_does() {
    for pair in fixtures()["numbers"].as_array().unwrap() {
        let value: f64 = pair[0].as_str().unwrap().parse().unwrap();
        assert_eq!(num(value), pair[1].as_str().unwrap(), "num({})", pair[0]);
    }
}

#[test]
fn escaping_matches_the_oracle() {
    for pair in fixtures()["escapes"].as_array().unwrap() {
        assert_eq!(esc(pair[0].as_str().unwrap()), pair[1].as_str().unwrap());
    }
}
