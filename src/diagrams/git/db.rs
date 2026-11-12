use std::cmp::Ordering;

use super::parser::{CommitTypeName, Direction, GitGraphAst, StatementAst};
use crate::errors::{PelersError, PelersErrorCode};
use crate::preprocess::Config;
use crate::util::chars::is_js_space;
use crate::util::ordered::OrderedMap;
use crate::util::value::Value;

pub mod commit_type {
    pub const NORMAL: u8 = 0;
    pub const REVERSE: u8 = 1;
    pub const HIGHLIGHT: u8 = 2;
    pub const MERGE: u8 = 3;
    pub const CHERRY_PICK: u8 = 4;
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitCommit {
    pub id: String,
    pub message: String,
    pub seq: usize,
    pub kind: u8,
    pub tags: Vec<String>,
    pub parents: Vec<String>,
    pub branch: String,
    pub custom_type: Option<u8>,
    pub custom_id: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GitModel {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub direction: Direction,
    pub commits: OrderedMap<GitCommit>,
    pub lanes: Vec<String>,
}

fn semantic(message: String) -> PelersError {
    let mut error = PelersError::new(message, PelersErrorCode::Semantic);
    error.diagram_type = Some("gitGraph".to_owned());
    error
}

fn type_code(name: CommitTypeName) -> u8 {
    match name {
        CommitTypeName::Normal => commit_type::NORMAL,
        CommitTypeName::Reverse => commit_type::REVERSE,
        CommitTypeName::Highlight => commit_type::HIGHLIGHT,
    }
}

fn non_empty(text: &str) -> Option<&str> {
    (!text.is_empty()).then_some(text)
}

struct Builder {
    commits: OrderedMap<GitCommit>,
    branches: OrderedMap<Option<String>>,
    orders: OrderedMap<f64>,
    head: Option<String>,
    current: String,
    direction: Direction,
    seq: usize,
}

impl Builder {
    fn next_id(&self) -> String {
        let mut salt = self.seq as u64 + 1;
        loop {
            let hash = (salt as u32).wrapping_mul(0x9e37_79b1);
            let id = format!("{}-{}", self.seq, &format!("{hash:08x}")[..7]);
            if !self.commits.contains(&id) {
                return id;
            }
            salt += 0x9e37_79b1;
        }
    }

    fn branch_head(&self, branch: &str) -> Option<String> {
        self.branches.get(branch).cloned().flatten()
    }

    fn add(&mut self, commit: GitCommit) {
        self.head = Some(commit.id.clone());
        self.branches.set(&self.current, Some(commit.id.clone()));
        self.commits.set(&commit.id.clone(), commit);
    }

    fn checkout(&mut self, branch: &str) -> Result<(), PelersError> {
        if !self.branches.contains(branch) {
            return Err(semantic(format!(
                "Trying to checkout branch which is not yet created. (Help try using \"branch {branch}\")"
            )));
        }
        self.current = branch.to_owned();
        self.head = self
            .branch_head(branch)
            .filter(|id| self.commits.contains(id));
        Ok(())
    }

    fn commit(&mut self, message: &str, id: Option<&str>, kind: u8, tags: Vec<String>) {
        let id = match id.and_then(non_empty) {
            Some(id) => id.to_owned(),
            None => self.next_id(),
        };
        let commit = GitCommit {
            id,
            message: message.to_owned(),
            seq: self.seq,
            kind,
            tags,
            parents: self.head.iter().cloned().collect(),
            branch: self.current.clone(),
            custom_type: None,
            custom_id: false,
        };
        self.seq += 1;
        self.add(commit);
    }

    fn branch(&mut self, name: &str, order: f64) -> Result<(), PelersError> {
        if self.branches.contains(name) {
            return Err(semantic(format!(
                "Trying to create an existing branch. (Help: Either use a new name if you want create a new branch or try using \"checkout {name}\")"
            )));
        }
        let head = self.head.clone();
        self.branches.set(name, head);
        self.orders.set(name, order);
        self.checkout(name)
    }

    fn merge(
        &mut self,
        other: &str,
        custom_id: Option<&str>,
        custom_type: Option<u8>,
        tags: Vec<String>,
    ) -> Result<(), PelersError> {
        let current_head = self.branch_head(&self.current);
        let other_head = self.branch_head(other);
        let current_commit = current_head.as_deref().and_then(|id| self.commits.get(id));
        let other_commit = other_head.as_deref().and_then(|id| self.commits.get(id));
        if let (Some(current), Some(_theirs)) = (current_commit, other_commit)
            && current.branch == other
        {
            return Err(semantic(format!(
                "Cannot merge branch '{other}' into itself."
            )));
        }
        if self.current == other {
            return Err(semantic(
                "Incorrect usage of \"merge\". Cannot merge a branch to itself".to_owned(),
            ));
        }
        let Some(current) = current_commit else {
            return Err(semantic(format!(
                "Incorrect usage of \"merge\". Current branch ({})has no commits",
                self.current
            )));
        };
        if !self.branches.contains(other) {
            return Err(semantic(format!(
                "Incorrect usage of \"merge\". Branch to be merged ({other}) does not exist"
            )));
        }
        let Some(theirs) = other_commit else {
            return Err(semantic(format!(
                "Incorrect usage of \"merge\". Branch to be merged ({other}) has no commits"
            )));
        };
        if current.id == theirs.id {
            return Err(semantic(
                "Incorrect usage of \"merge\". Both branches have same head".to_owned(),
            ));
        }
        if let Some(id) = custom_id.and_then(non_empty)
            && self.commits.contains(id)
        {
            return Err(semantic(format!(
                "Incorrect usage of \"merge\". Commit with id:{id} already exists, use different custom id"
            )));
        }
        let other_id = other_head.unwrap_or_default();
        let parents = match &self.head {
            Some(head) => vec![head.clone(), other_id],
            None => Vec::new(),
        };
        let id = match custom_id.and_then(non_empty) {
            Some(id) => id.to_owned(),
            None => self.next_id(),
        };
        let commit = GitCommit {
            id,
            message: format!("merged branch {other} into {}", self.current),
            seq: self.seq,
            kind: commit_type::MERGE,
            tags,
            parents,
            branch: self.current.clone(),
            custom_type,
            custom_id: custom_id.and_then(non_empty).is_some(),
        };
        self.seq += 1;
        self.add(commit);
        Ok(())
    }

    fn cherry_pick(
        &mut self,
        source_id: Option<&str>,
        tags: Option<Vec<String>>,
        parent: Option<&str>,
    ) -> Result<(), PelersError> {
        let parent = parent.and_then(non_empty);
        let source = source_id
            .and_then(non_empty)
            .and_then(|id| self.commits.get(id))
            .cloned();
        let Some(source) = source else {
            return Err(semantic(
                "Incorrect usage of \"cherryPick\". Source commit id should exist and provided"
                    .to_owned(),
            ));
        };
        if let Some(parent) = parent
            && !source.parents.iter().any(|p| p == parent)
        {
            return Err(semantic(
                "Invalid operation: The specified parent commit is not an immediate parent of the cherry-picked commit."
                    .to_owned(),
            ));
        }
        if source.kind == commit_type::MERGE && parent.is_none() {
            return Err(semantic(
                "Incorrect usage of cherry-pick: If the source commit is a merge commit, an immediate parent commit must be specified."
                    .to_owned(),
            ));
        }
        if source.branch == self.current {
            return Err(semantic(
                "Incorrect usage of \"cherryPick\". Source commit is already on current branch"
                    .to_owned(),
            ));
        }
        let current_head = self.branch_head(&self.current);
        if !current_head
            .as_deref()
            .is_some_and(|id| self.commits.contains(id))
        {
            return Err(semantic(format!(
                "Incorrect usage of \"cherry-pick\". Current branch ({})has no commits",
                self.current
            )));
        }
        let mut parents: Vec<String> = Vec::new();
        if let Some(head) = &self.head {
            parents.push(head.clone());
            parents.push(source.id.clone());
        }
        let tags = match tags {
            Some(tags) => tags.into_iter().filter(|t| !t.is_empty()).collect(),
            None => {
                let suffix = match (source.kind == commit_type::MERGE, parent) {
                    (true, Some(parent)) => format!("|parent:{parent}"),
                    _ => String::new(),
                };
                vec![format!("cherry-pick:{}{suffix}", source.id)]
            }
        };
        let commit = GitCommit {
            id: self.next_id(),
            message: format!("cherry-picked {} into {}", source.message, self.current),
            seq: self.seq,
            kind: commit_type::CHERRY_PICK,
            tags,
            parents,
            branch: self.current.clone(),
            custom_type: None,
            custom_id: false,
        };
        self.seq += 1;
        self.add(commit);
        Ok(())
    }
}

fn collapse_indented_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' && chars.peek().is_some_and(|&n| is_js_space(n)) {
            out.push('\n');
            while chars.peek().is_some_and(|&n| is_js_space(n)) {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn build_git(
    ast: &GitGraphAst,
    config: &Config,
    title: Option<&str>,
) -> Result<GitModel, PelersError> {
    let git = config.get("gitGraph").and_then(Value::as_object);
    let main_name = git
        .and_then(|section| section.get("mainBranchName"))
        .and_then(Value::as_str)
        .unwrap_or("main")
        .to_owned();
    let main_order = match git.and_then(|section| section.get("mainBranchOrder")) {
        Some(Value::Number(order)) => *order,
        _ => 0.0,
    };

    let mut builder = Builder {
        commits: OrderedMap::default(),
        branches: OrderedMap::default(),
        orders: OrderedMap::default(),
        head: None,
        current: main_name.clone(),
        direction: Direction::Lr,
        seq: 0,
    };
    builder.branches.set(&main_name, None);
    builder.orders.set(&main_name, main_order);
    if let Some(dir) = ast.dir {
        builder.direction = dir;
    }
    for statement in &ast.statements {
        match statement {
            StatementAst::Commit(node) => {
                let kind = node.commit_type.map_or(commit_type::NORMAL, type_code);
                let message = node.message.as_deref().unwrap_or("");
                builder.commit(message, node.id.as_deref(), kind, node.tags.clone());
            }
            StatementAst::Branch(node) => {
                builder.branch(&node.name, node.order.unwrap_or(0.0))?;
            }
            StatementAst::Merge(node) => {
                builder.merge(
                    &node.branch,
                    Some(node.id.as_deref().unwrap_or("")),
                    node.commit_type.map(type_code),
                    node.tags.clone(),
                )?;
            }
            StatementAst::Checkout(node) => builder.checkout(&node.branch)?,
            StatementAst::CherryPicking(node) => {
                let tags = (!node.tags.is_empty()).then(|| node.tags.clone());
                builder.cherry_pick(node.id.as_deref(), tags, node.parent.as_deref())?;
            }
        }
    }

    let mut ordered: Vec<(String, f64)> = builder
        .orders
        .iter()
        .map(|(name, order)| (name.to_owned(), *order))
        .collect();
    ordered.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));

    let title = match &ast.title {
        Some(title) if !title.is_empty() => Some(title.clone()),
        _ => title.map(str::to_owned),
    };
    Ok(GitModel {
        title,
        acc_title: ast
            .acc_title
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| t.trim_start_matches(is_js_space).to_owned()),
        acc_descr: ast
            .acc_descr
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(collapse_indented_lines),
        direction: builder.direction,
        commits: builder.commits,
        lanes: ordered.into_iter().map(|(name, _)| name).collect(),
    })
}
