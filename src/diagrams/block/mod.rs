mod arrows;
mod db;
mod layout;
mod lexer;
mod parser;
mod render;
mod tokens;
mod types;

use crate::errors::PelersError;

pub use db::BlockDb;
pub use render::render_block;

pub fn parse(source: &str, title: Option<String>) -> Result<BlockDb, PelersError> {
    let mut db = BlockDb::new();
    db.title = title;
    parser::parse_block(source, &mut db)?;
    Ok(db)
}
