mod layout;
mod lexer;
mod model;
mod parser;
mod render;
mod route;
mod tokens;

use crate::errors::PelersError;

pub use model::C4Db;
pub use render::render_c4;

pub fn parse(source: &str, title: Option<String>) -> Result<C4Db, PelersError> {
    let mut db = C4Db::default();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    parser::parse_c4(source, &mut db)?;
    Ok(db)
}
