use crate::errors::{PeleError, lexical_error, syntax_error};
use crate::util::chars::{is_js_space, trim};

pub type Matcher = fn(src: &str, at: usize) -> Option<usize>;

#[derive(Clone, Copy)]
pub enum Pattern {
    Literal(&'static str),
    Keyword(&'static str),
    Match(Matcher),
}

#[derive(Clone, Copy)]
pub struct TokenType {
    pub name: &'static str,
    pub pattern: Pattern,
    pub hidden: bool,
    pub opener: Option<Matcher>,
    pub first: Option<&'static str>,
    pub longer: &'static [&'static str],
}

impl TokenType {
    pub const fn new(name: &'static str, pattern: Pattern, first: Option<&'static str>) -> Self {
        Self {
            name,
            pattern,
            hidden: false,
            opener: None,
            first,
            longer: &[],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tokens {
    pub kinds: Vec<i32>,
    pub starts: Vec<usize>,
    pub ends: Vec<usize>,
    pub src: String,
}

fn blanks(src: &str, at: usize) -> usize {
    at + src[at..]
        .bytes()
        .take_while(|b| matches!(b, b'\t' | b' '))
        .count()
}

fn line_end(src: &str, at: usize) -> usize {
    src[at..].find(['\n', '\r']).map_or(src.len(), |i| at + i)
}

fn until_comment(src: &str, at: usize) -> usize {
    let end = line_end(src, at);
    src[at..end].find("%%").map_or(end, |i| at + i)
}

fn newline(src: &str, at: usize) -> Option<usize> {
    let rest = &src[at..];
    if rest.starts_with("\r\n") {
        Some(at + 2)
    } else if rest.starts_with('\n') {
        Some(at + 1)
    } else {
        None
    }
}

fn acc_descr(src: &str, at: usize) -> Option<usize> {
    let p = blanks(src, at);
    let rest = src[p..].strip_prefix("accDescr")?;
    let p = src.len() - rest.len();
    let q = blanks(src, p);
    if src[q..].starts_with(':') {
        return Some(until_comment(src, q + 1));
    }
    let q = p + (rest.len() - rest.trim_start_matches(is_js_space).len());
    let body = src[q..].strip_prefix('{')?;
    body.find('}').map(|close| q + 1 + close + 1)
}

fn acc_title(src: &str, at: usize) -> Option<usize> {
    let p = blanks(src, at);
    let rest = src[p..].strip_prefix("accTitle")?;
    let q = blanks(src, src.len() - rest.len());
    src[q..].starts_with(':').then(|| until_comment(src, q + 1))
}

fn title(src: &str, at: usize) -> Option<usize> {
    let p = blanks(src, at);
    let rest = src[p..].strip_prefix("title")?;
    let r = src.len() - rest.len();
    if matches!(src.as_bytes().get(r), Some(b'\t' | b' ')) {
        Some(until_comment(src, r + 1))
    } else {
        Some(r)
    }
}

fn newline_token(src: &str, at: usize) -> Option<usize> {
    newline(src, at)
}

fn whitespace(src: &str, at: usize) -> Option<usize> {
    let end = blanks(src, at);
    (end > at).then_some(end)
}

fn fence_end(src: &str, at: usize) -> Option<usize> {
    let rest = src[at..].strip_prefix("---")?;
    let t = at + 3;
    if let Some(end) = newline(src, t) {
        return Some(end);
    }
    rest.chars().next().is_none_or(is_js_space).then_some(t)
}

fn yaml_opener(src: &str, at: usize) -> Option<usize> {
    if !src[at..].starts_with("---") {
        return None;
    }
    newline(src, blanks(src, at + 3))
}

fn yaml(src: &str, at: usize) -> Option<usize> {
    let body = yaml_opener(src, at)?;
    let mut from = body;
    while let Some(found) = src[from..].find('\n') {
        let line = from + found;
        if let Some(end) = fence_end(src, line + 1) {
            return Some(end);
        }
        from = line + 1;
    }
    fence_end(src, body)
}

fn directive_opener(src: &str, at: usize) -> Option<usize> {
    let p = blanks(src, at);
    src[p..].starts_with("%%{").then_some(p)
}

fn directive(src: &str, at: usize) -> Option<usize> {
    let p = directive_opener(src, at)? + 3;
    let mut from = p;
    while let Some(found) = src[from..].find("}%%") {
        let close = from + found + 3;
        if let Some(end) = newline(src, close) {
            return Some(end);
        }
        if src[close..].chars().next().is_none_or(is_js_space) {
            return Some(close);
        }
        from += found + 1;
    }
    None
}

fn single_line_comment(src: &str, at: usize) -> Option<usize> {
    let p = blanks(src, at);
    src[p..].starts_with("%%").then(|| line_end(src, p + 2))
}

fn string(src: &str, at: usize) -> Option<usize> {
    let quote = src[at..]
        .chars()
        .next()
        .filter(|c| matches!(c, '"' | '\''))?;
    let mut chars = src[at + 1..].char_indices();
    while let Some((i, c)) = chars.next() {
        if c == quote {
            return Some(at + 1 + i + 1);
        }
        if c == '\\' {
            let (_, next) = chars.next()?;
            if is_terminator(next) {
                return None;
            }
        }
    }
    None
}

pub const ACC_DESCR: TokenType =
    TokenType::new("ACC_DESCR", Pattern::Match(acc_descr), Some("\t a"));
pub const ACC_TITLE: TokenType =
    TokenType::new("ACC_TITLE", Pattern::Match(acc_title), Some("\t a"));
pub const TITLE: TokenType = TokenType::new("TITLE", Pattern::Match(title), Some("\t t"));
pub const STRING: TokenType = TokenType::new("STRING", Pattern::Match(string), Some("\"'"));
pub const NEWLINE: TokenType =
    TokenType::new("NEWLINE", Pattern::Match(newline_token), Some("\r\n"));
pub const WHITESPACE: TokenType = TokenType {
    hidden: true,
    ..TokenType::new("WHITESPACE", Pattern::Match(whitespace), Some("\t "))
};
pub const YAML: TokenType = TokenType {
    hidden: true,
    opener: Some(yaml_opener),
    ..TokenType::new("YAML", Pattern::Match(yaml), Some("-"))
};
pub const DIRECTIVE: TokenType = TokenType {
    hidden: true,
    opener: Some(directive_opener),
    ..TokenType::new("DIRECTIVE", Pattern::Match(directive), Some("\t %"))
};
pub const SINGLE_LINE_COMMENT: TokenType = TokenType {
    hidden: true,
    ..TokenType::new(
        "SINGLE_LINE_COMMENT",
        Pattern::Match(single_line_comment),
        Some("\t %"),
    )
};

pub const fn keyword(word: &'static str) -> TokenType {
    TokenType::new(word, Pattern::Keyword(word), None)
}

fn match_at(
    types: &[TokenType],
    dead: &mut [bool],
    src: &str,
    k: usize,
    at: usize,
) -> Option<usize> {
    if dead[k] {
        return None;
    }
    let found = match types[k].pattern {
        Pattern::Literal(text) => src[at..].starts_with(text).then(|| at + text.len()),
        Pattern::Keyword(word) => src[at..].strip_prefix(word).and_then(|rest| {
            (rest.starts_with("%%") || rest.chars().next().is_none_or(is_js_space))
                .then(|| at + word.len())
        }),
        Pattern::Match(matcher) => matcher(src, at),
    };
    if found.is_none()
        && types[k]
            .opener
            .is_some_and(|opener| opener(src, at).is_some())
    {
        dead[k] = true;
    }
    found
}

fn starts_with(types: &[TokenType], k: usize, byte: u8) -> bool {
    match (types[k].pattern, types[k].first) {
        (Pattern::Literal(text), _) => text.as_bytes()[0] == byte,
        (_, None) => true,
        (_, Some(first)) => first.as_bytes().contains(&byte),
    }
}

pub fn tokenize(src: &str, types: &[TokenType], diagram: &str) -> Result<Tokens, PeleError> {
    let mut kinds = Vec::new();
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut dead = vec![false; types.len()];
    let mut p = 0;
    'scan: while p < src.len() {
        let byte = src.as_bytes()[p];
        for k in 0..types.len() {
            if byte < 128 && !starts_with(types, k, byte) {
                continue;
            }
            let Some(mut end) = match_at(types, &mut dead, src, k, p).filter(|&end| end > p) else {
                continue;
            };
            let mut kind = k;
            for name in types[k].longer {
                let Some(alt) = types.iter().position(|t| t.name == *name) else {
                    continue;
                };
                if let Some(alt_end) = match_at(types, &mut dead, src, alt, p)
                    && alt_end > end
                {
                    kind = alt;
                    end = alt_end;
                    break;
                }
            }
            if !types[kind].hidden {
                kinds.push(kind as i32);
                starts.push(p);
                ends.push(end);
            }
            p = end;
            continue 'scan;
        }
        return Err(lexical_error(diagram, src, p));
    }
    kinds.push(-1);
    starts.push(src.len());
    ends.push(src.len());
    Ok(Tokens {
        kinds,
        starts,
        ends,
        src: src.to_owned(),
    })
}

fn collapse_blanks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, '\t' | ' ') {
            let mut run = 1;
            while chars.next_if(|n| matches!(n, '\t' | ' ')).is_some() {
                run += 1;
            }
            if run >= 2 {
                out.push(' ');
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn single(text: &str) -> String {
    collapse_blanks(trim(text))
}

pub fn title_value(text: &str) -> String {
    let at = text.find("title").expect("a TITLE token holds `title`") + 5;
    single(&text[at..])
}

pub fn acc_title_value(text: &str) -> String {
    let at = text
        .find("accTitle")
        .expect("an ACC_TITLE token holds `accTitle`")
        + 8;
    let rest = text[at..].trim_start_matches(['\t', ' ']);
    single(rest.strip_prefix(':').unwrap_or(rest))
}

fn is_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

pub fn string_value(text: &str) -> String {
    let inner = &text[1..text.len() - 1];
    if !inner.contains('\\') {
        return inner.to_owned();
    }
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('v') => out.push('\u{b}'),
            Some('0') => out.push('\0'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn strip_line_indent(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if i == 0 || is_terminator(chars[i - 1]) {
            let mut j = i;
            while j < chars.len() && is_js_space(chars[j]) {
                j += 1;
            }
            if j > i {
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn strip_line_tail(chars: &[char]) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if is_js_space(chars[i]) {
            let mut run = i;
            while run < chars.len() && is_js_space(chars[run]) {
                run += 1;
            }
            if let Some(end) = (i + 1..=run)
                .rev()
                .find(|&e| e == chars.len() || is_terminator(chars[e]))
            {
                i = end;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn squeeze(chars: &[char], set: &[char], with: char) -> Vec<char> {
    let mut out = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if set.contains(&chars[i]) {
            let mut j = i;
            while j < chars.len() && set.contains(&chars[j]) {
                j += 1;
            }
            if j - i >= 2 {
                out.push(with);
            } else {
                out.push(chars[i]);
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

pub fn acc_descr_value(text: &str) -> String {
    let at = text
        .find("accDescr")
        .expect("an ACC_DESCR token holds `accDescr`")
        + 8;
    let rest = &text[at..];
    let line = rest.trim_start_matches(['\t', ' ']);
    if let Some(inline) = line.strip_prefix(':') {
        return single(&inline[..inline.find(['\n', '\r']).unwrap_or(inline.len())]);
    }
    let open = rest.trim_start_matches(is_js_space);
    let body = &open[1..open.find('}').expect("a block ends at `}`")];
    let chars: Vec<char> = body.chars().collect();
    let chars = strip_line_tail(&strip_line_indent(&chars));
    let chars = squeeze(&chars, &['\t', ' '], ' ');
    squeeze(&chars, &['\n', '\r'], '\n').into_iter().collect()
}

pub struct Reader<'a> {
    pub i: usize,
    tokens: Tokens,
    types: &'a [TokenType],
    diagram: &'a str,
}

impl<'a> Reader<'a> {
    pub fn new(tokens: Tokens, types: &'a [TokenType], diagram: &'a str) -> Self {
        Self {
            i: 0,
            tokens,
            types,
            diagram,
        }
    }

    pub fn kind(&self) -> i32 {
        self.tokens.kinds[self.i]
    }

    pub fn text(&self, index: usize) -> &str {
        &self.tokens.src[self.tokens.starts[index]..self.tokens.ends[index]]
    }

    pub fn take(&mut self) -> String {
        let text = self.text(self.i).to_owned();
        self.i += 1;
        text
    }

    pub fn accept(&mut self, kind: i32) -> bool {
        let found = self.kind() == kind;
        if found {
            self.i += 1;
        }
        found
    }

    pub fn expect(&mut self, kind: i32) -> Result<String, PeleError> {
        if self.kind() != kind {
            return Err(self.fail(&format!(
                "token of type '{}'",
                self.types[kind as usize].name
            )));
        }
        Ok(self.take())
    }

    pub fn fail(&self, expected: &str) -> PeleError {
        let found = if self.kind() == -1 {
            "end of input".to_owned()
        } else {
            format!("`{}`", self.text(self.i))
        };
        let Tokens {
            src, starts, ends, ..
        } = &self.tokens;
        let mut at = starts[self.i];
        if self.kind() == -1 {
            let mut k = self.i;
            while k > 0 && trim(&src[starts[k - 1]..ends[k - 1]]).is_empty() {
                k -= 1;
            }
            if k > 0 {
                at = ends[k - 1];
            }
        }
        syntax_error(
            self.diagram,
            src,
            at,
            format!("Expecting {expected} but found {found}."),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TYPES: [TokenType; 10] = [
        keyword("showInfo"),
        keyword("info"),
        ACC_DESCR,
        ACC_TITLE,
        TITLE,
        NEWLINE,
        WHITESPACE,
        YAML,
        DIRECTIVE,
        SINGLE_LINE_COMMENT,
    ];

    fn kinds(src: &str) -> Vec<i32> {
        tokenize(src, &TYPES, "info").unwrap().kinds
    }

    #[test]
    fn keywords_end_at_whitespace_a_comment_or_the_end() {
        assert_eq!(kinds("info\n"), vec![1, 5, -1]);
        assert_eq!(kinds("info showInfo"), vec![1, 0, -1]);
        assert!(tokenize("infox", &TYPES, "info").is_err());
    }

    #[test]
    fn titles_stop_at_a_comment() {
        let tokens = tokenize("title  A  B %% note\n", &TYPES, "info").unwrap();
        assert_eq!(tokens.kinds, vec![4, 5, -1]);
        assert_eq!(
            &tokens.src[tokens.starts[0]..tokens.ends[0]],
            "title  A  B "
        );
        assert_eq!(title_value("title  A  B "), "A B");
    }

    #[test]
    fn front_matter_and_directives_are_hidden() {
        assert_eq!(kinds("---\ntitle: x\n---\ninfo"), vec![1, -1]);
        assert_eq!(kinds("%%{init: {}}%%\ninfo"), vec![1, -1]);
    }

    #[test]
    fn a_longer_alternative_wins_over_the_token_it_extends() {
        let dash = TokenType {
            longer: &["YAML"],
            ..TokenType::new("-", Pattern::Literal("-"), None)
        };
        let types = [dash, NEWLINE, YAML, keyword("x")];
        assert_eq!(
            tokenize("---\na: 1\n---\nx", &types, "t").unwrap().kinds,
            vec![3, -1]
        );
        assert_eq!(tokenize("-\n", &types, "t").unwrap().kinds, vec![0, 1, -1]);
    }

    #[test]
    fn an_unclosed_fence_is_not_retried() {
        assert!(tokenize("---\ninfo", &TYPES, "info").is_err());
    }

    #[test]
    fn reads_accessible_text() {
        assert_eq!(acc_title_value("accTitle :  My   title"), "My title");
        assert_eq!(acc_descr_value("accDescr: one   two"), "one two");
        assert_eq!(
            acc_descr_value("accDescr {\n   first  line\n\n  second   \n}"),
            "first line\nsecond"
        );
    }

    #[test]
    fn reads_quoted_strings() {
        let types = [STRING, NEWLINE, WHITESPACE];
        let tokens = tokenize("\"a\\\"b\" 'c'\n", &types, "x").unwrap();
        assert_eq!(tokens.kinds, vec![0, 0, 1, -1]);
        assert_eq!(string_value("\"a\\\"b\\n\\q\""), "a\"b\nq");
        assert_eq!(string_value("'x'"), "x");
        assert!(tokenize("\"open", &types, "x").is_err());
        assert!(tokenize("\"a\\\nb\"", &types, "x").is_err());
        assert!(tokenize("\"a\nb\"", &types, "x").is_ok());
    }

    #[test]
    fn reports_where_the_parser_stopped() {
        let tokens = tokenize("info\ninfo", &TYPES, "info").unwrap();
        let mut reader = Reader::new(tokens, &TYPES, "info");
        reader.expect(1).unwrap();
        reader.expect(5).unwrap();
        let error = reader.fail("a title or the end of input");
        assert_eq!(error.line, 2);
        assert!(
            error
                .message
                .ends_with("Expecting a title or the end of input but found `info`.")
        );
    }
}
