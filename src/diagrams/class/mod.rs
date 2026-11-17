mod db;
mod graph;
mod lexer;
mod markers;
mod members;
mod parser;
mod render;
mod tokens;
mod types;

use crate::errors::PelersError;

pub use db::ClassDb;
pub use render::render_class;

pub fn parse(source: &str, title: Option<String>) -> Result<ClassDb, PelersError> {
    let mut db = ClassDb::new();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    parser::parse_class_diagram(source, &mut db)?;
    Ok(db)
}
