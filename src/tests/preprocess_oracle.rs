use std::fs;
use std::path::Path;

use crate::errors::input_offset;
use crate::preprocess::{encode_entities, preprocess, source_map};
use crate::util::jsnum::js_number_to_string;
use crate::util::value::Value;
use crate::util::yaml::parse_yaml;
use serde_json::Value as Json;

fn fixtures() -> Json {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/preprocess.json");
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn encode(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(n) if *n == 0.0 && n.is_sign_negative() => "n:-0".to_owned(),
        Value::Number(n) => format!("n:{}", js_number_to_string(*n)),
        Value::String(text) => json_string(text),
        Value::Array(items) => format!(
            "[{}]",
            items.iter().map(encode).collect::<Vec<_>>().join(",")
        ),
        Value::Object(object) => format!(
            "{{{}}}",
            object
                .iter()
                .map(|(key, value)| format!("{}:{}", json_string(key), encode(value)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

fn has_lone_surrogate_escape(expected: &str) -> bool {
    expected.contains("\\ud8")
        || expected.contains("\\ud9")
        || expected.contains("\\uda")
        || expected.contains("\\udb")
        || expected.contains("\\udc")
        || expected.contains("\\udd")
        || expected.contains("\\ude")
        || expected.contains("\\udf")
}

fn report(kind: &str, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} {kind} cases differ, first: {:#?}",
        failures.len(),
        &failures[..failures.len().min(4)]
    );
}

#[test]
fn yaml_reads_as_the_oracle_does() {
    let mut failures = Vec::new();
    for pair in fixtures()["yaml"].as_array().unwrap() {
        let (source, expected) = (pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
        if has_lone_surrogate_escape(expected) {
            continue;
        }
        let actual = match parse_yaml(source) {
            Ok(value) => encode(&value),
            Err(message) => format!("!{message}"),
        };
        if actual != expected {
            failures.push(format!("{source:?}\n  want {expected}\n  got  {actual}"));
        }
    }
    report("yaml", &failures);
}

#[test]
fn preprocess_matches_the_oracle() {
    let mut failures = Vec::new();
    for pair in fixtures()["pre"].as_array().unwrap() {
        let (source, expected) = (pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
        if has_lone_surrogate_escape(expected) {
            continue;
        }
        let actual = match preprocess(source) {
            Ok(out) => {
                let reference: Json = serde_json::from_str(expected).unwrap_or(Json::Null);
                let same = reference["text"] == out.text.as_str()
                    && reference["title"]
                        == out
                            .title
                            .as_deref()
                            .map_or(Json::Null, |t| Json::String(t.to_owned()))
                    && reference["config"].as_str()
                        == Some(encode(&Value::Object(out.config.clone())).as_str())
                    && reference["withComments"] == out.with_comments.as_str()
                    && reference["lineOffset"] == out.line_offset;
                if same {
                    expected.to_owned()
                } else {
                    format!(
                        "text={:?} title={:?} config={}",
                        out.text,
                        out.title,
                        encode(&Value::Object(out.config))
                    )
                }
            }
            Err(error) => format!("!{}", error.message),
        };
        if actual != expected {
            failures.push(format!("{source:?}\n  want {expected}\n  got  {actual}"));
        }
    }
    report("preprocess", &failures);
}

#[test]
fn entity_encoding_matches_the_oracle() {
    let mut failures = Vec::new();
    for pair in fixtures()["entities"].as_array().unwrap() {
        let (source, expected) = (pair[0].as_str().unwrap(), pair[1].as_str().unwrap());
        let actual = encode_entities(source, None);
        if actual != expected {
            failures.push(format!(
                "{source:?}\n  want {expected:?}\n  got  {actual:?}"
            ));
        }
    }
    report("entity", &failures);
}

#[test]
fn offsets_map_back_as_the_oracle_does() {
    let mut failures = Vec::new();
    for case in fixtures()["mapping"].as_array().unwrap() {
        let source = case[0].as_str().unwrap();
        let expected: Vec<usize> = case[1]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as usize)
            .collect();
        let parsed = case[2].as_str().unwrap();
        let starts: Vec<usize> = parsed.char_indices().map(|(at, _)| at).collect();
        let byte_at = |at: usize| {
            if at < starts.len() {
                starts[at]
            } else {
                parsed.len() + (at - starts.len())
            }
        };
        let (_, steps) = source_map(source, false);
        let actual: Vec<usize> = (0..expected.len())
            .map(|at| input_offset(&steps, byte_at(at)))
            .collect();
        if actual != expected {
            failures.push(format!(
                "{source:?}\n  want {expected:?}\n  got  {actual:?}"
            ));
        }
    }
    report("mapping", &failures);
}

#[test]
fn escapes_with_no_code_point_fail_like_v8() {
    assert_eq!(
        parse_yaml("a: \"\\xZZ\"").unwrap_err(),
        "Invalid code point NaN"
    );
    assert_eq!(
        parse_yaml("a: \"\\U00110000\"").unwrap_err(),
        "Invalid code point 1114112"
    );
}
