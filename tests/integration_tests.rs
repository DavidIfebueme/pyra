use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_compile_erc20_contract() {
    let temp_dir = TempDir::new().unwrap();
    let contract_path = temp_dir.path().join("ERC20.pyra");
    
    // Copy test contract to temp directory
    std::fs::copy("../contracts/ERC20.pyra", &contract_path).unwrap();
    
    // Run compiler
    let output = Command::new("../compiler/target/release/pyra")
        .arg("build")
        .arg(&contract_path)
        .output()
        .expect("Failed to execute compiler");
    
    assert!(output.status.success());
    
    // Check outputs exist
    assert!(temp_dir.path().join("ERC20.bin").exists());
    assert!(temp_dir.path().join("ERC20.abi").exists());
}

#[test]
fn test_compile_vault_contract() {
    let temp_dir = TempDir::new().unwrap();
    let contract_path = temp_dir.path().join("Vault.pyra");
    
    std::fs::copy("../contracts/Vault.pyra", &contract_path).unwrap();
    
    let output = Command::new("../compiler/target/release/pyra")
        .arg("build")
        .arg(&contract_path)
        .output()
        .expect("Failed to execute compiler");
    
    assert!(output.status.success());
    
    // Check outputs exist
    assert!(temp_dir.path().join("Vault.bin").exists());
    assert!(temp_dir.path().join("Vault.abi").exists());
}