use crate::diagrams::common::tokens::{
    ACC_DESCR, ACC_TITLE, DIRECTIVE, NEWLINE, Pattern, Reader, SINGLE_LINE_COMMENT, STRING, TITLE,
    TokenType, WHITESPACE, YAML, acc_descr_value, acc_title_value, string_value, title_value,
    tokenize,
};
use crate::errors::PelersError;
use crate::util::jsnum::js_parse_float;

pub mod kind {
    pub const LBRACKET: i32 = 0;
    pub const COMMA: i32 = 1;
    pub const RBRACKET: i32 = 2;
    pub const AT: i32 = 3;
    pub const SLASH: i32 = 4;
    pub const LPAREN: i32 = 5;
    pub const RPAREN: i32 = 6;
    pub const MINUS: i32 = 7;
    pub const LBRACE: i32 = 8;
    pub const RBRACE: i32 = 9;
    pub const NUMBER: i32 = 10;
    pub const ARROW: i32 = 11;
    pub const LINK_PORT: i32 = 12;
    pub const LINK_ARROW: i32 = 13;
    pub const LINK_LABEL: i32 = 14;
    pub const STRATEGY: i32 = 15;
    pub const WARDLEY: i32 = 16;
    pub const SIZE: i32 = 17;
    pub const EVOLUTION: i32 = 18;
    pub const ANCHOR: i32 = 19;
    pub const COMPONENT: i32 = 20;
    pub const LABEL: i32 = 21;
    pub const INERTIA: i32 = 22;
    pub const EVOLVE: i32 = 23;
    pub const PIPELINE: i32 = 24;
    pub const NOTE: i32 = 25;
    pub const ANNOTATIONS: i32 = 26;
    pub const ANNOTATION: i32 = 27;
    pub const ACCELERATOR: i32 = 28;
    pub const DEACCELERATOR: i32 = 29;
    pub const NAME: i32 = 30;
    pub const ACC_DESCR: i32 = 32;
    pub const ACC_TITLE: i32 = 33;
    pub const TITLE: i32 = 34;
    pub const INT: i32 = 35;
    pub const STRING: i32 = 36;
    pub const ID: i32 = 37;
    pub const NEWLINE: i32 = 38;
}

fn digits(src: &[u8], from: usize) -> usize {
    let mut i = from;
    while i < src.len() && src[i].is_ascii_digit() {
        i += 1;
    }
    i
}

fn number(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    let whole = digits(bytes, at);
    if whole == at || bytes.get(whole) != Some(&b'.') {
        return None;
    }
    let fraction = digits(bytes, whole + 1);
    (fraction > whole + 1).then_some(fraction)
}

fn link_arrow(src: &str, at: usize) -> Option<usize> {
    let rest = &src[at..];
    if rest.starts_with("-->") {
        return Some(at + 3);
    }
    if rest.starts_with("-.->") {
        return Some(at + 4);
    }
    if rest.starts_with('>') {
        return Some(at + 1);
    }
    let quoted = rest.strip_prefix("+'")?;
    let close = quoted.find('\'')?;
    let after = at + 2 + close + 1;
    let tail = &src[after..];
    for suffix in ["<>", "<", ">"] {
        if tail.starts_with(suffix) {
            return Some(after + suffix.len());
        }
    }
    None
}

fn link_port(src: &str, at: usize) -> Option<usize> {
    let rest = &src[at..];
    for text in ["+<>", "+>", "+<"] {
        if rest.starts_with(text) {
            return Some(at + text.len());
        }
    }
    None
}

fn link_label(src: &str, at: usize) -> Option<usize> {
    let rest = src[at..].strip_prefix(';')?;
    let run = rest.find(['\n', '\r']).unwrap_or(rest.len());
    (run > 0).then_some(at + 1 + run)
}

fn strategy(src: &str, at: usize) -> Option<usize> {
    for text in ["build", "buy", "outsource", "market"] {
        if src[at..].starts_with(text) {
            return Some(at + text.len());
        }
    }
    None
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')' | '&')
}

