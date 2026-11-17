mod db;
mod lexer;
mod parser;
mod render;

pub use db::QuadrantDb;
pub use render::render_quadrant;

use crate::diagrams::common::{acc_descr, acc_title};
use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<QuadrantDb, PelersError> {
    let mut db = QuadrantDb {
        title,
        ..QuadrantDb::default()
    };
    parser::parse_quadrant(source, &mut db)?;
    db.acc_title = acc_title(db.acc_title.take());
    db.acc_descr = acc_descr(db.acc_descr.take());
    Ok(db)
}
