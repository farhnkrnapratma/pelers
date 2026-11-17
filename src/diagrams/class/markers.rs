use super::graph::EndMarker;
use crate::svg::builder::num;
use crate::svg::edges::{marker, marker_trim};

const TRIANGLE: f64 = 12.0;
const DIAMOND: f64 = 16.0;
const HALF: f64 = 6.0;
const RING: f64 = 5.0;

pub fn class_marker_trim(kind: EndMarker) -> f64 {
    match kind {
        EndMarker::Extension => TRIANGLE,
        EndMarker::Composition | EndMarker::Aggregation => DIAMOND,
        EndMarker::Lollipop => 2.0 * RING,
        EndMarker::Dependency => marker_trim("arrow_point"),
        EndMarker::None => 0.0,
    }
}

pub fn class_marker(kind: EndMarker, x: f64, y: f64, dx: f64, dy: f64, color: &str) -> String {
    let nx = -dy;
    let ny = dx;
    match kind {
        EndMarker::Extension => {
            let bx = x - dx * TRIANGLE;
            let by = y - dy * TRIANGLE;
            format!(
                "<path class=\"pelers-marker\" d=\"M{},{}L{},{}L{},{}Z\" stroke-linejoin=\"round\"/>",
                num(x),
                num(y),
                num(bx + nx * HALF),
                num(by + ny * HALF),
                num(bx - nx * HALF),
                num(by - ny * HALF)
            )
        }
        EndMarker::Composition | EndMarker::Aggregation => {
            let mx = x - (dx * DIAMOND) / 2.0;
            let my = y - (dy * DIAMOND) / 2.0;
            let half = HALF - 1.0;
            let fill = if kind == EndMarker::Composition {
                format!(" fill=\"{color}\"")
            } else {
                String::new()
            };
            format!(
                "<path class=\"pelers-marker\" d=\"M{},{}L{},{}L{},{}L{},{}Z\" stroke-linejoin=\"round\"{fill}/>",
                num(x),
                num(y),
                num(mx + nx * half),
                num(my + ny * half),
                num(x - dx * DIAMOND),
                num(y - dy * DIAMOND),
                num(mx - nx * half),
                num(my - ny * half)
            )
        }
        EndMarker::Lollipop => format!(
            "<circle class=\"pelers-marker\" cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
            num(x - dx * RING),
            num(y - dy * RING),
            num(RING)
        ),
        EndMarker::Dependency => marker("arrow_point", x, y, dx, dy, color, "none"),
        EndMarker::None => String::new(),
    }
}
