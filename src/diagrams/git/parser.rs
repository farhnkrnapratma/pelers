use super::tokens::{GIT_TOKENS, kind as k};
use crate::diagrams::common::tokens::{
    Reader, acc_descr_value, acc_title_value, string_value, title_value, tokenize,
};
use crate::errors::PelersError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CommitTypeName {
    Normal,
    Reverse,
    Highlight,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Lr,
    Tb,
    Bt,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CommitAst {
    pub id: Option<String>,
    pub message: Option<String>,
    pub tags: Vec<String>,
    pub commit_type: Option<CommitTypeName>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BranchAst {
    pub name: String,
    pub order: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergeAst {
    pub branch: String,
    pub id: Option<String>,
    pub tags: Vec<String>,
    pub commit_type: Option<CommitTypeName>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CheckoutAst {
    pub branch: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CherryPickingAst {
    pub id: Option<String>,
    pub tags: Vec<String>,
    pub parent: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StatementAst {
    Commit(CommitAst),
    Branch(BranchAst),
    Merge(MergeAst),
    Checkout(CheckoutAst),
    CherryPicking(CherryPickingAst),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitGraphAst {
    pub dir: Option<Direction>,
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub statements: Vec<StatementAst>,
}

fn end_of_line(r: &mut Reader<'_>) -> Result<(), PelersError> {
    if r.kind() == -1 {
        return Ok(());
    }
    if r.kind() != k::NEWLINE {
        return Err(r.fail("a line break or the end of input"));
    }
    while r.kind() == k::NEWLINE {
        r.i += 1;
    }
    Ok(())
}

fn name(r: &mut Reader<'_>) -> Result<String, PelersError> {
    if r.kind() == k::REFERENCE {
        return Ok(r.take());
    }
    if r.kind() == k::STRING {
        return Ok(string_value(&r.take()));
    }
    Err(r.fail("a branch name"))
}

fn string(r: &mut Reader<'_>) -> Result<String, PelersError> {
    Ok(string_value(&r.expect(k::STRING)?))
}

fn commit_type(r: &mut Reader<'_>) -> Result<CommitTypeName, PelersError> {
    let kind = match r.kind() {
        k::NORMAL => CommitTypeName::Normal,
        k::REVERSE => CommitTypeName::Reverse,
        k::HIGHLIGHT => CommitTypeName::Highlight,
        _ => return Err(r.fail("NORMAL, REVERSE, or HIGHLIGHT")),
    };
    r.i += 1;
    Ok(kind)
}

fn parse_int(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(f64::NAN)
}

pub fn parse_git(src: &str) -> Result<GitGraphAst, PelersError> {
    let tokens = tokenize(src, GIT_TOKENS, "gitGraph")?;
    let mut r = Reader::new(tokens, GIT_TOKENS, "gitGraph");
    let mut ast = GitGraphAst::default();

    while r.kind() == k::NEWLINE {
        r.i += 1;
    }
    if !r.accept(k::GIT_GRAPH_COLON) {
        r.expect(k::GIT_GRAPH)?;
        let next = r.kind_at(r.i + 1);
        if matches!(r.kind(), k::LR | k::TB | k::BT) && next == k::COLON {
            ast.dir = Some(match r.kind() {
                k::LR => Direction::Lr,
                k::TB => Direction::Tb,
                _ => Direction::Bt,
            });
            r.i += 2;
        } else {
            r.accept(k::COLON);
        }
    }

    while r.kind() != -1 {
        match r.kind() {
            k::NEWLINE => r.i += 1,
            k::ACC_DESCR => {
                ast.acc_descr = Some(acc_descr_value(&r.take()));
                end_of_line(&mut r)?;
            }
            k::ACC_TITLE => {
                ast.acc_title = Some(acc_title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            k::TITLE => {
                ast.title = Some(title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            k::COMMIT => {
                r.i += 1;
                let mut node = CommitAst {
                    id: None,
                    message: None,
                    tags: Vec::new(),
                    commit_type: None,
                };
                loop {
                    if r.accept(k::ID) {
                        node.id = Some(string(&mut r)?);
                    } else if r.accept(k::TAG) {
                        node.tags.push(string(&mut r)?);
                    } else if r.accept(k::TYPE) {
                        node.commit_type = Some(commit_type(&mut r)?);
                    } else if r.accept(k::MSG) || r.kind_at(r.i) == k::STRING {
                        node.message = Some(string(&mut r)?);
                    } else {
                        break;
                    }
                }
                end_of_line(&mut r)?;
                ast.statements.push(StatementAst::Commit(node));
            }
            k::BRANCH => {
                r.i += 1;
                let mut node = BranchAst {
                    name: name(&mut r)?,
                    order: None,
                };
                if r.accept(k::ORDER) {
                    node.order = Some(parse_int(&r.expect(k::INT)?));
                }
                end_of_line(&mut r)?;
                ast.statements.push(StatementAst::Branch(node));
            }
            k::MERGE => {
                r.i += 1;
                let mut node = MergeAst {
                    branch: name(&mut r)?,
                    id: None,
                    tags: Vec::new(),
                    commit_type: None,
                };
                loop {
                    if r.accept(k::ID) {
                        node.id = Some(string(&mut r)?);
                    } else if r.accept(k::TAG) {
                        node.tags.push(string(&mut r)?);
                    } else if r.accept(k::TYPE) {
                        node.commit_type = Some(commit_type(&mut r)?);
                    } else {
                        break;
                    }
                }
                end_of_line(&mut r)?;
                ast.statements.push(StatementAst::Merge(node));
            }
            k::CHECKOUT | k::SWITCH => {
                r.i += 1;
                let node = CheckoutAst {
                    branch: name(&mut r)?,
                };
                end_of_line(&mut r)?;
                ast.statements.push(StatementAst::Checkout(node));
            }
            k::CHERRY_PICK => {
                r.i += 1;
                let mut node = CherryPickingAst {
                    id: None,
                    tags: Vec::new(),
                    parent: None,
                };
                loop {
                    if r.accept(k::ID) {
                        node.id = Some(string(&mut r)?);
                    } else if r.accept(k::TAG) {
                        node.tags.push(string(&mut r)?);
                    } else if r.accept(k::PARENT) {
                        node.parent = Some(string(&mut r)?);
                    } else {
                        break;
                    }
                }
                end_of_line(&mut r)?;
                ast.statements.push(StatementAst::CherryPicking(node));
            }
            _ => return Err(r.fail("a statement, a title, or a line break")),
        }
    }
    Ok(ast)
}
