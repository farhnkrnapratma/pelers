pub mod components;
pub mod compound;
pub mod lanes;
pub mod layered;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Node,
    Dummy,
    Label,
    StartPort,
    EndPort,
}

#[derive(Clone, Debug)]
pub struct LNode {
    pub w: f64,
    pub h: f64,
    pub kind: Kind,
    pub pin: i32,
    pub hint: f64,
    pub key: f64,
    pub span: f64,
    pub x: f64,
    pub y: f64,
    pub rank: usize,
}

#[derive(Clone, Debug)]
pub struct LEdge {
    pub tail: usize,
    pub head: usize,
    pub minlen: f64,
    pub label_w: f64,
    pub label_h: f64,
    pub tail_dx: f64,
    pub head_dx: f64,
    pub tail_fixed: bool,
    pub head_fixed: bool,
    pub reversed: bool,
    pub points: Vec<f64>,
    pub label_x: f64,
    pub label_y: f64,
}

pub fn lnode(w: f64, h: f64, kind: Kind) -> LNode {
    LNode {
        w,
        h,
        kind,
        pin: 0,
        hint: f64::NAN,
        key: f64::NAN,
        span: 0.0,
        x: 0.0,
        y: 0.0,
        rank: 0,
    }
}

pub fn ledge(tail: usize, head: usize, minlen: f64, label_w: f64, label_h: f64) -> LEdge {
    LEdge {
        tail,
        head,
        minlen,
        label_w,
        label_h,
        tail_dx: 0.0,
        head_dx: 0.0,
        tail_fixed: false,
        head_fixed: false,
        reversed: false,
        points: Vec::new(),
        label_x: 0.0,
        label_y: 0.0,
    }
}

#[derive(Clone, Debug, Default)]
pub struct Groups {
    pub of: Vec<i32>,
    pub parent: Vec<i32>,
}

#[derive(Clone, Debug, Default)]
pub struct LayeredOptions {
    pub node_sep: f64,
    pub edge_sep: f64,
    pub rank_sep: f64,
    pub port_sep: f64,
    pub tracks: bool,
    pub straight: bool,
    pub apart: bool,
    pub head_room: f64,
    pub groups: Option<Groups>,
    pub order_only: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayeredResult {
    pub width: f64,
    pub height: f64,
    pub crossings: Option<i32>,
    pub declared: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    TB,
    BT,
    LR,
    RL,
}

pub fn direction(dir: Option<&str>) -> Dir {
    match dir {
        Some("BT") => Dir::BT,
        Some("LR") => Dir::LR,
        Some("RL") => Dir::RL,
        _ => Dir::TB,
    }
}

#[derive(Clone, Debug)]
pub struct CNode {
    pub w: f64,
    pub h: f64,
    pub parent: i32,
    pub is_group: bool,
    pub dir: Option<Dir>,
    pub pad_x: f64,
    pub pad_top: f64,
    pub pad_bottom: f64,
    pub min_w: f64,
    pub seq: f64,
    pub span: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug)]
pub struct CEdge {
    pub src: usize,
    pub dst: usize,
    pub minlen: f64,
    pub label_w: f64,
    pub label_h: f64,
    pub route: Vec<f64>,
    pub label_x: f64,
    pub label_y: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompoundResult {
    pub width: f64,
    pub height: f64,
}

pub fn cnode(w: f64, h: f64, parent: i32) -> CNode {
    CNode {
        w,
        h,
        parent,
        is_group: false,
        dir: None,
        pad_x: 0.0,
        pad_top: 0.0,
        pad_bottom: 0.0,
        min_w: 0.0,
        seq: 0.0,
        span: 0.0,
        x: 0.0,
        y: 0.0,
    }
}
