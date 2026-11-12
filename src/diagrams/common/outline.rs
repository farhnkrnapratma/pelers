use crate::diagrams::common::scan::{space, word};
use crate::errors::{PelersError, lexical_error, syntax_error};
use crate::util::chars::utf16_to_byte;

pub const END: i32 = 0;
pub const ERROR: i32 = 1;
pub const SPACELINE: i32 = 2;
pub const NL: i32 = 3;
pub const EOF: i32 = 4;
pub const KEYWORD: i32 = 5;
pub const SPACELIST: i32 = 6;
pub const ICON: i32 = 7;
pub const CLASS: i32 = 8;
pub const NODE_DSTART: i32 = 9;
pub const NODE_ID: i32 = 10;
pub const NODE_DESCR: i32 = 11;
pub const NODE_DEND: i32 = 12;
pub const SHAPE_DATA: i32 = 13;

const NAMES: [&str; 14] = [
    "1",
    "INVALID",
    "SPACELINE",
    "NL",
    "EOF",
    "",
    "SPACELIST",
    "ICON",
    "CLASS",
    "NODE_DSTART",
    "NODE_ID",
    "NODE_DESCR",
    "NODE_DEND",
    "SHAPE_DATA",
];

fn token_name(kind: i32, kanban: bool) -> &'static str {
    match kind {
        KEYWORD if kanban => "KANBAN",
        KEYWORD => "MINDMAP",
        -1 => "undefined",
        _ => NAMES[kind as usize],
    }
}

pub const DEFAULT: i32 = 0;
pub const ROUNDED_RECT: i32 = 1;
pub const RECT: i32 = 2;
pub const CIRCLE: i32 = 3;
pub const CLOUD: i32 = 4;
pub const BANG: i32 = 5;
pub const HEXAGON: i32 = 6;

pub fn get_type(start: &str, end: &str) -> i32 {
    match start {
        "[" => RECT,
        "(" => {
            if end == ")" {
                ROUNDED_RECT
            } else {
                CLOUD
            }
        }
        "((" => CIRCLE,
        ")" => CLOUD,
        "))" => BANG,
        "{{" => HEXAGON,
        _ => DEFAULT,
    }
}

#[derive(Debug, Default)]
pub struct Tokens {
    pub types: Vec<i32>,
    pub texts: Vec<String>,
    pub starts: Vec<isize>,
}

#[derive(Default)]
pub struct Decoration {
    pub icon: Option<String>,
    pub class: Option<String>,
}

pub trait OutlineDb {
    fn add_node(
        &mut self,
        level: usize,
        id: &str,
        descr: &str,
        kind: i32,
        shape_data: Option<&str>,
    ) -> Result<(), PelersError>;
    fn decorate_node(&mut self, decoration: Decoration);
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Initial,
    Class,
    Icon,
    Node,
    Nstr,
    Nstr2,
    Data,
    DataStr,
}

#[derive(Clone, Copy)]
enum Rule {
    Space,
    Line,
    Id,
    IdKanban,
    Descr,
    Md,
    Data,
}

fn unit(src: &[u16], at: isize) -> Option<u16> {
    usize::try_from(at).ok().and_then(|i| src.get(i).copied())
}

fn is(src: &[u16], at: isize, value: u16) -> bool {
    unit(src, at) == Some(value)
}

fn letters(src: &[u16], at: isize, word: &str) -> bool {
    word.bytes()
        .enumerate()
        .all(|(k, c)| unit(src, at + k as isize).is_some_and(|u| (u | 32) == u16::from(c)))
}

fn sticky(src: &[u16], rule: Rule, from: isize) -> Option<isize> {
    let start = usize::try_from(from).unwrap_or(0);
    if start > src.len() {
        return None;
    }
    let mut e = start;
    while e < src.len() && matches_rule(rule, src[e]) {
        e += 1;
    }
    (e > start).then_some(e as isize)
}

fn matches_rule(rule: Rule, u: u16) -> bool {
    match rule {
        Rule::Space => space(u),
        Rule::Line => !matches!(u, 10 | 13 | 0x2028 | 0x2029),
        Rule::Id => !matches!(u, 40 | 91 | 10 | 41 | 123 | 125),
        Rule::IdKanban => !matches!(u, 40 | 91 | 10 | 41 | 123 | 125 | 64),
        Rule::Descr => !matches!(u, 41 | 93 | 40 | 125),
        Rule::Md => !matches!(u, 96 | 34),
        Rule::Data => !matches!(u, 125 | 94 | 34),
    }
}

