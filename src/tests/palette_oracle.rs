use std::fs;
use std::path::Path;

use crate::svg::palette::palette;
use crate::util::json::parse;
use crate::util::value::Value;
use serde_json::Value as Json;

#[test]
fn palettes_match_the_oracle() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/palette.json");
    let fixtures: Json = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let Some(Value::Object(config)) = parse(&case["config"].to_string()) else {
            panic!("fixture config is not an object");
        };
        let made = palette(case["type"].as_str().unwrap(), &config);
        let style = case["style"].as_str().unwrap();
        let inks: Vec<Option<&str>> = case["inks"]
            .as_array()
            .unwrap()
            .iter()
            .map(Json::as_str)
            .collect();
        if made.style != style || made.inks.to_vec() != inks {
            failures.push(format!(
                "{}\nrust {:?} {:?}\nts   {style:?} {inks:?}",
                case["config"], made.style, made.inks
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} palettes differ\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n")
    );
}
