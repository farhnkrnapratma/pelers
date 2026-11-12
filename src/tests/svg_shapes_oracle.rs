use std::fs;

use serde_json::Value;

use crate::layout::{CNode, cnode};
use crate::svg::shapes::{
    View, draw_shape, end_side, inset_route, shape_filled, shape_has_label, shape_inset,
    shape_size, shape_span,
};

fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

fn route_of(value: &Value) -> Vec<f64> {
    value.as_array().unwrap().iter().map(number).collect()
}

#[test]
fn shapes_match_the_oracle() {
    let path = crate::tests::fixture_path("svg-shapes.json");
    let fixture: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let attr = " fill=\"var(--_s)\" stroke=\"var(--_b)\"";
    let line = " stroke=\"var(--_b)\"";
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let shape = case["shape"].as_str().unwrap();
        let tw = number(&case["tw"]);
        let th = number(&case["th"]);
        let size = shape_size(shape, tw, th);
        assert_eq!(
            size.w,
            number(&case["size"]["w"]),
            "case {index} {shape}: w"
        );
        assert_eq!(
            size.h,
            number(&case["size"]["h"]),
            "case {index} {shape}: h"
        );
        assert_eq!(
            size.dy,
            number(&case["size"]["dy"]),
            "case {index} {shape}: dy"
        );
        assert_eq!(
            draw_shape(shape, size.w, size.h, attr, line),
            case["draw"].as_str().unwrap(),
            "case {index} {shape}: draw"
        );
        assert_eq!(
            draw_shape(shape, size.w, size.h, "", ""),
            case["drawPlain"].as_str().unwrap(),
            "case {index} {shape}: drawPlain"
        );
        for side in 0..4 {
            let t = number(&case["insetT"][side]);
            let got = shape_inset(shape, size.w, size.h, side as i32, t);
            assert_eq!(
                got,
                number(&case["insets"][side]),
                "case {index} {shape}: inset side {side}"
            );
        }
        assert_eq!(
            shape_span(shape, size.w, size.h, false),
            number(&case["spans"][0]),
            "case {index} {shape}: span"
        );
        assert_eq!(
            shape_span(shape, size.w, size.h, true),
            number(&case["spans"][1]),
            "case {index} {shape}: span sideways"
        );
        assert_eq!(
            shape_has_label(shape),
            case["hasLabel"].as_bool().unwrap(),
            "case {index} {shape}: hasLabel"
        );
        assert_eq!(
            shape_filled(shape),
            case["filled"].as_bool().unwrap(),
            "case {index} {shape}: filled"
        );
    }
    for (index, case) in fixture["routes"].as_array().unwrap().iter().enumerate() {
        let c: &Value = &case["c"];
        let mut node: CNode = cnode(number(&c["w"]), number(&c["h"]), -1);
        node.x = number(&c["x"]);
        node.y = number(&c["y"]);
        node.is_group = c["isGroup"].as_bool().unwrap();
        let route = route_of(&case["route"]);
        let at = case["at"].as_u64().unwrap() as usize;
        assert_eq!(
            end_side(&route, at, &node) as i64,
            case["side"].as_i64().unwrap(),
            "route {index}: side"
        );
        let shape = case["view"]["shape"].as_str().unwrap();
        let view = View {
            shape,
            w: number(&case["view"]["w"]),
            h: number(&case["view"]["h"]),
        };
        let mut inserted = route.clone();
        inset_route(&mut inserted, at, &view, &node);
        let want = route_of(&case["inserted"]);
        assert_eq!(inserted, want, "route {index}: inserted");
    }
}
