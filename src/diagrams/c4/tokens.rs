pub const END: usize = 0;
pub const ERROR: usize = 1;
pub const DIRECTION_TB: usize = 2;
pub const TITLE: usize = 6;
pub const ACC_DESCRIPTION: usize = 7;
pub const ACC_TITLE: usize = 8;
pub const ACC_TITLE_VALUE: usize = 9;
pub const ACC_DESCR: usize = 10;
pub const ACC_DESCR_VALUE: usize = 11;
pub const ACC_DESCR_MULTILINE_VALUE: usize = 12;
pub const NEWLINE: usize = 13;
pub const C4_CONTEXT: usize = 14;
pub const C4_CONTAINER: usize = 15;
pub const C4_COMPONENT: usize = 16;
pub const C4_DYNAMIC: usize = 17;
pub const C4_DEPLOYMENT: usize = 18;
pub const PERSON: usize = 19;
pub const PERSON_EXT: usize = 20;
pub const SYSTEM: usize = 21;
pub const SYSTEM_DB: usize = 22;
pub const SYSTEM_QUEUE: usize = 23;
pub const SYSTEM_EXT: usize = 24;
pub const SYSTEM_EXT_DB: usize = 25;
pub const SYSTEM_EXT_QUEUE: usize = 26;
pub const CONTAINER: usize = 27;
pub const CONTAINER_DB: usize = 28;
pub const CONTAINER_QUEUE: usize = 29;
pub const CONTAINER_EXT: usize = 30;
pub const CONTAINER_EXT_DB: usize = 31;
pub const CONTAINER_EXT_QUEUE: usize = 32;
pub const COMPONENT: usize = 33;
pub const COMPONENT_DB: usize = 34;
pub const COMPONENT_QUEUE: usize = 35;
pub const COMPONENT_EXT: usize = 36;
pub const COMPONENT_EXT_DB: usize = 37;
pub const COMPONENT_EXT_QUEUE: usize = 38;
pub const ENTERPRISE_BOUNDARY: usize = 39;
pub const SYSTEM_BOUNDARY: usize = 40;
pub const BOUNDARY: usize = 41;
pub const CONTAINER_BOUNDARY: usize = 42;
pub const NODE: usize = 43;
pub const NODE_L: usize = 44;
pub const NODE_R: usize = 45;
pub const REL: usize = 46;
pub const BIREL: usize = 47;
pub const REL_U: usize = 48;
pub const REL_D: usize = 49;
pub const REL_L: usize = 50;
pub const REL_R: usize = 51;
pub const REL_B: usize = 52;
pub const REL_INDEX: usize = 53;
pub const UPDATE_EL_STYLE: usize = 54;
pub const UPDATE_REL_STYLE: usize = 55;
pub const UPDATE_LAYOUT_CONFIG: usize = 56;
pub const EOF_IN_STRUCT: usize = 57;
pub const ATTRIBUTE_EMPTY: usize = 58;
pub const STR: usize = 59;
pub const STR_KEY: usize = 60;
pub const STR_VALUE: usize = 61;
pub const LBRACE: usize = 62;
pub const RBRACE: usize = 63;
pub const EOF: usize = 64;

pub const TOKEN_NAMES: [&str; 65] = [
    "$end",
    "INVALID",
    "direction_tb",
    "direction_bt",
    "direction_rl",
    "direction_lr",
    "title",
    "accDescription",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "NEWLINE",
    "C4_CONTEXT",
    "C4_CONTAINER",
    "C4_COMPONENT",
    "C4_DYNAMIC",
    "C4_DEPLOYMENT",
    "PERSON",
    "PERSON_EXT",
    "SYSTEM",
    "SYSTEM_DB",
    "SYSTEM_QUEUE",
    "SYSTEM_EXT",
    "SYSTEM_EXT_DB",
    "SYSTEM_EXT_QUEUE",
    "CONTAINER",
    "CONTAINER_DB",
    "CONTAINER_QUEUE",
    "CONTAINER_EXT",
    "CONTAINER_EXT_DB",
    "CONTAINER_EXT_QUEUE",
    "COMPONENT",
    "COMPONENT_DB",
    "COMPONENT_QUEUE",
    "COMPONENT_EXT",
    "COMPONENT_EXT_DB",
    "COMPONENT_EXT_QUEUE",
    "ENTERPRISE_BOUNDARY",
    "SYSTEM_BOUNDARY",
    "BOUNDARY",
    "CONTAINER_BOUNDARY",
    "NODE",
    "NODE_L",
    "NODE_R",
    "REL",
    "BIREL",
    "REL_U",
    "REL_D",
    "REL_L",
    "REL_R",
    "REL_B",
    "REL_INDEX",
    "UPDATE_EL_STYLE",
    "UPDATE_REL_STYLE",
    "UPDATE_LAYOUT_CONFIG",
    "EOF_IN_STRUCT",
    "ATTRIBUTE_EMPTY",
    "STR",
    "STR_KEY",
    "STR_VALUE",
    "LBRACE",
    "RBRACE",
    "EOF",
];
