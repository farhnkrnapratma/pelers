mod db;
mod graph;
mod json;
mod lexer;
mod parser;
mod render;
mod tokens;
mod types;

use crate::errors::PelersError;

pub use render::render_usecase;
pub use types::UsecaseModel;

pub fn parse(source: &str, title: Option<String>) -> Result<UsecaseModel, PelersError> {
    parser::parse_usecase(source, title)
}
