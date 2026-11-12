use std::time::Instant;

fn timed(mut run: impl FnMut()) -> f64 {
    for _ in 0..3 {
        run();
    }
    let mut iters = 0u32;
    let start = Instant::now();
    loop {
        run();
        iters += 1;
        if start.elapsed().as_millis() >= 25 {
            break;
        }
    }
    start.elapsed().as_secs_f64() * 1000.0 / f64::from(iters)
}

fn main() {
    let text = std::fs::read_to_string("bench/corpus.json").unwrap();
    let corpus: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    let (mut parse_total, mut render_total, mut n) = (0.0, 0.0, 0usize);
    let mut per_type: std::collections::BTreeMap<String, (f64, usize)> = Default::default();
    for item in &corpus {
        let source = item["source"].as_str().unwrap();
        let parse_ms = timed(|| {
            pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap();
        });
        let model = pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap();
        let render_ms = timed(|| {
            pelers::render_with(&model, &pelers::RenderOptions::default()).unwrap();
        });
        parse_total += parse_ms;
        render_total += render_ms;
        n += 1;
        let entry = per_type
            .entry(item["type"].as_str().unwrap().to_owned())
            .or_default();
        entry.0 += parse_ms + render_ms;
        entry.1 += 1;
    }
    println!(
        "parse mean {:.4} ms, render mean {:.4} ms",
        parse_total / n as f64,
        render_total / n as f64
    );
    let mut rows: Vec<_> = per_type
        .into_iter()
        .map(|(t, (ms, c))| (ms / c as f64, t))
        .collect();
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (ms, t) in rows.iter().take(8) {
        println!("  {t}: {ms:.4} ms");
    }
}
