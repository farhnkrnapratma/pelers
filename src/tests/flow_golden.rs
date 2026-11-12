use std::fs;
use std::path::Path;

use crate::diagrams::flowchart::db::{DbOptions, FlowDb};
use crate::diagrams::flowchart::parser::parse_flowchart;
use crate::diagrams::flowchart::render::{FlowConfig, FlowVariant, render_variant};
use crate::preprocess::{encode_entities, preprocess};
use crate::svg::root::RenderOptions;

fn render_source(source: &str, variant: Option<&FlowVariant>) -> String {
    let pre = preprocess(source).expect("preprocess");
    let mut db = FlowDb::new(DbOptions::default());
    db.title = pre.title.clone();
    let parsed = encode_entities(&pre.text, None) + "\n";
    parse_flowchart(&parsed, &mut db).expect("parse");
    render_variant(
        &db,
        &FlowConfig::default(),
        &RenderOptions::default(),
        variant,
    )
    .svg
}

pub fn check_golden(name: &str, variant: Option<&FlowVariant>) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let mut names: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "mmd"))
        .collect();
    names.sort();
    let mut matched = 0;
    let mut failed: Vec<String> = Vec::new();
    for mmd in &names {
        let source = fs::read_to_string(mmd).unwrap();
        let want = fs::read_to_string(mmd.with_extension("svg")).unwrap();
        let got = render_source(&source, variant);
        if got == want {
            matched += 1;
        } else {
            failed.push(mmd.file_name().unwrap().to_string_lossy().into_owned());
        }
    }
    eprintln!("{name} golden: {matched}/{} match", names.len());
    assert!(failed.is_empty(), "{name} golden mismatches: {failed:?}");
}

#[test]
fn flowchart_golden_cases_match() {
    check_golden("flowchart", None);
}
