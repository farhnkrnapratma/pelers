mod format;
mod layout;
mod lexer;
mod model;
mod parser;
mod render;

pub use model::TreemapModel;
pub use render::render_treemap;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<TreemapModel, PelersError> {
    Ok(model::build_treemap(parser::parse_treemap(source)?, title))
}
