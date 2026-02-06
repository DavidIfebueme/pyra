use assert_cmd::Command;
use predicates::str::contains;
use tempfile::NamedTempFile;
use tempfile::TempDir;
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
    assert!(bin.contains(&0x35));
    assert!(bin.contains(&0x39));
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

#[test]
fn pyra_build_erc20_contract() {
    let out_dir = TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build")
        .arg("../contracts/ERC20.pyra")
        .arg("--out-dir")
        .arg(out_dir.path())
        .assert()
        .success();

    assert!(out_dir.path().join("ERC20.abi").exists());
    assert!(out_dir.path().join("ERC20.bin").exists());
}

#[test]
fn pyra_build_vault_contract() {
    let out_dir = TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build")
        .arg("../contracts/Vault.pyra")
        .arg("--out-dir")
        .arg(out_dir.path())
        .assert()
        .success();

    assert!(out_dir.path().join("Vault.abi").exists());
    assert!(out_dir.path().join("Vault.bin").exists());
}

#[test]
fn pyra_build_gas_report() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "def t() -> bool: return true").unwrap();
    let path = file.path().to_path_buf();

    let out_dir = TempDir::new().unwrap();

    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pyra"));
    cmd.arg("build")
        .arg(&path)
        .arg("--out-dir")
        .arg(out_dir.path())
        .arg("--gas-report")
        .assert()
        .success()
        .stdout(contains("Gas Report"))
        .stdout(contains("gas"));
}
