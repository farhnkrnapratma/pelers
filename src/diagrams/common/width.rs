use crate::diagrams::common::Drawn;
use crate::svg::options::RenderOptions;

const BREAKPOINT: f64 = 640.0;

pub fn title_room(options: &RenderOptions) -> f64 {
    match options.max_width {
        Some(available) if available > 0.0 => {
            let pad = options.padding.unwrap_or(8.0);
            let room = available - 2.0 * pad;
            if room > 80.0 { room } else { 80.0 }
        }
        _ => 4000.0,
    }
}

pub fn runs_across(direction: Option<&str>) -> bool {
    matches!(direction, Some("LR") | Some("RL"))
}

pub fn turn_to_fit<T: Width>(options: &RenderOptions, mut draw: impl FnMut(bool) -> T) -> T {
    let across = draw(false);
    let Some(available) = options.max_width.filter(|a| *a > 0.0) else {
        return across;
    };
    if options.auto_direction == Some(false) || across.width() <= available {
        return across;
    }
    let breakpoint = match options.direction_breakpoint {
        Some(b) if b > 0.0 => b,
        _ => BREAKPOINT,
    };
    if available >= breakpoint {
        return across;
    }
    let down = draw(true);
    if down.width() < across.width() {
        down
    } else {
        across
    }
}

pub fn tighten<T: Width>(options: &RenderOptions, mut draw: impl FnMut(bool) -> T) -> T {
    let loose = draw(false);
    let Some(available) = options.max_width.filter(|a| *a > 0.0) else {
        return loose;
    };
    if loose.width() <= available {
        return loose;
    }
    let tight = draw(true);
    if tight.width() < loose.width() {
        tight
    } else {
        loose
    }
}

impl Width for Drawn {
    fn width(&self) -> f64 {
        self.width
    }
}

pub trait Width {
    fn width(&self) -> f64;
}
