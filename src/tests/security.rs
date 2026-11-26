use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::{RenderOptions, parse, render_with};

const PAYLOADS: [&str; 7] = [
    "<script>alert(1)</script>",
    "\"><img src=x onerror=alert(1)>",
    "' onmouseover='alert(1)' x='",
    "</text><svg onload=alert(1)>",
    "javascript:alert(1)",
    "&lt;script&gt;alert(1)&lt;/script&gt;",
    "<a href=javascript:alert(1)>x</a>",
];

const SAFE_TAGS: [&str; 20] = [
    "svg",
    "g",
    "text",
    "tspan",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "title",
    "desc",
    "a",
    "image",
    "defs",
    "marker",
    "linearGradient",
    "stop",
    "clipPath",
];

static QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"\n]*""#).unwrap());
static BRACKETED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[[^\]\n]*\]").unwrap());
static PARENTHESIZED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\([^()\n]*\)").unwrap());
static ENTITY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"&#(x[0-9a-fA-F]+|[0-9]+);?").unwrap());
static TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<(/?)([A-Za-z][A-Za-z0-9:-]*)([^>]*)>").unwrap());
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"([A-Za-z_:][-A-Za-z0-9_:.]*)\s*=\s*("([^"]*)"|'([^']*)'|([^\s"'>]+))"#).unwrap()
});

const BAD_SCHEMES: [&str; 4] = [
    "javascript:",
    "vbscript:",
    "data:text/html",
    "data:image/svg",
];

fn hostile_variants(source: &str) -> Vec<String> {
    let first = source
        .lines()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(0);
    let mut lines: Vec<&str> = source.lines().collect();
    let header: Vec<&str> = lines
        .drain(..=first.min(lines.len().saturating_sub(1)))
        .collect();
    let header = header.join("\n");
    let body = lines.join("\n");
    let mut variants = Vec::new();
    for payload in PAYLOADS {
        variants.push(
            QUOTED
                .replace_all(source, format!("\"{payload}\""))
                .into_owned(),
        );
        variants.push(
            BRACKETED
                .replace_all(source, format!("[{payload}]"))
                .into_owned(),
        );
        variants.push(
            PARENTHESIZED
                .replace_all(source, format!("({payload})"))
                .into_owned(),
        );
        variants.push(format!("---\ntitle: {payload}\n---\n{source}"));
        variants.push(format!(
            "{header}\n  accTitle: {payload}\n  accDescr: {payload}\n{body}"
        ));
        variants.push(format!("{header}\n  title {payload}\n{body}"));
    }
    variants.push(format!(
        "{source}\n  click A href \"javascript:alert(1)\"\n  click B \"javascript:alert(1)\" \"<script>\"\n  link A \"javascript:alert(1)\""
    ));
    variants.push(format!(
        "%%{{init: {{\"themeVariables\": {{\"primaryColor\": \"red;}}</style><script>alert(1)</script>\", \"fontFamily\": \"x\\\" onload=\\\"alert(1)\"}}, \"fontFamily\": \"x' onload='alert(1)\"}}}}%%\n{source}"
    ));
    variants
}

fn decoded(value: &str) -> String {
    let value = ENTITY.replace_all(value, |caps: &regex::Captures| {
        let code = &caps[1];
        let number = match code.strip_prefix('x') {
            Some(hex) => u32::from_str_radix(hex, 16).ok(),
            None => code.parse().ok(),
        };
        number
            .and_then(char::from_u32)
            .map(String::from)
            .unwrap_or_default()
    });
    value
        .replace("&amp;", "&")
        .replace("&colon;", ":")
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_control())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn violations(svg: &str) -> Vec<String> {
    let mut found = Vec::new();
    if svg.contains("<!") || svg.contains("<?") {
        found.push("markup declaration or processing instruction".to_owned());
    }
    for caps in TAG.captures_iter(svg) {
        let name = &caps[2];
        if !SAFE_TAGS.contains(&name) {
            found.push(format!("unexpected element <{name}>"));
        }
        for attr in ATTRIBUTE.captures_iter(&caps[3]) {
            let key = attr[1].to_ascii_lowercase();
            let value = attr
                .get(3)
                .or(attr.get(4))
                .or(attr.get(5))
                .map_or("", |m| m.as_str());
            if key.starts_with("on") {
                found.push(format!("event handler attribute {key}"));
            }
            let plain = decoded(value);
            if matches!(key.as_str(), "href" | "xlink:href" | "src")
                && BAD_SCHEMES.iter().any(|scheme| plain.starts_with(scheme))
            {
                found.push(format!("unsafe {key}={value}"));
            }
            if key == "style"
                && ["javascript:", "expression(", "@import", "</"]
                    .iter()
                    .any(|bad| plain.contains(bad))
            {
                found.push(format!("unsafe style={value}"));
            }
        }
    }
    found
}

#[test]
fn hostile_labels_links_and_config_never_reach_the_svg_as_markup() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    let options = RenderOptions {
        links: Some(true),
        images: Some(true),
        ..RenderOptions::default()
    };
    let mut rendered = 0;
    let mut types: Vec<_> = fs::read_dir(&golden)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    types.sort();
    for dir in types.iter().filter(|path| path.is_dir()) {
        let mut sources: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "mmd"))
            .collect();
        sources.sort();
        for path in sources.iter().take(6) {
            let source = fs::read_to_string(path).unwrap();
            for variant in hostile_variants(&source) {
                let Ok(model) = parse(&variant) else { continue };
                let Ok(result) = render_with(&model, &options) else {
                    continue;
                };
                rendered += 1;
                let problems = violations(&result.svg);
                assert!(
                    problems.is_empty(),
                    "{}: {problems:?}\n{variant}",
                    path.display()
                );
            }
        }
    }
    assert!(rendered > 1000, "only {rendered} hostile variants rendered");
}

#[test]
fn the_checker_catches_what_it_should() {
    assert!(violations("<svg><g onclick=\"x\"></g></svg>")[0].contains("onclick"));
    assert!(violations("<svg><script>x</script></svg>")[0].contains("<script>"));
    assert!(violations("<a href=\" JaVa&#x53;cript:x\">y</a>")[0].contains("unsafe href"));
    assert!(violations("<a href='data:text/html,x'>y</a>")[0].contains("unsafe href"));
    assert!(violations("<g style=\"background:url(javascript:x)\"></g>")[0].contains("style"));
    assert!(violations("<!DOCTYPE x><svg></svg>")[0].contains("declaration"));
    assert!(
        violations("<svg><text>&lt;script&gt;</text><a href=\"https://x\">y</a></svg>").is_empty()
    );
    assert_eq!(decoded("&#106;ava&#x73;cript&colon;"), "javascript:");
    assert_eq!(decoded("&#xZZ;"), "&#xzz;");
}
