use std::fs;

use serde_json::Value;

use crate::layout::components::wrap_components;
use crate::layout::{CEdge, CNode, cnode};

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn same(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

#[test]
fn components_match_the_oracle() {
    let path = crate::tests::fixture_path("layout-components.json");
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let input = &case["input"];
        let mut nodes: Vec<CNode> = input["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| {
                let mut node = cnode(
                    number(&n["w"]),
                    number(&n["h"]),
                    n["parent"].as_i64().unwrap() as i32,
                );
                node.x = number(&n["x"]);
                node.y = number(&n["y"]);
                node
            })
            .collect();
        let mut edges: Vec<CEdge> = input["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| CEdge {
                src: e["src"].as_u64().unwrap() as usize,
                dst: e["dst"].as_u64().unwrap() as usize,
                minlen: number(&e["minlen"]),
                label_w: number(&e["labelW"]),
                label_h: number(&e["labelH"]),
                route: e["route"].as_array().unwrap().iter().map(number).collect(),
                label_x: number(&e["labelX"]),
                label_y: number(&e["labelY"]),
            })
            .collect();

        let result = wrap_components(
            &mut nodes,
            &mut edges,
            number(&input["width"]),
            number(&input["gap"]),
        );

        let expected = &case["output"];
        match (result, expected.is_null()) {
            (None, true) => {}
            (Some(size), false) => {
                assert!(
                    same(size.width, number(&expected["width"])),
                    "case {index}: width"
                );
                assert!(
                    same(size.height, number(&expected["height"])),
                    "case {index}: height"
                );
                for (i, (node, want)) in nodes
                    .iter()
                    .zip(expected["nodes"].as_array().unwrap())
                    .enumerate()
                {
                    assert!(same(node.x, number(&want["x"])), "case {index} node {i}: x");
                    assert!(same(node.y, number(&want["y"])), "case {index} node {i}: y");
                }
                for (i, (edge, want)) in edges
                    .iter()
                    .zip(expected["edges"].as_array().unwrap())
                    .enumerate()
                {
                    let route: Vec<f64> = want["route"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(number)
                        .collect();
                    assert_eq!(
                        edge.route.len(),
                        route.len(),
                        "case {index} edge {i}: route length"
                    );
                    for (k, (a, b)) in edge.route.iter().zip(route.iter()).enumerate() {
                        assert!(same(*a, *b), "case {index} edge {i}: route {k}");
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
            (got, _) => panic!(
                "case {index}: wrapped {} but oracle wrapped {}",
                got.is_some(),
                !expected.is_null()
            ),
        }
    }
}
