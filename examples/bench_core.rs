#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::time::Instant;

fn main() {
    let text = std::fs::read_to_string("bench/corpus.json").unwrap();
    let corpus: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    let mut total = 0.0f64;
    let mut count = 0usize;
    for item in &corpus {
        let source = item["source"].as_str().unwrap();
        let run = || {
            let model = pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap();
            pelers::render_with(&model, &pelers::RenderOptions::default()).unwrap()
        };
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
        total += start.elapsed().as_secs_f64() * 1000.0 / f64::from(iters);
        count += 1;
    }
    println!(
        "core mean ms per render: {:.4} over {count} sources",
        total / count as f64
    );
}
