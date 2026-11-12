mod db;
mod lexer;
mod parser;
mod render;
mod tokens;

pub use db::RequirementModel;
pub use render::render_requirement;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<RequirementModel, PelersError> {
    let mut db = db::RequirementDb::new();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    parser::parse_requirement(source, &mut db)?;
    Ok(db.into_model())
}
