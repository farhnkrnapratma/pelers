use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct OrderedMap<V> {
    entries: Vec<(String, V)>,
    index: HashMap<String, usize>,
}

impl<V> Default for OrderedMap<V> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }
}

impl<V> OrderedMap<V> {
    pub fn get(&self, key: &str) -> Option<&V> {
        self.index.get(key).map(|&at| &self.entries[at].1)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut V> {
        match self.index.get(key) {
            Some(&at) => Some(&mut self.entries[at].1),
            None => None,
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }

    pub fn set(&mut self, key: &str, value: V) {
        match self.index.get(key) {
            Some(&at) => self.entries[at].1 = value,
            None => {
                self.index.insert(key.to_owned(), self.entries.len());
                self.entries.push((key.to_owned(), value));
            }
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<V> {
        let at = self.index.remove(key)?;
        let (_, value) = self.entries.remove(at);
        for (k, (name, _)) in self.entries.iter().enumerate().skip(at) {
            self.index.insert(name.clone(), k);
        }
        Some(value)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, value)| value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_str(), value))
    }
}
