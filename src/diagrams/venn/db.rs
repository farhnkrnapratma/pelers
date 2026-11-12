use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::trim;

#[derive(Clone, Debug, PartialEq)]
pub struct Subset {
    pub sets: Vec<String>,
    pub size: f64,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextNode {
    pub sets: Vec<String>,
    pub id: String,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StyleDecl {
    pub targets: Vec<String>,
    pub styles: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VennDb {
    pub title: Option<String>,
    pub subsets: Vec<Subset>,
    pub text_nodes: Vec<TextNode>,
    pub styles: Vec<StyleDecl>,
    known: Vec<String>,
    current: Option<Vec<String>>,
}

pub fn normalize_text(text: &str) -> String {
    let trimmed = trim(text);
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn normalize_ids(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = ids.iter().map(|id| normalize_text(id)).collect();
    out.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    out
}

impl VennDb {
    pub fn add_subset(&mut self, ids: &[String], label: Option<String>, size: Option<f64>) {
        let sets = normalize_ids(ids);
        if sets.len() == 1 && !self.known.contains(&sets[0]) {
            self.known.push(sets[0].clone());
        }
        let n = ids.len() as f64;
        self.current = Some(sets.clone());
        self.subsets.push(Subset {
            sets,
            size: size.unwrap_or(10.0 / (n * n)),
            label: label.filter(|l| !l.is_empty()).map(|l| normalize_text(&l)),
        });
    }

    pub fn add_text(&mut self, ids: Vec<String>, id: &str, label: Option<String>) {
        self.text_nodes.push(TextNode {
            sets: normalize_ids(&ids),
            id: normalize_text(id),
            label: label.filter(|l| !l.is_empty()).map(|l| normalize_text(&l)),
        });
    }

    pub fn add_style(&mut self, ids: &[String], data: Vec<(String, String)>) {
        let mut styles: Vec<(String, String)> = Vec::new();
        for (key, value) in data {
            let value = if value.is_empty() {
                value
            } else {
                normalize_text(&value)
            };
            match styles.iter_mut().find(|(k, _)| *k == key) {
                Some(entry) => entry.1 = value,
                None => styles.push((key, value)),
            }
        }
        self.styles.push(StyleDecl {
            targets: normalize_ids(ids),
            styles,
        });
    }

    pub fn validate_union(&self, ids: &[String]) -> Result<(), PelersError> {
        let unknown: Vec<String> = ids
            .iter()
            .map(|id| normalize_text(id))
            .filter(|id| !self.known.contains(id))
            .collect();
        if unknown.is_empty() {
            Ok(())
        } else {
            let mut error = PelersError::new(
                format!("unknown set identifier: {}", unknown.join(", ")),
                PelersErrorCode::Semantic,
            );
            error.diagram_type = Some("venn".to_owned());
            Err(error)
        }
    }

    pub fn current_sets(&self) -> Option<&Vec<String>> {
        self.current.as_ref()
    }
}
