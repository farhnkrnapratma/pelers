use crate::diagrams::flowchart::shapes::is_valid_shape as is_flowchart_shape;

pub const DEFAULT_SHAPE: &str = "roundedRect";

const SHAPE_ALIASES: [(&str, &str); 7] = [
    ("task", "roundedRect"),
    ("tool", "subroutine"),
    ("input", "lean-right"),
    ("decision", "diamond"),
    ("refdoc", "lin-doc"),
    ("action", "hexagon"),
    ("round", "rect"),
];

pub const REMOVED_SHAPES: [&str; 27] = [
    "doc",
    "stadium",
    "terminal",
    "circle",
    "trapezoid",
    "inv_trapezoid",
    "inv-trapezoid",
    "doublecircle",
    "double-circle",
    "typeDeclaration",
    "procs",
    "lean_left",
    "lean-left",
    "in-out",
    "cylinder",
    "ellipse",
    "odd",
    "tag-rect",
    "tagged-rectangle",
    "delay",
    "half-rounded-rectangle",
    "lin-rect",
    "lined-rectangle",
    "win-pane",
    "window-pane",
    "curv-trap",
    "curved-trapezoid",
];

pub const ALLOWED_SHAPES: [&str; 13] = [
    "roundedRect",
    "subroutine",
    "subprocess",
    "subproc",
    "framed-rectangle",
    "lean-right",
    "diamond",
    "lin-doc",
    "lined-document",
    "hexagon",
    "hex",
    "connector",
    "collapsedGroup",
];

pub fn resolve_shape_alias(shape: Option<&str>) -> Option<String> {
    let shape = shape?;
    if shape.is_empty() {
        return Some(String::new());
    }
    Some(
        SHAPE_ALIASES
            .iter()
            .find(|(from, _)| *from == shape)
            .map_or(shape, |(_, to)| to)
            .to_owned(),
    )
}

pub fn is_removed_shape(shape: &str) -> bool {
    REMOVED_SHAPES.contains(&shape)
}

pub fn is_allowed_shape(shape: &str) -> bool {
    ALLOWED_SHAPES.contains(&shape)
}

pub fn is_valid_shape(name: &str) -> bool {
    is_flowchart_shape(name) || name == "constructor"
}
