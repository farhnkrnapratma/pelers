use regex::Regex;
use std::sync::OnceLock;

use crate::text::entities::decode_entities;
use crate::util::jsnum::js_number;

const BLANK: &str = "about:blank";

struct Res {
    invalid_protocol: Regex,
    html_entities: Regex,
    html_ctrl_entity: Regex,
    ctrl: Regex,
    scheme: Regex,
    ws_escape: Regex,
}

fn res() -> &'static Res {
    static RES: OnceLock<Res> = OnceLock::new();
    RES.get_or_init(|| Res {
        invalid_protocol: Regex::new(r"(?im)^([^A-Za-z0-9_]*)(javascript|data|vbscript)")
            .expect("protocol"),
        html_entities: Regex::new(r"&#((?-u:\w)+)(;)?").expect("entities"),
        html_ctrl_entity: Regex::new(r"(?i)&(newline|tab);").expect("ctrl entity"),
        ctrl: Regex::new("[\u{0}-\u{1f}\u{7f}-\u{9f}\u{2000}-\u{200d}\u{feff}\u{fffe}\u{ffff}]")
            .expect("ctrl"),
        scheme: Regex::new(r"(?im)^[^\n\r\x{2028}\x{2029}]+(:|&colon;)").expect("scheme"),
        ws_escape: Regex::new(r"(\\|%5[cC])((%(6[eE]|72|74))|[nrt])").expect("ws escape"),
    })
}

fn from_char_code(value: f64) -> char {
    let unit = if value.is_finite() {
        (value.trunc().rem_euclid(65536.0)) as u32
    } else {
        0
    };
    char::from_u32(unit).unwrap_or('\u{fffd}')
}

fn decode_uri_component(uri: &str) -> Option<String> {
    let bytes = uri.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn decode(uri: &str) -> String {
    decode_uri_component(uri).unwrap_or_else(|| uri.to_owned())
}

fn unwrap_url(url: &str, percent: bool) -> Option<String> {
    let r = res();
    let mut out = if percent {
        decode(url.trim())
    } else {
        url.trim().to_owned()
    };
    let mut rounds = 0;
    loop {
        rounds += 1;
        if rounds > 32 {
            return None;
        }
        out = r.ctrl.replace_all(&out, "").into_owned();
        out = r
            .html_entities
            .replace_all(&out, |caps: &regex::Captures| {
                from_char_code(js_number(&caps[1])).to_string()
            })
            .into_owned();
        out = r.html_ctrl_entity.replace_all(&out, "").into_owned();
        out = r.ctrl.replace_all(&out, "").into_owned();
        out = r.ws_escape.replace_all(&out, "").into_owned();
        out = out.trim().to_owned();
        if percent {
            out = decode(&out);
        }
        let more = r.ctrl.is_match(&out)
            || r.html_entities.is_match(&out)
            || r.html_ctrl_entity.is_match(&out)
            || r.ws_escape.is_match(&out);
        if !more {
            break;
        }
    }
    Some(out)
}

pub fn sanitize_url(url: &str) -> String {
    let r = res();
    if url.is_empty() {
        return BLANK.to_owned();
    }
    let decoded = unwrap_url(url, true);
    let kept = unwrap_url(url, false);
    let (Some(decoded), Some(kept)) = (decoded, kept) else {
        return BLANK.to_owned();
    };
    if decoded.is_empty() || kept.is_empty() {
        return BLANK.to_owned();
    }
    if decoded.starts_with('.') || decoded.starts_with('/') {
        return kept;
    }
    let Some(scheme) = r.scheme.find(decoded.trim_start()) else {
        return kept;
    };
    let protocol = scheme.as_str().to_lowercase();
    let protocol = protocol.trim();
    if r.invalid_protocol.is_match(protocol) {
        return BLANK.to_owned();
    }
    let normalized = kept.trim_start().replace('\\', "/");
    if protocol == "mailto:" || protocol.contains("://") {
        return normalized;
    }
    if protocol == "http:" || protocol == "https:" {
        return match url::Url::parse(&normalized) {
            Ok(parsed) => parsed.to_string(),
            Err(_) => BLANK.to_owned(),
        };
    }
    normalized
}

pub fn safe_url(url: &str) -> String {
    sanitize_url(&decode_entities(url))
}

const LINK_SCHEMES: &[&str] = &["http", "https", "mailto", "tel"];
const IMAGE_SCHEMES: &[&str] = &["http", "https"];

fn scheme_name(url: &str) -> Option<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?i)^([a-z][a-z0-9+.-]*):").expect("scheme name"));
    re.captures(url).map(|c| c[1].to_lowercase())
}

fn network_path(url: &str) -> bool {
    url.starts_with("//")
        || url.starts_with("\\\\")
        || url.starts_with("/\\")
        || url.starts_with("\\/")
}

fn allowed(url: &str, schemes: &[String]) -> bool {
    if network_path(url) {
        return false;
    }
    match scheme_name(url) {
        None => true,
        Some(scheme) => schemes.contains(&scheme),
    }
}

fn allow(url: &str, schemes: &[String]) -> Option<String> {
    let decoded = unwrap_url(url, true)?;
    if allowed(url, schemes) && allowed(&decoded, schemes) {
        Some(url.to_owned())
    } else {
        None
    }
}

fn schemes_or(list: &Option<Vec<String>>, default: &[&str]) -> Vec<String> {
    match list {
        Some(list) => list.clone(),
        None => default.iter().map(|s| (*s).to_owned()).collect(),
    }
}

pub fn link_url(url: &str, options: &crate::svg::root::RenderOptions) -> Option<String> {
    if options.links == Some(false) {
        return None;
    }
    allow(url, &schemes_or(&options.link_schemes, LINK_SCHEMES))
}

pub fn image_url(url: &str, options: &crate::svg::root::RenderOptions) -> Option<String> {
    if options.images == Some(false) {
        return None;
    }
    allow(url, &schemes_or(&options.image_schemes, IMAGE_SCHEMES))
}

pub fn internal_link(name: &str, options: &crate::svg::root::RenderOptions) -> bool {
    let url = sanitize_url(name);
    if url == BLANK || link_url(&url, options).is_none() {
        return false;
    }
    let dropped: String = name
        .chars()
        .filter(|&c| !crate::svg::builder::invalid_xml(c))
        .collect();
    let mut seen: String = dropped
        .chars()
        .filter(|&c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let leading = seen.len() - seen.trim_start_matches(|c: char| (c as u32) <= 0x20).len();
    seen.drain(..leading);
    link_url(&seen, options).is_some()
}

pub fn rel_attr(options: &crate::svg::root::RenderOptions) -> String {
    let rel = options
        .link_rel
        .clone()
        .unwrap_or_else(|| "noopener".to_owned());
    if rel.is_empty() {
        String::new()
    } else {
        format!(" rel=\"{}\"", crate::svg::builder::esc(&rel))
    }
}

pub fn linked(
    body: &str,
    url: &str,
    id: &str,
    links: &mut Vec<crate::svg::options::LinkInfo>,
    options: &crate::svg::root::RenderOptions,
    attrs: &str,
) -> String {
    let Some(href) = link_url(url, options) else {
        return body.to_owned();
    };
    links.push(crate::svg::options::LinkInfo {
        id: id.to_owned(),
        href: href.clone(),
        internal: false,
    });
    format!(
        "<a href=\"{}\"{attrs}{}>{body}</a>",
        crate::svg::builder::esc(&href),
        rel_attr(options)
    )
}
