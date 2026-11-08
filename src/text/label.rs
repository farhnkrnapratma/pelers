use super::entities::decode_entities;
use super::measurer::{BOLD_STYLE, ITALIC, MONO, SUB, SUP, UNDERLINE, width};
use crate::util::chars::{is_js_space, is_word, trim, trim_end, trim_start};
use crate::util::jsnum::js_round;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub style: u32,
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub lines: Vec<Vec<Span>>,
    pub widths: Vec<f64>,
    pub width: f64,
    pub height: f64,
    pub size: f64,
    pub line_height: f64,
}

pub const LINE_HEIGHT: f64 = 1.5;
const WRAPPED_LINE_HEIGHT: f64 = 1.3;

const SHIFT: u32 = SUB | SUP;
const SHY: char = '\u{ad}';

const HTML_TAGS: &str = "a abbr address article aside audio b bdi bdo big blockquote body br button caption center cite code col \
colgroup data dd del details dfn dialog div dl dt em embed fieldset figcaption figure font footer form h1 h2 \
h3 h4 h5 h6 head header hr html i iframe img input ins kbd label legend li link main map mark meta meter nav \
noscript object ol optgroup option output p picture pre progress q rp rt ruby s samp script section select \
small source span strike strong style sub summary sup table tbody td template textarea tfoot th thead time \
title tr track tt u ul var video wbr";

fn tag_style(tag: &str) -> u32 {
    match tag {
        "b" | "strong" => BOLD_STYLE,
        "i" | "em" | "cite" | "dfn" | "var" => ITALIC,
        "code" | "kbd" | "samp" | "tt" => MONO,
        "sub" => SUB,
        "sup" => SUP,
        "u" | "ins" => UNDERLINE,
        _ => 0,
    }
}

fn is_cjk(c: char) -> bool {
    matches!(c, '\u{2e80}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{ff00}'..='\u{ff60}')
}

const CLOSE: &str = "、。，．：；！？）］｝」』】〕〉》〙〗ー…‥・ゝゞぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮ,.:;!?)]}";
const OPEN: &str = "（［｛「『【〔〈《〘〖([{";

fn is_close(c: char) -> bool {
    CLOSE.contains(c)
}

fn is_open(c: char) -> bool {
    OPEN.contains(c)
}

fn is_hint(c: char) -> bool {
    c == '\u{200b}' || c == SHY
}

fn pieces(text: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let at = |i: usize| chars.get(i).map(|&(_, c)| c);
    let byte = |i: usize| chars.get(i).map_or(text.len(), |&(b, _)| b);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let mut p = i;
        while at(p).is_some_and(is_js_space) {
            p += 1;
        }
        let mut end = None;
        let mut run = p;
        while at(run).is_some_and(is_open) {
            run += 1;
        }
        if let Some(k) = (p..=run).rev().find(|&k| at(k).is_some_and(is_cjk)) {
            let mut q = k + 1;
            while at(q).is_some_and(is_close) {
                q += 1;
            }
            end = Some(q);
        } else if at(p).is_some_and(|c| !is_js_space(c) && !is_cjk(c) && !is_hint(c)) {
            let mut q = p;
            while at(q).is_some_and(|c| !is_js_space(c) && !is_cjk(c) && !is_hint(c)) {
                q += 1;
            }
            while at(q).is_some_and(is_hint) {
                q += 1;
            }
            while at(q).is_some_and(is_close) {
                q += 1;
            }
            end = Some(q);
        } else if at(p).is_some_and(is_hint) {
            let mut q = p;
            while at(q).is_some_and(is_hint) {
                q += 1;
            }
            end = Some(q);
        }
        let stop = match end {
            Some(mut q) => {
                while at(q).is_some_and(is_js_space) {
                    q += 1;
                }
                q
            }
            None => p,
        };
        out.push(&text[byte(start)..byte(stop)]);
        i = stop;
    }
    out
}

fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut min = usize::MAX;
    for line in &lines {
        if trim(line).is_empty() {
            continue;
        }
        let indent = line.chars().count() - trim_start(line).chars().count();
        min = min.min(indent);
    }
    if min == usize::MAX || min == 0 {
        return text.to_owned();
    }
    lines
        .iter()
        .map(|line| {
            let own = line.chars().count() - trim_start(line).chars().count();
            let skip = min.min(own);
            let at = line.char_indices().nth(skip).map_or(line.len(), |(b, _)| b);
            &line[at..]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn split_breaks(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut last = 0;
    let mut i = 0;
    while i < bytes.len() {
        let len = match bytes[i] {
            b'\n' => 1,
            b'\\' if matches!(bytes.get(i + 1), Some(b'n' | b'N')) => 2,
            b'<' if matches!(bytes.get(i + 1), Some(b'b' | b'B'))
                && matches!(bytes.get(i + 2), Some(b'r' | b'R')) =>
            {
                let mut j = i + 3;
                let rest = &text[j..];
                j += rest.len() - trim_start(rest).len();
                if bytes.get(j) == Some(&b'/') {
                    j += 1;
                }
                if bytes.get(j) == Some(&b'>') {
                    j + 1 - i
                } else {
                    0
                }
            }
            _ => 0,
        };
        if len == 0 {
            i += 1;
        } else {
            out.push(&text[last..i]);
            i += len;
            last = i;
        }
    }
    out.push(&text[last..]);
    out
}

fn has_markup(raw: &str) -> bool {
    raw.contains(['<', '\\', '\n', '*', '_']) || find_icon(raw, 0).is_some()
}

fn find_icon(text: &str, from: usize) -> Option<(usize, usize)> {
    let mut at = from;
    while let Some(found) = text[at..].find("fa") {
        let start = at + found;
        let mut p = start + 2;
        if text[p..].starts_with(['b', 'k', 'l', 'r', 's']) {
            p += 1;
        }
        if !text[p..].starts_with(":fa-") {
            at = start + 1;
            continue;
        }
        p += 4;
        let n = text[p..]
            .find(|c: char| !(is_word(c) || c == '-'))
            .unwrap_or(text.len() - p);
        if n > 0 {
            return Some((start, p + n));
        }
        at = start + 1;
    }
    None
}

struct Open {
    style: u32,
    shifts: Vec<u32>,
}

fn push_plain(line: &mut Vec<Span>, text: &str, style: u32) {
    let decoded = decode_entities(text);
    match line.last_mut() {
        Some(last) if last.style == style => last.text.push_str(&decoded),
        _ => line.push(Span {
            text: decoded,
            style,
            width: 0.0,
        }),
    }
}

fn push_text(line: &mut Vec<Span>, text: &str, style: u32) {
    if !text.is_empty() {
        push_plain(line, text, style);
    }
}

fn push_icons(line: &mut Vec<Span>, text: &str, style: u32) {
    if !text.contains(":fa-") {
        push_text(line, text, style);
        return;
    }
    let mut last = 0;
    while let Some((start, end)) = find_icon(text, last) {
        push_text(line, &text[last..start], style);
        last = end;
        if text[last..].starts_with(' ') {
            last += 1;
        } else if last == text.len()
            && let Some(before) = line.last_mut()
        {
            before.text = trim_end(&before.text).to_owned();
        }
    }
    push_text(line, &text[last..], style);
}

fn tag_at(source: &[char], at: usize) -> Option<(String, usize)> {
    let mut p = at + 1;
    if source.get(p) == Some(&'/') {
        p += 1;
    }
    let name = p;
    if !source.get(p).is_some_and(char::is_ascii_alphabetic) {
        return None;
    }
    while source.get(p).is_some_and(|&c| is_word(c) || c == '-') {
        p += 1;
    }
    let tag: String = source[name..p]
        .iter()
        .collect::<String>()
        .to_ascii_lowercase();
    if source.get(p).is_some_and(|&c| is_js_space(c)) {
        p += 1;
        while source.get(p).is_some_and(|&c| c != '<' && c != '>') {
            p += 1;
        }
    } else if source.get(p) == Some(&'/') {
        p += 1;
    }
    (source.get(p) == Some(&'>')).then(|| (tag, p + 1 - at))
}

fn index_of(source: &[char], needle: &[char], from: usize) -> bool {
    (from..source.len().saturating_sub(needle.len() - 1)).any(|i| source[i..].starts_with(needle))
}

fn inline(line: &mut Vec<Span>, source: &str, markdown: bool, base: u32, open: &mut Open) {
    let source: Vec<char> = source.chars().collect();
    let n = source.len();
    let mut style = open.style;
    let mut marked = 0;
    let mut text = String::new();
    let mut i = 0;
    let flush = |line: &mut Vec<Span>, text: &mut String, style: u32, open: &Open| {
        let shift = open.shifts.last().copied().unwrap_or(0);
        push_icons(line, text, style | shift);
        text.clear();
    };
    while i < n {
        let ch = source[i];
        if ch == '<' {
            if let Some((tag, len)) = tag_at(&source, i) {
                let closing = source[i + 1] == '/';
                let bit = tag_style(&tag);
                if bit & SHIFT != 0 {
                    flush(line, &mut text, style, open);
                    let at = open.shifts.iter().rposition(|&s| s == bit);
                    if !closing {
                        open.shifts.push(bit);
                    } else if let Some(at) = at {
                        open.shifts.remove(at);
                    }
                } else if bit != 0 {
                    flush(line, &mut text, style, open);
                    open.style = if closing {
                        (open.style & !bit) | (base & bit)
                    } else {
                        open.style | bit
                    };
                    style = if closing {
                        (style & !bit) | (base & bit)
                    } else {
                        style | bit
                    };
                }
                if bit != 0 || HTML_TAGS.split(' ').any(|t| t == tag) {
                    i += len;
                    continue;
                }
            }
        } else if markdown && (ch == '*' || ch == '_') {
            let double = source.get(i + 1) == Some(&ch);
            let bit = if double { BOLD_STYLE } else { ITALIC };
            let len = if double { 2 } else { 1 };
            let opening = marked & bit == 0;
            let prev = if i > 0 { source[i - 1] } else { ' ' };
            let next = if i + len < n { source[i + len] } else { ' ' };
            let needle: &[char] = if double { &[ch, ch] } else { &[ch] };
            let can_open = opening && !is_js_space(next) && index_of(&source, needle, i + len);
            let can_close = !opening && !is_js_space(prev);
            let wordy = ch == '_' && is_word(prev) && is_word(next);
            if (can_open || can_close) && !wordy {
                flush(line, &mut text, style, open);
                marked = if can_open {
                    marked | bit
                } else {
                    marked & !bit
                };
                style = if can_open {
                    style | bit
                } else {
                    (style & !bit) | (base & bit)
                };
                i += len;
                continue;
            }
        }
        text.push(ch);
        i += 1;
    }
    flush(line, &mut text, style, open);
}

fn measure_spans(line: &mut [Span], size: f64) -> f64 {
    let mut total = 0.0;
    for span in line {
        span.width = width(&span.text, size, span.style);
        total += span.width;
    }
    total
}

struct Wrapped<'a> {
    out: &'a mut Vec<Vec<Span>>,
    widths: &'a mut Vec<f64>,
    current: Vec<Span>,
    width: f64,
    size: f64,
}

impl Wrapped<'_> {
    fn commit(&mut self, hyphen: bool) {
        if let Some(last) = self.current.last_mut() {
            let mut trimmed = trim_end(&last.text).to_owned();
            if hyphen && last.text.ends_with(SHY) {
                trimmed.pop();
                trimmed.push('-');
            }
            if trimmed != last.text {
                let w = width(&trimmed, self.size, last.style);
                self.width += w - last.width;
                last.width = w;
                last.text = trimmed;
            }
        }
        self.out.push(std::mem::take(&mut self.current));
        self.widths.push(self.width);
        self.width = 0.0;
    }
}

