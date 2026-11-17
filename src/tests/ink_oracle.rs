use std::fs;

use serde_json::Value;

use crate::svg::ink::{ink, paints, plain_color};

#[test]
fn ink_matches_the_oracle() {
    let path = crate::tests::fixture_path("ink.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        if let Some(text) = case.get("text") {
            let shape = case["shape"].as_str().unwrap();
            let text = text.as_str().unwrap();
            let got = ink(shape, text).map(str::to_owned);
            let want = case["ink"].as_str().map(str::to_owned);
            assert_eq!(got, want, "ink {shape:?} {text:?}");
        }
    }
    for case in fixture["cases"].as_array().unwrap() {
        if let Some(want) = case.get("paints") {
            let shape = case["shape"].as_str().unwrap();
            assert_eq!(paints(shape), want.as_bool().unwrap(), "paints {shape:?}");
        }
    }
    for case in fixture["plains"].as_array().unwrap() {
        assert_eq!(
            plain_color(case["raw"].as_str().unwrap()),
            case["out"].as_bool().unwrap(),
            "plain {:?}",
            case["raw"]
        );
    }
}
