use crate::diagrams::common::Drawn;
use crate::svg::root::RenderOptions;
use crate::util::jsnum::{js_max, js_min};

pub fn fit_width(
    options: &RenderOptions,
    natural: f64,
    least: f64,
    mut draw: impl FnMut(f64, bool) -> Drawn,
) -> Drawn {
    let mut result = draw(natural, false);
    let Some(available) = options.max_width else {
        return result;
    };
    if available.is_nan() || available <= 0.0 || result.width <= available {
        return result;
    }
    let floor = js_min(least, natural);
    let mut plot = natural;
    result = draw(plot, true);
    let mut round = 0;
    while round < 3 && result.width > available && plot > floor {
        plot = js_max(floor, plot - (result.width - available));
        result = draw(plot, true);
        round += 1;
    }
    result
}

const BREAKPOINT: f64 = 640.0;

pub fn turn_to_fit(options: &RenderOptions, mut draw: impl FnMut(bool) -> Drawn) -> Drawn {
    let across = draw(false);
    let Some(available) = options.max_width else {
        return across;
    };
    if options.auto_direction == Some(false)
        || available.is_nan()
        || available <= 0.0
        || across.width <= available
    {
        return across;
    }
    let breakpoint = match options.direction_breakpoint {
        Some(value) if value > 0.0 => value,
        _ => BREAKPOINT,
    };
    if available >= breakpoint {
        return across;
    }
    let down = draw(true);
    if down.width < across.width {
        down
    } else {
        across
    }
}

pub fn cap_width(options: &RenderOptions, natural: f64, least: f64, around: f64) -> f64 {
    match options.max_width {
        Some(available) if available > 0.0 => {
            js_max(js_min(least, natural), js_min(natural, available - around))
        }
        _ => natural,
    }
}

pub fn runs_across(direction: Option<&str>) -> bool {
    matches!(direction, Some("LR" | "RL"))
}

pub fn title_room(options: &RenderOptions) -> f64 {
    match options.max_width {
        Some(available) if available > 0.0 => {
            js_max(80.0, available - 2.0 * options.padding.unwrap_or(8.0))
        }
        _ => 4000.0,
    }
}

pub fn tighten(options: &RenderOptions, draw: impl Fn(bool) -> Drawn) -> Drawn {
    let loose = draw(false);
    let Some(available) = options.max_width.filter(|a| *a > 0.0) else {
        return loose;
    };
    if loose.width <= available {
        return loose;
    }
    let tight = draw(true);
    if tight.width < loose.width {
        tight
    } else {
        loose
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drawn(width: f64) -> Drawn {
        Drawn {
            links: Vec::new(),
            svg: String::new(),
            width,
            height: 0.0,
        }
    }

    #[test]
    fn keeps_the_natural_plot_when_it_fits_or_no_width_is_given() {
        let options = RenderOptions::default();
        assert_eq!(
            fit_width(&options, 240.0, 140.0, |p, _| drawn(p + 100.0)).width,
            340.0
        );
        let wide = RenderOptions {
            max_width: Some(400.0),
            ..RenderOptions::default()
        };
        assert_eq!(
            fit_width(&wide, 240.0, 140.0, |p, _| drawn(p + 100.0)).width,
            340.0
        );
    }

    #[test]
    fn shrinks_the_plot_down_to_the_least() {
        let options = RenderOptions {
            max_width: Some(300.0),
            ..RenderOptions::default()
        };
        assert_eq!(
            fit_width(&options, 240.0, 140.0, |p, _| drawn(p + 100.0)).width,
            300.0
        );
        let tiny = RenderOptions {
            max_width: Some(100.0),
            ..RenderOptions::default()
        };
        assert_eq!(
            fit_width(&tiny, 240.0, 140.0, |p, _| drawn(p + 100.0)).width,
            240.0
        );
    }

    #[test]
    fn turns_down_only_when_narrow_and_shorter() {
        let draw = |down: bool| drawn(if down { 200.0 } else { 500.0 });
        let narrow = RenderOptions {
            max_width: Some(300.0),
            ..RenderOptions::default()
        };
        assert_eq!(turn_to_fit(&narrow, draw).width, 200.0);
        let fixed = RenderOptions {
            auto_direction: Some(false),
            ..narrow.clone()
        };
        assert_eq!(turn_to_fit(&fixed, draw).width, 500.0);
        let wide = RenderOptions {
            max_width: Some(700.0),
            ..RenderOptions::default()
        };
        assert_eq!(turn_to_fit(&wide, draw).width, 500.0);
        let early = RenderOptions {
            direction_breakpoint: Some(250.0),
            ..narrow
        };
        assert_eq!(turn_to_fit(&early, draw).width, 500.0);
        assert_eq!(turn_to_fit(&RenderOptions::default(), draw).width, 500.0);
    }

    #[test]
    fn leaves_room_for_a_title() {
        assert_eq!(title_room(&RenderOptions::default()), 4000.0);
        let options = RenderOptions {
            max_width: Some(100.0),
            ..RenderOptions::default()
        };
        assert_eq!(title_room(&options), 84.0);
    }
}
