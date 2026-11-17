mod parser;
mod render;

use crate::diagrams::common::{acc_descr, acc_title};
use crate::errors::{PelersError, PelersErrorCode};
use crate::preprocess::Config;
use crate::util::jsnum::{js_min, js_number_to_string};
use crate::util::value::Value;

pub use parser::{PacketAst, PacketBlockAst, parse_packet};
pub use render::render_packet;

#[derive(Debug, Clone, PartialEq)]
pub struct PacketBlock {
    pub start: f64,
    pub end: f64,
    pub bits: f64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PacketModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub bits_per_row: f64,
    pub rows: Vec<Vec<PacketBlock>>,
}

const MAX_ROWS: usize = 10_000;

pub fn packet_section(config: &Config) -> Option<&crate::util::value::Object> {
    match config.get("packet") {
        Some(Value::Object(section)) => Some(section),
        _ => None,
    }
}

pub fn packet_option(config: &Config, key: &str, fallback: f64, min: f64, max: f64) -> f64 {
    match packet_section(config).and_then(|section| section.get(key)) {
        Some(Value::Number(value)) if *value >= min => js_min(*value, max),
        _ => fallback,
    }
}

fn invalid(message: String) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("packet".to_owned());
    error
}

pub fn build_packet(
    ast: PacketAst,
    config: &Config,
    title: Option<String>,
) -> Result<PacketModel, PelersError> {
    let bits_per_row = packet_option(config, "bitsPerRow", 32.0, 1.0, 4096.0).floor();
    let mut rows: Vec<Vec<PacketBlock>> = Vec::new();
    let mut last_bit = -1.0;
    let mut word: Vec<PacketBlock> = Vec::new();
    let mut row = 1.0;

    for block in &ast.blocks {
        if let (Some(from), Some(to)) = (block.start, block.end)
            && to < from
        {
            return Err(invalid(format!(
                "Packet block {} - {} is invalid. End must be greater than start.",
                js_number_to_string(from),
                js_number_to_string(to)
            )));
        }
        let mut from = block.start.unwrap_or(last_bit + 1.0);
        if from != last_bit + 1.0 {
            return Err(invalid(format!(
                "Packet block {} - {} is not contiguous. It should start from {}.",
                js_number_to_string(from),
                js_number_to_string(block.end.unwrap_or(from)),
                js_number_to_string(last_bit + 1.0)
            )));
        }
        if block.bits == Some(0.0) {
            return Err(invalid(format!(
                "Packet block {} is invalid. Cannot have a zero bit field.",
                js_number_to_string(from)
            )));
        }
        let to = block.end.unwrap_or(from + block.bits.unwrap_or(1.0) - 1.0);
        let mut width = block.bits.unwrap_or(to - from + 1.0);
        last_bit = to;

        while word.len() as f64 <= bits_per_row + 1.0 && rows.len() < MAX_ROWS {
            let row_end = row * bits_per_row - 1.0;
            if to <= row_end {
                word.push(PacketBlock {
                    start: from,
                    end: to,
                    bits: width,
                    label: block.label.clone(),
                });
                if to == row_end {
                    rows.push(std::mem::take(&mut word));
                    row += 1.0;
                }
                break;
            }
            word.push(PacketBlock {
                start: from,
                end: row_end,
                bits: row_end - from,
                label: block.label.clone(),
            });
            rows.push(std::mem::take(&mut word));
            row += 1.0;
            from = row_end + 1.0;
            width = to - from;
        }
    }
    if !word.is_empty() {
        rows.push(word);
    }

    Ok(PacketModel {
        title: ast.title.filter(|t| !t.is_empty()).or(title),
        acc_title: acc_title(ast.acc_title),
        acc_descr: acc_descr(ast.acc_descr),
        bits_per_row,
        rows,
    })
}

pub fn parse(
    source: &str,
    config: &Config,
    title: Option<String>,
) -> Result<PacketModel, PelersError> {
    build_packet(parse_packet(source)?, config, title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::json::parse as parse_json;

    fn config(text: &str) -> Config {
        parse_json(text).unwrap().as_object().cloned().unwrap()
    }

    fn model(source: &str, json: &str) -> Result<PacketModel, PelersError> {
        parse(source, &config(json), None)
    }

    #[test]
    fn splits_fields_across_rows() {
        let made = model("packet\n0-19: \"a\"\n20-31: \"b\"\n", "{}").unwrap();
        assert_eq!(made.rows.len(), 1);
        let split = model("packet\n0-39: \"a\"\n", r#"{"packet":{"bitsPerRow":16}}"#).unwrap();
        let ends: Vec<f64> = split.rows.iter().map(|r| r[0].end).collect();
        assert_eq!(ends, vec![15.0, 31.0, 39.0]);
        assert_eq!(split.rows[0][0].bits, 15.0);
        assert_eq!(split.rows[2][0].bits, 7.0);
    }

    #[test]
    fn counts_relative_widths() {
        let made = model("packet\n+16: \"a\"\n+16: \"b\"\n", "{}").unwrap();
        assert_eq!(made.rows[0][1].start, 16.0);
        assert_eq!(made.rows[0][1].end, 31.0);
    }

    #[test]
    fn rejects_bad_ranges() {
        let backwards = model("packet\n5-2: \"a\"\n", "{}").unwrap_err();
        assert_eq!(
            backwards.message,
            "Packet block 5 - 2 is invalid. End must be greater than start."
        );
        let gap = model("packet\n0-3: \"a\"\n8: \"b\"\n", "{}").unwrap_err();
        assert_eq!(
            gap.message,
            "Packet block 8 - 8 is not contiguous. It should start from 4."
        );
        let zero = model("packet\n+0: \"a\"\n", "{}").unwrap_err();
        assert_eq!(
            zero.message,
            "Packet block 0 is invalid. Cannot have a zero bit field."
        );
    }

    #[test]
    fn reads_clamped_options() {
        let cfg = config(r#"{"packet":{"bitsPerRow":99999,"rowHeight":0}}"#);
        assert_eq!(packet_option(&cfg, "bitsPerRow", 32.0, 1.0, 4096.0), 4096.0);
        assert_eq!(packet_option(&cfg, "rowHeight", 32.0, 1.0, 400.0), 32.0);
    }
}
