use std::collections::HashMap;

use crate::errors::{PelersError, PelersErrorCode};
use crate::util::chars::{trim, trim_end, trim_start};
use crate::util::jsnum::js_number;

#[derive(Clone, Debug, PartialEq)]
pub struct QuadrantText {
    pub text: String,
    pub markdown: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointStyle {
    pub radius: Option<f64>,
    pub color: Option<String>,
    pub stroke_color: Option<String>,
    pub stroke_width: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuadrantPoint {
    pub x: f64,
    pub y: f64,
    pub text: QuadrantText,
    pub class_name: String,
    pub style: PointStyle,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuadrantDb {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub quadrants: [Option<QuadrantText>; 4],
    pub x_axis_left: Option<QuadrantText>,
    pub x_axis_right: Option<QuadrantText>,
    pub y_axis_bottom: Option<QuadrantText>,
    pub y_axis_top: Option<QuadrantText>,
    pub points: Vec<QuadrantPoint>,
    pub classes: HashMap<String, PointStyle>,
}

fn invalid(style: &str, value: &str, kind: &str) -> PelersError {
    let mut error = PelersError::new(
        format!("value for {style} {value} is invalid, please use a valid {kind}"),
        PelersErrorCode::Semantic,
    );
    error.diagram_type = Some("quadrantChart".to_owned());
    error
}

fn unsupported(key: &str) -> PelersError {
    let mut error = PelersError::new(
        format!("style named {key} is not supported."),
        PelersErrorCode::Semantic,
    );
    error.diagram_type = Some("quadrantChart".to_owned());
    error
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

fn is_hex_color(text: &str) -> bool {
    let body = text.strip_prefix('#').unwrap_or(text);
    (body.len() == 6 || body.len() == 3) && body.bytes().all(|b| b.is_ascii_hexdigit())
}

fn is_pixels(text: &str) -> bool {
    text.strip_suffix("px").is_some_and(is_digits)
}

fn key_value(style: &str) -> (String, String) {
    let s = trim(style);
    let Some(colon) = s.find(':') else {
        return (s.to_owned(), "undefined".to_owned());
    };
    let rest = trim_start(&s[colon + 1..]);
    let key = trim_end(&s[..colon]).to_owned();
    let value = match rest.find(':') {
        Some(next) => trim_end(&rest[..next]).to_owned(),
        None => rest.to_owned(),
    };
    (key, value)
}

pub fn parse_styles(styles: &[String]) -> Result<PointStyle, PelersError> {
    let mut out = PointStyle::default();
    for style in styles {
        let (key, value) = key_value(style);
        match key.as_str() {
            "radius" => {
                if !is_digits(&value) {
                    return Err(invalid(&key, &value, "number"));
                }
                out.radius = Some(js_number(&value));
            }
            "color" => {
                if !is_hex_color(&value) {
                    return Err(invalid(&key, &value, "hex code"));
                }
                out.color = Some(value);
            }
            "stroke-color" => {
                if !is_hex_color(&value) {
                    return Err(invalid(&key, &value, "hex code"));
                }
                out.stroke_color = Some(value);
            }
            "stroke-width" => {
                if !is_pixels(&value) {
                    return Err(invalid(&key, &value, "number of pixels (eg. 10px)"));
                }
                out.stroke_width = Some(value);
            }
            _ => return Err(unsupported(&key)),
        }
    }
    Ok(out)
}

pub fn trimmed(label: QuadrantText) -> QuadrantText {
    QuadrantText {
        text: trim(&label.text).to_owned(),
        markdown: label.markdown,
    }
}

impl QuadrantDb {
    pub fn add_point(
        &mut self,
        label: QuadrantText,
        class_name: String,
        x: &str,
        y: &str,
        styles: &[String],
    ) -> Result<(), PelersError> {
        self.points.push(QuadrantPoint {
            x: js_number(x),
            y: js_number(y),
            text: trimmed(label),
            class_name,
            style: parse_styles(styles)?,
        });
        Ok(())
    }

    pub fn add_class(&mut self, class_name: String, styles: &[String]) -> Result<(), PelersError> {
        let style = parse_styles(styles)?;
        self.classes.insert(class_name, style);
        Ok(())
    }
}
