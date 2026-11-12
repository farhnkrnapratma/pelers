use crate::util::ordered::OrderedMap;
use crate::util::value::Value;

pub mod linetype {
    pub const SOLID: i32 = 0;
    pub const DOTTED: i32 = 1;
    pub const NOTE: i32 = 2;
    pub const SOLID_CROSS: i32 = 3;
    pub const DOTTED_CROSS: i32 = 4;
    pub const SOLID_OPEN: i32 = 5;
    pub const DOTTED_OPEN: i32 = 6;
    pub const LOOP_START: i32 = 10;
    pub const LOOP_END: i32 = 11;
    pub const ALT_START: i32 = 12;
    pub const ALT_ELSE: i32 = 13;
    pub const ALT_END: i32 = 14;
    pub const OPT_START: i32 = 15;
    pub const OPT_END: i32 = 16;
    pub const ACTIVE_START: i32 = 17;
    pub const ACTIVE_END: i32 = 18;
    pub const PAR_START: i32 = 19;
    pub const PAR_AND: i32 = 20;
    pub const PAR_END: i32 = 21;
    pub const RECT_START: i32 = 22;
    pub const RECT_END: i32 = 23;
    pub const SOLID_POINT: i32 = 24;
    pub const DOTTED_POINT: i32 = 25;
    pub const AUTONUMBER: i32 = 26;
    pub const CRITICAL_START: i32 = 27;
    pub const CRITICAL_OPTION: i32 = 28;
    pub const CRITICAL_END: i32 = 29;
    pub const BREAK_START: i32 = 30;
    pub const BREAK_END: i32 = 31;
    pub const PAR_OVER_START: i32 = 32;
    pub const BIDIRECTIONAL_SOLID: i32 = 33;
    pub const BIDIRECTIONAL_DOTTED: i32 = 34;
    pub const SOLID_TOP: i32 = 41;
    pub const SOLID_BOTTOM: i32 = 42;
    pub const STICK_TOP: i32 = 43;
    pub const STICK_BOTTOM: i32 = 44;
    pub const SOLID_ARROW_TOP_REVERSE: i32 = 45;
    pub const SOLID_ARROW_BOTTOM_REVERSE: i32 = 46;
    pub const STICK_ARROW_TOP_REVERSE: i32 = 47;
    pub const STICK_ARROW_BOTTOM_REVERSE: i32 = 48;
    pub const SOLID_TOP_DOTTED: i32 = 51;
    pub const SOLID_BOTTOM_DOTTED: i32 = 52;
    pub const STICK_TOP_DOTTED: i32 = 53;
    pub const STICK_BOTTOM_DOTTED: i32 = 54;
    pub const SOLID_ARROW_TOP_REVERSE_DOTTED: i32 = 55;
    pub const SOLID_ARROW_BOTTOM_REVERSE_DOTTED: i32 = 56;
    pub const STICK_ARROW_TOP_REVERSE_DOTTED: i32 = 57;
    pub const STICK_ARROW_BOTTOM_REVERSE_DOTTED: i32 = 58;
    pub const CENTRAL_CONNECTION: i32 = 59;
    pub const CENTRAL_CONNECTION_REVERSE: i32 = 60;
    pub const CENTRAL_CONNECTION_DUAL: i32 = 61;
}

pub mod placement {
    pub const LEFTOF: i32 = 0;
    pub const RIGHTOF: i32 = 1;
    pub const OVER: i32 = 2;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeqText {
    pub text: String,
    pub wrap: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeqBoxData {
    pub text: Option<String>,
    pub color: String,
    pub wrap: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeqBox {
    pub name: Option<String>,
    pub wrap: bool,
    pub fill: String,
    pub actor_keys: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeqActor {
    pub box_index: Option<usize>,
    pub name: String,
    pub description: String,
    pub wrap: bool,
    pub prev_actor: Option<String>,
    pub next_actor: Option<String>,
    pub links: OrderedMap<Value>,
    pub properties: OrderedMap<Value>,
    pub details: Option<String>,
    pub type_: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeqNumbering {
    pub start: Option<f64>,
    pub step: Option<f64>,
    pub visible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MessageBody {
    Text(String),
    Numbering(SeqNumbering),
}

impl Default for MessageBody {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SeqMessage {
    pub id: String,
    pub from: Option<String>,
    pub to: Option<String>,
    pub message: MessageBody,
    pub wrap: bool,
    pub type_: Option<i32>,
    pub activate: Option<bool>,
    pub placement: Option<i32>,
    pub central_connection: Option<i32>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Who {
    One(String),
    Many(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SeqNote {
    pub actor: Who,
    pub placement: i32,
    pub message: String,
    pub wrap: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Statement {
    pub type_: String,
    pub actor: Option<Who>,
    pub draw: Option<String>,
    pub description: Option<SeqText>,
    pub config: Option<String>,
    pub signal_type: Option<i32>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub msg: Option<SeqText>,
    pub activate: Option<bool>,
    pub central_connection: Option<i32>,
    pub placement: Option<i32>,
    pub text: Option<SeqText>,
    pub box_data: Option<SeqBoxData>,
    pub loop_text: Option<SeqText>,
    pub opt_text: Option<SeqText>,
    pub alt_text: Option<SeqText>,
    pub par_text: Option<SeqText>,
    pub critical_text: Option<SeqText>,
    pub option_text: Option<SeqText>,
    pub break_text: Option<SeqText>,
    pub color: Option<SeqText>,
    pub sequence_index: Option<f64>,
    pub sequence_index_step: Option<f64>,
    pub sequence_visible: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Statement(Box<Statement>),
    Text(String),
    List(Vec<Item>),
}
