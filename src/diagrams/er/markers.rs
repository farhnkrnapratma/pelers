use crate::svg::builder::num;
use crate::util::jsnum::js_number_to_string;

pub const MARKER_LENGTH: f64 = 18.0;

const BAR: f64 = 5.0;
const FOOT: f64 = 6.0;
const FOOT_LENGTH: f64 = 10.0;
const RING: f64 = 3.5;

pub fn er_marker(kind: &str, x: f64, y: f64, dx: f64, dy: f64) -> String {
    let at = |back: f64, side: f64| {
        format!(
            "{},{}",
            num(x - dx * back - dy * side),
            num(y - dy * back + dx * side)
        )
    };
    let bar = |back: f64| format!("M{}L{}", at(back, -BAR), at(back, BAR));
    let foot = format!(
        "M{}L{}L{}",
        at(0.0, -FOOT),
        at(FOOT_LENGTH, 0.0),
        at(0.0, FOOT)
    );
    let ring = |back: f64| {
        format!(
            "<circle class=\"pelers-marker\" cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"var(--_bg)\"/>",
            num(x - dx * back),
            num(y - dy * back),
            js_number_to_string(RING)
        )
    };
    let path = |d: &str| format!("<path class=\"pelers-marker\" d=\"{d}\"/>");

    match kind {
        "only_one" => path(&(bar(6.0) + &bar(10.0))),
        "zero_or_one" => path(&bar(6.0)) + &ring(13.5),
        "one_or_more" => path(&(foot.clone() + &bar(FOOT_LENGTH + 3.5))),
        "zero_or_more" => path(&foot) + &ring(FOOT_LENGTH + RING + 0.5),
        "md_parent" => format!(
            "<path class=\"pelers-marker\" d=\"M{}L{}L{}L{}Z\" fill=\"var(--_bg)\"/>",
            at(0.0, 0.0),
            at(6.0, 4.0),
            at(12.0, 0.0),
            at(6.0, -4.0)
        ),
        _ => String::new(),
    }
}
