use crate::errors::{Edit, PelersError, PelersErrorCode};
use crate::util::chars::{is_js_space, is_word, trim, trim_start};
use crate::util::json;
use crate::util::value::{Object, Value};
use crate::util::yaml::parse_yaml;

pub type Config = Object;

#[derive(Debug, Clone, PartialEq)]
pub struct Preprocessed {
    pub text: String,
    pub title: Option<String>,
    pub config: Config,
    pub with_comments: String,
    pub line_offset: usize,
}

type Steps<'a> = Option<&'a mut Vec<Vec<Edit>>>;

const UNSAFE_KEYS: [&str; 3] = ["__proto__", "constructor", "prototype"];

fn is_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn skip_space(text: &str, at: usize) -> usize {
    at + (text[at..].len() - trim_start(&text[at..]).len())
}

fn word_end(text: &str, at: usize) -> usize {
    at + text[at..]
        .find(|c: char| !is_word(c))
        .unwrap_or(text.len() - at)
}

fn comment_end(text: &str, start: usize) -> Option<usize> {
    let after = skip_space(text, start);
    let rest = text[after..].strip_prefix("%%")?;
    if rest.starts_with('{') {
        return None;
    }
    let body = rest.find('\n').unwrap_or(rest.len());
    if body == 0 {
        return None;
    }
    let end = after + 2 + body;
    Some(if text[end..].starts_with('\n') {
        end + 1
    } else {
        end
    })
}

fn comment_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut at = 0;
    while at <= text.len() {
        let line_start = at == 0 || text[..at].chars().next_back().is_some_and(is_terminator);
        if line_start {
            if let Some(end) = comment_end(text, at) {
                spans.push((at, end));
                at = end;
                continue;
            }
            let run = skip_space(text, at);
            if run > at {
                at = run;
                continue;
            }
        }
        match text[at..].chars().next() {
            Some(c) => at += c.len_utf8(),
            None => break,
        }
    }
    spans
}

struct Directive {
    start: usize,
    end: usize,
    kind: String,
    body: Option<String>,
}

fn directive_at(text: &str, start: usize) -> Option<Directive> {
    let p = skip_space(text, start + 3);
    let w1 = word_end(text, p);
    if w1 == p {
        return None;
    }
    let kind = text[p..w1].to_owned();
    let q = skip_space(text, w1);
    let mut pos = if text[q..].starts_with(':') {
        q + 1
    } else {
        w1
    };
    pos = skip_space(text, pos);
    let mut body = None;
    let w2 = word_end(text, pos);
    if w2 > pos {
        pos = w2;
    } else {
        let from = pos;
        while pos < text.len() && !text[pos..].starts_with("}%%") {
            let c = text[pos..].chars().next()?;
            if !is_terminator(c) {
                pos += c.len_utf8();
            } else if c == '\n' {
                pos += 1;
            } else if c == '\r' && text[pos + 1..].starts_with('\n') {
                pos += 2;
            } else {
                break;
            }
        }
        body = Some(text[from..pos].to_owned());
    }
    pos = skip_space(text, pos);
    if text[pos..].starts_with("}%%") {
        pos += 3;
    }
    Some(Directive {
        start,
        end: pos,
        kind,
        body,
    })
}

fn directives_in(text: &str) -> Vec<Directive> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(i) = text[at..].find("%%{") {
        let start = at + i;
        match directive_at(text, start) {
            Some(directive) => {
                at = directive.end;
                found.push(directive);
            }
            None => at = start + 1,
        }
    }
    found
}

fn remove_spans(text: &str, spans: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for &(start, end) in spans {
        out.push_str(&text[last..start]);
        last = end;
    }
    out.push_str(&text[last..]);
    out
}

fn remove_all(text: &str, spans: Vec<(usize, usize)>, steps: &mut Steps) -> String {
    if let Some(steps) = steps.as_deref_mut() {
        steps.push(
            spans
                .iter()
                .map(|&(from, to)| Edit::cut(from, to))
                .collect(),
        );
    }
    remove_spans(text, &spans)
}

