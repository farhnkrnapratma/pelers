use super::tokens::{T, keyword_kind};
use crate::diagrams::common::spans::{Span, Spans, empty};
use crate::util::chars::is_js_space;

pub struct Tokens {
    pub types: Vec<i32>,
    pub spans: Vec<Span>,
    pub starts: Vec<usize>,
    pub spans_of: Spans,
}

#[derive(Clone, Copy, PartialEq)]
enum S {
    Initial,
    String,
    Style,
    AccTitle,
    AccDescr,
    AccDescrMultiline,
}

fn is_space(c: u16) -> bool {
    char::from_u32(u32::from(c)).is_some_and(is_js_space)
}

fn is_word(c: u16) -> bool {
    matches!(c, 48..=57 | 65..=90 | 97..=122 | 95)
}

fn is_line_end(c: u16) -> bool {
    matches!(c, 10 | 13 | 0x2028 | 0x2029)
}

fn lower(c: u16) -> u16 {
    if (65..=90).contains(&c) { c + 32 } else { c }
}

fn ascii_ci(units: &[u16], at: usize, word: &str) -> Option<usize> {
    let mut i = at;
    for b in word.bytes() {
        if i >= units.len() || lower(units[i]) != u16::from(b) {
            return None;
        }
        i += 1;
    }
    Some(i)
}

fn skip_spaces(units: &[u16], mut i: usize) -> usize {
    while i < units.len() && is_space(units[i]) {
        i += 1;
    }
    i
}

fn accessor(units: &[u16], at: usize, word: &str, sep: u16) -> Option<usize> {
    let i = ascii_ci(units, at, word)?;
    let i = skip_spaces(units, i);
    if units.get(i) != Some(&sep) {
        return None;
    }
    Some(skip_spaces(units, i + 1))
}

fn acc_title(units: &[u16], at: usize) -> Option<usize> {
    accessor(units, at, "acctitle", u16::from(b':'))
}

fn acc_descr(units: &[u16], at: usize) -> Option<usize> {
    accessor(units, at, "accdescr", u16::from(b':'))
}

fn acc_descr_multiline(units: &[u16], at: usize) -> Option<usize> {
    let i = ascii_ci(units, at, "accdescr")?;
    let i = skip_spaces(units, i);
    if units.get(i) != Some(&u16::from(b'{')) {
        return None;
    }
    Some(skip_spaces(units, i + 1))
}

fn direction_kind(units: &[u16], at: usize) -> Option<(usize, usize)> {
    let i = ascii_ci(units, at, "direction")?;
    let j = skip_spaces(units, i);
    if j == i {
        return None;
    }
    for (k, word) in ["tb", "bt", "rl", "lr"].iter().enumerate() {
        if let Some(end) = ascii_ci(units, j, word) {
            return Some((k, end));
        }
    }
    None
}

fn find_directions(units: &[u16]) -> [(Vec<usize>, Vec<usize>); 4] {
    let mut out: [(Vec<usize>, Vec<usize>); 4] = Default::default();
    let mut i = 0;
    while i < units.len() {
        match direction_kind(units, i) {
            Some((k, end)) => {
                out[k].0.push(i);
                out[k].1.push(end);
                i = end;
            }
            None => i += 1,
        }
    }
    out
}

fn text_of(units: &[u16], s: usize, e: usize) -> String {
    String::from_utf16_lossy(&units[s.min(units.len())..e.min(units.len())])
}

fn emit(out: &mut Tokens, units: &[u16], kind: i32, s: usize, e: usize) -> usize {
    out.types.push(kind);
    let span = span_of(&out.spans_of, units, s, e);
    out.spans.push(span);
    out.starts.push(s);
    e
}

fn span_of(spans_of: &Spans, units: &[u16], s: usize, e: usize) -> Span {
    let n = units.len();
    let (a, b) = (s.min(n), e.min(n));
    if a >= b {
        empty()
    } else {
        spans_of.span(units, a, b)
    }
}

fn line_break(units: &[u16], q: usize) -> usize {
    units[q.min(units.len())..]
        .iter()
        .position(|&c| c == 10)
        .map_or(units.len(), |i| q + i)
}

fn first_line_end(units: &[u16], from: usize) -> Option<usize> {
    units[from.min(units.len())..]
        .iter()
        .position(|&c| matches!(c, 10 | 13 | 0x2028 | 0x2029))
        .map(|i| from + i)
}

fn is_stop(c: u16) -> bool {
    c < 128
        && matches!(
            c as u8,
            b':' | b',' | b'\r' | b'\n' | b'{' | b'<' | b'>' | b'-' | b'='
        )
}

