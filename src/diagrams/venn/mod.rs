mod db;
mod layout;
mod lexer;
mod parser;
mod render;

pub use db::VennDb;
pub use render::render_venn;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<VennDb, PelersError> {
    let mut db = VennDb::default();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    parser::parse_venn(source, &mut db)?;
    Ok(db)
}
