use std::time::Instant;

fn time(label: &str, mut f: impl FnMut()) {
    for _ in 0..200 {
        f();
    }
    let mut best = f64::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        let mut n = 0u32;
        loop {
            f();
            n += 1;
            if start.elapsed().as_millis() >= 20 {
                break;
            }
        }
        best = best.min(start.elapsed().as_secs_f64() * 1e6 / f64::from(n));
    }
    println!("{label}: {best:.2} us");
}

fn main() {
    root_only();
    let source = "info\n";
    time("preprocess", || {
        std::hint::black_box(pelers::preprocess(source).unwrap());
    });
    let prepared = pelers::preprocess(source).unwrap();
    time("detect", || {
        std::hint::black_box(pelers::detect_diagram(&prepared.text));
    });
    time("parse_with", || {
        std::hint::black_box(pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap());
    });
    let model = pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap();
    time("render_with", || {
        std::hint::black_box(
            pelers::render_with(&model, &pelers::RenderOptions::default()).unwrap(),
        );
    });
    time("full", || {
        let m = pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap();
        std::hint::black_box(pelers::render_with(&m, &pelers::RenderOptions::default()).unwrap());
    });
}

#[allow(dead_code)]
fn root_only() {
    let options = pelers::RenderOptions::default();
    time("svg_document", || {
        std::hint::black_box(pelers::svg::root::svg_document(
            "info",
            100.0,
            40.0,
            16.0,
            &options,
            pelers::svg::root::Accessible::default(),
            "",
        ));
    });
    time("palette_style", || {
        std::hint::black_box(pelers::svg::palette::palette_style());
    });
    time("font_style", || {
        std::hint::black_box(pelers::svg::theme::font_style(16.0, None, None));
    });
}
