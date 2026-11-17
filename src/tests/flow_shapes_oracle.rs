use std::fs;

use serde_json::Value;

use crate::diagrams::flowchart::shapes::{canonical_shape, is_valid_shape};

#[test]
fn flowchart_shapes_match_the_oracle() {
    let path = crate::tests::fixture_path("flow-shapes.json");
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for case in &cases {
        let name = case["name"].as_str();
        if let Some(name) = name {
            assert_eq!(
                is_valid_shape(name),
                case["valid"].as_bool().unwrap(),
                "valid {name}"
            );
        }
        assert_eq!(
            canonical_shape(name),
            case["canonical"].as_str().unwrap(),
            "canonical {name:?}"
        );
    }
}
