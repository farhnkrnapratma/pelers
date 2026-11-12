use crate::diagrams::common::generics::parse_generic_types;
use crate::util::chars::trim;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClassMember {
    pub id: String,
    pub method: bool,
    pub visibility: String,
    pub classifier: String,
    pub parameters: String,
    pub return_type: String,
}

fn is_line_end(c: u16) -> bool {
    matches!(c, 10 | 13 | 0x2028 | 0x2029)
}

fn is_visibility(c: u16) -> bool {
    matches!(c, 0x23 | 0x2b | 0x7e | 0x2d)
}

fn unit_string(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

fn slice(u: &[u16], from: isize, to: isize) -> &[u16] {
    let n = u.len() as isize;
    let from = from.clamp(0, n) as usize;
    let to = to.clamp(0, n) as usize;
    if from >= to { &[] } else { &u[from..to] }
}

impl ClassMember {
    pub fn new(input: &str, method: bool) -> Self {
        let units: Vec<u16> = input.encode_utf16().collect();
        let mut member = Self {
            method,
            ..Self::default()
        };
        if !method || !member.parse_method(&units) {
            member.parse_attribute(&units);
        }
        member.id = if member.id.starts_with(' ') {
            format!(" {}", trim(&member.id))
        } else {
            trim(&member.id).to_owned()
        };
        member
    }

    fn parse_method(&mut self, u: &[u16]) -> bool {
        let n = u.len() as isize;
        let mut start: isize = 0;
        while start < n {
            let mut end = start;
            while end < n && !is_line_end(u[end as usize]) {
                end += 1;
            }
            let mut close = end - 1;
            while close > start && u[close as usize] != 41 {
                close -= 1;
            }
            let mut open = close - 1;
            while open > start && u[open as usize] != 40 {
                open -= 1;
            }
            if open > start {
                let first = u[start as usize];
                let visible = is_visibility(first) && open > start + 1;
                if visible {
                    self.visibility = unit_string(&[first]);
                }
                let id_from = if visible { start + 1 } else { start };
                self.id = unit_string(slice(u, id_from, open));
                self.parameters = trim(&unit_string(slice(u, open + 1, close))).to_owned();
                let mut rest = close + 1;
                let mut classifier = String::new();
                if rest < n {
                    let next = u[rest as usize];
                    if next == 0x24
                        || next == 0x2a
                        || crate::util::chars::is_js_space(
                            char::from_u32(u32::from(next)).unwrap_or('x'),
                        )
                    {
                        classifier = trim(&unit_string(&[next])).to_owned();
                        rest += 1;
                    }
                }
                let mut stop = rest;
                while stop < n && !is_line_end(u[stop as usize]) {
                    stop += 1;
                }
                self.return_type = trim(&unit_string(slice(u, rest, stop))).to_owned();
                if classifier.is_empty()
                    && let Some(last) = self.return_type.chars().last()
                    && (last == '$' || last == '*')
                {
                    classifier = last.to_string();
                    self.return_type.pop();
                }
                self.classifier = classifier;
                return true;
            }
            start = end + 1;
        }
        self.method = false;
        false
    }

    fn parse_attribute(&mut self, u: &[u16]) {
        if let Some(&first) = u.first()
            && is_visibility(first)
        {
            self.visibility = unit_string(&[first]);
        }
        if let Some(&last) = u.last()
            && (last == 0x24 || last == 0x2a)
        {
            self.classifier = unit_string(&[last]);
        }
        let len = u.len() as isize;
        let mut from = if self.visibility.is_empty() { 0 } else { 1 };
        let mut to = if self.classifier.is_empty() {
            len
        } else {
            len - 1
        };
        if from > to {
            std::mem::swap(&mut from, &mut to);
        }
        self.id = unit_string(slice(u, from, to));
    }

    pub fn display_details(&self) -> (String, String) {
        let mut display = format!("{}{}", self.visibility, parse_generic_types(&self.id));
        if self.method {
            display.push_str(&format!(
                "({})",
                parse_generic_types(trim(&self.parameters))
            ));
            if !self.return_type.is_empty() {
                display.push_str(" : ");
                display.push_str(&parse_generic_types(&self.return_type));
            }
        }
        (trim(&display).to_owned(), self.parse_classifier())
    }

    pub fn parse_classifier(&self) -> String {
        match self.classifier.as_str() {
            "*" => "font-style:italic;".to_owned(),
            "$" => "text-decoration:underline;".to_owned(),
            _ => String::new(),
        }
    }
}
