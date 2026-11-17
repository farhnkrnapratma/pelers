use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::layout::layered::layered;
use crate::layout::{Kind, LayeredOptions, ledge, lnode};

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn same(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

#[test]
fn layered_matches_the_oracle() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/layout-layered.json");
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let input = &case["input"];
        let expected = &case["output"];
        let mut nodes: Vec<_> = input["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                let mut node = lnode(number(&n["w"]), number(&n["h"]), Kind::Node);
                node.pin = n["pin"].as_i64().unwrap() as i32;
                node.hint = number(&n["hint"]);
                node.key = number(&n["key"]);
                node.span = number(&n["span"]);
                node
            })
            .collect();
        let mut edges: Vec<_> = input["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let mut edge = ledge(
                    e["tail"].as_u64().unwrap() as usize,
                    e["head"].as_u64().unwrap() as usize,
                    number(&e["minlen"]),
                    number(&e["labelW"]),
                    number(&e["labelH"]),
                );
                edge.tail_dx = number(&e["tailDx"]);
                edge.head_dx = number(&e["headDx"]);
                edge.tail_fixed = e["tailFixed"].as_bool().unwrap();
                edge.head_fixed = e["headFixed"].as_bool().unwrap();
                edge
            })
            .collect();
        let opt = &input["options"];
        let options = LayeredOptions {
            node_sep: number(&opt["nodeSep"]),
            edge_sep: number(&opt["edgeSep"]),
            rank_sep: number(&opt["rankSep"]),
            port_sep: number(&opt["portSep"]),
            tracks: opt["tracks"].as_bool().unwrap(),
            straight: opt["straight"].as_bool().unwrap(),
            apart: opt["apart"].as_bool().unwrap(),
            head_room: number(&opt["headRoom"]),
            groups: None,
            order_only: opt["orderOnly"].as_bool().unwrap(),
        };

        let result = layered(&mut nodes, &mut edges, &options);

        assert!(
            same(result.width, number(&expected["width"])),
            "case {index}: width"
        );
        assert!(
            same(result.height, number(&expected["height"])),
            "case {index}: height"
        );
        for (i, (node, want)) in nodes
            .iter()
            .zip(expected["nodes"].as_array().unwrap())
            .enumerate()
        {
            assert!(
                same(node.x, number(&want["x"])),
                "case {index} node {i}: x {} != {}",
                node.x,
                want["x"]
            );
            assert!(
                same(node.y, number(&want["y"])),
                "case {index} node {i}: y {} != {}",
                node.y,
                want["y"]
            );
            assert_eq!(
                node.rank as u64,
                want["rank"].as_u64().unwrap(),
                "case {index} node {i}: rank"
            );
        }
        for (i, (edge, want)) in edges
            .iter()
            .zip(expected["edges"].as_array().unwrap())
            .enumerate()
        {
            assert_eq!(
                edge.reversed,
                want["reversed"].as_bool().unwrap(),
                "case {index} edge {i}: reversed"
            );
            assert!(
                same(edge.tail_dx, number(&want["tailDx"])),
                "case {index} edge {i}: tailDx"
            );
            assert!(
                same(edge.head_dx, number(&want["headDx"])),
                "case {index} edge {i}: headDx"
            );
            let points: Vec<f64> = want["points"]
                .as_array()
                .unwrap()
                .iter()
                .map(number)
                .collect();
            assert_eq!(
                edge.points.len(),
                points.len(),
                "case {index} edge {i}: point count"
            );
            for (k, (a, b)) in edge.points.iter().zip(points.iter()).enumerate() {
                assert!(same(*a, *b), "case {index} edge {i}: point {k} {a} != {b}");
            }
            assert!(
                same(edge.label_x, number(&want["labelX"])),
                "case {index} edge {i}: labelX"
            );
            assert!(
                same(edge.label_y, number(&want["labelY"])),
                "case {index} edge {i}: labelY"
            );
        }
    }
}
