mod axis;
mod dates;
mod db;
pub mod jsdate;
pub mod legacy_date;
mod lexer;
mod parser;
mod render;
mod tokens;

use crate::errors::PelersError;
use crate::preprocess::Config;
use crate::util::value::Value;

pub use db::GanttDb;
pub use render::render_gantt;

pub fn parse(source: &str, config: &Config, title: Option<String>) -> Result<GanttDb, PelersError> {
    let mut db = GanttDb::new();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    if let Some(Value::Object(map)) = config.get("gantt")
        && let Some(Value::String(mode)) = map.get("displayMode")
    {
        db.display_mode = mode.clone();
    }
    parser::parse_gantt(source, &mut db)?;
    Ok(db)
}
