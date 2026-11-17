pub const END: usize = 0;
pub const ERROR: usize = 1;
pub const KEY: usize = 2;
pub const ID_BLOCK: usize = 3;
pub const COLUMNS: usize = 4;
pub const MD_STR: usize = 5;
pub const STR: usize = 6;
pub const SPACE_BLOCK: usize = 7;
pub const DEFAULT: usize = 8;
pub const LINKSTYLE: usize = 9;
pub const INTERPOLATE: usize = 10;
pub const CLASSDEF: usize = 11;
pub const DEFAULT_CLASSDEF_ID: usize = 12;
pub const CLASSDEF_ID: usize = 13;
pub const CLASSDEF_STYLEOPTS: usize = 14;
pub const CLASS: usize = 15;
pub const CLASSENTITY_IDS: usize = 16;
pub const STYLECLASS: usize = 17;
pub const STYLE: usize = 18;
pub const STYLE_ENTITY_IDS: usize = 19;
pub const STYLE_DEFINITION_DATA: usize = 20;
pub const ACC_TITLE: usize = 21;
pub const ACC_TITLE_VALUE: usize = 22;
pub const ACC_DESCR: usize = 23;
pub const ACC_DESCR_VALUE: usize = 24;
pub const ACC_DESCR_MULTILINE_VALUE: usize = 25;
pub const END_KW: usize = 26;
pub const NODE_DSTART: usize = 27;
pub const NODE_DEND: usize = 28;
pub const BLOCK_ARROW_START: usize = 29;
pub const BLOCK_ARROW_END: usize = 30;
pub const DIR: usize = 31;
pub const NODE_ID: usize = 32;
pub const LINK: usize = 33;
pub const START_LINK: usize = 34;
pub const LINK_LABEL: usize = 35;
pub const SIZE: usize = 36;
pub const EOF: usize = 37;

pub const TOKEN_NAMES: [&str; 38] = [
    "$end",
    "INVALID",
    "BLOCK_DIAGRAM_KEY",
    "id-block",
    "COLUMNS",
    "MD_STR",
    "STR",
    "SPACE_BLOCK",
    "DEFAULT",
    "LINKSTYLE",
    "INTERPOLATE",
    "classDef",
    "DEFAULT_CLASSDEF_ID",
    "CLASSDEF_ID",
    "CLASSDEF_STYLEOPTS",
    "class",
    "CLASSENTITY_IDS",
    "STYLECLASS",
    "style",
    "STYLE_ENTITY_IDS",
    "STYLE_DEFINITION_DATA",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "end",
    "NODE_DSTART",
    "NODE_DEND",
    "BLOCK_ARROW_START",
    "BLOCK_ARROW_END",
    "DIR",
    "NODE_ID",
    "LINK",
    "START_LINK",
    "LINK_LABEL",
    "SIZE",
    "EOF",
];
