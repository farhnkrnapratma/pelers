mod db;
mod graph;
mod lexer;
mod parser;
mod render;
mod tokens;
mod types;

use crate::errors::PelersError;

pub use db::StateDb;
pub use render::render_state;

pub fn parse(source: &str, title: Option<String>) -> Result<StateDb, PelersError> {
    let mut db = StateDb::default();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    parser::parse_state(source, &mut db)?;
    Ok(db)
}
