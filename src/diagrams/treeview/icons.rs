use super::model::NodeType;
use crate::util::value::{Object, Value};

pub const BUILTIN_PACK: &str = "mermaid-treeview";

pub fn glyph(name: &str) -> Option<&'static str> {
    match name {
        "folder" => Some(
            "M3 6.5A1.5 1.5 0 0 1 4.5 5h4.6l2.4 2.5h8A1.5 1.5 0 0 1 21 9v9a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18z",
        ),
        "file" => Some(
            "M6.5 3h7L19 8.5v11a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 5 19.5v-15A1.5 1.5 0 0 1 6.5 3zM13.5 3v5.5H19",
        ),
        _ => None,
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IconConfig {
    pub show_icons: bool,
    pub default_icon_pack: String,
    pub filename_icons: Option<Object>,
    pub extension_icons: Option<Object>,
}

fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => *n != 0.0 && !n.is_nan(),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

fn own(map: Option<&Object>, key: &str) -> Option<String> {
    match map?.get(key)? {
        Value::String(text) => Some(text.clone()),
        _ => None,
    }
}

pub fn detect_icon(name: &str, config: &IconConfig) -> Option<String> {
    if let Some(by_name) = own(config.filename_icons.as_ref(), name).filter(|s| !s.is_empty()) {
        return Some(by_name);
    }
    let dot = name.rfind('.')?;
    if dot == 0 {
        return None;
    }
    let ext = name[dot..].to_lowercase();
    own(config.extension_icons.as_ref(), &ext)
        .or_else(|| own(config.extension_icons.as_ref(), &ext[1..]))
}

fn qualify(icon: &str, pack: &str) -> String {
    if icon.contains(':') {
        return icon.to_owned();
    }
    if glyph(icon).is_some() || pack.is_empty() {
        return format!("{BUILTIN_PACK}:{icon}");
    }
    format!("{pack}:{icon}")
}

pub fn node_icon(
    name: &str,
    node_type: NodeType,
    icon: Option<&str>,
    config: &IconConfig,
) -> Option<String> {
    if icon == Some("none") {
        return None;
    }
    if let Some(icon) = icon.filter(|s| !s.is_empty()) {
        return Some(qualify(icon, &config.default_icon_pack));
    }
    if !config.show_icons {
        return None;
    }
    if node_type == NodeType::File
        && let Some(detected) = detect_icon(name, config).filter(|s| !s.is_empty())
    {
        if detected == "none" {
            return None;
        }
        return Some(qualify(&detected, &config.default_icon_pack));
    }
    let base = match node_type {
        NodeType::Directory => "folder",
        NodeType::File => "file",
    };
    Some(format!("{BUILTIN_PACK}:{base}"))
}

pub fn icon_config(config: &Object) -> IconConfig {
    let section = match config.get("treeView") {
        Some(Value::Object(object)) => object.clone(),
        _ => Object::default(),
    };
    IconConfig {
        show_icons: truthy(section.get("showIcons")),
        default_icon_pack: match section.get("defaultIconPack") {
            Some(Value::String(pack)) => pack.clone(),
            _ => String::new(),
        },
        filename_icons: match section.get("filenameIcons") {
            Some(Value::Object(object)) => Some(object.clone()),
            _ => None,
        },
        extension_icons: match section.get("extensionIcons") {
            Some(Value::Object(object)) => Some(object.clone()),
            _ => None,
        },
    }
}
