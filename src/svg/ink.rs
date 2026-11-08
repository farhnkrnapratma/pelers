use crate::util::chars::{is_js_space, trim};
use crate::util::jsnum::{js_max, js_min};

const DARK: &str = "black blue blueviolet brown crimson darkblue darkcyan darkgreen darkmagenta darkolivegreen darkorchid darkred darkslateblue darkslategray darkviolet dimgray firebrick forestgreen green indigo maroon mediumblue mediumslateblue mediumvioletred midnightblue navy olive purple rebeccapurple royalblue saddlebrown seagreen sienna slateblue steelblue teal";
const LIGHT: &str = "aliceblue antiquewhite aqua aquamarine azure beige bisque blanchedalmond burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk cyan darkgoldenrod darkgray darkkhaki darkorange darksalmon darkseagreen darkturquoise deeppink deepskyblue dodgerblue floralwhite fuchsia gainsboro ghostwhite gold goldenrod gray greenyellow honeydew hotpink indianred ivory khaki lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan lightgoldenrodyellow lightgray lightgreen lightpink lightsalmon lightseagreen lightskyblue lightslategray lightsteelblue lightyellow lime limegreen linen magenta mediumaquamarine mediumorchid mediumpurple mediumseagreen mediumspringgreen mediumturquoise mintcream mistyrose moccasin navajowhite oldlace olivedrab orange orangered orchid palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum powderblue red rosybrown salmon sandybrown seashell silver skyblue slategray snow springgreen tan thistle tomato turquoise violet wheat white whitesmoke yellow yellowgreen";

pub const ON_LIGHT: &str = "var(--pele-fg-on-light,#1f1f1f)";
pub const ON_DARK: &str = "var(--pele-fg-on-dark,#fff)";

fn listed(set: &str, name: &str) -> bool {
    set.split(' ').any(|entry| entry == name)
}

fn linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance(r: f64, g: f64, b: f64) -> f64 {
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn clamp(n: f64) -> f64 {
    js_min(js_max(n, 0.0), 1.0)
}

fn even() -> f64 {
    (1.05 * (luminance(31.0 / 255.0, 31.0 / 255.0, 31.0 / 255.0) + 0.05)).sqrt() - 0.05
}

fn is_digit(byte: u8) -> bool {
    byte.is_ascii_digit()
}

fn digits(bytes: &[u8], from: usize) -> usize {
    from + bytes[from..].iter().take_while(|b| is_digit(**b)).count()
}

struct Number<'a> {
    value: f64,
    unit: Option<&'a str>,
}

fn number(raw: &str) -> Option<Number<'_>> {
    let bytes = raw.as_bytes();
    let mut at = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let int_end = digits(bytes, at);
    if int_end > at {
        at = int_end;
        if bytes.get(at) == Some(&b'.') {
            let frac_end = digits(bytes, at + 1);
            if frac_end > at + 1 {
                at = frac_end;
            }
        }
    } else if bytes.get(at) == Some(&b'.') {
        let frac_end = digits(bytes, at + 1);
        if frac_end == at + 1 {
            return None;
        }
        at = frac_end;
    } else {
        return None;
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        let mut sign = at + 1;
        if matches!(bytes.get(sign), Some(b'+' | b'-')) {
            sign += 1;
        }
        let exp_end = digits(bytes, sign);
        if exp_end > sign {
            at = exp_end;
        }
    }
    let rest = &raw[at..];
    let unit = if rest.is_empty() {
        None
    } else if ["%", "deg", "grad", "rad", "turn"]
        .iter()
        .any(|u| rest.eq_ignore_ascii_case(u))
    {
        Some(rest)
    } else {
        return None;
    };
    let value = raw[..at].parse::<f64>().ok()?;
    Some(Number { value, unit })
}

fn amount(raw: &str, full: f64) -> f64 {
    match number(raw) {
        Some(Number { value, unit: None }) => clamp(value / full),
        Some(Number {
            value,
            unit: Some("%"),
        }) => clamp(value / 100.0),
        _ => f64::NAN,
    }
}

