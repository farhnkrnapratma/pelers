pub const END: usize = 0;
pub const ERROR: usize = 1;
pub const INVALID: usize = 2;
pub const NEWLINE: usize = 3;
pub const SD: usize = 4;
pub const CREATE: usize = 5;
pub const BOX: usize = 6;
pub const RESTOFLINE: usize = 7;
pub const END_KW: usize = 8;
pub const AUTONUMBER: usize = 9;
pub const NUM: usize = 10;
pub const OFF: usize = 11;
pub const ACTIVATE: usize = 12;
pub const DEACTIVATE: usize = 13;
pub const TITLE: usize = 14;
pub const LEGACY_TITLE: usize = 15;
pub const ACC_TITLE: usize = 16;
pub const ACC_TITLE_VALUE: usize = 17;
pub const ACC_DESCR: usize = 18;
pub const ACC_DESCR_VALUE: usize = 19;
pub const ACC_DESCR_MULTILINE_VALUE: usize = 20;
pub const LOOP: usize = 21;
pub const RECT: usize = 22;
pub const OPT: usize = 23;
pub const ALT: usize = 24;
pub const PAR: usize = 25;
pub const PAR_OVER: usize = 26;
pub const CRITICAL: usize = 27;
pub const BREAK: usize = 28;
pub const OPTION: usize = 29;
pub const AND: usize = 30;
pub const ELSE: usize = 31;
pub const PARTICIPANT: usize = 32;
pub const AS: usize = 33;
pub const PARTICIPANT_ACTOR: usize = 34;
pub const DESTROY: usize = 35;
pub const NOTE: usize = 36;
pub const OVER: usize = 37;
pub const LINKS: usize = 38;
pub const LINK: usize = 39;
pub const PROPERTIES: usize = 40;
pub const DETAILS: usize = 41;
pub const COMMA: usize = 42;
pub const LEFT_OF: usize = 43;
pub const RIGHT_OF: usize = 44;
pub const PLUS: usize = 45;
pub const MINUS: usize = 46;
pub const CENTRAL: usize = 47;
pub const ACTOR: usize = 48;
pub const CONFIG_START: usize = 49;
pub const CONFIG_CONTENT: usize = 50;
pub const CONFIG_END: usize = 51;
pub const TXT: usize = 52;
pub const SOLID_OPEN_ARROW: usize = 53;
pub const DOTTED_OPEN_ARROW: usize = 54;
pub const SOLID_ARROW: usize = 55;
pub const BIDIRECTIONAL_SOLID_ARROW: usize = 56;
pub const DOTTED_ARROW: usize = 57;
pub const BIDIRECTIONAL_DOTTED_ARROW: usize = 58;
pub const SOLID_CROSS: usize = 59;
pub const DOTTED_CROSS: usize = 60;
pub const SOLID_POINT: usize = 61;
pub const DOTTED_POINT: usize = 62;
pub const SOLID_ARROW_TOP: usize = 63;
pub const SOLID_ARROW_BOTTOM: usize = 64;
pub const STICK_ARROW_TOP: usize = 65;
pub const STICK_ARROW_BOTTOM: usize = 66;
pub const SOLID_ARROW_TOP_DOTTED: usize = 67;
pub const SOLID_ARROW_BOTTOM_DOTTED: usize = 68;
pub const STICK_ARROW_TOP_DOTTED: usize = 69;
pub const STICK_ARROW_BOTTOM_DOTTED: usize = 70;
pub const SOLID_ARROW_TOP_REVERSE: usize = 71;
pub const SOLID_ARROW_BOTTOM_REVERSE: usize = 72;
pub const STICK_ARROW_TOP_REVERSE: usize = 73;
pub const STICK_ARROW_BOTTOM_REVERSE: usize = 74;
pub const SOLID_ARROW_TOP_REVERSE_DOTTED: usize = 75;
pub const SOLID_ARROW_BOTTOM_REVERSE_DOTTED: usize = 76;
pub const STICK_ARROW_TOP_REVERSE_DOTTED: usize = 77;
pub const STICK_ARROW_BOTTOM_REVERSE_DOTTED: usize = 78;

pub const TOKEN_NAMES: [&str; 79] = [
    "$end",
    "INVALID",
    "INVALID",
    "NEWLINE",
    "SD",
    "create",
    "box",
    "restOfLine",
    "end",
    "autonumber",
    "NUM",
    "off",
    "activate",
    "deactivate",
    "title",
    "legacy_title",
    "acc_title",
    "acc_title_value",
    "acc_descr",
    "acc_descr_value",
    "acc_descr_multiline_value",
    "loop",
    "rect",
    "opt",
    "alt",
    "par",
    "par_over",
    "critical",
    "break",
    "option",
    "and",
    "else",
    "participant",
    "AS",
    "participant_actor",
    "destroy",
    "note",
    "over",
    "links",
    "link",
    "properties",
    "details",
    ",",
    "left_of",
    "right_of",
    "+",
    "-",
    "()",
    "ACTOR",
    "CONFIG_START",
    "CONFIG_CONTENT",
    "CONFIG_END",
    "TXT",
    "SOLID_OPEN_ARROW",
    "DOTTED_OPEN_ARROW",
    "SOLID_ARROW",
    "BIDIRECTIONAL_SOLID_ARROW",
    "DOTTED_ARROW",
    "BIDIRECTIONAL_DOTTED_ARROW",
    "SOLID_CROSS",
    "DOTTED_CROSS",
    "SOLID_POINT",
    "DOTTED_POINT",
    "SOLID_ARROW_TOP",
    "SOLID_ARROW_BOTTOM",
    "STICK_ARROW_TOP",
    "STICK_ARROW_BOTTOM",
    "SOLID_ARROW_TOP_DOTTED",
    "SOLID_ARROW_BOTTOM_DOTTED",
    "STICK_ARROW_TOP_DOTTED",
    "STICK_ARROW_BOTTOM_DOTTED",
    "SOLID_ARROW_TOP_REVERSE",
    "SOLID_ARROW_BOTTOM_REVERSE",
    "STICK_ARROW_TOP_REVERSE",
    "STICK_ARROW_BOTTOM_REVERSE",
    "SOLID_ARROW_TOP_REVERSE_DOTTED",
    "SOLID_ARROW_BOTTOM_REVERSE_DOTTED",
    "STICK_ARROW_TOP_REVERSE_DOTTED",
    "STICK_ARROW_BOTTOM_REVERSE_DOTTED",
];
