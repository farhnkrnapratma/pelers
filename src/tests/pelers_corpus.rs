use std::fs;

use crate::{ParseOptions, RenderOptions, parse_with, render_with};
use serde_json::Value as Json;

#[test]
fn corpus_matches_pelers_1_13_0() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/pelers-corpus.json"
    );
    let text = fs::read_to_string(path).unwrap();
    let file: Json = serde_json::from_str(&text).unwrap();
    let cases = file["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    let mut failures = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let source = case["source"].as_str().unwrap();
        let expected = case["svg"].as_str().unwrap();
        let actual = parse_with(source, &ParseOptions::default())
            .and_then(|model| render_with(&model, &RenderOptions::default()))
            .map(|rendered| rendered.svg);
        match actual {
            Ok(svg) if svg == expected => {}
            Ok(_) => failures.push(format!("{index} {} differs", case["type"])),
            Err(error) => failures.push(format!("{index} {} {error}", case["type"])),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} differ: {:?}",
        failures.len(),
        cases.len(),
        &failures[..failures.len().min(20)]
    );
}
