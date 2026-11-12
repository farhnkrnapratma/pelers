use std::fs;

use serde_json::{Value, json};

use crate::diagrams::flowchart::db::{DbOptions, FlowDb};
use crate::diagrams::flowchart::parser::parse_flowchart;
use crate::util::value::Value as YamlValue;

fn opt_str(value: &Option<String>) -> Value {
    value.clone().map_or(Value::Null, Value::from)
}

fn opt_f64(value: Option<f64>) -> Value {
    value.map_or(Value::Null, |v| json!(v))
}

fn yaml_to_json(value: &YamlValue) -> Value {
    match value {
        YamlValue::Null => Value::Null,
        YamlValue::Bool(b) => json!(b),
        YamlValue::Number(n) => json!(n),
        YamlValue::String(s) => json!(s),
        YamlValue::Array(items) => Value::Array(items.iter().map(yaml_to_json).collect()),
        YamlValue::Object(object) => {
            let mut map = serde_json::Map::new();
            for (k, v) in object.iter() {
                map.insert(k.to_owned(), yaml_to_json(v));
            }
            Value::Object(map)
        }
    }
}

fn dump(db: &FlowDb) -> Value {
    let nodes: Vec<Value> = db
        .nodes
        .iter()
        .map(|n| {
            let props: serde_json::Map<String, Value> =
                n.props.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
            json!({
                "id": n.id,
                "labelType": n.label_type.as_str(),
                "text": opt_str(&n.text),
                "type": opt_str(&n.kind),
                "styles": n.styles,
                "classes": n.classes,
                "dir": opt_str(&n.dir),
                "props": props,
                "link": opt_str(&n.link),
                "linkTarget": opt_str(&n.link_target),
                "icon": opt_str(&n.icon),
                "form": opt_str(&n.form),
                "pos": opt_str(&n.pos),
                "img": opt_str(&n.img),
                "constraint": opt_str(&n.constraint),
                "assetWidth": opt_f64(n.asset_width),
                "assetHeight": opt_f64(n.asset_height),
            })
        })
        .collect();
    let edges: Vec<Value> = db
        .edges
        .iter()
        .map(|e| {
            json!({
                "start": e.start,
                "end": e.end,
                "type": opt_str(&e.kind),
                "text": e.text,
                "labelType": e.label_type.as_str(),
                "classes": e.classes,
                "style": e.style.clone().map_or(Value::Null, |s| json!(s)),
                "stroke": opt_str(&e.stroke),
                "length": opt_f64(e.length),
                "id": opt_str(&e.id),
                "isUserDefinedId": e.is_user_defined_id,
                "interpolate": e.interpolate.as_ref().map_or(Value::Null, yaml_to_json),
                "animate": e.animate.as_ref().map_or(Value::Null, yaml_to_json),
                "animation": e.animation.as_ref().map_or(Value::Null, yaml_to_json),
            })
        })
        .collect();
    let subgraphs: Vec<Value> = db
        .subgraphs
        .iter()
        .map(|s| {
            let metadata: serde_json::Map<String, Value> = s
                .metadata
                .iter()
                .map(|(k, v)| (k.clone(), yaml_to_json(v)))
                .collect();
            json!({
                "id": s.id,
                "nodes": s.nodes,
                "title": s.title,
                "classes": s.classes,
                "dir": opt_str(&s.dir),
                "labelType": s.label_type.as_str(),
                "metadata": metadata,
            })
        })
        .collect();
    let classes: Vec<Value> = db
        .classes
        .iter()
        .map(|c| json!({ "id": c.id, "styles": c.styles, "textStyles": c.text_styles }))
        .collect();
    let tooltips: Vec<Value> = db.tooltips.iter().map(|(k, v)| json!([k, v])).collect();
    json!({
        "direction": opt_str(&db.direction),
        "nodes": nodes,
        "edges": edges,
        "subgraphs": subgraphs,
        "classes": classes,
        "tooltips": tooltips,
        "defaultEdgeStyle": db.default_edge_style.clone().map_or(Value::Null, |s| json!(s)),
        "defaultInterpolate": opt_str(&db.default_interpolate),
        "accTitle": opt_str(&db.acc_title),
        "accDescr": opt_str(&db.acc_descr),
    })
}

fn normalize(value: &Value) -> Value {
    match value {
        Value::Number(n) => json!(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(items) => Value::Array(items.iter().map(normalize).collect()),
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), normalize(v))).collect())
        }
        other => other.clone(),
    }
}

#[test]
fn flowchart_parse_state_matches_the_oracle() {
    let path = crate::tests::fixture_path("flow-parse.json");
    let cases: Vec<Value> = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    for (index, case) in cases.iter().enumerate() {
        let source = case["source"].as_str().unwrap();
        let mut db = FlowDb::new(DbOptions::default());
        let result = parse_flowchart(source, &mut db);
        match (&case["error"], result) {
            (Value::Null, Ok(())) => {}
            (want, Err(error)) => {
                assert_eq!(
                    error.message,
                    want["message"].as_str().unwrap(),
                    "input {index}: error message for {source:?}"
                );
            }
            (want, Ok(())) => {
                panic!("input {index}: parsed but oracle failed with {want} for {source:?}")
            }
        }
        if case["error"].is_null() {
            assert_eq!(
                normalize(&dump(&db)),
                normalize(&case["db"]),
                "input {index}: db state for {source:?}"
            );
        }
    }
}
