use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_pelers");
const FLOW: &str = "flowchart LR\n  A --> B";

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let mut pipe = child.stdin.take().unwrap();
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).unwrap();
        }
    }
    child.wait_with_output().unwrap()
}

#[test]
fn renders_stdin_to_stdout() {
    let out = run(&[], Some(FLOW));
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("<svg"));
    assert!(text.ends_with("</svg>\n"));
}

#[test]
fn reads_stdin_when_named_with_a_dash() {
    let out = run(&["-"], Some(FLOW));
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().starts_with("<svg"));
}

#[test]
fn writes_one_svg_per_input_into_the_output_directory() {
    let dir = scratch("out-dir");
    let input = dir.join("flow.mmd");
    fs::write(&input, FLOW).unwrap();
    let out_dir = dir.join("svgs");
    let out = run(
        &["--out", out_dir.to_str().unwrap(), input.to_str().unwrap()],
        None,
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    let svg = fs::read_to_string(out_dir.join("flow.svg")).unwrap();
    assert!(svg.starts_with("<svg"));
}

#[test]
fn names_stdin_output_by_its_index_in_the_output_directory() {
    let dir = scratch("stdin-out");
    let out = run(&["--out", dir.to_str().unwrap(), "-"], Some(FLOW));
    assert!(out.status.success());
    assert!(dir.join("-.svg").exists());
}

#[test]
fn accepts_a_thread_count() {
    let out = run(&["--threads", "2"], Some(FLOW));
    assert!(out.status.success());
}

#[test]
fn rejects_a_zero_or_missing_thread_count() {
    for args in [&["--threads", "0"][..], &["--threads", "x"], &["--threads"]] {
        let out = run(args, Some(FLOW));
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("pelers:"));
    }
}

#[test]
fn rejects_an_out_flag_without_a_directory() {
    let out = run(&["--out"], Some(FLOW));
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--out needs a directory"));
}

#[test]
fn prints_usage_for_help() {
    for flag in ["-h", "--help"] {
        let out = run(&[flag], None);
        assert!(out.status.success());
        assert!(String::from_utf8_lossy(&out.stdout).starts_with("usage: pelers"));
    }
}

#[test]
fn fails_on_an_unknown_diagram() {
    let out = run(&[], Some("hello"));
    assert_eq!(out.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&out.stderr).is_empty());
}

#[test]
fn reports_a_missing_input_file() {
    let out = run(&["/nonexistent/pelers-input.mmd"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("pelers-input.mmd"));
}

#[test]
fn keeps_going_after_a_bad_input_and_fails_at_the_end() {
    let dir = scratch("mixed");
    let good = dir.join("good.mmd");
    let bad = dir.join("bad.mmd");
    fs::write(&good, FLOW).unwrap();
    fs::write(&bad, "hello").unwrap();
    let out_dir = dir.join("svgs");
    let out = run(
        &[
            "--out",
            out_dir.to_str().unwrap(),
            good.to_str().unwrap(),
            bad.to_str().unwrap(),
        ],
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(out_dir.join("good.svg").exists());
    assert!(String::from_utf8_lossy(&out.stderr).contains("bad.mmd"));
}

#[test]
fn reports_an_output_directory_that_cannot_be_created() {
    let dir = scratch("blocked");
    let blocker = dir.join("file");
    fs::write(&blocker, "x").unwrap();
    let out_dir = blocker.join("sub");
    let out = run(&["--out", out_dir.to_str().unwrap(), "-"], Some(FLOW));
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("pelers:"));
}

#[test]
fn reports_an_output_file_that_cannot_be_written() {
    let dir = scratch("unwritable");
    let input = dir.join("flow.mmd");
    fs::write(&input, FLOW).unwrap();
    let out_dir = dir.join("svgs");
    fs::create_dir_all(out_dir.join("flow.svg")).unwrap();
    let out = run(
        &["--out", out_dir.to_str().unwrap(), input.to_str().unwrap()],
        None,
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("flow.svg"));
}
