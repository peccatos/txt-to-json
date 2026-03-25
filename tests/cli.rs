use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn binary_path() -> PathBuf {
    let current = std::env::current_exe().expect("current test exe");
    let debug_dir = current
        .parent()
        .and_then(|path| path.parent())
        .expect("debug dir");
    debug_dir.join(format!("txt-to-json{}", std::env::consts::EXE_SUFFIX))
}

fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn unique_temp_dir() -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("txt-to-json-{}-{}", std::process::id(), unique));
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn run_cli(current_dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(binary_path())
        .args(args)
        .current_dir(current_dir)
        .output()
        .expect("binary should run")
}

fn run_cli_with_stdin(current_dir: &Path, args: &[&str], stdin_data: &str) -> std::process::Output {
    let mut child = Command::new(binary_path())
        .args(args)
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("binary should spawn");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_data.as_bytes()).expect("write stdin");
    }

    child.wait_with_output().expect("binary should run")
}

#[test]
fn compile_valid_sample_writes_root_output() {
    let dir = unique_temp_dir();
    let input = example_path("valid.eva");

    let output = run_cli(&dir, &["compile", input.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "compile should not write to stdout"
    );
    assert!(
        output.stderr.is_empty(),
        "compile should not write to stderr"
    );

    let out_path = dir.join("вывод.json");
    assert!(out_path.exists(), "compile must write ./вывод.json in cwd");

    let json: Value = serde_json::from_str(&fs::read_to_string(out_path).expect("output json"))
        .expect("valid output json");
    assert_eq!(json["meta"]["contract"], "calibration");
    assert_eq!(
        json["formulas"][0]["rhs"],
        "confidence * (1 - prediction_error)"
    );
    assert_eq!(json["pipeline"][0], "confidence_update");
}

#[test]
fn validate_valid_sample_returns_ok() {
    let dir = unique_temp_dir();
    let input = example_path("valid.eva");

    let output = run_cli(&dir, &["validate", input.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "validate should not write to stdout"
    );
    assert!(
        output.stderr.is_empty(),
        "validate should not write to stderr"
    );
    assert!(
        !dir.join("вывод.json").exists(),
        "validate must not write output"
    );
}

#[test]
fn print_ast_valid_sample_returns_deterministic_ast() {
    let dir = unique_temp_dir();
    let input = example_path("valid.eva");

    let first = run_cli(&dir, &["print-ast", input.to_str().unwrap()]);
    let second = run_cli(&dir, &["print-ast", input.to_str().unwrap()]);
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        second.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        !dir.join("вывод.json").exists(),
        "print-ast must not write output"
    );

    let stdout_first = String::from_utf8(first.stdout).expect("utf8");
    let stdout_second = String::from_utf8(second.stdout).expect("utf8");
    assert_eq!(stdout_first, stdout_second);

    let ast: Value = serde_json::from_str(&stdout_first).expect("valid ast json");
    assert_eq!(ast["formulas"][0]["rhs"]["Binary"]["op"], "Mul");
    assert!(stdout_first.contains("\"Paren\""));
}

#[test]
fn help_without_arguments_prints_usage() {
    let dir = unique_temp_dir();
    let output = run_cli(&dir, &[]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("txt-to-json - EVA DSL compiler"));
    assert!(stdout.contains("compile <path>"));
    assert!(stdout.contains("validate <path>"));
    assert!(stdout.contains("print-ast <path>"));
    assert!(stdout.contains("ui"));
}

#[test]
fn version_flag_prints_version() {
    let dir = unique_temp_dir();
    let output = run_cli(&dir, &["--version"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("txt-to-json"));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn interactive_ui_can_compile_sample() {
    let dir = unique_temp_dir();
    let input = example_path("valid.eva");
    let stdin = format!("1\n{}\nq\n", input.display());

    let output = run_cli_with_stdin(&dir, &["ui"], &stdin);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("interactive terminal interface"));
    assert!(stdout.contains("compiled and wrote ./вывод.json"));

    let out_path = dir.join("вывод.json");
    assert!(out_path.exists(), "ui must write ./вывод.json in cwd");

    let json: Value = serde_json::from_str(&fs::read_to_string(out_path).expect("output json"))
        .expect("valid output json");
    assert_eq!(json["meta"]["version"], "v1");
}

fn assert_structured_error(output: std::process::Output, kind: &str) {
    assert!(!output.status.success(), "command should fail");
    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    let value: Value = serde_json::from_str(&stderr).expect("structured error json");
    assert_eq!(value["kind"], kind);
    assert!(value.get("message").is_some());
    assert!(value.get("line").is_some());
}

#[test]
fn invalid_unknown_section_returns_structured_error() {
    let dir = unique_temp_dir();
    let input = example_path("invalid_unknown_section.eva");
    let output = run_cli(&dir, &["validate", input.to_str().unwrap()]);
    assert_structured_error(output, "UnknownSection");
}

#[test]
fn invalid_formula_returns_structured_error() {
    let dir = unique_temp_dir();
    let input = example_path("invalid_formula.eva");
    let output = run_cli(&dir, &["validate", input.to_str().unwrap()]);
    assert_structured_error(output, "InvalidFormula");
}

#[test]
fn invalid_unknown_variable_returns_structured_error() {
    let dir = unique_temp_dir();
    let input = example_path("invalid_unknown_variable.eva");
    let output = run_cli(&dir, &["validate", input.to_str().unwrap()]);
    assert_structured_error(output, "UnknownVariable");
}

#[test]
fn invalid_duplicate_meta_key_returns_structured_error() {
    let dir = unique_temp_dir();
    let input = example_path("invalid_duplicate_meta_key.eva");
    let output = run_cli(&dir, &["validate", input.to_str().unwrap()]);
    assert_structured_error(output, "DuplicateMetaKey");
}
