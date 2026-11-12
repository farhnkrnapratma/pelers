use std::fs;

use crate::diagrams::gantt::legacy_date::parse;
use serde_json::Value as Json;

#[test]
fn legacy_date_parse_matches_the_oracle() {
    let path = crate::tests::fixture_path("date-parse.json");
    let fixtures: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let mut failures = Vec::new();
    let cases = fixtures["cases"].as_array().unwrap();
    for case in cases {
        let source = case["source"].as_str().unwrap();
        let got = parse(source);
        let expected = case["time"].as_f64();
        let same = match expected {
            None => got.is_nan(),
            Some(time) => got == time,
        };
        if !same {
            failures.push(format!("{source:?} got {got} expected {expected:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} date strings differ\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(60)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
