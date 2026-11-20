use std::fs;
use std::io::Read;
use std::rc::Rc;

use flate2::read::GzDecoder;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::api::{ParseOptions, parse_with, render_with};
use crate::svg::root::RenderOptions;
use crate::text::math::MathMarkup;
use crate::util::hook::Hook;

fn with_callbacks(options: &mut RenderOptions) {
    options.icons = Some(Hook(Rc::new(|name: &str| {
        name.starts_with("fa:")
            .then(|| "<path d=\"M1 1\"/>".to_owned())
    })));
    options.math = Some(Hook(Rc::new(|tex: &str, size: f64| {
        if tex.contains('!') {
            return None;
        }
        let width = tex.encode_utf16().count() as f64 * size * 0.5 + 2.0;
        let height = size * 1.2;
        Some(if tex.contains('?') {
            MathMarkup::Html {
                html: "<span>x</span>".to_owned(),
                width,
                height,
            }
        } else {
            MathMarkup::Svg {
                svg: "<path d=\"M0 0\"/>".to_owned(),
                width,
                height,
            }
        })
    })));
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn options_from(value: &Value) -> RenderOptions {
    let number = |key: &str| value.get(key).and_then(Value::as_f64);
    let boolean = |key: &str| value.get(key).and_then(Value::as_bool);
    let strings = |key: &str| {
        value.get(key).and_then(Value::as_array).map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
    };
    RenderOptions {
        font_size: number("fontSize"),
        max_width: number("maxWidth"),
        padding: number("padding"),
        direction_breakpoint: number("directionBreakpoint"),
        responsive: boolean("responsive"),
        auto_direction: boolean("autoDirection"),
        font_family: value
            .get("fontFamily")
            .and_then(Value::as_str)
            .map(str::to_owned),
        font_family_mono: value
            .get("fontFamilyMono")
            .and_then(Value::as_str)
            .map(str::to_owned),
        id_prefix: value
            .get("idPrefix")
            .and_then(Value::as_str)
            .map(str::to_owned),
        link_rel: value
            .get("linkRel")
            .and_then(Value::as_str)
            .map(str::to_owned),
        links: boolean("links"),
        images: boolean("images"),
        link_schemes: strings("linkSchemes"),
        image_schemes: strings("imageSchemes"),
        ..RenderOptions::default()
    }
}

#[test]
fn mutants_match_the_oracle() {
    let path = crate::tests::fixture_path("mutants.json.gz");
    let mut text = String::new();
    GzDecoder::new(fs::File::open(path).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    let fixture: Value = serde_json::from_str(&text).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for (index, case) in cases.iter().enumerate() {
        let source = case["source"].as_str().unwrap();
        let kind = case["type"].as_str().unwrap();
        let mut options = options_from(&case["options"]);
        if case["callbacks"].as_bool() == Some(true) {
            with_callbacks(&mut options);
        }
        let parse_options = ParseOptions {
            max_edges: case["options"]["maxEdges"]
                .as_u64()
                .map(|edges| edges as usize),
            limit: case["options"]["limit"]
                .as_u64()
                .map(|limit| limit as usize),
            config: None,
        };
        let rendered =
            parse_with(source, &parse_options).and_then(|model| render_with(&model, &options));
        if case["ok"].as_bool().unwrap() {
            let svg = rendered
                .unwrap_or_else(|error| panic!("case {index} ({kind}) should render: {error}"))
                .svg;
            let digest = hex(&Sha256::digest(svg.as_bytes()));
            assert_eq!(
                digest,
                case["sha256"].as_str().unwrap(),
                "case {index} ({kind}) differs"
            );
        } else {
            assert!(rendered.is_err(), "case {index} ({kind}) should fail");
        }
    }
}
