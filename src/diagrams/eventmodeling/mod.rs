mod model;
mod parser;
mod render;

pub use model::EventModelingModel;
pub use render::render_eventmodel;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<EventModelingModel, PelersError> {
    let ast = parser::parse_eventmodel(source)?;
    model::build_eventmodel(&ast, title.as_deref())
}
