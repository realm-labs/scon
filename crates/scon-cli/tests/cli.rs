use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct TestFile {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl TestFile {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn temp_file(name: &str, contents: &str) -> TestFile {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(name);
    fs::write(&path, contents).unwrap();
    TestFile { _dir: dir, path }
}

#[test]
fn check_accepts_valid_file() {
    let file = temp_file("app.scon", "name = \"demo\"\n");
    let status = Command::new(env!("CARGO_BIN_EXE_scon"))
        .args(["check", file.path().to_str().unwrap()])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn print_outputs_resolved_scon() {
    let file = temp_file("app.scon", "name=\"demo\"\n");
    let output = Command::new(env!("CARGO_BIN_EXE_scon"))
        .args(["print", file.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("name = \"demo\""));
}

#[test]
fn to_json_preserves_number_variants() {
    let file = temp_file(
        "app.scon",
        "signed = -1\nunsigned = 18446744073709551615\nfloat = 1.25\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_scon"))
        .args(["to-json", "--compact", file.path().to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["signed"], serde_json::json!(-1));
    assert_eq!(json["unsigned"], serde_json::json!(18446744073709551615u64));
    assert_eq!(json["float"], serde_json::json!(1.25));
}

#[test]
fn fmt_check_reports_unformatted_file() {
    let file = temp_file("app.scon", "name=\"demo\"\n");
    let status = Command::new(env!("CARGO_BIN_EXE_scon"))
        .args(["fmt", "--check", file.path().to_str().unwrap()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}
