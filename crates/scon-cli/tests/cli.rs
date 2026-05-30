use std::fs;
use std::process::Command;

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "scon-cli-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn check_accepts_valid_file() {
    let file = temp_file("app.scon", "name = \"demo\"\n");
    let status = Command::new(env!("CARGO_BIN_EXE_scon"))
        .args(["check", file.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn print_outputs_resolved_scon() {
    let file = temp_file("app.scon", "name=\"demo\"\n");
    let output = Command::new(env!("CARGO_BIN_EXE_scon"))
        .args(["print", file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("name = \"demo\""));
}

#[test]
fn fmt_check_reports_unformatted_file() {
    let file = temp_file("app.scon", "name=\"demo\"\n");
    let status = Command::new(env!("CARGO_BIN_EXE_scon"))
        .args(["fmt", "--check", file.to_str().unwrap()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}
