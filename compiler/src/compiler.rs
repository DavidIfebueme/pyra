use crate::parser::{parse_from_source, ParseError};
use crate::{program_to_abi_json, AbiError};
use crate::{program_to_runtime_bytecode, CodegenError};
use crate::Program;
use std::path::Path;
use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum CompileError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("parse failed")]
    Parse(Vec<ParseError>),

    #[error("abi failed: {0}")]
    Abi(#[from] AbiError),

    #[error("codegen failed: {0}")]
    Codegen(#[from] CodegenError),
}

pub fn compile_file(path: &Path) -> Result<Program, CompileError> {
    let source = std::fs::read_to_string(path)?;
    parse_from_source(&source).map_err(CompileError::Parse)
}

pub fn compile_file_to_abi(path: &Path, out_dir: Option<&Path>) -> Result<PathBuf, CompileError> {
    let program = compile_file(path)?;
    let abi = program_to_abi_json(&program)?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid input path"))?;

    let dir = match out_dir {
        Some(d) => d.to_path_buf(),
        None => path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    std::fs::create_dir_all(&dir)?;
    let out_path = dir.join(format!("{stem}.abi"));
    std::fs::write(&out_path, abi)?;
    Ok(out_path)
}

pub fn compile_file_to_abi_and_bin(
    path: &Path,
    out_dir: Option<&Path>,
) -> Result<(PathBuf, PathBuf), CompileError> {
    let program = compile_file(path)?;
    let abi = program_to_abi_json(&program)?;
    let bin = program_to_runtime_bytecode(&program)?;

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid input path"))?;

    let dir = match out_dir {
        Some(d) => d.to_path_buf(),
        None => path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    };

    std::fs::create_dir_all(&dir)?;

    let abi_path = dir.join(format!("{stem}.abi"));
    std::fs::write(&abi_path, abi)?;

    let bin_path = dir.join(format!("{stem}.bin"));
    std::fs::write(&bin_path, hex::encode(bin))?;

    Ok((abi_path, bin_path))
}
