use std::fs;

use serde_json::Value;

use crate::svg::theme::{class_names, resolve_style, sets_fill};

#[test]
fn theme_helpers_match_the_oracle() {
    let path = crate::tests::fixture_path("theme.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for case in fixture["styles"].as_array().unwrap() {
        let declarations: Vec<String> = case["declarations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        let got = resolve_style(&declarations);
        let want = &case["resolved"];
        assert_eq!(
            got.shape,
            want["shape"].as_str().unwrap(),
            "shape for {declarations:?}"
        );
        assert_eq!(
            got.line,
            want["line"].as_str().unwrap(),
            "line for {declarations:?}"
        );
        assert_eq!(
            got.text,
            want["text"].as_str().unwrap(),
            "text for {declarations:?}"
        );
        assert_eq!(
            got.stroke.as_deref(),
            want["stroke"].as_str(),
            "stroke for {declarations:?}"
        );
        assert_eq!(
            got.bold,
            want["bold"].as_bool().unwrap(),
            "bold for {declarations:?}"
        );
        assert_eq!(
            got.italic,
            want["italic"].as_bool().unwrap(),
            "italic for {declarations:?}"
        );
        assert_eq!(
            got.font_size,
            want["fontSize"].as_f64(),
            "fontSize for {declarations:?}"
        );
    }
    for case in fixture["classes"].as_array().unwrap() {
        assert_eq!(
            class_names(case["raw"].as_str().unwrap()),
            case["out"].as_str().unwrap(),
            "classNames {:?}",
            case["raw"]
        );
    }
    for case in fixture["fills"].as_array().unwrap() {
        assert_eq!(
            sets_fill(case["shape"].as_str().unwrap()),
            case["out"].as_bool().unwrap(),
            "setsFill {:?}",
            case["shape"]
        );
    }
}
