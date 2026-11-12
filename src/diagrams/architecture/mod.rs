mod db;
mod layout;
mod parser;
mod render;

pub use db::ArchitectureModel;
pub use render::render_architecture;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<ArchitectureModel, PelersError> {
    let ast = parser::parse_architecture(source)?;
    db::build_architecture(&ast, title.as_deref())
}