fn js_slice(src: &[u16], from: isize, to: isize) -> String {
    let n = src.len() as isize;
    let clamp = |x: isize| if x < 0 { (n + x).max(0) } else { x.min(n) };
    let (a, b) = (clamp(from), clamp(to));
    if a >= b {
        String::new()
    } else {
        String::from_utf16_lossy(&src[a as usize..b as usize])
    }
}

struct Lexer<'a> {
    src: &'a [u16],
    out: Tokens,
    p: isize,
    state: State,
}

impl Lexer<'_> {
    fn put(&mut self, kind: i32, text: String, end: isize) {
        self.out.types.push(kind);
        self.out.texts.push(text);
        self.out.starts.push(self.p);
        self.p = end;
    }

    fn emit(&mut self, kind: i32, end: isize, next: State) {
        let text = js_slice(self.src, self.p, end);
        self.put(kind, text, end);
        self.state = next;
    }

    fn up_to(&self, value: u16) -> isize {
        let from = usize::try_from(self.p).unwrap_or(0);
        self.src
            .get(from..)
            .and_then(|rest| rest.iter().position(|&u| u == value))
            .map_or(self.src.len() as isize, |at| (from + at) as isize)
    }
}

pub fn tokenize(source: &str, kanban: bool) -> Tokens {
    let src: Vec<u16> = source.encode_utf16().collect();
    let n = src.len() as isize;
    let keyword = if kanban { "kanban" } else { "mindmap" };
    let id_rule = if kanban { Rule::IdKanban } else { Rule::Id };
    let mut lx = Lexer {
        src: &src,
        out: Tokens::default(),
        p: 0,
        state: State::Initial,
    };
    let mut next_break: isize = -1;

    while lx.p < n {
        let p = lx.p;
        let c = unit(&src, p);
        let d = unit(&src, p + 1);
        let cq = |v: u16| c == Some(v);
        let dq = |v: u16| d == Some(v);
        let keep_going = match lx.state {
            State::Initial => {
                let w = if c.is_some_and(|u| u <= 32 || u >= 160) {
                    sticky(&src, Rule::Space, p).unwrap_or(p)
                } else {
                    p
                };
                if kanban && cq(64) && dq(123) {
                    lx.put(SHAPE_DATA, String::new(), p + 2);
                    lx.state = State::Data;
                } else if is(&src, w, 37) && is(&src, w + 1, 37) {
                    let end = sticky(&src, Rule::Line, w).unwrap_or(w);
                    lx.emit(SPACELINE, end, State::Initial);
                } else if w > p {
                    if next_break < p {
                        next_break = lx.up_to(10);
                    }
                    if next_break >= w {
                        lx.emit(SPACELIST, w, State::Initial);
                    } else {
                        let last = src[..w as usize].iter().rposition(|&u| u == 10);
                        match last {
                            Some(e) if e as isize > p => {
                                lx.emit(SPACELINE, e as isize + 1, State::Initial)
                            }
                            _ => lx.emit(NL, p + 1, State::Initial),
                        }
                    }
                } else if letters(&src, p, keyword)
                    && !word(unit(&src, p + keyword.len() as isize).as_ref())
                {
                    lx.emit(KEYWORD, p + keyword.len() as isize, State::Initial);
                } else if cq(58) && dq(58) && is(&src, p + 2, 58) {
                    lx.p += 3;
                    lx.state = State::Class;
                } else if cq(58) && dq(58) && letters(&src, p + 2, "icon") && is(&src, p + 6, 40) {
                    lx.p += 7;
                    lx.state = State::Icon;
                } else if (cq(45) && dq(41))
                    || (cq(40) && (dq(45) || dq(40)))
                    || ((cq(41) || cq(123)) && d.is_some() && d == c)
                {
                    lx.emit(NODE_DSTART, p + 2, State::Node);
                } else if cq(41) || cq(40) || cq(91) {
                    lx.emit(NODE_DSTART, p + 1, State::Node);
                } else if let Some(e) = sticky(&src, id_rule, p) {
                    lx.emit(NODE_ID, e, State::Initial);
                } else {
                    break;
                }
                true
            }
            State::Node => {
                if cq(34) {
                    lx.state = if dq(96) { State::Nstr2 } else { State::Nstr };
                    lx.p += if dq(96) { 2 } else { 1 };
                } else if cq(41) || cq(40) {
                    let end = if (d.is_some() && d == c) || (cq(40) && dq(45)) {
                        p + 2
                    } else {
                        p + 1
                    };
                    lx.emit(NODE_DEND, end, State::Initial);
                } else if cq(93) {
                    lx.emit(NODE_DEND, p + 1, State::Initial);
                } else if (cq(125) && dq(125)) || (cq(45) && dq(41)) {
                    lx.emit(NODE_DEND, p + 2, State::Initial);
                } else {
                    let rule = if cq(125) { Rule::Line } else { Rule::Descr };
                    let end = sticky(&src, rule, p).unwrap_or(-1);
                    lx.emit(NODE_DESCR, end, State::Node);
                }
                true
            }
            State::Nstr => {
                if cq(34) {
                    lx.p += 1;
                    lx.state = State::Node;
                } else {
                    let end = lx.up_to(34);
                    lx.emit(NODE_DESCR, end, State::Nstr);
                }
                true
            }
            State::Nstr2 => {
                if cq(96) && dq(34) {
                    lx.p += 2;
                    lx.state = State::Node;
                    true
                } else if let Some(e) = sticky(&src, Rule::Md, p) {
                    lx.emit(NODE_DESCR, e, State::Nstr2);
                    true
                } else {
                    false
                }
            }
            State::Class => {
                if let Some(e) = sticky(&src, Rule::Line, p) {
                    lx.emit(CLASS, e, State::Initial);
                    true
                } else if cq(10) {
                    lx.p += 1;
                    lx.state = State::Initial;
                    true
                } else {
                    false
                }
            }
            State::Icon => {
                if cq(41) {
                    lx.p += 1;
                    lx.state = State::Initial;
                } else {
                    let end = lx.up_to(41);
                    lx.emit(ICON, end, State::Icon);
                }
                true
            }
            State::Data => {
                if cq(34) {
                    lx.emit(SHAPE_DATA, p + 1, State::DataStr);
                    true
                } else if cq(125) {
                    lx.p += 1;
                    lx.state = State::Initial;
                    true
                } else if let Some(e) = sticky(&src, Rule::Data, p) {
                    lx.emit(SHAPE_DATA, e, State::Data);
                    true
                } else {
                    false
                }
            }
            State::DataStr => {
                if cq(34) {
                    lx.emit(SHAPE_DATA, p + 1, State::Data);
                } else {
                    let end = lx.up_to(34);
                    let text = break_runs(&js_slice(&src, p, end));
                    lx.put(SHAPE_DATA, text, end);
                }
                true
            }
        };
        if !keep_going || lx.p == p {
            break;
        }
    }

    if lx.p < n {
        let p = lx.p;
        lx.put(ERROR, String::new(), p);
    } else {
        if lx.state == State::Initial {
            lx.put(EOF, String::new(), n);
        }
        lx.put(END, String::new(), n);
    }
    lx.out
}

