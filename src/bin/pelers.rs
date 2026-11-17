#[cfg(not(feature = "napi"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: pelers [--threads N] [--out DIR] [FILE|-]...\n\
renders Mermaid diagrams to SVG; with no FILE or '-', reads one diagram from stdin";

struct Options {
    threads: usize,
    out: Option<PathBuf>,
    inputs: Vec<String>,
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options {
        threads: std::thread::available_parallelism().map_or(1, usize::from),
        out: None,
        inputs: Vec::new(),
    };
    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--threads" => {
                let value = args.next().ok_or("--threads needs a value")?;
                options.threads = value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or("--threads must be a positive integer")?;
            }
            "--out" => {
                options.out = Some(PathBuf::from(args.next().ok_or("--out needs a directory")?));
            }
            "-h" | "--help" => return Err(String::new()),
            _ => options.inputs.push(arg),
        }
    }
    if options.inputs.is_empty() {
        options.inputs.push("-".to_owned());
    }
    Ok(options)
}

fn read_input(name: &str) -> std::io::Result<String> {
    if name == "-" {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text)?;
        Ok(text)
    } else {
        std::fs::read_to_string(name)
    }
}

fn output_path(dir: &Path, input: &str, index: usize) -> PathBuf {
    let stem = Path::new(input)
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .map_or_else(|| format!("diagram-{index}"), str::to_owned);
    dir.join(format!("{stem}.svg"))
}

fn run() -> Result<ExitCode, String> {
    let options = parse_args(std::env::args().skip(1))?;
    let mut sources = Vec::with_capacity(options.inputs.len());
    for name in &options.inputs {
        sources.push(read_input(name).map_err(|e| format!("{name}: {e}"))?);
    }
    let borrowed: Vec<&str> = sources.iter().map(String::as_str).collect();
    let results = pelers::api::render_batch(&borrowed, options.threads);
    let mut failed = false;
    if let Some(dir) = &options.out {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    for (index, (name, result)) in options.inputs.iter().zip(results).enumerate() {
        match (result, &options.out) {
            (Ok(rendered), Some(dir)) => {
                let path = output_path(dir, name, index);
                std::fs::write(&path, rendered.svg)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
            }
            (Ok(rendered), None) => {
                let mut stdout = std::io::stdout().lock();
                stdout
                    .write_all(rendered.svg.as_bytes())
                    .and_then(|()| stdout.write_all(b"\n"))
                    .map_err(|e| e.to_string())?;
            }
            (Err(error), _) => {
                failed = true;
                eprintln!("{name}: {error}");
            }
        }
    }
    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) if message.is_empty() => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("pelers: {message}");
            ExitCode::FAILURE
        }
    }
}
