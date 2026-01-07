use assert_cmd::Command;

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
