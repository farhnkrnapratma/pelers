mod db;
mod parser;
mod render;
mod tokens;

pub use db::GitModel;
pub use render::render_git;

use crate::errors::PelersError;
use crate::preprocess::Config;

pub fn parse(
    source: &str,
    config: &Config,
    title: Option<String>,
) -> Result<GitModel, PelersError> {
    let ast = parser::parse_git(source)?;
    db::build_git(&ast, config, title.as_deref())
}
