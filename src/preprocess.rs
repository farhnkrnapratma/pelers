use serde_json::{Map, Value};

use crate::errors::{PeleError, PeleErrorCode};

pub type Config = Map<String, Value>;

#[derive(Debug, Clone, PartialEq)]
pub struct Preprocessed {
    pub text: String,
    pub title: Option<String>,
    pub config: Config,
    pub with_comments: String,
    pub line_offset: usize,
}

fn merge(target: &mut Config, source: &Config) {
    for (key, value) in source {
        if matches!(key.as_str(), "__proto__" | "constructor" | "prototype") {
            continue;
        }
        match (target.get_mut(key), value) {
            (Some(Value::Object(current)), Value::Object(incoming)) => merge(current, incoming),
            _ => {
                target.insert(key.clone(), value.clone());
            }
        }
    }
}

fn cleanup_comments(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("%%") || trimmed.starts_with("%%{")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim_start()
        .to_string()
}

fn parse_front_matter(text: &str) -> Result<(String, Option<String>, Config, usize), PeleError> {
    let start = text.trim_start_matches([' ', '\t']);
    if !start.starts_with("---\n") {
        return Ok((text.to_string(), None, Map::new(), 0));
    }
    let closing = start[4..]
        .find("\n---")
        .ok_or_else(|| PeleError::new("Front matter is not closed.", PeleErrorCode::Syntax))?;
    let body = &start[4..4 + closing];
    let indent = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let body = body
        .lines()
        .map(|line| line.get(indent..).unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    let parsed: serde_yaml::Value = serde_yaml::from_str(&body).map_err(|error| {
        PeleError::new(
            format!("Front matter is not valid YAML. {error}"),
            PeleErrorCode::Syntax,
        )
    })?;
    let json = serde_json::to_value(parsed).map_err(|error| {
        PeleError::new(
            format!("Front matter conversion failed. {error}"),
            PeleErrorCode::Syntax,
        )
    })?;
    let object = json.as_object().cloned().unwrap_or_default();
    let title = object
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let config = object
        .get("config")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let consumed = 4 + closing + 4;
    let remainder = start[consumed..].trim_start_matches('\n').to_string();
    let line_offset = start[..consumed]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count();
    Ok((remainder, title, config, line_offset))
}

fn apply_directives(text: &str, config: &mut Config) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("%%{") {
        output.push_str(&rest[..start]);
        let Some(end) = rest[start + 3..].find("}%%") else {
            output.push_str(&rest[start..]);
            break;
        };
        let body = &rest[start + 3..start + 3 + end];
        let normalized = body.trim();
        if let Some(json) = normalized.strip_prefix("init:") {
            if let Ok(Value::Object(parsed)) = serde_json::from_str::<Value>(json.trim()) {
                merge(config, &parsed);
            }
        } else if normalized == "wrap" {
            config.insert("wrap".into(), Value::Bool(true));
        }
        rest = &rest[start + 3 + end + 3..];
    }
    if !rest.is_empty() {
        output.push_str(rest);
    }
    output
}

pub fn preprocess(source: &str) -> Result<Preprocessed, PeleError> {
    let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
    let (without_front_matter, title, mut config, line_offset) = parse_front_matter(&normalized)?;
    let with_comments = apply_directives(&without_front_matter, &mut config);
    let text = cleanup_comments(&with_comments);
    Ok(Preprocessed {
        text,
        title,
        config,
        with_comments,
        line_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::preprocess;
    use serde_json::json;

    #[test]
    fn normalizes_line_breaks_and_removes_comments() {
        let result = preprocess("%% comment\r\ngraph TD\r\n  A --> B").unwrap();
        assert_eq!(result.text, "graph TD\n  A --> B");
    }

    #[test]
    fn reads_front_matter() {
        let result = preprocess(
            "---\ntitle: My chart\nconfig:\n  flowchart:\n    curve: linear\n---\ngraph TD",
        )
        .unwrap();
        assert_eq!(result.title.as_deref(), Some("My chart"));
        assert_eq!(result.config["flowchart"]["curve"], json!("linear"));
        assert_eq!(result.line_offset, 5);
    }

    #[test]
    fn merges_and_removes_init_directive() {
        let result =
            preprocess("%%{init: {\"flowchart\": {\"rankSpacing\": 99}}}%%\ngraph TD").unwrap();
        assert_eq!(result.config["flowchart"]["rankSpacing"], json!(99));
        assert_eq!(result.text, "graph TD");
    }
}
