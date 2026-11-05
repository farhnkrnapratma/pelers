use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use pelers::{PeleErrorCode, parse, render};

#[derive(Default)]
struct Tally {
    matched: usize,
    differed: usize,
    unsupported: usize,
}

fn tally(dir: &Path) -> Tally {
    let mut tally = Tally::default();
    let mut cases: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "mmd"))
        .collect();
    cases.sort();
    for case in cases {
        let source = fs::read_to_string(&case).unwrap();
        let expected = fs::read_to_string(case.with_extension("svg")).ok();
        match parse(&source).and_then(|model| render(&model)) {
            Ok(rendered) if expected.as_deref() == Some(rendered.svg.as_str()) => {
                tally.matched += 1
            }
            Ok(_) => tally.differed += 1,
            Err(error) if error.code == PeleErrorCode::UnsupportedDiagram => tally.unsupported += 1,
            Err(_) => tally.differed += 1,
        }
    }
    tally
}

#[test]
fn listed_types_match_the_oracle() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let required: BTreeSet<String> = fs::read_to_string(root.join("parity.txt"))
        .unwrap()
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();

    let mut failures = Vec::new();
    for entry in fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let tally = tally(&path);
        let total = tally.matched + tally.differed + tally.unsupported;
        eprintln!(
            "{name}: {} / {total} match, {} differ, {} unsupported",
            tally.matched, tally.differed, tally.unsupported
        );
        if required.contains(&name) && tally.matched != total {
            failures.push(name);
        }
    }
    assert!(
        failures.is_empty(),
        "types listed in parity.txt differ from the oracle: {failures:?}"
    );
}
