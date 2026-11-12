use std::fs;

use crate::api::{Model, parse};
use crate::diagrams::agentflow::types::{EdgeSemantic, SemanticModel};
use crate::util::value::Value;
use serde_json::{Map, Value as Json, json};

fn value_json(value: &Value) -> Json {
    match value {
        Value::Null => Json::Null,
        Value::Bool(flag) => Json::Bool(*flag),
        Value::Number(number) => json!(number),
        Value::String(text) => Json::String(text.clone()),
        Value::Array(items) => Json::Array(items.iter().map(value_json).collect()),
        Value::Object(object) => Json::Object(
            object
                .iter()
                .map(|(key, entry)| (key.to_owned(), value_json(entry)))
                .collect(),
        ),
    }
}

fn metadata_json(metadata: &Option<crate::util::value::Object>) -> Option<Json> {
    metadata
        .as_ref()
        .map(|object| value_json(&Value::Object(object.clone())))
}

fn put(map: &mut Map<String, Json>, key: &str, value: Option<Json>) {
    if let Some(value) = value {
        map.insert(key.to_owned(), value);
    }
}

fn edge_semantic_name(semantic: EdgeSemantic) -> &'static str {
    semantic.as_str()
}

fn semantic_json(model: &SemanticModel) -> Json {
    let vertices: Vec<Json> = model
        .vertices
        .iter()
        .map(|vertex| {
            let mut map = Map::new();
            map.insert("id".to_owned(), json!(vertex.id));
            put(&mut map, "label", vertex.label.as_ref().map(|l| json!(l)));
            put(&mut map, "shape", vertex.shape.as_ref().map(|s| json!(s)));
            map.insert("vertexKind".to_owned(), json!(vertex.vertex_kind));
            put(&mut map, "metadata", metadata_json(&vertex.metadata));
            Json::Object(map)
        })
        .collect();
    let edges: Vec<Json> = model
        .edges
        .iter()
        .map(|edge| {
            let mut map = Map::new();
            map.insert("start".to_owned(), json!(edge.start));
            map.insert("end".to_owned(), json!(edge.end));
            put(&mut map, "id", edge.id.as_ref().map(|v| json!(v)));
            put(&mut map, "label", edge.label.as_ref().map(|v| json!(v)));
            put(&mut map, "type", edge.kind.as_ref().map(|v| json!(v)));
            put(&mut map, "stroke", edge.stroke.as_ref().map(|v| json!(v)));
            put(
                &mut map,
                "edgeSemantic",
                edge.edge_semantic.map(|s| json!(edge_semantic_name(s))),
            );
            put(&mut map, "length", edge.length.map(|v| json!(v)));
            put(&mut map, "metadata", metadata_json(&edge.metadata));
            Json::Object(map)
        })
        .collect();
    let sub_graphs: Vec<Json> = model
        .sub_graphs
        .iter()
        .map(|sub| {
            let mut map = Map::new();
            map.insert("id".to_owned(), json!(sub.id));
            map.insert("type".to_owned(), json!(sub.kind));
            map.insert("nodes".to_owned(), json!(sub.nodes));
            map.insert("title".to_owned(), json!(sub.title));
            put(
                &mut map,
                "direction",
                sub.direction.as_ref().map(|v| json!(v)),
            );
            put(&mut map, "metadata", metadata_json(&sub.metadata));
            Json::Object(map)
        })
        .collect();
    let connectors: Vec<Json> = model
        .connectors
        .iter()
        .map(|connector| {
            let mut map = Map::new();
            map.insert("id".to_owned(), json!(connector.id));
            put(
                &mut map,
                "title",
                connector.title.as_ref().map(|v| json!(v)),
            );
            put(&mut map, "metadata", metadata_json(&connector.metadata));
            Json::Object(map)
        })
        .collect();
    let diagnostics: Vec<Json> = model
        .diagnostics
        .iter()
        .map(|d| {
            let mut map = Map::new();
            map.insert("id".to_owned(), json!(d.id.as_str()));
            map.insert(
                "severity".to_owned(),
                json!(match d.severity {
                    crate::diagrams::agentflow::types::Severity::Warning => "warning",
                    crate::diagrams::agentflow::types::Severity::Error => "error",
                }),
            );
            map.insert("message".to_owned(), json!(d.message));
            put(&mut map, "nodeId", d.node_id.as_ref().map(|v| json!(v)));
            put(&mut map, "edgeId", d.edge_id.as_ref().map(|v| json!(v)));
            put(
                &mut map,
                "position",
                d.position.map(|p| {
                    json!({
                        "startLine": p.start_line,
                        "startColumn": p.start_column,
                        "endLine": p.end_line,
                        "endColumn": p.end_column,
                        "startIndex": p.start_index,
                        "endIndex": p.end_index,
                    })
                }),
            );
            Json::Object(map)
        })
        .collect();
    let mut out = Map::new();
    put(
        &mut out,
        "direction",
        model.direction.as_ref().map(|v| json!(v)),
    );
    out.insert("vertices".to_owned(), Json::Array(vertices));
    out.insert("edges".to_owned(), Json::Array(edges));
    out.insert("subGraphs".to_owned(), Json::Array(sub_graphs));
    out.insert("connectors".to_owned(), Json::Array(connectors));
    out.insert("diagnostics".to_owned(), Json::Array(diagnostics));
    Json::Object(out)
}

fn normalize(value: &Json) -> Json {
    match value {
        Json::Number(number) => json!(number.as_f64()),
        Json::Array(items) => Json::Array(items.iter().map(normalize).collect()),
        Json::Object(map) => Json::Object(
            map.iter()
                .map(|(key, entry)| (key.clone(), normalize(entry)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[test]
fn agentflow_semantic_models_match_the_oracle() {
    let fixtures: Json = serde_json::from_str(
        &fs::read_to_string(crate::tests::fixture_path("semantic-agentflow.json")).unwrap(),
    )
    .unwrap();
    let mut failures = Vec::new();
    let cases = fixtures["cases"].as_array().unwrap();
    for case in cases {
        let source = case["source"].as_str().unwrap();
        let result = parse(source);
        match (&result, case.get("error")) {
            (Ok(parsed), None) => {
                let Model::Agentflow(agentflow) = &parsed.model else {
                    failures.push(format!("not agentflow: {source:?}"));
                    continue;
                };
                let got = normalize(&semantic_json(&agentflow.db.semantic_model()));
                let want = normalize(&case["model"]);
                if got != want {
                    failures.push(format!("{source:?}\n got {got}\nwant {want}"));
                }
            }
            (Err(error), Some(expected)) => {
                if error.message != expected["message"].as_str().unwrap() {
                    failures.push(format!(
                        "error {source:?}: {:?} vs {expected}",
                        error.message
                    ));
                }
            }
            (Ok(_), Some(expected)) => {
                failures.push(format!("rendered, oracle failed: {expected}"))
            }
            (Err(error), None) => failures.push(format!("failed, oracle ok: {error:?}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} semantic cases differ\n{}",
        failures.len(),
        cases.len(),
        failures
            .iter()
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
