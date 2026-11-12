use std::fs;

use serde_json::Value;

use crate::util::url::{safe_url, sanitize_url};

#[test]
fn url_sanitizer_matches_the_oracle() {
    let path = crate::tests::fixture_path("url.json");
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for case in &cases {
        let input = case["input"].as_str().unwrap();
        assert_eq!(
            sanitize_url(input),
            case["sanitize"].as_str().unwrap(),
            "sanitize {input:?}"
        );
        assert_eq!(
            safe_url(input),
            case["safe"].as_str().unwrap(),
            "safe {input:?}"
        );
    }
}
