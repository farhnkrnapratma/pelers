mod api_integration;
mod date_parse_oracle;
mod flow_golden;
mod flow_lexer_oracle;
mod flow_parse_oracle;
mod flow_shapes_oracle;
mod golden;
mod ink_oracle;
mod json_error_oracle;
mod layout_components_oracle;
mod layout_compound_oracle;
mod layout_lanes_oracle;
mod layout_oracle;
mod numfmt_oracle;
mod palette_oracle;
mod pelers_corpus;
mod preprocess_oracle;
mod render_oracle;
mod semantic_oracle;
mod svg_edges_oracle;
mod svg_shapes_oracle;
mod swimlane_golden;
mod text_oracle;
mod theme_oracle;
mod ui_oracle;
mod units;
mod url_oracle;

pub fn fixture_path(name: &str) -> std::path::PathBuf {
    match std::env::var_os("PELERS_FUZZ_DIR") {
        Some(dir) => std::path::PathBuf::from(dir).join(name),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    }
}
