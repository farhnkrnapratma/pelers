use crate::text::label::{LINE_HEIGHT, Label, Span};
use crate::text::measurer::width;
use crate::util::chars::{is_js_space, trim_end};
use crate::util::jsnum::{js_max, js_round};

pub fn literal_label(text: &str, size: f64, room: f64, style: u32) -> Label {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let chars: Vec<char> = raw.chars().collect();
        if chars.is_empty() {
            lines.push(String::new());
        }
        let mut from = 0;
        while from < chars.len() {
            let fits = |length: usize| -> bool {
                let piece: String = chars[from..from + length].iter().collect();
                width(&piece, size, style) <= room
            };
            let remaining = chars.len() - from;
            let mut hi = 1;
            while hi < remaining && fits(hi) {
                hi = remaining.min(hi * 2);
            }
            let mut lo = 1;
            while lo < hi {
                let mid = (lo + hi).div_ceil(2);
                if fits(mid) {
                    lo = mid;
                } else {
                    hi = mid - 1;
                }
            }
            let mut take = lo;
            let wraps = from + take < chars.len();
            if wraps
                && !is_js_space(chars[from + take])
                && let Some(i) = (1..take).rev().find(|&i| is_js_space(chars[from + i]))
            {
                take = i + 1;
            }
            let line: String = chars[from..from + take].iter().collect();
            if wraps {
                let trimmed = trim_end(&line);
                lines.push(if trimmed.is_empty() {
                    line.clone()
                } else {
                    trimmed.to_owned()
                });
            } else {
                lines.push(line);
            }
            from += take;
            while wraps && from < chars.len() && is_js_space(chars[from]) {
                from += 1;
            }
        }
    }
    if text.is_empty() {
        lines.clear();
    }
    let widths: Vec<f64> = lines.iter().map(|line| width(line, size, style)).collect();
    let mut widest = 0.0;
    for w in &widths {
        widest = js_max(widest, *w);
    }
    let line_height = js_round(size * LINE_HEIGHT);
    Label {
        lines: lines
            .into_iter()
            .zip(&widths)
            .map(|(line, w)| {
                vec![Span {
                    text: line,
                    style,
                    icon: None,
                    math: None,
                    width: *w,
                }]
            })
            .collect(),
        height: widths.len() as f64 * line_height,
        widths,
        width: widest,
        size,
        line_height,
        heights: None,
    }
}

pub fn break_label(label: &Label, room: f64) -> Label {
    if label.width <= room {
        return label.clone();
    }
    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut widths: Vec<f64> = Vec::new();
    for (i, spans) in label.lines.iter().enumerate() {
        if label.widths[i] <= room {
            lines.push(spans.clone());
            widths.push(label.widths[i]);
            continue;
        }
        let mut current: Vec<Span> = Vec::new();
        let mut at = 0.0;
        for span in spans {
            if span.icon.is_some() || span.math.is_some() {
                if !current.is_empty() && at + span.width > room {
                    lines.push(std::mem::take(&mut current));
                    widths.push(at);
                    at = 0.0;
                }
                current.push(span.clone());
                at += span.width;
                continue;
            }
            for ch in span.text.chars() {
                let plain =
                    |s: &Span| s.icon.is_none() && s.math.is_none() && s.style == span.style;
                let mut last = current.len().checked_sub(1).filter(|&k| plain(&current[k]));
                let joined = last.is_some();
                let mut piece = String::new();
                if let Some(k) = last {
                    piece.push_str(&current[k].text);
                }
                piece.push(ch);
                let next_width = width(&piece, label.size, span.style);
                let mut added = next_width
                    - if joined {
                        current[last.unwrap()].width
                    } else {
                        0.0
                    };
                if !current.is_empty() && at + added > room {
                    lines.push(std::mem::take(&mut current));
                    widths.push(at);
                    at = 0.0;
                    last = None;
                    added = width(&ch.to_string(), label.size, span.style);
                }
                match last {
                    Some(k) => {
                        current[k].text.push(ch);
                        current[k].width += added;
                    }
                    None => current.push(Span::text(ch.to_string(), span.style, added)),
                }
                at += added;
            }
        }
        if !current.is_empty() || label.lines[i].is_empty() {
            lines.push(current);
            widths.push(at);
        }
    }
    let mut out_width = 0.0;
    for &w in &widths {
        out_width = js_max(out_width, w);
    }
    let broken = Label {
        height: lines.len() as f64 * label.line_height,
        lines,
        widths,
        width: out_width,
        size: label.size,
        line_height: label.line_height,
        heights: label.heights.clone(),
    };
    if label.heights.is_some() {
        crate::text::label::with_heights(broken)
    } else {
        broken
    }
}

pub fn left_label_svg(label: &Label, x: f64, top: f64, attrs: &str) -> String {
    use crate::svg::builder::{esc, label_svg, num, shared_style, span_style, text_style};
    let shared = shared_style(label);
    let open = format!("<text{}", text_style(attrs, shared));
    let mut body = String::new();
    for (i, spans) in label.lines.iter().enumerate() {
        let cy = top + (i as f64 + 0.5) * label.line_height;
        if spans.iter().any(|span| span.icon.is_some()) {
            let width = label.widths[i];
            let single = Label {
                lines: vec![spans.clone()],
                widths: vec![width],
                width,
                height: label.line_height,
                ..label.clone()
            };
            body.push_str(&label_svg(&single, x + width / 2.0, cy, attrs));
            continue;
        }
        let mut text = String::new();
        for span in spans {
            let own = span_style(span.style & !shared);
            if own.is_empty() {
                text.push_str(&esc(&span.text));
            } else {
                text.push_str(&format!("<tspan{own}>{}</tspan>", esc(&span.text)));
            }
        }
        if !text.is_empty() {
            body.push_str(&format!(
                "{open} x=\"{}\" y=\"{}\" text-anchor=\"start\" xml:space=\"preserve\">{text}</text>",
                num(x),
                num(cy + label.size * 0.35)
            ));
        }
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(label: &Label) -> Vec<&str> {
        label.lines.iter().map(|l| l[0].text.as_str()).collect()
    }

    #[test]
    fn keeps_a_short_label_on_one_line() {
        let label = literal_label("Port", 14.0, 200.0, 0);
        assert_eq!(texts(&label), vec!["Port"]);
        assert_eq!(label.height, 21.0);
    }

    #[test]
    fn breaks_at_spaces_and_inside_long_words() {
        let label = literal_label("Source Port", 14.0, 50.0, 0);
        assert_eq!(texts(&label), vec!["Source", "Port"]);
        let long = literal_label("abcdefghijklmnop", 14.0, 30.0, 0);
        assert!(long.lines.len() > 1);
        assert_eq!(texts(&long).concat(), "abcdefghijklmnop");
    }

    #[test]
    fn keeps_blank_lines_and_drops_empty_text() {
        assert_eq!(
            texts(&literal_label("a\n\nb", 14.0, 100.0, 0)),
            vec!["a", "", "b"]
        );
        assert!(literal_label("", 14.0, 100.0, 0).lines.is_empty());
    }

    #[test]
    fn keeps_whitespace_only_lines_that_wrap() {
        let label = literal_label("          x", 14.0, 12.0, 0);
        assert!(texts(&label)[0].chars().all(|c| c == ' '));
        assert_eq!(texts(&label).last(), Some(&"x"));
    }
}
