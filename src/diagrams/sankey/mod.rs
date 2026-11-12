mod layout;
mod lexer;
mod parser;
mod render;

pub use parser::SankeyDb;
pub use render::render_sankey;

use crate::errors::PelersError;

pub fn parse(source: &str, title: Option<String>) -> Result<SankeyDb, PelersError> {
    let mut db = SankeyDb::default();
    if let Some(title) = title.filter(|t| !t.is_empty()) {
        db.title = Some(title);
    }
    let units: Vec<u16> = source.encode_utf16().collect();
    let (trimmed, _) = parser::prepare_text_for_parsing(&units);
    parser::parse_sankey(&trimmed, &mut db, &units)?;
    Ok(db)
}
