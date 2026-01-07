use crate::parser::{parse_from_source, ParseError};
use crate::Program;
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum CompileError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("parse failed")]
    Parse(Vec<ParseError>),
}

pub fn compile_file(path: &Path) -> Result<Program, CompileError> {
    let source = std::fs::read_to_string(path)?;
    parse_from_source(&source).map_err(CompileError::Parse)
}