pub fn tokenize(units: &[u16], spans_of: Spans) -> Tokens {
    let n = units.len();
    let at_u = |q: usize| -> u16 { units.get(q).copied().unwrap_or(0) };
    let mut out = Tokens {
        types: Vec::new(),
        spans: Vec::new(),
        starts: Vec::new(),
        spans_of,
    };
    let mut stack: Vec<S> = Vec::new();
    let mut state = S::Initial;
    let mut p = 0usize;
    let directions = find_directions(units);
    let any_dir = directions.iter().any(|(s, _)| !s.is_empty());
    let mut dir_seen = [0usize; 4];
    let mut line_end: isize = -1;

    let direction = |out: &mut Tokens,
                     p: &mut usize,
                     line_end: &mut isize,
                     dir_seen: &mut [usize; 4]|
     -> bool {
        if *line_end < *p as isize {
            *line_end = match first_line_end(units, *p) {
                Some(e) => e as isize,
                None => n as isize,
            };
            for k in 0..4 {
                let list = &directions[k].0;
                let mut j = dir_seen[k];
                while j < list.len() && list[j] as isize <= *line_end {
                    j += 1;
                }
                dir_seen[k] = j;
            }
        }
        for k in 0..4 {
            let j = dir_seen[k] as isize - 1;
            if j >= 0 && directions[k].0[j as usize] >= *p {
                let end = directions[k].1[j as usize];
                let e = line_break(units, end);
                let kind = T::DirectionTb as i32 + k as i32;
                *p = emit(out, units, kind, *p, e);
                return true;
            }
        }
        false
    };

    loop {
        if p >= n {
            break;
        }
        let c = units[p];
        let mut ok = true;
        match state {
            S::Initial => {
                let lowered = lower(c);
                let mut handled = false;
                if lowered == u16::from(b't')
                    && units.len() >= p + 6
                    && ascii_ci(units, p + 1, "itle").is_some()
                    && is_space(at_u(p + 5))
                {
                    let mut q = p + 6;
                    while q < n {
                        let d = units[q];
                        if d == 35 || d == 10 || d == 59 {
                            break;
                        }
                        q += 1;
                    }
                    if q > p + 6 {
                        p = emit(&mut out, units, T::Title as i32, p, q);
                        handled = true;
                    }
                }
                if !handled
                    && lowered == u16::from(b'a')
                    && lower(at_u(p + 1)) == u16::from(b'c')
                    && lower(at_u(p + 2)) == u16::from(b'c')
                {
                    if let Some(e) = acc_title(units, p) {
                        stack.push(state);
                        state = S::AccTitle;
                        p = emit(&mut out, units, T::AccTitle as i32, p, e);
                        handled = true;
                    } else if let Some(e) = acc_descr(units, p) {
                        stack.push(state);
                        state = S::AccDescr;
                        p = emit(&mut out, units, T::AccDescr as i32, p, e);
                        handled = true;
                    } else if let Some(e) = acc_descr_multiline(units, p) {
                        stack.push(state);
                        state = S::AccDescrMultiline;
                        p = e;
                        handled = true;
                    }
                }
                if !handled && any_dir && !is_line_end(c) {
                    handled = direction(&mut out, &mut p, &mut line_end, &mut dir_seen);
                }
                if !handled {
                    if c == 10 || (c == 13 && at_u(p + 1) == 10) {
                        let mut q = p;
                        while q < n {
                            let d = units[q];
                            if d == 10 {
                                q += 1;
                            } else if d == 13 && at_u(q + 1) == 10 {
                                q += 2;
                            } else {
                                break;
                            }
                        }
                        p = emit(&mut out, units, T::Newline as i32, p, q);
                        handled = true;
                    } else if is_space(c) {
                        p += 1;
                        while p < n && is_space(units[p]) {
                            p += 1;
                        }
                        handled = true;
                    } else if c == 35 || c == 37 {
                        p = line_break(units, p);
                        handled = true;
                    }
                }
                if !handled && is_word(c) {
                    let mut q = p + 1;
                    while q < n && is_word(units[q]) {
                        q += 1;
                    }
                    let mut keyword_found = None;
                    if q - p <= 22 {
                        let word = text_of(units, p, q).to_lowercase();
                        keyword_found = keyword_kind(&word);
                    }
                    if let Some(kind) = keyword_found {
                        if kind == T::Style as i32
                            || kind == T::Classdef as i32
                            || kind == T::Class as i32
                        {
                            stack.push(state);
                            state = S::Style;
                        }
                        p = emit(&mut out, units, kind, p, q);
                    } else {
                        let mut q2 = q;
                        while q2 < n && !is_stop(units[q2]) {
                            q2 += 1;
                        }
                        out.types.push(T::UnqString as i32);
                        out.spans.push(Span::Lossy(
                            crate::util::chars::trim(&text_of(units, p, q2)).to_owned(),
                        ));
                        out.starts.push(p);
                        p = q2;
                    }
                    handled = true;
                }
                if !handled {
                    match c {
                        123 => p = emit(&mut out, units, T::StructStart as i32, p, p + 1),
                        125 => p = emit(&mut out, units, T::StructStop as i32, p, p + 1),
                        58 => {
                            if at_u(p + 1) == 58 && at_u(p + 2) == 58 {
                                p = emit(&mut out, units, T::StyleSeparator as i32, p, p + 3);
                            } else {
                                p = emit(&mut out, units, T::Colonsep as i32, p, p + 1);
                            }
                        }
                        60 => {
                            if at_u(p + 1) != 45 {
                                ok = false;
                            } else {
                                p = emit(&mut out, units, T::EndArrowL as i32, p, p + 2);
                            }
                        }
                        45 => {
                            if at_u(p + 1) == 62 {
                                p = emit(&mut out, units, T::EndArrowR as i32, p, p + 2);
                            } else {
                                p = emit(&mut out, units, T::Line as i32, p, p + 1);
                            }
                        }
                        34 => {
                            stack.push(state);
                            state = S::String;
                            p += 1;
                        }
                        44 => p = emit(&mut out, units, T::Comma as i32, p, p + 1),
                        _ => ok = false,
                    }
                }
            }
            S::Style => {
                if is_word(c) {
                    let mut q = p + 1;
                    while q < n && is_word(units[q]) {
                        q += 1;
                    }
                    p = emit(&mut out, units, T::Alpha as i32, p, q);
                } else {
                    match c {
                        58 => p = emit(&mut out, units, T::Colon as i32, p, p + 1),
                        59 => p = emit(&mut out, units, T::Semicolon as i32, p, p + 1),
                        37 => p = emit(&mut out, units, T::Percent as i32, p, p + 1),
                        45 => p = emit(&mut out, units, T::Minus as i32, p, p + 1),
                        35 => p = emit(&mut out, units, T::Brkt as i32, p, p + 1),
                        32 => p += 1,
                        34 => {
                            stack.push(state);
                            state = S::String;
                            p += 1;
                        }
                        10 => {
                            if let Some(s) = stack.pop() {
                                state = s;
                            }
                            p += 1;
                        }
                        44 => p = emit(&mut out, units, T::Comma as i32, p, p + 1),
                        _ => ok = false,
                    }
                }
            }
            S::String => {
                if c == 34 {
                    if let Some(s) = stack.pop() {
                        state = s;
                    }
                    p += 1;
                } else {
                    let e = units[p..]
                        .iter()
                        .position(|&u| u == 34)
                        .map_or(n, |i| p + i);
                    p = emit(&mut out, units, T::QString as i32, p, e);
                }
            }
            S::AccTitle => {
                if let Some(s) = stack.pop() {
                    state = s;
                }
                let e = line_break(units, p);
                p = emit(&mut out, units, T::AccTitleValue as i32, p, e);
            }
            S::AccDescr => {
                if let Some(s) = stack.pop() {
                    state = s;
                }
                let e = line_break(units, p);
                p = emit(&mut out, units, T::AccDescrValue as i32, p, e);
            }
            S::AccDescrMultiline => {
                if c == 125 {
                    if let Some(s) = stack.pop() {
                        state = s;
                    }
                    p += 1;
                } else {
                    let e = units[p..]
                        .iter()
                        .position(|&u| u == 125)
                        .map_or(n, |i| p + i);
                    p = emit(&mut out, units, T::AccDescrMultilineValue as i32, p, e);
                }
            }
        }
        if !ok {
            out.types.push(T::Error as i32);
            out.spans.push(empty());
            out.starts.push(p);
            return out;
        }
    }

    let last = match state {
        S::Initial => T::Eof as i32,
        S::String => T::QString as i32,
        S::AccTitle => T::AccTitleValue as i32,
        S::AccDescr => T::AccDescrValue as i32,
        S::AccDescrMultiline => T::AccDescrMultilineValue as i32,
        S::Style => -1,
    };
    if last != -1 {
        emit(&mut out, units, last, n, n);
    }
    emit(&mut out, units, T::End as i32, n, n);
    out
}
