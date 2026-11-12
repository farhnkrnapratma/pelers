mod boundaries;
mod model;
mod parser;
mod render;

pub use model::CynefinModel;
pub use render::render_cynefin;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<CynefinModel, PelersError> {
    let ast = parser::parse_cynefin(source)?;
    Ok(model::build_cynefin(&ast, title.as_deref()))
}
