use std::fs;

use crate::util::jsnum::{js_to_exponential, js_to_fixed, js_to_precision};
use serde_json::Value as Json;

#[test]
fn number_formatting_matches_the_oracle() {
    let path = crate::tests::fixture_path("numfmt.json");
    let fixtures: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let mut failures = Vec::new();
    for case in fixtures["cases"].as_array().unwrap() {
        let value: f64 = case["value"].as_str().unwrap().parse().unwrap();
        let digits = case["digits"].as_u64().unwrap() as usize;
        let significant = case["significant"].as_u64().unwrap() as usize;
        let fixed = js_to_fixed(value, digits);
        let exponential = js_to_exponential(value, digits);
        let precision = js_to_precision(value, significant);
        if fixed != case["fixed"].as_str().unwrap() {
            failures.push(format!("toFixed({value:?}, {digits}) = {fixed}"));
        }
        if exponential != case["exponential"].as_str().unwrap() {
            failures.push(format!(
                "toExponential({value:?}, {digits}) = {exponential}"
            ));
        }
        if precision != case["precision"].as_str().unwrap() {
            failures.push(format!(
                "toPrecision({value:?}, {significant}) = {precision}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} differences, first: {:?}",
        failures.len(),
        failures.iter().take(5).collect::<Vec<_>>()
    );
}