pub fn cleanup_comments(text: &str, steps: &mut Steps) -> String {
    let text = if text.contains("%%") {
        let spans = comment_spans(text);
        remove_all(text, spans, steps)
    } else {
        text.to_owned()
    };
    let trimmed = trim_start(&text);
    if let Some(steps) = steps.as_deref_mut() {
        steps.push(vec![Edit::cut(0, text.len() - trimmed.len())]);
    }
    trimmed.to_owned()
}

pub fn normalize_line_breaks(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        text.to_owned()
    }
}

struct FrontMatter {
    indent: String,
    body: String,
    length: usize,
}

fn is_line_break(c: char) -> bool {
    c == '\n' || c == '\r'
}

fn closing_end(text: &str, i: usize, fence: &str) -> Option<usize> {
    let mut last = None;
    for (offset, c) in text[i + 1 + fence.len()..].char_indices() {
        if !is_js_space(c) {
            break;
        }
        if is_line_break(c) {
            last = Some(i + 1 + fence.len() + offset);
        }
    }
    last
}

fn last_index_from(text: &str, needle: &str, from: i64) -> Option<usize> {
    let limit = (from.max(0) as usize + needle.len()).min(text.len());
    text[..text.floor_char_boundary(limit)].rfind(needle)
}

fn match_front_matter(text: &str) -> Option<FrontMatter> {
    let cursor = text.len()
        - text
            .trim_start_matches(|c| is_js_space(c) && !is_line_break(c))
            .len();
    let indent = &text[..cursor];
    if !text[cursor..].starts_with("---") {
        return None;
    }
    let mut opening_ends = Vec::new();
    for (offset, c) in text[cursor + 3..].char_indices() {
        if !is_js_space(c) {
            break;
        }
        if is_line_break(c) {
            opening_ends.push(cursor + 3 + offset);
        }
    }
    let first_opening = *opening_ends.first()?;

    let fence = format!("{indent}---");
    let needle = format!("\n{fence}");
    let mut last_fence = None;
    let mut i = last_index_from(text, &needle, text.len() as i64);
    while let Some(at) = i {
        if at <= first_opening {
            break;
        }
        if closing_end(text, at, &fence).is_some() {
            last_fence = Some(at);
            break;
        }
        i = last_index_from(text, &needle, at as i64 - 1);
    }
    let last_fence = last_fence?;

    let opening_end = opening_ends
        .iter()
        .copied()
        .rfind(|&c| c < last_fence)
        .unwrap_or(first_opening);
    let body_start = opening_end + 1;
    let first_at = body_start.saturating_sub(1);
    let first = text[first_at..].find(&needle).map(|found| first_at + found);
    std::iter::successors(first, |&at| {
        text[at + 1..].find(&needle).map(|found| at + 1 + found)
    })
    .take_while(|&at| at <= last_fence)
    .filter(|&at| at >= body_start)
    .find_map(|at| {
        closing_end(text, at, &fence).map(|end| FrontMatter {
            indent: indent.to_owned(),
            body: text[body_start..at].to_owned(),
            length: end + 1,
        })
    })
}

fn merge(target: &mut Object, source: &Object) {
    for (key, value) in source.iter() {
        if UNSAFE_KEYS.contains(&key) {
            continue;
        }
        match value {
            Value::Object(incoming) => match target.get_mut(key) {
                Some(Value::Object(current)) => merge(current, incoming),
                _ => {
                    let mut fresh = Object::new();
                    merge(&mut fresh, incoming);
                    target.set(key, Value::Object(fresh));
                }
            },
            other => target.set(key, other.clone()),
        }
    }
}

fn apply_directives(text: &str, config: &mut Config, steps: &mut Steps) -> String {
    if !text.contains("%%{") {
        return text.to_owned();
    }
    let normalized = trim(text).replace('\'', "\"");
    for directive in directives_in(&normalized) {
        if directive.kind == "wrap" {
            config.set("wrap", Value::Bool(true));
        } else if matches!(directive.kind.as_str(), "init" | "initialize")
            && let Some(body) = directive.body.as_deref().filter(|body| !body.is_empty())
            && let Some(Value::Object(args)) = json::parse(trim(body))
        {
            merge(config, &args);
        }
    }
    let spans = directives_in(text)
        .iter()
        .map(|d| (d.start, d.end))
        .collect();
    remove_all(text, spans, steps)
}