fn wrap(line: &[Span], size: f64, max: f64, out: &mut Vec<Vec<Span>>, widths: &mut Vec<f64>) {
    let mut w = Wrapped {
        out,
        widths,
        current: Vec::new(),
        width: 0.0,
        size,
    };
    for (index, span) in line.iter().enumerate() {
        let parts = pieces(&span.text);
        let soft = span.text.contains(SHY);
        let n = if soft { parts.len() } else { 0 };
        let mut own = vec![0.0_f64; n];
        let mut rest = vec![0.0_f64; n + 1];
        for q in (0..n).rev() {
            let shy = parts[q].ends_with(SHY);
            let end = trim_end(parts[q]);
            own[q] = width(
                if shy {
                    &end[..end.len() - SHY.len_utf8()]
                } else {
                    end
                },
                size,
                span.style,
            );
            rest[q] = if !shy {
                own[q]
            } else if q == n - 1 {
                if index == line.len() - 1 {
                    own[q]
                } else {
                    f64::INFINITY
                }
            } else {
                own[q] + rest[q + 1]
            };
        }
        for (p, piece) in parts.iter().enumerate() {
            let mut part = *piece;
            let part_width = if soft {
                own[p]
            } else {
                width(trim_end(part), size, span.style)
            };
            let fits = if !soft || !part.ends_with(SHY) {
                w.width + part_width <= max
            } else {
                let hyphenated = format!("{}-", &part[..part.len() - SHY.len_utf8()]);
                w.width + width(&hyphenated, size, span.style) <= max || w.width + rest[p] <= max
            };
            if w.width > 0.0 && !fits {
                w.commit(trim_start(part) == part);
                part = trim_start(part);
            }
            match w.current.last_mut() {
                Some(last) if last.style == span.style => {
                    last.text.push_str(part);
                    if part_width > 0.0 || trim_end(part) != part {
                        let before = last.width;
                        last.width = width(&last.text, size, last.style);
                        w.width += last.width - before;
                    }
                }
                _ => {
                    let measured = width(part, size, span.style);
                    w.current.push(Span {
                        text: part.to_owned(),
                        style: span.style,
                        width: measured,
                    });
                    w.width += measured;
                }
            }
        }
    }
    if !w.current.is_empty() || w.out.is_empty() {
        w.commit(false);
    }
}

