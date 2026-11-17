#[derive(Clone, Debug, PartialEq)]
pub struct IshikawaNode {
    pub text: String,
    pub children: Vec<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct IshikawaDb {
    pub title: Option<String>,
    pub nodes: Vec<IshikawaNode>,
    pub root: Option<usize>,
    stack: Vec<(i64, usize)>,
    base_level: Option<i64>,
}

impl IshikawaDb {
    pub fn add_node(&mut self, raw_level: i64, text: &str) {
        let index = self.nodes.len();
        self.nodes.push(IshikawaNode {
            text: text.to_owned(),
            children: Vec::new(),
        });
        let Some(_) = self.root else {
            self.root = Some(index);
            self.stack = vec![(0, index)];
            return;
        };
        let base = *self.base_level.get_or_insert(raw_level);
        let level = (raw_level - base + 1).max(1);
        while self.stack.len() > 1 && self.stack[self.stack.len() - 1].0 >= level {
            self.stack.pop();
        }
        let (_, parent) = self.stack[self.stack.len() - 1];
        self.nodes[parent].children.push(index);
        self.stack.push((level, index));
    }
}

pub fn parse(
    source: &str,
    title: Option<String>,
) -> Result<IshikawaDb, crate::errors::PelersError> {
    let mut db = IshikawaDb {
        title,
        ..IshikawaDb::default()
    };
    super::parser::parse_ishikawa(source, &mut db)?;
    Ok(db)
}
