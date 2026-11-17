#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::collections::BTreeMap;
use std::time::Instant;

fn main() {
    let text = std::fs::read_to_string("bench/corpus.json").unwrap();
    let corpus: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    let mut per: BTreeMap<String, (f64, usize)> = BTreeMap::new();
    for item in &corpus {
        let source = item["source"].as_str().unwrap();
        let run = || {
            let model = pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap();
            pelers::render_with(&model, &pelers::RenderOptions::default()).unwrap()
        };
        for _ in 0..3 {
            run();
        }
        let mut best = f64::MAX;
        for _ in 0..3 {
            let start = Instant::now();
            let mut iters = 0u32;
            loop {
                run();
                iters += 1;
                if start.elapsed().as_millis() >= 10 {
                    break;
                }
            }
            best = best.min(start.elapsed().as_secs_f64() * 1000.0 / f64::from(iters));
        }
        let entry = per
            .entry(item["type"].as_str().unwrap().to_owned())
            .or_default();
        entry.0 += best;
        entry.1 += 1;
    }
    for (t, (ms, n)) in per {
        println!("TYPE {t} {:.5}", ms / n as f64);
    }
}
