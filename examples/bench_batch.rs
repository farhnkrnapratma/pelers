#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::time::Instant;

fn main() {
    let text = std::fs::read_to_string("bench/corpus.json").unwrap();
    let corpus: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    let sources: Vec<&str> = corpus
        .iter()
        .map(|item| item["source"].as_str().unwrap())
        .collect();
    let sequential: Vec<String> = sources
        .iter()
        .map(|s| {
            pelers::api::render_batch(&[s], 1)[0]
                .as_ref()
                .unwrap()
                .svg
                .clone()
        })
        .collect();
    for threads in [1usize, 2, 4, 8] {
        let mut best = f64::MAX;
        for _ in 0..3 {
            let start = Instant::now();
            let out = pelers::api::render_batch(&sources, threads);
            let elapsed = start.elapsed().as_secs_f64();
            best = best.min(elapsed);
            let identical = out.len() == sequential.len()
                && out
                    .iter()
                    .zip(&sequential)
                    .all(|(a, b)| a.as_ref().unwrap().svg == *b);
            assert!(identical, "batch output diverged at {threads} threads");
        }
        println!(
            "threads {threads}: {:.2} ms for {} sources ({:.4} ms/source, identical)",
            best * 1000.0,
            sources.len(),
            best * 1000.0 / sources.len() as f64
        );
    }
}