fn name_word(src: &str, from: usize, first_ok: fn(char) -> bool) -> Option<usize> {
    let rest = &src[from..];
    let first = rest.chars().next()?;
    if !first_ok(first) {
        return None;
    }
    let mut end = from + first.len_utf8();
    let bytes = src.as_bytes();
    while let Some(c) = src[end..].chars().next() {
        if name_char(c) {
            end += c.len_utf8();
        } else if c == '-' && bytes.get(end + 1) != Some(&b'>') {
            end += 1;
        } else {
            break;
        }
    }
    Some(end)
}

fn name_with_spaces(src: &str, at: usize) -> Option<usize> {
    let rest = &src[at..];
    let blocked = rest.strip_prefix("title").is_some_and(|after| {
        after
            .chars()
            .next()
            .is_some_and(crate::util::chars::is_js_space)
    }) || rest.starts_with("accTitle")
        || rest.starts_with("accDescr");
    if blocked {
        return None;
    }
    let mut end = name_word(src, at, is_name_start)?;
    loop {
        let bytes = src.as_bytes();
        let mut blanks = end;
        while blanks < bytes.len() && (bytes[blanks] == b' ' || bytes[blanks] == b'\t') {
            blanks += 1;
        }
        if blanks == end {
            break;
        }
        let Some(c) = src[blanks..].chars().next() else {
            break;
        };
        if !(c.is_ascii_alphabetic() || c == '(') {
            break;
        }
        match name_word(src, blanks, |c| c.is_ascii_alphabetic() || c == '(') {
            Some(next) => end = next,
            None => break,
        }
    }
    Some(end)
}

fn ws(src: &str, at: usize) -> Option<usize> {
    let run = src.as_bytes()[at..]
        .iter()
        .take_while(|&&b| b == b' ' || b == b'\t')
        .count();
    (run > 0).then_some(at + run)
}

