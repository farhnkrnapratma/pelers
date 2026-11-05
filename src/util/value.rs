use std::collections::HashMap;

use super::jsnum::js_number_to_string;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Object),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Object {
    entries: Vec<(String, Value)>,
    index: HashMap<String, usize>,
    array_keys: bool,
}

fn is_array_index(key: &str) -> bool {
    let bytes = key.as_bytes();
    if bytes.is_empty() || bytes.len() > 10 || !bytes.iter().all(u8::is_ascii_digit) {
        return false;
    }
    if bytes[0] == b'0' && bytes.len() > 1 {
        return false;
    }
    key.parse::<u64>().is_ok_and(|n| n < 4_294_967_295)
}

impl Object {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.index.get(key).map(|&at| &self.entries[at].1)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        let at = *self.index.get(key)?;
        Some(&mut self.entries[at].1)
    }

    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        let key = key.into();
        match self.index.get(&key) {
            Some(&at) => self.entries[at].1 = value,
            None => {
                self.array_keys |= is_array_index(&key);
                self.index.insert(key.clone(), self.entries.len());
                self.entries.push((key, value));
            }
        }
    }

    pub fn keys(&self) -> Vec<&str> {
        self.iter().map(|(key, _)| key).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Value)> {
        let mut order: Vec<usize> = (0..self.entries.len()).collect();
        if self.array_keys {
            let (mut numeric, rest): (Vec<usize>, Vec<usize>) = order
                .into_iter()
                .partition(|&at| is_array_index(&self.entries[at].0));
            numeric.sort_by_key(|&at| self.entries[at].0.parse::<u64>().unwrap_or(0));
            numeric.extend(rest);
            order = numeric;
        }
        order
            .into_iter()
            .map(|at| (self.entries[at].0.as_str(), &self.entries[at].1))
    }
}

impl Value {
    pub fn as_object(&self) -> Option<&Object> {
        match self {
            Value::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut Object> {
        match self {
            Value::Object(object) => Some(object),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(text) => Some(text),
            _ => None,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(flag) => *flag,
            Value::Number(n) => !(n.is_nan() || *n == 0.0),
            Value::String(text) => !text.is_empty(),
            Value::Array(_) | Value::Object(_) => true,
        }
    }

    pub fn to_js_string(&self) -> String {
        match self {
            Value::Null => "null".to_owned(),
            Value::Bool(flag) => flag.to_string(),
            Value::Number(n) => js_number_to_string(*n),
            Value::String(text) => text.clone(),
            Value::Array(items) => items
                .iter()
                .map(|item| {
                    if matches!(item, Value::Null) {
                        String::new()
                    } else {
                        item.to_js_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            Value::Object(_) => "[object Object]".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterates_integer_keys_first_like_a_javascript_object() {
        let mut object = Object::new();
        for key in ["b", "10", "a", "2", "01", "4294967295"] {
            object.set(key, Value::Null);
        }
        assert_eq!(object.keys(), ["2", "10", "b", "a", "01", "4294967295"]);
    }

    #[test]
    fn replacing_a_key_keeps_its_place() {
        let mut object = Object::new();
        object.set("x", Value::Number(1.0));
        object.set("y", Value::Null);
        object.set("x", Value::Bool(true));
        assert_eq!(object.keys(), ["x", "y"]);
        assert_eq!(object.get("x"), Some(&Value::Bool(true)));
    }

    #[test]
    fn converts_like_javascript() {
        let array = Value::Array(vec![
            Value::Number(1.0),
            Value::Null,
            Value::Array(vec![Value::Bool(true)]),
        ]);
        assert_eq!(array.to_js_string(), "1,,true");
        assert_eq!(
            Value::Object(Object::new()).to_js_string(),
            "[object Object]"
        );
        assert!(!Value::Number(f64::NAN).is_truthy() && !Value::Number(-0.0).is_truthy());
        assert!(Value::Array(Vec::new()).is_truthy());
    }

    #[test]
    fn exposes_object_and_scalar_accessors() {
        let mut object = Object::new();
        assert!(object.is_empty());
        object.set("a", Value::Number(1.0));
        assert!(!object.is_empty());
        assert_eq!(object.len(), 1);
        *object.get_mut("a").unwrap() = Value::Number(2.0);
        assert_eq!(object.get("a"), Some(&Value::Number(2.0)));
        assert!(object.get_mut("b").is_none());
        let mut value = Value::Object(object);
        assert!(value.as_object_mut().is_some());
        assert!(Value::Null.as_object().is_none());
        assert!(Value::Null.as_object_mut().is_none());
        assert_eq!(Value::String("x".to_owned()).as_str(), Some("x"));
        assert_eq!(Value::Null.as_str(), None);
    }

    #[test]
    fn evaluates_truthiness_like_javascript() {
        assert!(!Value::Null.is_truthy());
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(!Value::Number(f64::NAN).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(Value::Number(2.0).is_truthy());
        assert!(!Value::String(String::new()).is_truthy());
        assert!(Value::String("a".to_owned()).is_truthy());
        assert!(Value::Array(Vec::new()).is_truthy());
        assert!(Value::Object(Object::new()).is_truthy());
    }
}
