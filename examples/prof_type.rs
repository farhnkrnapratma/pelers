fn main() {
    let wanted = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "sequence".to_owned());
    let seconds: u64 = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    let text = std::fs::read_to_string("bench/corpus.json").unwrap();
    let corpus: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    let sources: Vec<&str> = corpus
        .iter()
        .filter(|item| item["type"].as_str() == Some(wanted.as_str()))
        .map(|item| item["source"].as_str().unwrap())
        .collect();
    let start = std::time::Instant::now();
    let mut renders = 0u64;
    while start.elapsed().as_secs() < seconds {
        for source in &sources {
            let model = pelers::parse_with(source, &pelers::ParseOptions::default()).unwrap();
            std::hint::black_box(
                pelers::render_with(&model, &pelers::RenderOptions::default()).unwrap(),
            );
            renders += 1;
        }
    }
    println!("renders: {renders}");
}
