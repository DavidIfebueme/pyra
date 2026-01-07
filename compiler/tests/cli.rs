use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;
use tempfile::TempDir;
use std::io::Write;
use pyra_compiler::evm::runtime_return_word;

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

    let out_dir = TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build")
        .arg(&path)
        .arg("--out-dir")
        .arg(out_dir.path())
        .assert()
        .success();

    let stem = path.file_stem().unwrap().to_str().unwrap();
    let abi_path = out_dir.path().join(format!("{stem}.abi"));
    assert!(abi_path.exists());

    let bin_path = out_dir.path().join(format!("{stem}.bin"));
    assert!(bin_path.exists());
    let bin_hex = std::fs::read_to_string(bin_path).unwrap();
    let bin = hex::decode(bin_hex.trim()).unwrap();
    assert!(!bin.is_empty());

    let mut word = [0u8; 32];
    word[31] = 1;
    let runtime = runtime_return_word(word);
    assert!(bin.ends_with(&runtime));
    let runtime_start = bin.len() - runtime.len();
    assert_eq!(bin[runtime_start], 0x7f);
    assert_eq!(bin[runtime_start - 1], 0xf3);
    assert!(bin[..runtime_start].contains(&0x39));
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

#[test]
fn pyra_build_parses_multiline_require() {
    let mut file = NamedTempFile::new().unwrap();
    write!(
        file,
        "def t() -> bool:\n    require true\n    return true\n"
    )
    .unwrap();
    let path = file.path().to_path_buf();

    let out_dir = TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build")
        .arg(&path)
        .arg("--out-dir")
        .arg(out_dir.path())
        .assert()
        .success();

    let stem = path.file_stem().unwrap().to_str().unwrap();
    assert!(out_dir.path().join(format!("{stem}.abi")).exists());
    assert!(out_dir.path().join(format!("{stem}.bin")).exists());
}
