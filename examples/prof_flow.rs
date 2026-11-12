fn main() {
    let mut source = String::from("flowchart TD\n");
    for i in 1..100 {
        source.push_str(&format!(
            "  n{}[Node {}] --> n{}[Node {}]\n",
            (i - 1) / 3,
            (i - 1) / 3,
            i,
            i
        ));
    }
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    let start = std::time::Instant::now();
    let mut renders = 0u64;
    while start.elapsed().as_secs() < seconds {
        let model = pelers::parse_with(&source, &pelers::ParseOptions::default()).unwrap();
        std::hint::black_box(
            pelers::render_with(&model, &pelers::RenderOptions::default()).unwrap(),
        );
        renders += 1;
    }
    println!("renders: {renders}");
}
