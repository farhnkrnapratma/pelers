use super::chars::{is_js_space, trim, trim_start};
use super::jsnum::js_number_to_string;
use super::value::{Object, Value};

const MAX_DEPTH: usize = 64;

#[derive(Debug)]
enum Fail {
    Error(String),
    Incomplete(String),
}

type Parsed<T> = Result<T, Fail>;

fn error<T>(message: &str) -> Parsed<T> {
    Err(Fail::Error(message.to_owned()))
}

fn incomplete<T>(message: &str) -> Parsed<T> {
    Err(Fail::Incomplete(message.to_owned()))
}

struct Reader {
    lines: Vec<String>,
    i: usize,
}

impl Reader {
    fn new(src: &str) -> Self {
        let unified = if src.contains('\r') {
            src.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            src.to_owned()
        };
        Self {
            lines: unified.split('\n').map(str::to_owned).collect(),
            i: 0,
        }
    }

    fn skip_blank(&mut self) {
        while self.i < self.lines.len() {
            let t = trim(&self.lines[self.i]);
            if !t.is_empty() && !t.starts_with('#') && t != "---" {
                break;
            }
            self.i += 1;
        }
    }

    fn indent(&self) -> Parsed<usize> {
        let line = self.lines[self.i].as_bytes();
        let k = line.iter().take_while(|&&b| b == b' ').count();
        if line.get(k) == Some(&b'\t') {
            return error("YAML: tabs are not allowed for indentation");
        }
        Ok(k)
    }
}

fn leading_space(line: &str) -> usize {
    line.chars().count() - trim_start(line).chars().count()
}

fn is_blank(b: Option<&u8>) -> bool {
    matches!(b, Some(b' ' | b'\t' | b'\n' | b'\r'))
}

fn key_start(c: char) -> bool {
    is_js_space(c) || "#:,[]{}&*!|>'\"%@`".contains(c)
}

fn split_key(line: &str) -> Option<(&str, usize)> {
    let b = line.as_bytes();
    let first = *b.first()?;
    let end;
    let mut colon;
    if first == b'"' || first == b'\'' {
        let mut i = 1;
        loop {
            if i >= b.len() {
                return None;
            }
            let c = b[i];
            if first == b'"' && c == b'\\' {
                i += 2;
            } else if c == first {
                if first == b'"' || b.get(i + 1) != Some(&b'\'') {
                    break;
                }
                i += 2;
            } else {
                i += 1;
            }
        }
        end = i + 1;
        colon = end;
        while is_blank(b.get(colon)) {
            colon += 1;
        }
        if b.get(colon) != Some(&b':') {
            return None;
        }
    } else {
        if key_start(line.chars().next()?) {
            return None;
        }
        if first == b'-' && (b.len() < 2 || is_blank(b.get(1)) || b[1] == b':' || b[1] == b'#') {
            return None;
        }
        colon = 1;
        while colon < b.len() && b[colon] != b':' && b[colon] != b'#' {
            colon += 1;
        }
        if b.get(colon) != Some(&b':') {
            return None;
        }
        let mut stop = colon;
        while is_blank(b.get(stop - 1)) {
            stop -= 1;
        }
        end = stop;
    }
    let mut length = colon + 1;
    if length < b.len() && !is_blank(b.get(length)) {
        return None;
    }
    while is_blank(b.get(length)) {
        length += 1;
    }
    Some((&line[..end], length))
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn unsigned(s: &str) -> &str {
    s.strip_prefix(['-', '+']).unwrap_or(s)
}

fn is_int(s: &str) -> bool {
    let d = unsigned(s);
    d == "0" || (all_digits(d) && !d.starts_with('0'))
}

fn is_float(s: &str) -> bool {
    let d = unsigned(s);
    let (mantissa, exponent) = match d.find(['e', 'E']) {
        Some(at) => (&d[..at], Some(&d[at + 1..])),
        None => (d, None),
    };
    if let Some(exponent) = exponent
        && !all_digits(unsigned(exponent))
    {
        return false;
    }
    match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            let fraction_ok = fraction.bytes().all(|b| b.is_ascii_digit());
            if whole.is_empty() {
                !fraction.is_empty() && fraction_ok
            } else {
                all_digits(whole) && fraction_ok
            }
        }
        None => all_digits(mantissa),
    }
}