fn empty() -> Label {
    Label {
        lines: Vec::new(),
        widths: Vec::new(),
        width: 0.0,
        height: 0.0,
        size: 0.0,
        line_height: 0.0,
    }
}

fn widest(widths: &[f64]) -> f64 {
    widths.iter().fold(0.0, |m, &w| if w > m { w } else { m })
}

pub fn layout_label(
    raw: Option<&str>,
    markdown: bool,
    size: f64,
    max_width: f64,
    base: u32,
) -> Label {
    let raw = match raw {
        None | Some("") => return empty(),
        Some(raw) => raw,
    };
    let line_height = js_round(size * LINE_HEIGHT);

    if !has_markup(raw) && !raw.contains('&') && !raw.contains('ﬂ') {
        let text = trim(raw);
        let w = width(text, size, base);
        let breakable = text.contains(' ') || text.chars().any(is_cjk) || text.chars().any(is_hint);
        if w <= max_width || !breakable {
            return Label {
                lines: vec![vec![Span {
                    text: text.to_owned(),
                    style: base,
                    width: w,
                }]],
                widths: vec![w],
                width: w,
                height: line_height,
                size,
                line_height,
            };
        }
        let mut lines = Vec::new();
        let mut widths = Vec::new();
        wrap(
            &[Span {
                text: text.to_owned(),
                style: base,
                width: w,
            }],
            size,
            max_width,
            &mut lines,
            &mut widths,
        );
        let snug = if lines.len() > 1 {
            js_round(size * WRAPPED_LINE_HEIGHT)
        } else {
            line_height
        };
        return Label {
            width: widest(&widths),
            height: lines.len() as f64 * snug,
            lines,
            widths,
            size,
            line_height: snug,
        };
    }
    layout_lines(raw, markdown, size, max_width, base, line_height)
}

fn layout_lines(
    raw: &str,
    markdown: bool,
    size: f64,
    max_width: f64,
    base: u32,
    line_height: f64,
) -> Label {
    let source = if markdown {
        let unified = raw.replace("\r\n", "\n").replace('\r', "\n");
        trim(&dedent(&unified)).to_owned()
    } else {
        trim(raw).to_owned()
    };
    let mut lines = Vec::new();
    let mut widths = Vec::new();
    let mut open = Open {
        style: base,
        shifts: Vec::new(),
    };
    for part in split_breaks(&source) {
        let mut line = Vec::new();
        inline(&mut line, trim(part), markdown, base, &mut open);
        let w = measure_spans(&mut line, size);
        if w > max_width {
            wrap(&line, size, max_width, &mut lines, &mut widths);
        } else {
            lines.push(line);
            widths.push(w);
        }
    }
    let snug = if lines.len() > 1 {
        js_round(size * WRAPPED_LINE_HEIGHT)
    } else {
        line_height
    };
    Label {
        width: widest(&widths),
        height: lines.len() as f64 * snug,
        lines,
        widths,
        size,
        line_height: snug,
    }
}
