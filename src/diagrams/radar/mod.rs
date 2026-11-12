mod lexer;
mod model;
mod parser;
mod render;

pub use model::{RadarModel, build_radar};
pub use parser::parse_radar;
pub use render::render_radar;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<RadarModel, PelersError> {
    build_radar(parse_radar(source)?, title)
}
