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
                    width: *w,
                }]
            })
            .collect(),
        height: widths.len() as f64 * line_height,
        widths,
        width: widest,
        size,
        line_height,
    }
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
