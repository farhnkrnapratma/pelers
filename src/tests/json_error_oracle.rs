use std::fs;

use crate::util::json_errors::error_message;
use serde_json::Value as Json;

#[test]
fn json_error_messages_match_the_oracle() {
    let fixtures: Json = serde_json::from_str(
        &fs::read_to_string(crate::tests::fixture_path("json-errors.json")).unwrap(),
    )
    .unwrap();
    let mut failures = Vec::new();
    let cases = fixtures["cases"].as_array().unwrap();
    for case in cases {
        let text = case["text"].as_str().unwrap();
        let got = error_message(text);
        let want = case["message"].as_str().map(str::to_owned);
        if got != want {
            failures.push(format!("{text:?}\n got  {got:?}\n want {want:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} JSON texts differ\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
