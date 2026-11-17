pub const END: usize = 0;
pub const ERROR: usize = 1;
pub const CLASS_DIAGRAM: usize = 2;
pub const NEWLINE: usize = 3;
pub const EOF: usize = 4;
pub const SQS: usize = 5;
pub const STR: usize = 6;
pub const SQE: usize = 7;
pub const DOT: usize = 8;
pub const GENERICTYPE: usize = 9;
pub const LABEL: usize = 10;
pub const ACC_TITLE: usize = 11;
pub const ACC_TITLE_VALUE: usize = 12;
pub const ACC_DESCR: usize = 13;
pub const ACC_DESCR_VALUE: usize = 14;
pub const ACC_DESCR_MULTILINE_VALUE: usize = 15;
pub const STRUCT_START: usize = 16;
pub const STRUCT_STOP: usize = 17;
pub const NAMESPACE: usize = 18;
pub const STYLE_SEPARATOR: usize = 19;
pub const ANNOTATION_START: usize = 20;
pub const ANNOTATION_END: usize = 21;
pub const CLASS: usize = 22;
pub const MEMBER: usize = 23;
pub const NOTE_FOR: usize = 24;
pub const NOTE: usize = 25;
pub const CLASSDEF: usize = 26;
pub const ALPHA: usize = 27;
pub const COMMA: usize = 28;
pub const DIRECTION_TB: usize = 29;
pub const DIRECTION_BT: usize = 30;
pub const DIRECTION_RL: usize = 31;
pub const DIRECTION_LR: usize = 32;
pub const AGGREGATION: usize = 33;
pub const EXTENSION: usize = 34;
pub const COMPOSITION: usize = 35;
pub const DEPENDENCY: usize = 36;
pub const LOLLIPOP: usize = 37;
pub const LINE: usize = 38;
pub const DOTTED_LINE: usize = 39;
pub const CALLBACK: usize = 40;
pub const LINK: usize = 41;
pub const LINK_TARGET: usize = 42;
pub const CLICK: usize = 43;
pub const CALLBACK_NAME: usize = 44;
pub const CALLBACK_ARGS: usize = 45;
pub const HREF: usize = 46;
pub const STYLE: usize = 47;
pub const CSSCLASS: usize = 48;
pub const COLON: usize = 49;
pub const BRKT: usize = 50;
pub const PCT: usize = 51;
pub const MINUS: usize = 52;
pub const UNICODE_TEXT: usize = 53;
pub const BQUOTE_STR: usize = 54;
pub const EDGE_STATE: usize = 55;
pub const EOF_IN_STRUCT: usize = 56;
pub const OPEN_IN_STRUCT: usize = 57;
pub const PLUS: usize = 58;
pub const EQUALS: usize = 59;
pub const PUNCTUATION: usize = 60;

pub const TOKEN_NAMES: [&str; 61] = [
    "$end",
    "INVALID",
    "CLASS_DIAGRAM",
    "NEWLINE",
    "EOF",
    "SQS",
    "STR",
    "SQE",
    "DOT",
    "GENERICTYPE",
    "LABEL",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "STRUCT_START",
    "STRUCT_STOP",
    "NAMESPACE",
    "STYLE_SEPARATOR",
    "ANNOTATION_START",
    "ANNOTATION_END",
    "CLASS",
    "MEMBER",
    "NOTE_FOR",
    "NOTE",
    "CLASSDEF",
    "ALPHA",
    "COMMA",
    "direction_tb",
    "direction_bt",
    "direction_rl",
    "direction_lr",
    "AGGREGATION",
    "EXTENSION",
    "COMPOSITION",
    "DEPENDENCY",
    "LOLLIPOP",
    "LINE",
    "DOTTED_LINE",
    "CALLBACK",
    "LINK",
    "LINK_TARGET",
    "CLICK",
    "CALLBACK_NAME",
    "CALLBACK_ARGS",
    "HREF",
    "STYLE",
    "CSSCLASS",
    "COLON",
    "BRKT",
    "PCT",
    "MINUS",
    "UNICODE_TEXT",
    "BQUOTE_STR",
    "EDGE_STATE",
    "EOF_IN_STRUCT",
    "OPEN_IN_STRUCT",
    "PLUS",
    "EQUALS",
    "PUNCTUATION",
];
