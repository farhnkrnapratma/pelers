use std::collections::HashMap;

use crate::errors::{PelersError, PelersErrorCode};
use crate::util::ordered::OrderedMap;
use crate::util::value::Value;

pub struct OrderedJson {
    pub value: Value,
    pub property_order: OrderedMap<Vec<String>>,
}

#[derive(Debug)]
struct WalkError {
    offset: usize,
    message: String,
}

struct Frame {
    object: bool,
    pointer: String,
    keys: Vec<String>,
    ranges: HashMap<String, (usize, usize)>,
    key: String,
    from: usize,
    index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Value,
    Key,
    After,
}

fn is_digit(c: u16) -> bool {
    (48..=57).contains(&c)
}

fn token_text(c: u16) -> String {
    String::from_utf16_lossy(&[c])
}

fn walk(text: &str) -> Result<OrderedMap<Vec<String>>, WalkError> {
    let u: Vec<u16> = text.encode_utf16().collect();
    let n = u.len();
    let mut order: OrderedMap<Vec<String>> = OrderedMap::default();
    let mut created: Vec<String> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut at = 0usize;
    let mut pointer = String::new();
    let mut state = State::Value;
    let at_c = |at: usize| u.get(at).copied().unwrap_or(0);

    let unexpected = |at: usize| -> WalkError {
        if at >= n {
            WalkError {
                offset: at,
                message: "Unexpected end of JSON input".to_owned(),
            }
        } else {
            WalkError {
                offset: at,
                message: if u[at] >= 128 {
                    format!("Unrecognized token '{}'", token_text(u[at]))
                } else {
                    format!("Unexpected token '{}'", token_text(u[at]))
                },
            }
        }
    };

    let skip = |at: &mut usize| {
        while *at < n {
            let c = u[*at];
            if c != 32 && c != 9 && c != 10 && c != 13 {
                return;
            }
            *at += 1;
        }
    };

    let read_string = |at: &mut usize, decode: bool| -> Result<String, WalkError> {
        *at += 1;
        let mut value: Vec<u16> = Vec::new();
        while *at < n {
            let start = *at;
            let ch = u[*at];
            *at += 1;
            if ch == 34 {
                return Ok(String::from_utf16_lossy(&value));
            }
            if ch < 0x20 {
                return Err(WalkError {
                    offset: start,
                    message: "Unterminated string".to_owned(),
                });
            }
            if ch != 92 {
                if decode {
                    value.push(ch);
                }
                continue;
            }
            let escape_at = *at;
            let escape = u.get(*at).copied().unwrap_or(0);
            *at += 1;
            let decoded: u16 = match escape {
                34 | 92 | 47 => escape,
                98 => 8,
                102 => 12,
                110 => 10,
                114 => 13,
                116 => 9,
                117 => {
                    let hex: Vec<u16> = u[(*at).min(n)..(*at + 4).min(n)].to_vec();
                    let valid = hex.len() == 4
                        && hex.iter().all(|&h| {
                            is_digit(h) || (65..=70).contains(&h) || (97..=102).contains(&h)
                        });
                    if !valid {
                        return Err(WalkError {
                            offset: *at,
                            message: format!("Bad Unicode escape in JSON at position {}", *at),
                        });
                    }
                    let code =
                        u16::from_str_radix(&String::from_utf16_lossy(&hex), 16).unwrap_or(0);
                    *at += 4;
                    code
                }
                _ => {
                    return Err(WalkError {
                        offset: escape_at,
                        message: format!("Invalid escape character {}", token_text(escape)),
                    });
                }
            };
            if decode {
                value.push(decoded);
            }
        }
        Err(WalkError {
            offset: *at,
            message: "Unterminated string in JSON".to_owned(),
        })
    };

    let literal = |at: &mut usize, word: &str| -> Result<(), WalkError> {
        for (k, w) in word.bytes().enumerate() {
            if at_c(*at + k) != u16::from(w) {
                return Err(unexpected(*at + k));
            }
        }
        *at += word.len();
        Ok(())
    };

    let number = |at: &mut usize| -> Result<(), WalkError> {
        if at_c(*at) == 45 {
            *at += 1;
        }
        if at_c(*at) == 48 {
            *at += 1;
        } else if is_digit(at_c(*at)) {
            while is_digit(at_c(*at)) {
                *at += 1;
            }
        } else {
            return Err(unexpected(*at));
        }
        if at_c(*at) == 46 {
            *at += 1;
            if !is_digit(at_c(*at)) {
                return Err(unexpected(*at));
            }
            while is_digit(at_c(*at)) {
                *at += 1;
            }
        }
        if (at_c(*at) | 32) == 101 {
            *at += 1;
            if at_c(*at) == 43 || at_c(*at) == 45 {
                *at += 1;
            }
            if !is_digit(at_c(*at)) {
                return Err(unexpected(*at));
            }
            while is_digit(at_c(*at)) {
                *at += 1;
            }
        }
        Ok(())
    };

    let created_index =
        |order: &mut OrderedMap<Vec<String>>, created: &mut Vec<String>, pointer: &str| {
            order.set(pointer, Vec::new());
            created.push(pointer.to_owned());
        };

    skip(&mut at);
    loop {
        if state == State::Value {
            skip(&mut at);
            let c = at_c(at);
            state = State::After;
            if at >= n {
                return Err(unexpected(at));
            }
            if c == 123 {
                at += 1;
                created_index(&mut order, &mut created, &pointer);
                skip(&mut at);
                if at_c(at) == 125 {
                    at += 1;
                } else {
                    stack.push(Frame {
                        object: true,
                        pointer: pointer.clone(),
                        keys: Vec::new(),
                        ranges: HashMap::new(),
                        key: String::new(),
                        from: 0,
                        index: 0,
                    });
                    state = State::Key;
                }
            } else if c == 91 {
                at += 1;
                skip(&mut at);
                if at_c(at) == 93 {
                    at += 1;
                } else {
                    stack.push(Frame {
                        object: false,
                        pointer: pointer.clone(),
                        keys: Vec::new(),
                        ranges: HashMap::new(),
                        key: String::new(),
                        from: 0,
                        index: 0,
                    });
                    pointer.push_str("/0");
                    state = State::Value;
                }
            } else if c == 34 {
                read_string(&mut at, false)?;
            } else if c == 116 {
                literal(&mut at, "true")?;
            } else if c == 102 {
                literal(&mut at, "false")?;
            } else if c == 110 {
                literal(&mut at, "null")?;
            } else if c == 45 || is_digit(c) {
                number(&mut at)?;
            } else if c == 39 {
                return Err(WalkError {
                    offset: at,
                    message: "Single quotes (') are not allowed in JSON".to_owned(),
                });
            } else {
                return Err(unexpected(at));
            }
        } else if state == State::Key {
            let Some(frame) = stack.last_mut() else {
                break;
            };
            if at_c(at) != 34 {
                if frame.keys.is_empty() {
                    return Err(WalkError {
                        offset: at,
                        message: "Expected '}'".to_owned(),
                    });
                }
                return Err(WalkError {
                    offset: at,
                    message: "Property name must be a string literal".to_owned(),
                });
            }
            let key = read_string(&mut at, true)?;
            match frame.ranges.get(&key).copied() {
                None => frame.keys.push(key.clone()),
                Some((from, to)) => {
                    for pointer in &created[from..to] {
                        order.remove(pointer);
                    }
                }
            }
            frame.key = key.clone();
            frame.from = created.len();
            let frame_pointer = frame.pointer.clone();
            skip(&mut at);
            if at_c(at) != 58 {
                return Err(unexpected(at));
            }
            at += 1;
            pointer = format!(
                "{}/{}",
                frame_pointer,
                key.replace('~', "~0").replace('/', "~1")
            );
            state = State::Value;
        } else {
            let Some(frame) = stack.last_mut() else {
                break;
            };
            if frame.object {
                frame
                    .ranges
                    .insert(frame.key.clone(), (frame.from, created.len()));
            } else {
                frame.index += 1;
            }
            skip(&mut at);
            let c = at_c(at);
            let close = if frame.object { 125 } else { 93 };
            if c == close {
                at += 1;
                if frame.object {
                    order.set(&frame.pointer, frame.keys.clone());
                }
                stack.pop();
            } else if c != 44 {
                if at >= n {
                    return Err(unexpected(at));
                }
                let message = if frame.object {
                    "Expected '}'".to_owned()
                } else {
                    "Expected ']'".to_owned()
                };
                return Err(WalkError {
                    offset: at,
                    message,
                });
            } else {
                at += 1;
                skip(&mut at);
                if !frame.object && at_c(at) == 93 {
                    return Err(WalkError {
                        offset: at,
                        message: "Unexpected comma at the end of array expression".to_owned(),
                    });
                }
                let frame_pointer = frame.pointer.clone();
                let index = frame.index;
                pointer = format!("{frame_pointer}/{index}");
                state = if frame.object {
                    State::Key
                } else {
                    State::Value
                };
            }
        }
    }
    skip(&mut at);
    if at != n {
        return Err(unexpected(at));
    }
    Ok(order)
}

