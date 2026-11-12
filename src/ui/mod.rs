use crate::util::chars::{is_js_space, trim};
use crate::util::jsnum::js_number_to_string;

pub const STEP: f64 = 1.25;
pub const PAN: f64 = 48.0;
pub const GAP: f64 = 4.0;

pub const BAR_STYLE: &str = "--_bg:var(--pelers-bg,#fff);--_fg:var(--pelers-fg,#1f1f1f);--_b:var(--pelers-border,#8a8a8a);position:absolute;top:var(--pelers-zoom-top,8px);right:var(--pelers-zoom-right,8px);bottom:var(--pelers-zoom-bottom,auto);left:var(--pelers-zoom-left,auto);display:flex;flex-direction:column;gap:2px;padding:2px;width:auto;max-width:none;border:1px solid var(--_b);border-radius:var(--pelers-radius,4px);background:var(--_bg);color:var(--_fg);line-height:0";

pub const BUTTON_STYLE: &str = "appearance:none;display:grid;place-items:center;width:24px;height:24px;margin:0;padding:0;border:0;border-radius:calc(var(--pelers-radius,4px) - 1px);background:transparent;color:inherit;font:inherit;cursor:var(--pelers-zoom-cursor,pointer)";

pub const OVER_STYLE: &str = "position:relative;height:1.3em;margin-top:-1.3em;line-height:1.3;font-size:0.8em;color:var(--pelers-error,#e45756);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;";

pub const ALONE_STYLE: &str = "margin:0;font-size:0.8em;font-family:var(--pelers-font-mono,monospace);color:var(--pelers-error,#e45756);white-space:pre-wrap;";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Icon {
    ZoomIn,
    ZoomOut,
    Reset,
}

impl Icon {
    pub fn path(self) -> &'static str {
        match self {
            Self::ZoomIn => "M5 12h14M12 5v14",
            Self::ZoomOut => "M5 12h14",
            Self::Reset => "M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8M3 3v5h5",
        }
    }

    pub fn default_label(self) -> &'static str {
        match self {
            Self::ZoomIn => "Zoom in",
            Self::ZoomOut => "Zoom out",
            Self::Reset => "Reset zoom",
        }
    }
}

pub fn zoom_icon_markup(icon: Icon) -> String {
    format!(
        "<svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" aria-hidden=\"true\"><path d=\"{}\"/></svg>",
        icon.path()
    )
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

fn js_min(a: f64, b: f64) -> f64 {
    crate::util::jsnum::js_min(a, b)
}

fn js_max(a: f64, b: f64) -> f64 {
    crate::util::jsnum::js_max(a, b)
}

pub fn frame(natural: Size, box_: Size, scale: f64, x: f64, y: f64) -> Frame {
    let width = box_.width / scale;
    let height = box_.height / scale;
    let within = |at: f64, view: f64, whole: f64| -> f64 {
        if view >= whole {
            js_min(js_max(at, whole - view), 0.0)
        } else {
            js_min(js_max(at, 0.0), whole - view)
        }
    };
    Frame {
        x: within(x, width, natural.width),
        y: within(y, height, natural.height),
        width,
        height,
    }
}

pub fn room_within(client_width: f64, padding_left: f64, padding_right: f64) -> f64 {
    let width = client_width - padding_left - padding_right;
    if width > 0.0 { width } else { 0.0 }
}

pub fn summary(message: &str) -> String {
    let lines: Vec<&str> = message
        .split('\n')
        .filter(|line| !trim(line).is_empty())
        .collect();
    if lines.len() > 1 {
        format!("{} {}", lines[0], lines[lines.len() - 1])
    } else {
        lines.first().copied().unwrap_or("").to_owned()
    }
}

pub enum Thrown {
    Error(String),
    Text(String),
    Number(f64),
    Null,
    Undefined,
}

pub fn message_of(error: &Thrown) -> String {
    match error {
        Thrown::Error(message) => message.clone(),
        Thrown::Text(text) => text.clone(),
        Thrown::Number(number) => js_number_to_string(*number),
        Thrown::Null => "null".to_owned(),
        Thrown::Undefined => "undefined".to_owned(),
    }
}

pub fn is_blank_line(line: &str) -> bool {
    line.chars().all(is_js_space)
}
