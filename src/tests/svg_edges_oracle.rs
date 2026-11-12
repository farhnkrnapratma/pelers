use std::fs;

use serde_json::Value;

use crate::svg::edges::{
    Fill, LoopEdge, LoopNode, casing, edge_ink, hollow_on, loop_path, marker, over_fills,
    route_path,
};

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn numbers(value: &Value) -> Vec<f64> {
    value.as_array().unwrap().iter().map(number).collect()
}

fn same(a: f64, b: f64) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

#[test]
fn edges_match_the_oracle() {
    let path = crate::tests::fixture_path("svg-edges.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();

    for (index, case) in fixture["pathCases"].as_array().unwrap().iter().enumerate() {
        let route = numbers(&case["route"]);
        let curve = case["curve"].as_str();
        let start_reach = case.get("startReach").map(number);
        let end_reach = case.get("endReach").map(number);
        let out = route_path(
            &route,
            curve,
            number(&case["startTrim"]),
            number(&case["endTrim"]),
            start_reach,
            end_reach,
        );
        let want = &case["out"];
        assert_eq!(out.d, want["d"].as_str().unwrap(), "path {index}: d");
        let tokens = numbers(&want["tokens"]);
        assert_eq!(
            out.tokens.as_deref(),
            Some(tokens.as_slice()),
            "path {index}: tokens"
        );
        for (name, got) in [
            ("sx", out.sx),
            ("sy", out.sy),
            ("sdx", out.sdx),
            ("sdy", out.sdy),
            ("ex", out.ex),
            ("ey", out.ey),
            ("edx", out.edx),
            ("edy", out.edy),
        ] {
            assert!(
                same(got, number(&want[name])),
                "path {index}: {name} {got} != {}",
                want[name]
            );
        }
    }

    for (index, case) in fixture["markerCases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let got = marker(
            case["kind"].as_str().unwrap(),
            number(&case["x"]),
            number(&case["y"]),
            number(&case["dx"]),
            number(&case["dy"]),
            case["color"].as_str().unwrap(),
            case["stroke"].as_str().unwrap(),
        );
        assert_eq!(got, case["out"].as_str().unwrap(), "marker {index}");
        assert!(
            same(
                crate::svg::edges::marker_trim(case["kind"].as_str().unwrap()),
                number(&case["trim"])
            ),
            "marker {index}: trim"
        );
    }

    for (index, case) in fixture["casingCases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let got = casing(
            case["d"].as_str().unwrap(),
            case["markers"].as_str().unwrap(),
            case["line"].as_str().unwrap(),
            case["thick"].as_bool().unwrap(),
        );
        assert_eq!(got, case["out"].as_str().unwrap(), "casing {index}");
    }

    for (index, case) in fixture["fillCases"].as_array().unwrap().iter().enumerate() {
        let fills: Vec<Fill> = case["fills"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| Fill {
                box_: [
                    number(&f["box"][0]),
                    number(&f["box"][1]),
                    number(&f["box"][2]),
                    number(&f["box"][3]),
                ],
                shape: f["shape"].as_str().unwrap().to_owned(),
            })
            .collect();
        let tips = numbers(&case["tips"]);
        let tokens = case["tokens"].as_array().map(|_| numbers(&case["tokens"]));
        let got = over_fills(
            case["d"].as_str().unwrap(),
            &tips,
            &fills,
            case["line"].as_str().unwrap(),
            tokens.as_deref(),
        );
        let want = if case["overFills"].is_null() {
            None
        } else {
            Some(case["overFills"].as_str().unwrap().to_owned())
        };
        assert_eq!(got, want, "fill {index}");
    }

    for (index, case) in fixture["hollowCases"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let fill = Fill {
            box_: [0.0, 0.0, 1.0, 1.0],
            shape: case["fill"]["shape"].as_str().unwrap().to_owned(),
        };
        assert_eq!(
            hollow_on(case["markers"].as_str().unwrap(), &fill),
            case["out"].as_str().unwrap(),
            "hollow {index}"
        );
        let _ = edge_ink(&fill, "");
    }

    for (index, case) in fixture["loopCases"].as_array().unwrap().iter().enumerate() {
        let c = LoopNode {
            x: number(&case["c"]["x"]),
            y: number(&case["c"]["y"]),
        };
        let mut e = LoopEdge {
            label_w: number(&case["labelW"]),
            label_h: number(&case["labelH"]),
            label_x: 0.0,
            label_y: 0.0,
        };
        let out = loop_path(
            &c,
            (number(&case["w"]), number(&case["h"])),
            number(&case["k"]),
            case["sideways"].as_bool().unwrap(),
            number(&case["base"]),
            number(&case["trim"]),
            &mut e,
        );
        let want = &case["out"];
        assert_eq!(out.d, want["d"].as_str().unwrap(), "loop {index}: d");
        for (name, got) in [
            ("sx", out.sx),
            ("sy", out.sy),
            ("sdx", out.sdx),
            ("sdy", out.sdy),
            ("ex", out.ex),
            ("ey", out.ey),
            ("edx", out.edx),
            ("edy", out.edy),
        ] {
            assert!(same(got, number(&want[name])), "loop {index}: {name}");
        }
        assert!(
            same(e.label_x, number(&case["labelX"])),
            "loop {index}: labelX"
        );
        assert!(
            same(e.label_y, number(&case["labelY"])),
            "loop {index}: labelY"
        );
    }
}
