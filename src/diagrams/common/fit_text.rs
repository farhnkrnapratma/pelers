use crate::text::measurer::width;
use crate::util::chars::trim_end;
use crate::util::jsnum::js_max;

#[derive(Debug, Clone, PartialEq)]
pub struct Fitted {
    pub text: String,
    pub size: f64,
    pub width: f64,
}

pub fn fit_text(text: &str, max: f64, size: f64, min: f64, style: u32) -> Option<Fitted> {
    if text.is_empty() || max <= 0.0 {
        return None;
    }
    let mut measured = width(text, size, style);
    if measured <= max {
        return Some(Fitted {
            text: text.to_owned(),
            size,
            width: measured,
        });
    }

    let mut fitted = js_max(min, ((size * max) / measured).floor());
    while fitted >= min {
        measured = width(text, fitted, style);
        if measured <= max {
            return Some(Fitted {
                text: text.to_owned(),
                size: fitted,
                width: measured,
            });
        }
        fitted -= 1.0;
    }

    let chars: Vec<char> = text.chars().collect();
    let cut_at = |count: usize| -> String {
        let head: String = chars[..count].iter().collect();
        format!("{}…", trim_end(&head))
    };
    let mut low = 0;
    let mut high = chars.len() - 1;
    while low < high {
        let mid = (low + high).div_ceil(2);
        if width(&cut_at(mid), min, style) <= max {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    if low == 0 {
        return None;
    }
    let cut = cut_at(low);
    measured = width(&cut, min, style);
    (measured <= max).then_some(Fitted {
        text: cut,
        size: min,
        width: measured,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_text_that_fits() {
        let fitted = fit_text("abc", 100.0, 14.0, 8.0, 0).unwrap();
        assert_eq!((fitted.text.as_str(), fitted.size), ("abc", 14.0));
    }

    #[test]
    fn shrinks_then_cuts_with_an_ellipsis() {
        let shrunk = fit_text("abcdefghij", 60.0, 14.0, 8.0, 0).unwrap();
        assert!(shrunk.size < 14.0 && shrunk.size >= 8.0 && shrunk.text == "abcdefghij");
        let cut = fit_text("abcdefghijklmnopqrstuvwxyz", 40.0, 14.0, 8.0, 0).unwrap();
        assert!(cut.text.ends_with('…') && cut.size == 8.0 && cut.width <= 40.0);
        assert_eq!(fit_text("abc", 1.0, 14.0, 8.0, 0), None);
        assert_eq!(fit_text("", 50.0, 14.0, 8.0, 0), None);
    }
}
