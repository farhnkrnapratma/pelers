use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::util::chars::is_js_space;

const KEYWORDS: &str = "
    aliceblue antiquewhite aqua aquamarine azure beige bisque black blanchedalmond blue
    blueviolet brown burlywood cadetblue chartreuse chocolate coral cornflowerblue cornsilk
    crimson cyan darkblue darkcyan darkgoldenrod darkgray darkgreen darkgrey darkkhaki
    darkmagenta darkolivegreen darkorange darkorchid darkred darksalmon darkseagreen
    darkslateblue darkslategray darkslategrey darkturquoise darkviolet deeppink deepskyblue
    dimgray dimgrey dodgerblue firebrick floralwhite forestgreen fuchsia gainsboro ghostwhite
    gold goldenrod gray green greenyellow grey honeydew hotpink indianred indigo ivory khaki
    lavender lavenderblush lawngreen lemonchiffon lightblue lightcoral lightcyan
    lightgoldenrodyellow lightgray lightgreen lightgrey lightpink lightsalmon lightseagreen
    lightskyblue lightslategray lightslategrey lightsteelblue lightyellow lime limegreen linen
    magenta maroon mediumaquamarine mediumblue mediumorchid mediumpurple mediumseagreen
    mediumslateblue mediumspringgreen mediumturquoise mediumvioletred midnightblue mintcream
    mistyrose moccasin navajowhite navy oldlace olive olivedrab orange orangered orchid
    palegoldenrod palegreen paleturquoise palevioletred papayawhip peachpuff peru pink plum
    powderblue purple rebeccapurple red rosybrown royalblue saddlebrown salmon sandybrown
    seagreen seashell sienna silver skyblue slateblue slategray slategrey snow springgreen
    steelblue tan teal thistle tomato turquoise violet wheat white whitesmoke yellow
    yellowgreen transparent currentcolor inherit initial unset revert accentcolor
    accentcolortext activetext buttonborder buttonface buttontext canvas canvastext field
    fieldtext graytext highlight highlighttext linktext mark marktext selecteditem
    selecteditemtext visitedtext activeborder activecaption appworkspace background
    buttonhighlight buttonshadow captiontext inactiveborder inactivecaption inactivecaptiontext
    infobackground infotext menu menutext scrollbar threeddarkshadow threedface threedhighlight
    threedlightshadow threedshadow window windowframe windowtext
";

#[derive(Clone, Copy, PartialEq, Eq)]
enum K {
    Bad,
    Number,
    Percent,
    Angle,
    None,
}

fn term_pattern() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(
            r"(?i)^[+-]?(?:[0-9]+(?:\.[0-9]+)?|\.[0-9]+)(?:e[+-]?[0-9]+)?(%|deg|grad|rad|turn)?$",
        )
        .expect("term pattern")
    })
}

fn keywords() -> &'static HashSet<&'static str> {
    static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    WORDS.get_or_init(|| KEYWORDS.split(' ').collect())
}

fn kind(term: &str) -> K {
    if term.to_lowercase() == "none" {
        return K::None;
    }
    match term_pattern().captures(term) {
        None => K::Bad,
        Some(m) => match m.get(1) {
            None => K::Number,
            Some(unit) if unit.as_str() == "%" => K::Percent,
            Some(_) => K::Angle,
        },
    }
}

fn is_alpha(term: &str, modern: bool) -> bool {
    let k = kind(term);
    k == K::Number || k == K::Percent || (modern && k == K::None)
}

fn split_space(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return vec![""];
    }
    text.split(is_js_space)
        .filter(|part| !part.is_empty())
        .collect()
}

fn is_function(color: &str) -> bool {
    let hsl = color.starts_with("hsl");
    let Some(open) = color.find('(') else {
        return false;
    };
    let name = &color[..open];
    if name != "rgb" && name != "rgba" && name != "hsl" && name != "hsla" {
        return false;
    }
    if color.encode_utf16().count() > 120 || color.find(')') != Some(color.len() - 1) {
        return false;
    }
    let inner = color[open + 1..color.len() - 1].trim_matches(is_js_space);
    let mut parts: Vec<&str>;
    let mut alpha: Option<&str> = None;
    let legacy = inner.contains(',');
    if legacy {
        parts = inner
            .split(',')
            .map(|s| s.trim_matches(is_js_space))
            .collect();
        if parts.len() == 4 {
            alpha = parts.pop();
        }
    } else {
        let slash: Vec<&str> = inner.split('/').collect();
        if slash.len() > 2 {
            return false;
        }
        parts = split_space(slash[0].trim_matches(is_js_space));
        if slash.len() == 2 {
            alpha = Some(slash[1].trim_matches(is_js_space));
        }
    }
    if parts.len() != 3 || alpha.is_some_and(|a| !is_alpha(a, !legacy)) {
        return false;
    }
    let kinds: Vec<K> = parts.iter().map(|term| kind(term)).collect();
    if legacy {
        if hsl {
            return (kinds[0] == K::Number || kinds[0] == K::Angle)
                && kinds[1] == K::Percent
                && kinds[2] == K::Percent;
        }
        return (kinds[0] == K::Number || kinds[0] == K::Percent)
            && kinds[1] == kinds[0]
            && kinds[2] == kinds[0];
    }
    for (i, &k) in kinds.iter().enumerate() {
        let ok = k == K::Number
            || k == K::None
            || if hsl && i == 0 {
                k == K::Angle
            } else {
                k == K::Percent
            };
        if !ok {
            return false;
        }
    }
    true
}

pub fn is_css_color(color: &str) -> bool {
    if color.contains('(') {
        return is_function(color);
    }
    keywords().contains(color.to_lowercase().as_str())
}