fn break_runs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for c in text.chars() {
        if skipping && crate::util::chars::is_js_space(c) {
            continue;
        }
        skipping = c == '\n';
        if skipping {
            out.push_str("<br/>");
        } else {
            out.push(c);
        }
    }
    out
}

fn is_stop(kind: i32) -> bool {
    kind == SPACELINE || kind == NL || kind == EOF
}

fn starts_node(kind: i32) -> bool {
    matches!(
        kind,
        SPACELINE | SPACELIST | ICON | CLASS | NODE_DSTART | NODE_ID
    )
}

const STOP: &str = "'SPACELINE', 'NL', 'EOF'";

pub fn parse_outline<D: OutlineDb>(
    source: &str,
    db: &mut D,
    kanban: bool,
) -> Result<(), PelersError> {
    let diagram = if kanban { "kanban" } else { "mindmap" };
    let Tokens {
        types,
        texts,
        starts,
    } = tokenize(source, kanban);
    let kind = |i: usize| types.get(i).copied().unwrap_or(-1);
    let text = |i: usize| texts.get(i).cloned().unwrap_or_default();
    let data = if kanban { ", 'SHAPE_DATA'" } else { "" };
    let fail = |i: usize, expected: &str| {
        let offset = utf16_to_byte(
            source,
            usize::try_from(starts.get(i).copied().unwrap_or(0)).unwrap_or(0),
        );
        if kind(i) == ERROR {
            return lexical_error(diagram, source, offset);
        }
        syntax_error(
            diagram,
            source,
            offset,
            format!(
                "Expecting {expected}, got '{}'",
                token_name(kind(i), kanban)
            ),
        )
    };

    let keyword = format!("'{}'", token_name(KEYWORD, kanban));
    let mut i = 0usize;
    if kind(i) == SPACELINE {
        i += 1;
        while kind(i) == SPACELINE || kind(i) == NL {
            i += 1;
        }
        if kind(i) != KEYWORD {
            return Err(fail(i, &format!("'SPACELINE', 'NL', {keyword}")));
        }
    } else if kind(i) != KEYWORD {
        return Err(fail(i, &format!("'SPACELINE', {keyword}")));
    }
    i += 1;
    let mut first = "'SPACELINE', 'NL', 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'";
    if kind(i) == NL {
        i += 1;
        first = "'SPACELINE', 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'";
    }

    loop {
        let mut t = kind(i);
        let mut level = 0usize;
        let indented = t == SPACELIST;
        if indented {
            level = text(i).encode_utf16().count();
            i += 1;
            t = kind(i);
        }
        if t == NODE_ID || t == NODE_DSTART {
            let mut id: Option<String> = None;
            if t == NODE_ID {
                id = Some(text(i));
                i += 1;
            }
            let descr: String;
            let mut node_type = DEFAULT;
            if kind(i) == NODE_DSTART {
                let start = text(i);
                i += 1;
                if kind(i) != NODE_DESCR {
                    return Err(fail(i, "'NODE_DESCR'"));
                }
                descr = text(i);
                i += 1;
                if kind(i) != NODE_DEND {
                    return Err(fail(i, "'NODE_DEND'"));
                }
                let end = text(i);
                i += 1;
                if !is_stop(kind(i)) && kind(i) != SHAPE_DATA {
                    return Err(fail(i, &format!("{STOP}{data}")));
                }
                node_type = get_type(&start, &end);
            } else {
                if !is_stop(kind(i)) && kind(i) != SHAPE_DATA {
                    return Err(fail(i, &format!("{STOP}, 'NODE_DSTART'{data}")));
                }
                descr = id.clone().unwrap_or_default();
            }
            let node_id = id.unwrap_or_else(|| descr.clone());
            if kind(i) == SHAPE_DATA {
                let mut shape_data = text(i);
                i += 1;
                while kind(i) == SHAPE_DATA {
                    shape_data.push_str(&text(i));
                    i += 1;
                }
                if !is_stop(kind(i)) {
                    return Err(fail(i, &format!("{STOP}{data}")));
                }
                db.add_node(level, &node_id, &descr, node_type, Some(&shape_data))?;
            } else {
                db.add_node(level, &node_id, &descr, node_type, None)?;
            }
        } else if t == ICON || t == CLASS {
            let value = text(i);
            i += 1;
            if !is_stop(kind(i)) {
                return Err(fail(i, STOP));
            }
            db.decorate_node(if t == ICON {
                Decoration {
                    icon: Some(value),
                    class: None,
                }
            } else {
                Decoration {
                    icon: None,
                    class: Some(value),
                }
            });
        } else if indented {
            if !is_stop(t) {
                return Err(fail(
                    i,
                    &format!("{STOP}, 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'"),
                ));
            }
        } else if t == SPACELINE {
            i += 1;
            if !is_stop(kind(i)) {
                return Err(fail(i, STOP));
            }
        } else {
            return Err(fail(i, first));
        }

        i += 1;
        while kind(i) == NL || kind(i) == EOF {
            i += 1;
        }
        if kind(i) == END {
            return Ok(());
        }
        if !starts_node(kind(i)) {
            return Err(fail(
                i,
                &format!("{STOP}, 'SPACELIST', 'ICON', 'CLASS', 'NODE_DSTART', 'NODE_ID'"),
            ));
        }
    }
}