fn json_error(message: &str, line: usize, column: usize) -> PelersError {
    let mut error = PelersError::new(
        format!("{message} (line {line}, column {column})"),
        PelersErrorCode::Syntax,
    );
    error.diagram_type = Some("usecase".to_owned());
    error.line = line;
    error.column = column;
    error
}

fn value_of(text: &str) -> Option<Value> {
    crate::util::json::parse(text)
}

pub fn parse_ordered_json(
    text: &str,
    position: impl Fn(usize) -> (usize, usize),
) -> Result<OrderedJson, PelersError> {
    if value_of(text).is_none() {
        let (offset, message) = match walk(text) {
            Err(error) => (error.offset, error.message),
            Ok(_) => (0, "Unexpected token".to_owned()),
        };
        let (line, column) = position(offset);
        return Err(json_error(
            &format!("Invalid JSON: JSON Parse error: {message}"),
            line,
            column,
        ));
    }
    let value = value_of(text).expect("checked");
    if !matches!(value, Value::Object(_)) {
        let (line, column) = position(0);
        return Err(json_error(
            "JSON value must have an object root",
            line,
            column,
        ));
    }
    let property_order = match walk(text) {
        Ok(order) => order,
        Err(error) => {
            let (line, column) = position(error.offset);
            return Err(json_error(
                &format!("Invalid JSON: JSON Parse error: {}", error.message),
                line,
                column,
            ));
        }
    };
    Ok(OrderedJson {
        value,
        property_order,
    })
}
