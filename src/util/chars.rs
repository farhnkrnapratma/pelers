pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

pub fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn trim(text: &str) -> &str {
    text.trim_matches(is_js_space)
}

pub fn trim_start(text: &str) -> &str {
    text.trim_start_matches(is_js_space)
}

pub fn trim_end(text: &str) -> &str {
    text.trim_end_matches(is_js_space)
}

pub fn utf16_string(units: &[u16]) -> String {
    if units.iter().all(|&unit| unit < 0x80) {
        units.iter().map(|&unit| char::from(unit as u8)).collect()
    } else {
        String::from_utf16_lossy(units)
    }
}

pub fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

pub fn utf16_to_byte(text: &str, unit: usize) -> usize {
    let mut at = 0usize;
    for (byte, ch) in text.char_indices() {
        let width = ch.len_utf16();
        if at + width > unit {
            return byte;
        }
        at += width;
    }
    text.len()
}

pub fn find(hay: &str, needle: &str) -> Option<usize> {
    let (bytes, wanted) = (hay.as_bytes(), needle.as_bytes());
    let Some(&first) = wanted.first() else {
        return Some(0);
    };
    let mut from = 0;
    while let Some(i) = bytes[from..].iter().position(|&b| b == first) {
        let at = from + i;
        if bytes[at..].starts_with(wanted) {
            return Some(at);
        }
        from = at + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_substrings_like_str_find() {
        for (hay, needle) in [
            ("a%%b%%", "%%"),
            ("%%{x}%%", "}%%"),
            ("no", "%%"),
            ("ü---", "---"),
            ("", ""),
            ("abc", ""),
            ("a=\"b", "=\""),
            ("%", "%%"),
        ] {
            assert_eq!(find(hay, needle), hay.find(needle), "{hay} {needle}");
        }
    }

    #[test]
    fn follows_javascript_whitespace() {
        assert_eq!(trim("\u{feff} a \u{3000}"), "a");
        assert_eq!(trim("\u{85}a"), "\u{85}a");
        assert_eq!(trim_start("\u{a0}\u{2003}x "), "x ");
        assert_eq!(trim_end(" x\u{2028}"), " x");
    }

    #[test]
    fn word_characters_are_ascii() {
        assert!(is_word('_') && is_word('9') && !is_word('é') && !is_word('-'));
    }
}
