pub const END: i32 = 0;
pub const ERROR: i32 = 1;
pub const ER_DIAGRAM: i32 = 2;
pub const EOF: i32 = 3;
pub const NEWLINE: i32 = 4;
pub const COLON: i32 = 5;
pub const STYLE_SEPARATOR: i32 = 6;
pub const BLOCK_START: i32 = 7;
pub const BLOCK_STOP: i32 = 8;
pub const SQS: i32 = 9;
pub const SQE: i32 = 10;
pub const ACC_TITLE: i32 = 11;
pub const ACC_TITLE_VALUE: i32 = 12;
pub const ACC_DESCR: i32 = 13;
pub const ACC_DESCR_VALUE: i32 = 14;
pub const ACC_DESCR_MULTILINE_VALUE: i32 = 15;
pub const END_KW: i32 = 16;
pub const SUBGRAPH: i32 = 17;
pub const DIRECTION_TB: i32 = 18;
pub const DIRECTION_BT: i32 = 19;
pub const DIRECTION_RL: i32 = 20;
pub const DIRECTION_LR: i32 = 21;
pub const CLASSDEF: i32 = 22;
pub const UNICODE_TEXT: i32 = 23;
pub const STYLE_TEXT: i32 = 24;
pub const COMMA: i32 = 25;
pub const CLASS: i32 = 26;
pub const STYLE: i32 = 27;
pub const SEMI: i32 = 28;
pub const NUM: i32 = 29;
pub const BRKT: i32 = 30;
pub const ENTITY_NAME: i32 = 31;
pub const DECIMAL_NUM: i32 = 32;
pub const ENTITY_ONE: i32 = 33;
pub const ATTRIBUTE_WORD: i32 = 34;
pub const ATTRIBUTE_KEY: i32 = 35;
pub const COMMENT: i32 = 36;
pub const ZERO_OR_ONE: i32 = 37;
pub const ZERO_OR_MORE: i32 = 38;
pub const ONE_OR_MORE: i32 = 39;
pub const ONLY_ONE: i32 = 40;
pub const MD_PARENT: i32 = 41;
pub const NON_IDENTIFYING: i32 = 42;
pub const IDENTIFYING: i32 = 43;
pub const WORD: i32 = 44;
pub const CHAR: i32 = 45;

pub const TOKEN_NAMES: &[&str] = &[
    "$end",
    "INVALID",
    "ER_DIAGRAM",
    "EOF",
    "NEWLINE",
    "COLON",
    "STYLE_SEPARATOR",
    "BLOCK_START",
    "BLOCK_STOP",
    "SQS",
    "SQE",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "END",
    "SUBGRAPH",
    "direction_tb",
    "direction_bt",
    "direction_rl",
    "direction_lr",
    "CLASSDEF",
    "UNICODE_TEXT",
    "STYLE_TEXT",
    "COMMA",
    "CLASS",
    "STYLE",
    "SEMI",
    "NUM",
    "BRKT",
    "ENTITY_NAME",
    "DECIMAL_NUM",
    "ENTITY_ONE",
    "ATTRIBUTE_WORD",
    "ATTRIBUTE_KEY",
    "COMMENT",
    "ZERO_OR_ONE",
    "ZERO_OR_MORE",
    "ONE_OR_MORE",
    "ONLY_ONE",
    "MD_PARENT",
    "NON_IDENTIFYING",
    "IDENTIFYING",
    "WORD",
    "CHAR",
];
