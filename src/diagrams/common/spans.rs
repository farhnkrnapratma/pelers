#[derive(Debug)]
pub enum Span {
    Bytes(usize, usize),
    Lossy(String),
}

const NOT_A_BOUNDARY: usize = usize::MAX;

pub fn empty() -> Span {
    Span::Bytes(0, 0)
}

#[derive(Debug)]
pub struct Spans {
    byte_of: Option<Vec<usize>>,
}

impl Spans {
    pub fn new(src: &str) -> Self {
        if src.is_ascii() {
            return Self { byte_of: None };
        }
        let mut map = Vec::with_capacity(src.len());
        for (at, c) in src.char_indices() {
            map.push(at);
            map.extend(std::iter::repeat_n(NOT_A_BOUNDARY, c.len_utf16() - 1));
        }
        map.push(src.len());
        Self { byte_of: Some(map) }
    }

    fn byte(&self, at: usize) -> Option<usize> {
        match &self.byte_of {
            None => Some(at),
            Some(map) => map.get(at).copied().filter(|&b| b != NOT_A_BOUNDARY),
        }
    }

    pub fn span(&self, units: &[u16], from: usize, to: usize) -> Span {
        match (self.byte(from), self.byte(to)) {
            (Some(a), Some(b)) => Span::Bytes(a, b),
            _ => Span::Lossy(String::from_utf16_lossy(&units[from..to])),
        }
    }
}

pub fn text<'a>(src: &'a str, span: &'a Span) -> &'a str {
    match span {
        Span::Bytes(from, to) => &src[*from..*to],
        Span::Lossy(text) => text,
    }
}
