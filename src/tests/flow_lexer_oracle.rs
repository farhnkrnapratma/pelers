use std::fs;

use serde_json::Value;

use crate::diagrams::flowchart::lexer::tokenize;

#[test]
fn flowchart_lexer_matches_the_oracle() {
    let path = crate::tests::fixture_path("flow-lexer.json");
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let source = case["source"].as_str().unwrap();
        let got = tokenize(source);
        let types: Vec<u64> = got.types.iter().map(|t| *t as u64).collect();
        let want_types: Vec<u64> = case["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(types, want_types, "input {index}: types for {source:?}");
        let texts: Vec<&str> = case["texts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(got.texts, texts, "input {index}: texts for {source:?}");
        let starts: Vec<u64> = got.starts.iter().map(|s| *s as u64).collect();
        let want_starts: Vec<u64> = case["starts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        assert_eq!(starts, want_starts, "input {index}: starts for {source:?}");
    }
}
