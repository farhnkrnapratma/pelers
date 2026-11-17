mod color_slots;
mod db;
pub mod diagnostics;
mod graph;
mod lexer;
mod parser;
mod render;
mod shapes;
mod tokens;
pub mod types;

pub use db::{AgentflowDb, DbOptions};
pub use graph::AgentGraph;
pub use parser::parse_agentflow;
pub use render::{PALETTE, agent_graph_for, render_agentflow};

use crate::errors::PelersError;
use crate::preprocess::Config;
use crate::util::value::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct AgentflowModel {
    pub db: AgentflowDb,
    pub graph: AgentGraph,
}

pub fn parse(
    source: &str,
    config: &Config,
    title: Option<String>,
    line_offset: usize,
    max_edges: usize,
) -> Result<AgentflowModel, PelersError> {
    let flow = config.get("flowchart").and_then(Value::as_object);
    let inherit_dir = matches!(
        flow.and_then(|f| f.get("inheritDir")),
        Some(Value::Bool(true))
    );
    let direction = match config.get("direction") {
        Some(Value::String(text)) => Some(text.clone()),
        _ => None,
    };
    let mut db = AgentflowDb::new(DbOptions {
        max_edges: Some(max_edges),
        inherit_dir,
        direction,
        line_offset,
        source: source.to_owned(),
    });
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    parse_agentflow(source, &mut db)?;
    let curve = flow
        .and_then(|f| f.get("curve"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let graph = agent_graph_for(&mut db, curve.as_deref());
    Ok(AgentflowModel { db, graph })
}
