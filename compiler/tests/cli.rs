use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;
use std::io::Write;

#[test]
fn pyra_help_works() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("--help").assert().success();
}

#[test]
fn pyra_build_help_works() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build").arg("--help").assert().success();
}

#[test]
fn pyra_build_parses_valid_file() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "def t() -> bool: return true").unwrap();
    let path = file.path().to_path_buf();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build").arg(path).assert().success();
}

#[test]
fn pyra_build_fails_on_parse_error() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "def t( -> bool: return true").unwrap();
    let path = file.path().to_path_buf();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build")
        .arg(path)
        .assert()
        .failure()
        .stderr(contains("parse failed"));
}
