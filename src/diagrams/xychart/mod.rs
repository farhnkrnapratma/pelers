mod db;
mod lexer;
mod parser;
mod render;
mod ticks;

pub use db::XyChartDb;
pub use render::render_xychart;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<XyChartDb, PelersError> {
    let mut db = XyChartDb {
        title,
        ..XyChartDb::default()
    };
    parser::parse_xychart(source, &mut db)?;
    Ok(db.finish())
}