fn hue(raw: &str) -> f64 {
    let Some(Number { value, unit }) = number(raw) else {
        return f64::NAN;
    };
    match unit.map(str::to_ascii_lowercase).as_deref() {
        Some("%") => f64::NAN,
        Some("turn") => value * 360.0,
        Some("rad") => (value * 180.0) / std::f64::consts::PI,
        Some("grad") => value * 0.9,
        _ => value,
    }
}

fn split_runs(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_space = false;
    for c in text.chars() {
        if is_js_space(c) {
            if !in_space {
                out.push(std::mem::take(&mut current));
                in_space = true;
            }
        } else {
            in_space = false;
            current.push(c);
        }
    }
    out.push(current);
    out
}

fn args(body: &str, hsl: bool) -> Option<Vec<String>> {
    let legacy = body.contains(',');
    let pieces: Vec<&str> = if legacy {
        vec![body]
    } else {
        body.split('/').collect()
    };
    if pieces.len() >= 3 {
        return None;
    }
    let main = trim(pieces[0]);
    let mut list: Vec<String> = if legacy {
        main.split(',').map(|part| trim(part).to_owned()).collect()
    } else {
        split_runs(main)
    };
    if if legacy {
        list.len() < 3 || list.len() > 4
    } else {
        list.len() != 3
    } {
        return None;
    }
    if let Some(alpha) = pieces.get(1) {
        list.push(trim(alpha).to_owned());
    }
    if !legacy {
        return Some(
            list.into_iter()
                .map(|c| if c == "none" { "0".to_owned() } else { c })
                .collect(),
        );
    }
    let start = usize::from(hsl);
    let percent = list[start..3].iter().filter(|c| c.ends_with('%')).count();
    (percent == if hsl { 2 } else { 3 } || (!hsl && percent == 0)).then_some(list)
}

fn is_hex(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('#') else {
        return false;
    };
    matches!(digits.len(), 3 | 4 | 6 | 8) && digits.bytes().all(|b| b.is_ascii_hexdigit())
}

fn grey_to_gray(value: &str) -> String {
    value.replacen("grey", "gray", 1)
}

pub fn plain_color(raw: &str) -> bool {
    let value = trim(raw).to_lowercase();
    let name = grey_to_gray(&value);
    is_hex(&value)
        || listed(DARK, &name)
        || listed(LIGHT, &name)
        || value == "transparent"
        || value == "currentcolor"
}

fn function_parts(value: &str) -> Option<(&str, &str)> {
    let open = value.find('(')?;
    let body = value[open + 1..].strip_suffix(')')?;
    if body.contains(['(', ')']) {
        return None;
    }
    Some((&value[..open], body))
}

fn color(raw: &str) -> Option<(f64, f64)> {
    let value = trim(raw).to_lowercase();
    let name = grey_to_gray(&value);
    if listed(DARK, &name) {
        return Some((0.0, 1.0));
    }
    if listed(LIGHT, &name) {
        return Some((1.0, 1.0));
    }
    if is_hex(&value) {
        let hex = &value[1..];
        let digits: String = if value.len() < 6 {
            hex.chars().flat_map(|c| [c, c]).collect()
        } else {
            hex.to_owned()
        };
        let channel = |i: usize| {
            f64::from(u8::from_str_radix(&digits[2 * i..2 * i + 2], 16).unwrap()) / 255.0
        };
        let alpha = if digits.len() == 8 { channel(3) } else { 1.0 };
        return Some((luminance(channel(0), channel(1), channel(2)), alpha));
    }
    let (name, body) = function_parts(&value)?;
    let hsl = match name {
        "rgb" | "rgba" => false,
        "hsl" | "hsla" => true,
        _ => return None,
    };
    let list = args(body, hsl)?;
    let alpha = if list.len() == 4 {
        amount(&list[3], 1.0)
    } else {
        1.0
    };
    let rgb: Vec<f64> = if hsl {
        let h = (((hue(&list[0]) % 360.0) + 360.0) % 360.0) / 30.0;
        let s = amount(&list[1], 100.0);
        let l = amount(&list[2], 100.0);
        let a = s * js_min(l, 1.0 - l);
        [0.0, 8.0, 4.0]
            .iter()
            .map(|k| {
                let t = (k + h) % 12.0;
                l - a * js_max(-1.0, js_min(js_min(t - 3.0, 9.0 - t), 1.0))
            })
            .collect()
    } else {
        list[..3].iter().map(|c| amount(c, 255.0)).collect()
    };
    let lum = luminance(rgb[0], rgb[1], rgb[2]);
    (!(lum + alpha).is_nan()).then_some((lum, alpha))
}

