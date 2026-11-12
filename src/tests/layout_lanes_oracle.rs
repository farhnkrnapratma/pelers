use std::fs;

use serde_json::Value;

use crate::layout::lanes::lane_layout;
use crate::layout::{CEdge, CNode, LayeredOptions, cnode, direction};

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn same(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

#[test]
fn lanes_match_the_oracle() {
    let path = crate::tests::fixture_path("layout-lanes.json");
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let input = &case["input"];
        let expected = &case["output"];
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
                node.is_group = n["isGroup"].as_bool().unwrap();
                node.pad_x = number(&n["padX"]);
                node.pad_top = number(&n["padTop"]);
                node.pad_bottom = number(&n["padBottom"]);
                node.min_w = number(&n["minW"]);
                node.seq = number(&n["seq"]);
                node.span = number(&n["span"]);
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
                route: Vec::new(),
                label_x: 0.0,
                label_y: 0.0,
            })
            .collect();
        let opt = &input["options"];
        let options = LayeredOptions {
            node_sep: number(&opt["nodeSep"]),
            edge_sep: number(&opt["edgeSep"]),
            rank_sep: number(&opt["rankSep"]),
            port_sep: number(&opt["portSep"]),
            tracks: false,
            straight: opt["straight"].as_bool().unwrap(),
            apart: false,
            head_room: 0.0,
            groups: None,
            order_only: false,
        };
        let dir = direction(input["dir"].as_str());

        let result = lane_layout(&mut nodes, &mut edges, dir, &options, 24.0);

        assert!(
            same(result.width, number(&expected["width"])),
            "case {index}: width {} != {}",
            result.width,
            expected["width"]
        );
        assert!(
            same(result.height, number(&expected["height"])),
            "case {index}: height {} != {}",
            result.height,
            expected["height"]
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
            assert!(
                same(node.w, number(&want["w"])),
                "case {index} node {i}: w {} != {}",
                node.w,
                want["w"]
            );
            assert!(
                same(node.h, number(&want["h"])),
                "case {index} node {i}: h {} != {}",
                node.h,
                want["h"]
            );
            assert!(
                same(node.pad_top, number(&want["padTop"])),
                "case {index} node {i}: padTop"
            );
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
                assert!(same(*a, *b), "case {index} edge {i}: route {k} {a} != {b}");
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
