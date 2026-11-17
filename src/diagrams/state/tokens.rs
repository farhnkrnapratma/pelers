pub const END: usize = 0;
pub const ERROR: usize = 1;
pub const NL: usize = 2;
pub const SD: usize = 3;
pub const DESCR: usize = 4;
pub const ARROW: usize = 5;
pub const HIDE_EMPTY: usize = 6;
pub const SCALE: usize = 7;
pub const WIDTH: usize = 8;
pub const COMPOSIT_STATE: usize = 9;
pub const STRUCT_START: usize = 10;
pub const STRUCT_STOP: usize = 11;
pub const STATE_DESCR: usize = 12;
pub const AS: usize = 13;
pub const ID: usize = 14;
pub const FORK: usize = 15;
pub const JOIN: usize = 16;
pub const CHOICE: usize = 17;
pub const CONCURRENT: usize = 18;
pub const NOTE: usize = 19;
pub const NOTE_TEXT: usize = 20;
pub const ACC_TITLE: usize = 21;
pub const ACC_TITLE_VALUE: usize = 22;
pub const ACC_DESCR: usize = 23;
pub const ACC_DESCR_VALUE: usize = 24;
pub const ACC_DESCR_MULTILINE_VALUE: usize = 25;
pub const CLICK: usize = 26;
pub const STRING: usize = 27;
pub const HREF: usize = 28;
pub const CLASSDEF: usize = 29;
pub const CLASSDEF_ID: usize = 30;
pub const DEFAULT_CLASSDEF_ID: usize = 31;
pub const CLASSDEF_STYLEOPTS: usize = 32;
pub const DEFAULT: usize = 33;
pub const STYLE: usize = 34;
pub const STYLE_IDS: usize = 35;
pub const STYLEDEF_STYLEOPTS: usize = 36;
pub const CLASS: usize = 37;
pub const CLASSENTITY_IDS: usize = 38;
pub const STYLECLASS: usize = 39;
pub const DIRECTION_TB: usize = 40;
pub const DIRECTION_LR: usize = 43;
pub const EDGE_STATE: usize = 44;
pub const STYLE_SEPARATOR: usize = 45;
pub const LEFT_OF: usize = 46;
pub const RIGHT_OF: usize = 47;
pub const INVALID: usize = 48;

pub const TOKEN_NAMES: [&str; 49] = [
    "$end",
    "INVALID",
    "NL",
    "SD",
    "DESCR",
    "-->",
    "HIDE_EMPTY",
    "scale",
    "WIDTH",
    "COMPOSIT_STATE",
    "STRUCT_START",
    "STRUCT_STOP",
    "STATE_DESCR",
    "AS",
    "ID",
    "FORK",
    "JOIN",
    "CHOICE",
    "CONCURRENT",
    "note",
    "NOTE_TEXT",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "CLICK",
    "STRING",
    "HREF",
    "classDef",
    "CLASSDEF_ID",
    "DEFAULT_CLASSDEF_ID",
    "CLASSDEF_STYLEOPTS",
    "DEFAULT",
    "style",
    "STYLE_IDS",
    "STYLEDEF_STYLEOPTS",
    "class",
    "CLASSENTITY_IDS",
    "STYLECLASS",
    "direction_tb",
    "direction_bt",
    "direction_rl",
    "direction_lr",
    "EDGE_STATE",
    "STYLE_SEPARATOR",
    "left_of",
    "right_of",
    "INVALID",
];