fn fill_of(shape: &str) -> Option<&str> {
    let bytes = shape.as_bytes();
    (0..bytes.len().saturating_sub(5)).find_map(|i| {
        if matches!(bytes[i], b'"' | b';') && shape[i + 1..].starts_with("fill:") {
            let rest = &shape[i + 6..];
            Some(&rest[..rest.find([';', '"']).unwrap_or(rest.len())])
        } else {
            None
        }
    })
}

fn opacity(shape: &str) -> f64 {
    let bytes = shape.as_bytes();
    let mut product = 1.0;
    let mut i = 0;
    while i < bytes.len() {
        if matches!(bytes[i], b'"' | b';') {
            let rest = &shape[i + 1..];
            let key = rest
                .strip_prefix("fill-opacity:")
                .or_else(|| rest.strip_prefix("opacity:"));
            if let Some(after) = key {
                let value = &after[..after.find([';', '"']).unwrap_or(after.len())];
                product *= amount(trim(value), 1.0);
                i = shape.len() - after.len() + value.len();
                continue;
            }
        }
        i += 1;
    }
    product
}

pub fn ink(shape: &str, text: &str) -> Option<&'static str> {
    if fill_of(text).is_some() {
        return None;
    }
    let (lum, alpha) = color(fill_of(shape)?)?;
    let weight = alpha * opacity(shape);
    if weight.is_nan() || weight < 0.5 {
        return None;
    }
    Some(if lum > even() { ON_LIGHT } else { ON_DARK })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_ink_with_more_contrast() {
        assert_eq!(ink(";fill:#000", ""), Some(ON_DARK));
        assert_eq!(ink(";fill:#fff", ""), Some(ON_LIGHT));
        assert_eq!(ink(";fill:navy", ""), Some(ON_DARK));
        assert_eq!(ink(";fill:gold", ""), Some(ON_LIGHT));
        assert_eq!(ink(";fill:rgb(0 0 0)", ""), Some(ON_DARK));
        assert_eq!(ink(";fill:hsl(0, 0%, 100%)", ""), Some(ON_LIGHT));
        assert_eq!(ink(";fill:#0008", ""), Some(ON_DARK));
        assert_eq!(ink(";fill:#0004", ""), None);
        assert_eq!(ink(";fill:rebeccapurplex", ""), None);
        assert_eq!(ink(";fill:red", ";fill:blue"), None);
    }

    #[test]
    fn reads_css_numbers() {
        assert_eq!(amount("50%", 255.0), 0.5);
        assert_eq!(amount("127.5", 255.0), 0.5);
        assert!(amount("1.5deg", 255.0).is_nan());
        assert_eq!(hue("0.5turn"), 180.0);
        assert!(hue("5%").is_nan());
        assert!(number("1.").is_none());
        assert!(number(".5e1").is_some());
    }

    #[test]
    fn accepts_plain_colors_only() {
        assert!(plain_color(" #FFF "));
        assert!(plain_color("Grey"));
        assert!(!plain_color("#ff"));
        assert!(!plain_color("rgb(0,0,0)"));
    }

    #[test]
    fn scans_number_edges() {
        assert!(number(".").is_none());
        assert!(number("-.x").is_none());
        assert_eq!(number("1e+2").map(|n| n.value), Some(100.0));
        assert!(number("1e").is_none());
        assert!(number("1e+").is_none());
        assert_eq!(split_runs(""), vec![String::new()]);
    }

    #[test]
    fn finds_fill_after_other_declarations() {
        assert_eq!(fill_of("stroke:red;fill:#fff"), Some("#fff"));
        assert_eq!(fill_of("stroke:red"), None);
    }

    #[test]
    fn weighs_fill_by_opacity() {
        assert_eq!(ink(";fill:#fff;opacity:0.2", ""), None);
        assert_eq!(ink(";fill:#fff;fill-opacity:50%;", ""), Some(ON_LIGHT));
        assert_eq!(ink(";fill:#fff;opacity:x", ""), None);
        assert_eq!(opacity(";opacity:0.5;fill-opacity:0.5\""), 0.25);
    }
}