fn rewrite_attributes(attrs: &str) -> String {
    let mut out = String::with_capacity(attrs.len());
    let mut rest = attrs;
    while let Some(at) = rest.find("=\"") {
        let Some(close) = rest[at + 2..].find('"') else {
            break;
        };
        out.push_str(&rest[..at]);
        out.push_str("='");
        out.push_str(&rest[at + 2..at + 2 + close]);
        out.push('\'');
        rest = &rest[at + 2 + close + 1..];
    }
    out.push_str(rest);
    out
}

fn rewrite_tags(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(i) = text[at..].find('<') {
        let start = at + i;
        let name = word_end(text, start + 1);
        let close = text[name..].find('>').map(|c| name + c);
        match close {
            Some(close) if name > start + 1 => {
                out.push_str(&text[at..start]);
                out.push('<');
                out.push_str(&text[start + 1..name]);
                out.push_str(&rewrite_attributes(&text[name..close]));
                out.push('>');
                at = close + 1;
            }
            _ => {
                out.push_str(&text[at..start + 1]);
                at = start + 1;
            }
        }
    }
    out.push_str(&text[at..]);
    out
}

fn string_of(value: &Value) -> Option<String> {
    value.is_truthy().then(|| value.to_js_string())
}

pub fn preprocess(source: &str) -> Result<Preprocessed, PelersError> {
    preprocess_steps(source, None)
}

pub fn preprocess_steps(source: &str, mut steps: Steps) -> Result<Preprocessed, PelersError> {
    let mut text = normalize_line_breaks(source);
    let last_close = if text.contains('<') {
        text.rfind('>')
    } else {
        None
    };
    if let Some(close) = last_close.filter(|&close| close > 0) {
        text = rewrite_tags(&text[..close + 1]) + &text[close + 1..];
    }

    let mut config = Config::new();
    let mut title = None;
    let mut line_offset = 0;

    let front = if text.contains("---") {
        match_front_matter(&text)
    } else {
        None
    };
    if let Some(front) = front {
        let body = if front.indent.is_empty() {
            front.body.clone()
        } else {
            front
                .body
                .split('\n')
                .map(|line| line.strip_prefix(front.indent.as_str()).unwrap_or(line))
                .collect::<Vec<_>>()
                .join("\n")
        };
        let parsed = parse_yaml(&body).map_err(|message| {
            PelersError::new(
                format!("Front matter is not valid YAML. {message}"),
                PelersErrorCode::Syntax,
            )
        })?;
        if let Value::Object(map) = &parsed {
            title = map.get("title").and_then(string_of);
            if let Some(Value::Object(given)) = map.get("config") {
                merge(&mut config, given);
            }
            if let Some(mode) = map.get("displayMode").filter(|mode| mode.is_truthy()) {
                if !matches!(config.get("gantt"), Some(Value::Object(_))) {
                    config.set("gantt", Value::Object(Object::new()));
                }
                config
                    .get_mut("gantt")
                    .and_then(Value::as_object_mut)
                    .expect("gantt was just set to an object")
                    .set("displayMode", Value::String(mode.to_js_string()));
            }
        }
        line_offset = text[..front.length].matches('\n').count();
        text = text[front.length..].to_owned();
        if let Some(steps) = steps.as_deref_mut() {
            steps.push(vec![Edit::cut(0, front.length)]);
        }
    }

    let with_comments = apply_directives(&text, &mut config, &mut steps);
    Ok(Preprocessed {
        text: cleanup_comments(&with_comments, &mut None),
        title,
        config,
        with_comments,
        line_offset,
    })
}

