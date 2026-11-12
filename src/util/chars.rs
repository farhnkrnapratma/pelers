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

#[cfg(test)]
mod tests {
    use super::*;

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
