use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_aria-javac")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("failed to run aria-javac")
}

fn read_golden(path: &str) -> String {
    fs::read_to_string(path).expect("failed to read golden file")
}

fn normalize_path_in_text(text: &str, path: &Path) -> String {
    let path_text = path.display().to_string();
    text.replace(&path_text, "<SRC>")
}

#[test]
fn help_output_matches_golden() {
    let output = run(&["--aria-backend=aria", "--help"]);
    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        read_golden("tests/golden/help.stdout")
    );
    assert!(output.stderr.is_empty(), "expected empty stderr");
}

#[test]
fn version_output_matches_golden() {
    let output = run(&["--aria-backend=aria", "-version"]);
    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        read_golden("tests/golden/version.stdout")
    );
    assert!(output.stderr.is_empty(), "expected empty stderr");
}

#[test]
fn unsupported_option_stderr_matches_golden() {
    let output = run(&["--aria-backend=aria", "--definitely-unsupported"]);
    assert!(
        !output.status.success(),
        "expected failure for unsupported option"
    );
    assert!(output.stdout.is_empty(), "expected empty stdout");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        read_golden("tests/golden/unsupported_option.stderr")
    );
}

#[test]
fn release_mismatch_stderr_matches_golden() {
    let output = run(&["--aria-backend=aria", "--release", "21", "A.java"]);
    assert!(
        !output.status.success(),
        "expected failure for non-17 release"
    );
    assert!(output.stdout.is_empty(), "expected empty stdout");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        read_golden("tests/golden/release21.stderr")
    );
}

#[test]
fn parse_error_diagnostic_matches_golden() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aria-golden-parse-{}", stamp));
    fs::create_dir_all(&dir).expect("mkdir");
    let src = dir.join("Bad.java");
    fs::write(
        &src,
        r#"public class Bad {
  public static void main(String[] args) {
    int x =
  }
}
"#,
    )
    .expect("write source");

    let output = run(&["--aria-backend=aria", src.to_string_lossy().as_ref()]);
    let _ = fs::remove_dir_all(&dir);

    assert!(!output.status.success(), "expected compilation failure");
    assert!(output.stdout.is_empty(), "expected empty stdout");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let normalized = normalize_path_in_text(&stderr, &src);
    assert_eq!(normalized, read_golden("tests/golden/parse_error.stderr"));
}
