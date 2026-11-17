mod lexer;
mod parser;
mod render;

use crate::diagrams::common::{acc_descr, acc_title};
use crate::errors::PelersError;

pub use render::render_journey;

#[derive(Debug, Clone, PartialEq)]
pub struct JourneyTask {
    pub task: String,
    pub score: f64,
    pub people: Vec<String>,
    pub section: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JourneyModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub tasks: Vec<JourneyTask>,
}

pub fn journey_actors(tasks: &[JourneyTask]) -> Vec<String> {
    let mut names: Vec<String> = tasks
        .iter()
        .flat_map(|task| task.people.iter().cloned())
        .collect();
    names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    names.dedup();
    names
}

pub fn parse(source: &str, title: Option<String>) -> Result<JourneyModel, PelersError> {
    let mut model = parser::parse_journey(source, title)?;
    model.acc_title = acc_title(model.acc_title.take());
    model.acc_descr = acc_descr(model.acc_descr.take());
    Ok(model)
}
