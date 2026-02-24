use std::process::Command;

fn cargo_run(args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .args(["run", "--package", "worklog-cli", "--quiet", "--"])
        .args(args)
        .output()
        .expect("Failed to execute cargo run")
}

#[test]
fn help_flag_shows_usage() {
    let output = cargo_run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("Aggregate git commits"));
}

#[test]
fn json_flag_produces_valid_json() {
    let output = cargo_run(&["--json", "--path", "/tmp"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    assert!(
        serde_json::from_str::<serde_json::Value>(trimmed).is_ok(),
        "Output was not valid JSON: {trimmed}"
    );
}

#[test]
fn invalid_period_shows_error() {
    let output = cargo_run(&["-p", "foobar"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("Unknown period") || stderr.contains("invalid value"),
        "Unexpected error: {stderr}"
    );
}

#[test]
fn nonexistent_path_shows_message() {
    let output = cargo_run(&["--path", "/tmp/nonexistent_worklog_test_dir"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No git repositories found") || output.status.success(),
        "Unexpected output: {stderr}"
    );
}
