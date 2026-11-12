mod model;
mod parser;
mod render;

pub use model::WardleyModel;
pub use render::render_wardley;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<WardleyModel, PelersError> {
    let ast = parser::parse_wardley(source)?;
    model::build_wardley(&ast, title.as_deref())
}