fn integer(src: &str, at: usize) -> Option<usize> {
    let bytes = src.as_bytes();
    match bytes.get(at)? {
        b'0' => Some(at + 1),
        b'1'..=b'9' => {
            let mut max = at + 1;
            while bytes.get(max).is_some_and(u8::is_ascii_digit) {
                max += 1;
            }
            (at + 1..=max)
                .rev()
                .find(|&end| bytes.get(end) != Some(&b'.'))
        }
        _ => None,
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn id(src: &str, at: usize) -> Option<usize> {
    let first = src[at..].chars().next().filter(|&c| is_word(c))?;
    let base = at + first.len_utf8();
    let mut found = base;
    for (offset, c) in src[base..].char_indices() {
        if !(is_word(c) || c == '-') {
            break;
        }
        if is_word(c) {
            found = base + offset + c.len_utf8();
        }
    }
    Some(found)
}

const fn literal(text: &'static str) -> TokenType {
    TokenType::new(text, Pattern::Literal(text), None)
}

const fn word(name: &'static str, text: &'static str) -> TokenType {
    TokenType::new(name, Pattern::Literal(text), None)
}

const ARROW: TokenType = TokenType::new("ARROW", Pattern::Literal("->"), Some("-"));
const LINK_ARROW: TokenType = TokenType::new("LINK_ARROW", Pattern::Match(link_arrow), Some("->+"));

pub const WARDLEY_TOKENS: &[TokenType] = &[
    literal("["),
    literal(","),
    literal("]"),
    literal("@"),
    literal("/"),
    literal("("),
    literal(")"),
    TokenType {
        longer: &["ARROW", "LINK_ARROW", "YAML"],
        ..literal("-")
    },
    literal("{"),
    literal("}"),
    TokenType::new("WARDLEY_NUMBER", Pattern::Match(number), Some("0123456789")),
    ARROW,
    TokenType::new("LINK_PORT", Pattern::Match(link_port), Some("+")),
    LINK_ARROW,
    TokenType::new("LINK_LABEL", Pattern::Match(link_label), Some(";")),
    TokenType::new("STRATEGY", Pattern::Match(strategy), Some("bom")),
    word("KW_WARDLEY", "wardley-beta"),
    word("KW_SIZE", "size"),
    word("KW_EVOLUTION", "evolution"),
    word("KW_ANCHOR", "anchor"),
    word("KW_COMPONENT", "component"),
    word("KW_LABEL", "label"),
    word("KW_INERTIA", "inertia"),
    word("KW_EVOLVE", "evolve"),
    word("KW_PIPELINE", "pipeline"),
    word("KW_NOTE", "note"),
    word("KW_ANNOTATIONS", "annotations"),
    word("KW_ANNOTATION", "annotation"),
    word("KW_ACCELERATOR", "accelerator"),
    word("KW_DEACCELERATOR", "deaccelerator"),
    TokenType::new(
        "NAME_WITH_SPACES",
        Pattern::Match(name_with_spaces),
        Some("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"),
    ),
    TokenType {
        hidden: true,
        ..TokenType::new("WS", Pattern::Match(ws), Some("\t "))
    },
    ACC_DESCR,
    ACC_TITLE,
    TITLE,
    TokenType::new("INT", Pattern::Match(integer), Some("0123456789")),
    STRING,
    TokenType::new(
        "ID",
        Pattern::Match(id),
        Some("_0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"),
    ),
    NEWLINE,
    WHITESPACE,
    YAML,
    DIRECTIVE,
    SINGLE_LINE_COMMENT,
];

#[derive(Clone, Debug, PartialEq)]
pub struct LabelAst {
    pub neg_x: bool,
    pub offset_x: f64,
    pub neg_y: bool,
    pub offset_y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StageAst {
    pub name: String,
    pub boundary: Option<f64>,
    pub second_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecoratorAst {
    pub strategy: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentAst {
    pub name: String,
    pub visibility: f64,
    pub evolution: f64,
    pub label: Option<LabelAst>,
    pub decorator: Option<DecoratorAst>,
    pub inertia: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipelineComponentAst {
    pub name: String,
    pub evolution: f64,
    pub label: Option<LabelAst>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkAst {
    pub from: String,
    pub from_port: Option<String>,
    pub arrow: Option<String>,
    pub to: String,
    pub to_port: Option<String>,
    pub link_label: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnchorAst {
    pub name: String,
    pub visibility: f64,
    pub evolution: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoteAst {
    pub text: String,
    pub visibility: f64,
    pub evolution: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationAst {
    pub number: f64,
    pub x: f64,
    pub y: f64,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedAst {
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WardleyAst {
    pub title: Option<String>,
    pub acc_title: Option<String>,
    pub acc_descr: Option<String>,
    pub size: Option<(f64, f64)>,
    pub evolution: Option<Vec<StageAst>>,
    pub anchors: Vec<AnchorAst>,
    pub components: Vec<ComponentAst>,
    pub links: Vec<LinkAst>,
    pub evolves: Vec<(String, f64)>,
    pub pipelines: Vec<(String, Vec<PipelineComponentAst>)>,
    pub notes: Vec<NoteAst>,
    pub annotations: Vec<(f64, f64)>,
    pub annotation: Vec<AnnotationAst>,
    pub accelerators: Vec<PlacedAst>,
    pub deaccelerators: Vec<PlacedAst>,
}

fn is_name(k: i32) -> bool {
    k == kind::STRING || k == kind::ID || k == kind::NAME
}

pub fn parse_wardley(src: &str) -> Result<WardleyAst, PelersError> {
    let tokens = tokenize(src, WARDLEY_TOKENS, "wardley")?;
    let mut r = Reader::new(tokens, WARDLEY_TOKENS, "wardley");
    let mut ast = WardleyAst::default();

    fn read_name(r: &mut Reader<'_>) -> Result<String, PelersError> {
        if r.kind() == kind::STRING {
            return Ok(string_value(&r.take()));
        }
        if r.kind() == kind::ID || r.kind() == kind::NAME {
            return Ok(r.take());
        }
        Err(r.fail("a name"))
    }
    fn number(r: &mut Reader<'_>) -> Result<f64, PelersError> {
        Ok(js_parse_float(&r.expect(kind::NUMBER)?))
    }
    fn int(r: &mut Reader<'_>) -> Result<f64, PelersError> {
        Ok(js_parse_float(&r.expect(kind::INT)?))
    }
    fn coordinate(r: &mut Reader<'_>) -> Result<f64, PelersError> {
        if r.kind() == kind::NUMBER {
            return Ok(js_parse_float(&r.take()));
        }
        int(r)
    }
    fn end_of_line(r: &mut Reader<'_>) -> Result<(), PelersError> {
        if r.kind() == -1 {
            return Ok(());
        }
        if r.kind() != kind::NEWLINE {
            return Err(r.fail("a line break or the end of input"));
        }
        while r.kind() == kind::NEWLINE {
            r.i += 1;
        }
        Ok(())
    }
    fn pair(
        r: &mut Reader<'_>,
        read: fn(&mut Reader<'_>) -> Result<f64, PelersError>,
    ) -> Result<(f64, f64), PelersError> {
        r.expect(kind::LBRACKET)?;
        let first = read(r)?;
        r.expect(kind::COMMA)?;
        let second = read(r)?;
        r.expect(kind::RBRACKET)?;
        Ok((first, second))
    }
    fn label(r: &mut Reader<'_>) -> Result<LabelAst, PelersError> {
        r.expect(kind::LABEL)?;
        r.expect(kind::LBRACKET)?;
        let neg_x = r.accept(kind::MINUS);
        let offset_x = int(r)?;
        r.expect(kind::COMMA)?;
        let neg_y = r.accept(kind::MINUS);
        let offset_y = int(r)?;
        r.expect(kind::RBRACKET)?;
        Ok(LabelAst {
            neg_x,
            offset_x,
            neg_y,
            offset_y,
        })
    }
    fn stage(r: &mut Reader<'_>) -> Result<StageAst, PelersError> {
        let name = read_name(r)?;
        let mut out = StageAst {
            name,
            boundary: None,
            second_name: None,
        };
        if r.accept(kind::AT) {
            out.boundary = Some(number(r)?);
        }
        if r.accept(kind::SLASH) {
            out.second_name = Some(read_name(r)?);
        }
        Ok(out)
    }

    while r.kind() == kind::NEWLINE {
        r.i += 1;
    }
    r.expect(kind::WARDLEY)?;
    while r.kind() != -1 {
        match r.kind() {
            kind::NEWLINE => r.i += 1,
            kind::ACC_DESCR => {
                ast.acc_descr = Some(acc_descr_value(&r.take()));
                end_of_line(&mut r)?;
            }
            kind::ACC_TITLE => {
                ast.acc_title = Some(acc_title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            kind::TITLE => {
                ast.title = Some(title_value(&r.take()));
                end_of_line(&mut r)?;
            }
            kind::SIZE => {
                r.i += 1;
                let size = pair(&mut r, int)?;
                end_of_line(&mut r)?;
                ast.size = Some(size);
            }
            kind::EVOLUTION => {
                r.i += 1;
                let mut stages = vec![stage(&mut r)?];
                r.expect(kind::ARROW)?;
                stages.push(stage(&mut r)?);
                while r.accept(kind::ARROW) {
                    stages.push(stage(&mut r)?);
                }
                end_of_line(&mut r)?;
                ast.evolution = Some(stages);
            }
            kind::ANCHOR => {
                r.i += 1;
                let anchor = read_name(&mut r)?;
                let (visibility, evolution) = pair(&mut r, number)?;
                end_of_line(&mut r)?;
                ast.anchors.push(AnchorAst {
                    name: anchor,
                    visibility,
                    evolution,
                });
            }
            kind::COMPONENT => {
                r.i += 1;
                let name = read_name(&mut r)?;
                let (visibility, evolution) = pair(&mut r, number)?;
                let mut component = ComponentAst {
                    name,
                    visibility,
                    evolution,
                    label: None,
                    decorator: None,
                    inertia: false,
                };
                if r.kind() == kind::LABEL {
                    component.label = Some(label(&mut r)?);
                }
                if r.kind() == kind::LPAREN && r.kind_at(r.i + 1) == kind::STRATEGY {
                    r.i += 1;
                    component.decorator = Some(DecoratorAst { strategy: r.take() });
                    r.expect(kind::RPAREN)?;
                }
                if r.accept(kind::INERTIA) {
                    component.inertia = true;
                } else if r.accept(kind::LPAREN) {
                    r.expect(kind::INERTIA)?;
                    r.expect(kind::RPAREN)?;
                    component.inertia = true;
                }
                end_of_line(&mut r)?;
                ast.components.push(component);
            }
            kind::EVOLVE => {
                r.i += 1;
                let component = read_name(&mut r)?;
                let target = number(&mut r)?;
                end_of_line(&mut r)?;
                ast.evolves.push((component, target));
            }
            kind::PIPELINE => {
                r.i += 1;
                let parent = read_name(&mut r)?;
                r.expect(kind::LBRACE)?;
                r.expect(kind::NEWLINE)?;
                while r.kind() == kind::NEWLINE {
                    r.i += 1;
                }
                let mut components: Vec<PipelineComponentAst> = Vec::new();
                loop {
                    r.expect(kind::COMPONENT)?;
                    let name = read_name(&mut r)?;
                    r.expect(kind::LBRACKET)?;
                    let evolution = number(&mut r)?;
                    r.expect(kind::RBRACKET)?;
                    let mut component = PipelineComponentAst {
                        name,
                        evolution,
                        label: None,
                    };
                    if r.kind() == kind::LABEL {
                        component.label = Some(label(&mut r)?);
                    }
                    end_of_line(&mut r)?;
                    components.push(component);
                    if r.kind() != kind::COMPONENT {
                        break;
                    }
                }
                r.expect(kind::RBRACE)?;
                end_of_line(&mut r)?;
                ast.pipelines.push((parent, components));
            }
            kind::NOTE => {
                r.i += 1;
                let text = string_value(&r.expect(kind::STRING)?);
                let (visibility, evolution) = pair(&mut r, number)?;
                end_of_line(&mut r)?;
                ast.notes.push(NoteAst {
                    text,
                    visibility,
                    evolution,
                });
            }
            kind::ANNOTATIONS => {
                r.i += 1;
                let (x, y) = pair(&mut r, coordinate)?;
                end_of_line(&mut r)?;
                ast.annotations.push((x, y));
            }
            kind::ANNOTATION => {
                r.i += 1;
                let n = int(&mut r)?;
                r.expect(kind::COMMA)?;
                let (x, y) = pair(&mut r, coordinate)?;
                let text = string_value(&r.expect(kind::STRING)?);
                end_of_line(&mut r)?;
                ast.annotation.push(AnnotationAst {
                    number: n,
                    x,
                    y,
                    text,
                });
            }
            kind::ACCELERATOR | kind::DEACCELERATOR => {
                let slow = r.kind() == kind::DEACCELERATOR;
                r.i += 1;
                let force = read_name(&mut r)?;
                let (x, y) = pair(&mut r, number)?;
                end_of_line(&mut r)?;
                let placed = PlacedAst { name: force, x, y };
                if slow {
                    ast.deaccelerators.push(placed);
                } else {
                    ast.accelerators.push(placed);
                }
            }
            _ => {
                if !is_name(r.kind()) {
                    return Err(r.fail("a statement"));
                }
                let from = read_name(&mut r)?;
                let mut link = LinkAst {
                    from,
                    from_port: None,
                    arrow: None,
                    to: String::new(),
                    to_port: None,
                    link_label: None,
                };
                if r.kind() == kind::LINK_PORT {
                    link.from_port = Some(r.take());
                }
                if r.kind() == kind::LINK_ARROW || r.kind() == kind::ARROW {
                    link.arrow = Some(r.take());
                }
                link.to = read_name(&mut r)?;
                if r.kind() == kind::LINK_PORT {
                    link.to_port = Some(r.take());
                }
                if r.kind() == kind::LINK_LABEL {
                    let text = r.take();
                    link.link_label = Some(crate::util::chars::trim(&text[1..]).to_owned());
                }
                end_of_line(&mut r)?;
                ast.links.push(link);
            }
        }
    }
    Ok(ast)
}
