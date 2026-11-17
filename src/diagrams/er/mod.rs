mod graph;
mod lexer;
mod markers;
mod model;
mod parser;
mod render;
mod tokens;

pub use model::ErDb;
pub use render::render_er;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<ErDb, PelersError> {
    let mut db = ErDb::default();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    parser::parse_er(source, &mut db)?;
    Ok(db)
}
