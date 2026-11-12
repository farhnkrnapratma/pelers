mod db;
mod lexer;
mod parser;
mod render;

pub use db::TimelineModel;
pub use render::render_timeline;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<TimelineModel, PelersError> {
    let mut db = TimelineModel::default();
    db.title = title;
    parser::parse_timeline(source, &mut db)?;
    Ok(db.finish())
}
