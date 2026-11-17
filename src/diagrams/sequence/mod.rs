mod actors;
mod colors;
mod db;
mod lexer;
mod parser;
mod render;
mod tokens;
mod types;

use crate::errors::PelersError;
use crate::preprocess::Config;
use crate::util::value::Value;

pub use db::SeqDb;
pub use render::render_sequence;

pub fn parse(source: &str, config: &Config, title: Option<String>) -> Result<SeqDb, PelersError> {
    let sequence = match config.get("sequence") {
        Some(Value::Object(map)) => map.clone(),
        _ => Config::default(),
    };
    let wrap = match config.get("wrap") {
        Some(Value::Bool(value)) => Some(*value),
        _ => None,
    };
    let mut db = SeqDb::new(db::SeqDbOptions {
        wrap,
        sequence_wrap: matches!(sequence.get("wrap"), Some(Value::Bool(true))),
    });
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.set_diagram_title(&title);
    }
    parser::parse_sequence(source, &mut db)?;
    Ok(db)
}
