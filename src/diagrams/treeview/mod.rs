mod boxdrawing;
mod icons;
mod model;
mod parser;
mod render;

pub use model::TreeViewModel;
pub use render::render_treeview;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<TreeViewModel, PelersError> {
    let ast = model::read_treeview(source)?;
    Ok(model::build_treeview(&ast, title.as_deref()))
}
