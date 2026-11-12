use crate::util::chars::is_js_space;

#[derive(Debug, Default)]
pub struct Tokens {
    pub types: Vec<i32>,
    pub texts: Vec<String>,
    pub starts: Vec<usize>,
}

pub fn space(unit: u16) -> bool {
    char::from_u32(u32::from(unit)).is_some_and(is_js_space)
}

pub fn word_at(src: &[u16], at: usize, word: &str) -> bool {
    word.bytes().enumerate().all(|(k, w)| {
        src.get(at + k).is_some_and(|&c| {
            let w = u16::from(w);
            c == w || ((97..=122).contains(&w) && c == w - 32)
        })
    })
}

pub fn slice(src: &[u16], from: usize, to: usize) -> String {
    String::from_utf16_lossy(&src[from..to])
}

pub fn line_end(src: &[u16], from: usize) -> usize {
    src.get(from..)
        .and_then(|rest| rest.iter().position(|&u| u == 10))
        .map_or(src.len(), |at| from + at)
}

pub fn keyword_at(src: &[u16], p: usize, word: &str) -> bool {
    word.bytes()
        .enumerate()
        .all(|(i, c)| src.get(p + i).is_some_and(|&u| (u | 32) == u16::from(c)))
}

fn skip_space(src: &[u16], from: usize) -> usize {
    let mut p = from;
    while src.get(p).is_some_and(|&u| space(u)) {
        p += 1;
    }
    p
}

pub fn skip_trivia(src: &[u16], from: usize) -> usize {
    let n = src.len();
    let mut p = from;
    while p < n {
        let c = src[p];
        if c == 37 && src.get(p + 1) != Some(&123) {
            p = line_end(src, p);
        } else if c == 10 {
            return p;
        } else if space(c) {
            p = skip_space(src, p);
        } else if c == 35 {
            p = line_end(src, p);
        } else {
            return p;
        }
    }
    p
}

fn push(out: &mut Tokens, src: &[u16], kind: i32, start: usize, end: usize) {
    out.types.push(kind);
    out.texts.push(slice(src, start, end));
    out.starts.push(start);
}

pub fn accessibility(src: &[u16], p: usize, out: &mut Tokens, first: i32) -> usize {
    let descr = keyword_at(src, p, "accdescr");
    if !descr && !keyword_at(src, p, "acctitle") {
        return p;
    }
    let n = src.len();
    let mut q = skip_space(src, p + 8);
    let c = src.get(q).copied();
    if c == Some(58) {
        q = skip_space(src, q + 1);
        let end = line_end(src, q);
        let (head, value) = if descr {
            (first + 2, first + 3)
        } else {
            (first, first + 1)
        };
        push(out, src, head, p, q);
        push(out, src, value, q, end);
        return end;
    }
    if !descr || c != Some(123) {
        return p;
    }
    q = skip_space(src, q + 1);
    while src.get(q) != Some(&125) {
        let end = src[q.min(n)..]
            .iter()
            .position(|&u| u == 125)
            .map_or(n, |at| q + at);
        push(out, src, first + 4, q, end);
        if q == n {
            return n;
        }
        q = end;
    }
    q + 1
}

pub fn word(unit: Option<&u16>) -> bool {
    unit.is_some_and(|&u| matches!(u, 48..=57 | 65..=90 | 97..=122 | 95))
}