fn drop_color_semicolons(text: &str, keyword: &str, steps: &mut Steps) -> String {
    let mut drops = Vec::new();
    let mut from = text.find(keyword);
    while let Some(start) = from {
        let line_end = text[start..]
            .find(is_terminator)
            .map_or(text.len(), |i| start + i);
        let mut after_colon = false;
        let mut hash = None;
        for (j, c) in text[start + keyword.len()..line_end].char_indices() {
            match c {
                ':' => after_colon = true,
                '#' if after_colon => {
                    hash = Some(start + keyword.len() + j);
                    break;
                }
                c if is_js_space(c) => after_colon = false,
                _ => {}
            }
        }
        if let Some(hash) = hash
            && let Some(semi) = text[..line_end].rfind(';').filter(|&semi| semi > hash)
        {
            drops.push(semi);
        }
        from = if line_end >= text.len() {
            None
        } else {
            let next = line_end + text[line_end..].chars().next().map_or(1, char::len_utf8);
            text[next..].find(keyword).map(|found| next + found)
        };
    }
    if let Some(steps) = steps.as_deref_mut() {
        steps.push(
            drops
                .iter()
                .map(|&semi| Edit::cut(semi, semi + 1))
                .collect(),
        );
    }
    let spans: Vec<(usize, usize)> = drops.iter().map(|&semi| (semi, semi + 1)).collect();
    remove_spans(text, &spans)
}

pub fn encode_entities(text: &str, mut steps: Steps) -> String {
    if !text.contains('#') {
        return text.to_owned();
    }
    let text = drop_color_semicolons(
        &drop_color_semicolons(text, "style", &mut steps),
        "classDef",
        &mut steps,
    );
    let mut edits = Vec::new();
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    let mut consumed = 0;
    while let Some(at) = rest.find('#') {
        out.push_str(&rest[..at]);
        let body = &rest[at + 1..];
        let word = word_end(body, 0);
        if word > 0 && body[word..].starts_with(';') {
            let inner = &body[..word];
            let mut code = String::from(if inner.bytes().all(|b| b.is_ascii_digit()) {
                "ﬂ°°"
            } else {
                "ﬂ°"
            });
            code.push_str(inner);
            code.push_str("¶ß");
            out.push_str(&code);
            edits.push(Edit::swap(consumed + at, consumed + at + word + 2, code));
            consumed += at + word + 2;
            rest = &body[word + 1..];
        } else {
            out.push('#');
            consumed += at + 1;
            rest = body;
        }
    }
    out.push_str(rest);
    if let Some(steps) = steps {
        steps.push(edits);
    }
    out
}

pub fn source_map(source: &str, keep_comments: bool) -> (String, Vec<Vec<Edit>>) {
    let mut steps = Vec::new();
    if let Ok(pre) = preprocess_steps(source, Some(&mut steps)) {
        let kept = if keep_comments {
            pre.with_comments
        } else {
            cleanup_comments(&pre.with_comments, &mut Some(&mut steps))
        };
        encode_entities(&kept, Some(&mut steps));
    }
    (normalize_line_breaks(source) + "\n", steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_directive_bodies_across_crlf_and_stops_at_other_terminators() {
        let crlf = directives_in("%%{init: {\"a\":\r\n1}}%%");
        assert_eq!(crlf.len(), 1);
        assert_eq!(crlf[0].body.as_deref(), Some("{\"a\":\r\n1}"));
        let lone = directives_in("%%{init: {\"a\":\r1}}%%");
        assert_eq!(lone[0].body.as_deref(), Some("{\"a\":"));
        assert_eq!(lone[0].end, 15);
    }

    #[test]
    fn maps_sources_with_kept_comments_and_invalid_front_matter() {
        let (text, steps) = source_map("%%{wrap}%%\ninfo\n%% c\n", true);
        assert_eq!(text, "%%{wrap}%%\ninfo\n%% c\n\n");
        assert!(!steps.is_empty());
        let (text, steps) = source_map("---\n{ a: 1\n---\ninfo", false);
        assert_eq!(text, "---\n{ a: 1\n---\ninfo\n");
        assert!(steps.is_empty());
    }
}