fn radix_number(s: &str) -> Option<f64> {
    let d = unsigned(s);
    let (radix, digits) = d
        .strip_prefix("0x")
        .or_else(|| d.strip_prefix("0X"))
        .map(|rest| (16, rest))
        .or_else(|| d.strip_prefix("0o").map(|rest| (8, rest)))
        .or_else(|| d.strip_prefix("0b").map(|rest| (2, rest)))?;
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let value = u128::from_str_radix(digits, radix)
        .map(|n| n as f64)
        .unwrap_or_else(|_| {
            digits.chars().fold(0.0, |n, c| {
                n * f64::from(radix) + f64::from(c.to_digit(radix).unwrap_or(0))
            })
        });
    Some(if s.starts_with('-') { -value } else { value })
}

fn scalar(raw: &str) -> Value {
    let s = trim(raw);
    match s {
        "" | "~" | "null" | "Null" | "NULL" => return Value::Null,
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    let c = s.as_bytes()[0];
    if c != b'-' && c != b'+' && c != b'.' && !c.is_ascii_digit() {
        return Value::String(s.to_owned());
    }
    if (is_int(s) || is_float(s))
        && let Ok(n) = s.parse::<f64>()
    {
        return Value::Number(n);
    }
    if let Some(n) = radix_number(s) {
        return Value::Number(n);
    }
    if matches!(unsigned(s), ".inf" | ".Inf" | ".INF") {
        return Value::Number(if s.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    if matches!(s, ".nan" | ".NaN" | ".NAN") {
        return Value::Number(f64::NAN);
    }
    Value::String(s.to_owned())
}

fn escape(e: char) -> Option<char> {
    Some(match e {
        '0' => '\0',
        'a' => '\u{7}',
        'b' => '\u{8}',
        't' => '\t',
        'n' => '\n',
        'v' => '\u{b}',
        'f' => '\u{c}',
        'r' => '\r',
        'e' => '\u{1b}',
        ' ' => ' ',
        '"' => '"',
        '/' => '/',
        '\\' => '\\',
        'N' => '\u{85}',
        '_' => '\u{a0}',
        'L' => '\u{2028}',
        'P' => '\u{2029}',
        _ => return None,
    })
}

fn fold(text: &str) -> String {
    if !text.contains('\n') {
        return text.to_owned();
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let last = lines.len() - 1;
    let trimmed: Vec<&str> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let mut line: &str = line;
            if i > 0 {
                line = line.trim_start_matches([' ', '\t']);
            }
            if i < last {
                line = line.trim_end_matches([' ', '\t']);
            }
            line
        })
        .collect();
    let joined = trimmed.join("\n");
    let mut out = String::with_capacity(joined.len());
    let mut run = 0;
    for c in joined.chars() {
        if c == '\n' {
            run += 1;
            continue;
        }
        flush_breaks(&mut out, run);
        run = 0;
        out.push(c);
    }
    flush_breaks(&mut out, run);
    out
}

fn flush_breaks(out: &mut String, run: usize) {
    match run {
        0 => {}
        1 => out.push(' '),
        n => out.extend(std::iter::repeat_n('\n', n - 1)),
    }
}

fn js_parse_int_16(text: &str) -> f64 {
    let t = trim_start(text);
    let (negative, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let t = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .unwrap_or(t);
    let digits: String = t.chars().take_while(char::is_ascii_hexdigit).collect();
    if digits.is_empty() {
        return f64::NAN;
    }
    let value = digits.chars().fold(0.0, |n, c| {
        n * 16.0 + f64::from(c.to_digit(16).unwrap_or(0))
    });
    if negative { -value } else { value }
}

fn from_code_point(n: f64) -> Parsed<char> {
    if n.is_nan() || n < 0.0 || n > 1_114_111.0 || n.fract() != 0.0 {
        return Err(Fail::Error(format!(
            "Invalid code point {}",
            js_number_to_string(n)
        )));
    }
    Ok(char::from_u32(n as u32).unwrap_or('\u{fffd}'))
}

fn remove_escaped_breaks(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find("\\\n") {
        out.push_str(&rest[..at]);
        rest = rest[at + 2..].trim_start_matches([' ', '\t']);
    }
    out.push_str(rest);
    out
}

fn unquote_double(body: &str) -> Parsed<String> {
    if !body.contains('\\') {
        return Ok(fold(body));
    }
    let s = fold(&remove_escaped_breaks(body));
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch != '\\' {
            out.push(ch);
            i += 1;
            continue;
        }
        i += 1;
        let e = chars.get(i).copied();
        match e {
            Some(kind @ ('x' | 'u' | 'U')) => {
                let len = match kind {
                    'x' => 2,
                    'u' => 4,
                    _ => 8,
                };
                let from = (i + 1).min(chars.len());
                let to = (i + 1 + len).min(chars.len());
                let digits: String = chars[from..to].iter().collect();
                out.push(from_code_point(js_parse_int_16(&digits))?);
                i += len;
            }
            Some(other) => match escape(other) {
                Some(mapped) => out.push(mapped),
                None => return error("YAML: unknown escape sequence"),
            },
            None => return error("YAML: unknown escape sequence"),
        }
        i += 1;
    }
    Ok(out)
}

fn char_before(s: &str, pos: usize) -> Option<char> {
    s[..pos].chars().next_back()
}

fn char_at(s: &str, pos: usize) -> Option<char> {
    s.get(pos..).and_then(|rest| rest.chars().next())
}

fn skip(s: &str, mut pos: usize, commas: bool) -> usize {
    let b = s.as_bytes();
    while pos < b.len() {
        let c = b[pos];
        if c == b' ' || c == b'\n' || c == b'\t' || c == b'\r' || (commas && c == b',') {
            pos += 1;
        } else if c == b'#' && (pos == 0 || char_before(s, pos).is_some_and(is_js_space)) {
            pos = s[pos..].find('\n').map_or(s.len(), |nl| pos + nl);
        } else {
            break;
        }
    }
    pos
}

fn entry_start(ch: u8, separated: bool) -> Parsed<()> {
    if !separated {
        return error("YAML: missed comma between flow collection entries");
    }
    if ch == b',' {
        return error("YAML: expected the node content, but found ','");
    }
    Ok(())
}

fn flow_key(value: &Value) -> String {
    value.to_js_string()
}

fn flow(s: &str, pos: usize, stops: &str, depth: usize) -> Parsed<(Value, usize)> {
    if depth > MAX_DEPTH {
        return error("YAML: nesting is too deep");
    }
    let b = s.as_bytes();
    let mut pos = skip(s, pos, false);
    let ch = b.get(pos).copied();
    if ch == Some(b'{') {
        let mut out = Object::new();
        pos += 1;
        let mut separated = true;
        loop {
            pos = skip(s, pos, false);
            if pos >= b.len() {
                return incomplete("YAML: unexpected end of the stream within a flow collection");
            }
            if b[pos] == b'}' {
                return Ok((Value::Object(out), pos + 1));
            }
            entry_start(b[pos], separated)?;
            let (key, next) = flow(s, pos, ":,}", depth + 1)?;
            pos = skip(s, next, false);
            let mut value = Value::Null;
            if b.get(pos) == Some(&b':') {
                let (parsed, next) = flow(s, pos + 1, ",}", depth + 1)?;
                value = parsed;
                pos = next;
            }
            out.set(flow_key(&key), value);
            pos = skip(s, pos, false);
            separated = b.get(pos) == Some(&b',');
            if separated {
                pos += 1;
            }
        }
    }
    if ch == Some(b'[') {
        let mut out = Vec::new();
        pos += 1;
        let mut separated = true;
        loop {
            pos = skip(s, pos, false);
            if pos >= b.len() {
                return incomplete("YAML: unexpected end of the stream within a flow collection");
            }
            if b[pos] == b']' {
                return Ok((Value::Array(out), pos + 1));
            }
            entry_start(b[pos], separated)?;
            let (value, next) = flow(s, pos, ",]", depth + 1)?;
            out.push(value);
            pos = skip(s, next, false);
            separated = b.get(pos) == Some(&b',');
            if separated {
                pos += 1;
            }
        }
    }
    if ch == Some(b'"') {
        let mut end = pos + 1;
        while end < b.len() && b[end] != b'"' {
            end += if b[end] == b'\\' { 2 } else { 1 };
        }
        if end >= b.len() {
            return incomplete("YAML: unexpected end of the stream within a double quoted scalar");
        }
        return Ok((Value::String(unquote_double(&s[pos + 1..end])?), end + 1));
    }
    if ch == Some(b'\'') {
        let mut end = pos + 1;
        while end < b.len() {
            if b[end] == b'\'' {
                if b.get(end + 1) != Some(&b'\'') {
                    break;
                }
                end += 1;
            }
            end += 1;
        }
        if end >= b.len() {
            return incomplete("YAML: unexpected end of the stream within a single quoted scalar");
        }
        return Ok((
            Value::String(fold(&s[pos + 1..end]).replace("''", "'")),
            end + 1,
        ));
    }
    let mut end = pos;
    while end < b.len() {
        let c = b[end];
        if stops.as_bytes().contains(&c)
            && (c != b':'
                || end + 1 >= b.len()
                || char_at(s, end + 1)
                    .is_some_and(|n| is_js_space(n) || matches!(n, ',' | '[' | ']' | '{' | '}')))
        {
            break;
        }
        if c == b'#' && end > pos && char_before(s, end).is_some_and(is_js_space) {
            let next = s[end..].find('\n').map_or(s.len(), |nl| end + nl);
            return Ok((scalar(&fold(trim(&s[pos..end]))), next));
        }
        end += 1;
    }
    Ok((scalar(&fold(trim(&s[pos..end]))), end))
}

fn strip_comment(s: &str) -> &str {
    let b = s.as_bytes();
    let mut quote = 0u8;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if quote != 0 {
            if c == b'\\' && quote == b'"' {
                i += 1;
            } else if c == quote {
                quote = 0;
            }
        } else if c == b'"' || c == b'\'' {
            quote = c;
        } else if c == b'#' && (i == 0 || b[i - 1] == b' ' || b[i - 1] == b'\t') {
            return &s[..i];
        }
        i += 1;
    }
    s
}

fn block_scalar(r: &mut Reader, header: &str, parent_indent: isize) -> String {
    let folded = header.starts_with('>');
    let chomp = if header.contains('-') {
        '-'
    } else if header.contains('+') {
        '+'
    } else {
        ' '
    };
    let explicit = header.chars().find(|c| ('1'..='9').contains(c));
    let mut indent: Option<isize> =
        explicit.map(|d| parent_indent + d.to_digit(10).unwrap_or(0) as isize);
    let mut out: Vec<String> = Vec::new();
    while r.i < r.lines.len() {
        let line = &r.lines[r.i];
        if trim(line).is_empty() {
            out.push(String::new());
            r.i += 1;
            continue;
        }
        let k = leading_space(line) as isize;
        let level = match indent {
            Some(level) => level,
            None => {
                if k <= parent_indent {
                    break;
                }
                indent = Some(k);
                k
            }
        };
        if k < level {
            break;
        }
        let at = line
            .char_indices()
            .nth(level as usize)
            .map_or(line.len(), |(b, _)| b);
        out.push(line[at..].to_owned());
        r.i += 1;
    }
    let mut trailing = 0;
    while out.last().is_some_and(String::is_empty) {
        out.pop();
        trailing += 1;
    }
    let text = if folded {
        let mut text = String::new();
        for (i, cur) in out.iter().enumerate() {
            if i > 0 {
                let prev = &out[i - 1];
                if cur.is_empty() {
                    text.push('\n');
                } else if !prev.is_empty() {
                    let spaced = prev.starts_with([' ', '\t']) || cur.starts_with([' ', '\t']);
                    text.push(if spaced { '\n' } else { ' ' });
                }
            }
            text.push_str(cur);
        }
        text
    } else {
        out.join("\n")
    };
    if out.is_empty() {
        return if chomp == '+' {
            "\n".repeat(trailing)
        } else {
            String::new()
        };
    }
    match chomp {
        '-' => text,
        '+' => text + &"\n".repeat(trailing + 1),
        _ => text + "\n",
    }
}

fn block_header(rest: &str) -> bool {
    let after = &rest[1..];
    let after = after.trim_start_matches(|c: char| c == '-' || c == '+' || c.is_ascii_digit());
    let after = trim_start(after);
    match after.strip_prefix('#') {
        Some(comment) => !comment.contains(['\n', '\r', '\u{2028}', '\u{2029}']),
        None => after.is_empty(),
    }
}

fn inline_value(r: &mut Reader, rest: &str, indent: isize) -> Parsed<Value> {
    let first = rest.chars().next();
    if matches!(first, Some('|' | '>')) && block_header(rest) {
        return Ok(Value::String(block_scalar(r, rest, indent)));
    }
    if matches!(first, Some('{' | '[' | '"' | '\'')) {
        let start = r.i;
        let mut text = rest.to_owned();
        let mut taken = 0;
        let mut batch = 1;
        loop {
            match flow(&text, 0, "", 0) {
                Ok((value, end)) => {
                    let used = text[..end].matches('\n').count();
                    let line_end = text[end..].find('\n').map_or(text.len(), |nl| end + nl);
                    if !trim(strip_comment(&text[end..line_end])).is_empty() {
                        return error("YAML: unexpected content after a value");
                    }
                    r.i = start + used;
                    return Ok(value);
                }
                Err(Fail::Incomplete(message)) => {
                    if start + taken >= r.lines.len() {
                        return Err(Fail::Error(message));
                    }
                    let upto = (start + taken + batch).min(r.lines.len());
                    let more = &r.lines[start + taken..upto];
                    text.push('\n');
                    text.push_str(&more.join("\n"));
                    taken += more.len();
                    batch *= 2;
                }
                Err(other) => return Err(other),
            }
        }
    }
    let mut text = trim(strip_comment(rest)).to_owned();
    while r.i < r.lines.len() {
        let line = &r.lines[r.i];
        if trim(line).is_empty() {
            let mut j = r.i;
            while j < r.lines.len() && trim(&r.lines[j]).is_empty() {
                j += 1;
            }
            if j >= r.lines.len() || leading_space(&r.lines[j]) as isize <= indent {
                break;
            }
            text.push_str(&"\n".repeat(j - r.i));
            r.i = j;
            continue;
        }
        if leading_space(line) as isize <= indent {
            break;
        }
        let more = trim(strip_comment(line)).to_owned();
        if !text.ends_with('\n') {
            text.push(' ');
        }
        text.push_str(&more);
        r.i += 1;
    }
    Ok(scalar(&text))
}

fn is_sequence_item(line: &str) -> bool {
    line == "-" || line.starts_with("- ")
}

fn block(r: &mut Reader, min_indent: usize, depth: usize) -> Parsed<Value> {
    if depth > MAX_DEPTH {
        return error("YAML: nesting is too deep");
    }
    r.skip_blank();
    if r.i >= r.lines.len() {
        return Ok(Value::Null);
    }
    let indent = r.indent()?;
    if indent < min_indent {
        return Ok(Value::Null);
    }
    let line = r.lines[r.i][indent..].to_owned();

    if is_sequence_item(&line) {
        let mut out = Vec::new();
        while r.i < r.lines.len() {
            r.skip_blank();
            if r.i >= r.lines.len() || r.indent()? != indent {
                break;
            }
            let cur = r.lines[r.i][indent..].to_owned();
            if !is_sequence_item(&cur) {
                break;
            }
            let rest = &cur[1..];
            let pad = rest.chars().count() - trim_start(rest).chars().count() + 1;
            let body = trim(strip_comment(rest));
            if body.is_empty() {
                r.i += 1;
                out.push(block(r, indent + 1, depth + 1)?);
            } else if split_key(body).is_some() {
                r.lines[r.i] = " ".repeat(indent + pad) + trim_start(rest);
                out.push(block(r, indent + pad, depth + 1)?);
            } else {
                r.i += 1;
                out.push(inline_value(r, trim(rest), indent as isize)?);
            }
        }
        return Ok(Value::Array(out));
    }

    if split_key(&line).is_none() {
        r.i += 1;
        return inline_value(r, trim(&line), indent as isize - 1);
    }

    let mut out = Object::new();
    while r.i < r.lines.len() {
        r.skip_blank();
        if r.i >= r.lines.len() {
            break;
        }
        let k = r.indent()?;
        if k < indent {
            break;
        }
        if k > indent {
            return error("YAML: bad indentation of a mapping entry");
        }
        let cur = r.lines[r.i][indent..].to_owned();
        let Some((raw_key, length)) = split_key(&cur) else {
            return error("YAML: can not read a block mapping entry");
        };
        let key = flow_key(&flow(raw_key, 0, "", 0)?.0);
        let rest = &cur[length..];
        r.i += 1;
        if trim(strip_comment(rest)).is_empty() {
            r.skip_blank();
            if r.i < r.lines.len() {
                let next = r.indent()?;
                let sequence = is_sequence_item(&r.lines[r.i][next..]);
                let value = if next > indent || (sequence && next == indent) {
                    block(r, if sequence { indent } else { indent + 1 }, depth + 1)?
                } else {
                    Value::Null
                };
                out.set(key, value);
            } else {
                out.set(key, Value::Null);
            }
        } else {
            let value = inline_value(r, trim(rest), indent as isize)?;
            out.set(key, value);
        }
    }
    Ok(Value::Object(out))
}

pub fn parse_yaml(src: &str) -> Result<Value, String> {
    run(src).map_err(|fail| match fail {
        Fail::Error(message) | Fail::Incomplete(message) => message,
    })
}

fn run(src: &str) -> Parsed<Value> {
    let trimmed = trim(src);
    if trimmed.starts_with(['{', '[']) {
        match flow(trimmed, 0, "", 0) {
            Ok((value, end)) => {
                if skip(trimmed, end, false) == trimmed.len() {
                    return Ok(value);
                }
            }
            Err(Fail::Incomplete(message)) => return Err(Fail::Error(message)),
            Err(other) => return Err(other),
        }
    }
    let mut r = Reader::new(src);
    let value = block(&mut r, 0, 0)?;
    r.skip_blank();
    if r.i < r.lines.len() && trim(&r.lines[r.i]) != "..." {
        return error("YAML: end of the stream or a document separator is expected");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(value: &Value) -> Vec<&str> {
        value.as_object().unwrap().keys()
    }

    #[test]
    fn reads_flow_and_block_collections() {
        let value =
            parse_yaml("title: Hello\nconfig:\n  theme: dark\nlist:\n  - a\n  - b: 1\n").unwrap();
        assert_eq!(keys(&value), ["title", "config", "list"]);
        let config = value.as_object().unwrap().get("config").unwrap();
        assert_eq!(
            config.as_object().unwrap().get("theme").unwrap().as_str(),
            Some("dark")
        );
        assert_eq!(
            parse_yaml("{ a: [1, 2], b: x }")
                .unwrap()
                .as_object()
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn reads_scalars_like_the_json_schema() {
        let value =
            parse_yaml("a: 0x1f\nb: -12\nc: 1e3\nd: .5\ne: \"12\"\nf: True\ng: Null\n").unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.get("a"), Some(&Value::Number(31.0)));
        assert_eq!(object.get("b"), Some(&Value::Number(-12.0)));
        assert_eq!(object.get("c"), Some(&Value::Number(1000.0)));
        assert_eq!(object.get("d"), Some(&Value::Number(0.5)));
        assert_eq!(object.get("e").unwrap().as_str(), Some("12"));
        assert_eq!(object.get("f"), Some(&Value::Bool(true)));
        assert_eq!(object.get("g"), Some(&Value::Null));
    }

    #[test]
    fn reads_block_scalars() {
        let value = parse_yaml("label: |\n  This is a\n  multiline string\nother: 1\n").unwrap();
        assert_eq!(
            value.as_object().unwrap().get("label").unwrap().as_str(),
            Some("This is a\nmultiline string\n")
        );
    }

    #[test]
    fn reports_errors_with_the_oracle_wording() {
        assert_eq!(
            parse_yaml("{ a: 1").unwrap_err(),
            "YAML: unexpected end of the stream within a flow collection"
        );
        assert_eq!(
            parse_yaml("a:\n\tb: 1").unwrap_err(),
            "YAML: tabs are not allowed for indentation"
        );
        assert!(parse_yaml("{ a: ").is_err());
        assert!(
            parse_yaml(&format!("{{ a: {} }}", "[".repeat(20000)))
                .unwrap_err()
                .contains("too deep")
        );
    }

    #[test]
    fn keeps_a_proto_key_as_data() {
        let value = parse_yaml("{ __proto__: { polluted: true } }").unwrap();
        assert_eq!(keys(&value), ["__proto__"]);
    }

    fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
        value.as_object().unwrap().get(key).unwrap()
    }

    #[test]
    fn normalizes_carriage_returns() {
        let value = parse_yaml("a: 1\r\nb: 2\rc: 3").unwrap();
        assert_eq!(keys(&value), ["a", "b", "c"]);
    }

    #[test]
    fn reads_oversized_radix_numbers() {
        let value = parse_yaml(&format!("a: 0x{}\nb: -0o17", "f".repeat(40))).unwrap();
        assert!(matches!(field(&value, "a"), Value::Number(n) if *n > 1e47));
        assert_eq!(field(&value, "b"), &Value::Number(-15.0));
    }

    #[test]
    fn maps_every_double_quoted_escape() {
        let value = parse_yaml(
            "a: \"\\0\\a\\b\\t\\n\\v\\f\\r\\e\\ \\\"\\/\\\\\\N\\_\\L\\P\\x41\\u0042\\U00000043\"",
        )
        .unwrap();
        assert_eq!(
            field(&value, "a").as_str(),
            Some("\0\u{7}\u{8}\t\n\u{b}\u{c}\r\u{1b} \"/\\\u{85}\u{a0}\u{2028}\u{2029}ABC")
        );
    }

    #[test]
    fn rejects_bad_double_quoted_escapes() {
        assert!(parse_yaml("a: \"\\q\"").is_err());
        assert!(parse_yaml("a: \"\\x-1\"").is_err());
        assert!(parse_yaml("a: \"b\\\\\n\"").is_err());
    }

    #[test]
    fn rejects_unterminated_single_quotes() {
        assert!(parse_yaml("a: 'abc").is_err());
    }

    #[test]
    fn reads_empty_and_commented_block_scalars() {
        let value = parse_yaml("a: |\nb: 1").unwrap();
        assert_eq!(field(&value, "a").as_str(), Some(""));
        let value = parse_yaml("a: |+\n\n\nb: 1").unwrap();
        assert_eq!(field(&value, "a").as_str(), Some("\n\n"));
        let value = parse_yaml("a: | # note\n  x\nb: 1").unwrap();
        assert_eq!(field(&value, "a").as_str(), Some("x\n"));
    }

    #[test]
    fn refuses_deep_block_nesting() {
        let source = (0..80)
            .map(|level| format!("{}k:", " ".repeat(level)))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(parse_yaml(&source).is_err());
    }
}
