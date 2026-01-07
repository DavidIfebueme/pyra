use crate::evm::runtime_return_word;
use crate::{BinaryOp, Expression, Item, Program, Statement};
use num_bigint::BigUint;

#[derive(thiserror::Error, Debug)]
pub enum CodegenError {
	#[error("no function found")]
	NoFunction,

	#[error("no return statement found")]
	NoReturn,

	#[error("unsupported expression")]
	UnsupportedExpression,

	#[error("division by zero")]
	DivisionByZero,

	#[error("underflow")]
	Underflow,
}

pub fn program_to_runtime_bytecode(program: &Program) -> Result<Vec<u8>, CodegenError> {
	let func = program
		.items
		.iter()
		.find_map(|it| match it {
			Item::Function(f) => Some(f),
			_ => None,
		})
		.ok_or(CodegenError::NoFunction)?;

	let expr = func
		.body
		.statements
		.iter()
		.find_map(|st| match st {
			Statement::Return(e) => Some(e.clone()),
			_ => None,
		})
		.ok_or(CodegenError::NoReturn)?;

	let value = match expr {
		Some(e) => eval_const_expr(&e)?,
		None => BigUint::from(0u8),
	};

	Ok(runtime_return_word(biguint_to_word(&value)))
}

fn eval_const_expr(expr: &Expression) -> Result<BigUint, CodegenError> {
	match expr {
		Expression::Number(n) => Ok(n.clone()),
		Expression::HexNumber(n) => Ok(n.clone()),
		Expression::Bool(b) => Ok(BigUint::from(if *b { 1u8 } else { 0u8 })),
		Expression::Binary(op, left, right) => {
			let a = eval_const_expr(left)?;
			let b = eval_const_expr(right)?;
			match op {
				BinaryOp::Add => Ok(a + b),
				BinaryOp::Sub => {
					if a < b {
						return Err(CodegenError::Underflow);
					}
					Ok(a - b)
				}
				BinaryOp::Mul => Ok(a * b),
				BinaryOp::Div => {
					if b == BigUint::from(0u8) {
						return Err(CodegenError::DivisionByZero);
					}
					Ok(a / b)
				}
				BinaryOp::Mod => {
					if b == BigUint::from(0u8) {
						return Err(CodegenError::DivisionByZero);
					}
					Ok(a % b)
				}
				_ => Err(CodegenError::UnsupportedExpression),
			}
		}
		_ => Err(CodegenError::UnsupportedExpression),
	}
}

fn biguint_to_word(value: &BigUint) -> [u8; 32] {
	let mut out = [0u8; 32];
	let bytes = value.to_bytes_be();
	let take = bytes.len().min(32);
	out[32 - take..].copy_from_slice(&bytes[bytes.len() - take..]);
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parser::parse_from_source;

	#[test]
	fn generates_bytecode_for_return_1() {
		let program = parse_from_source("def t() -> uint256: return 1").unwrap();
		let code = program_to_runtime_bytecode(&program).unwrap();
		assert!(!code.is_empty());
		assert_eq!(code[0], 0x7f);
		assert_eq!(code[40], 0xf3);
	}

	#[test]
	fn rejects_identifier_expression() {
		let program = parse_from_source("def t() -> uint256: return x").unwrap();
		let err = program_to_runtime_bytecode(&program).unwrap_err();
		let msg = format!("{err}");
		assert!(msg.contains("unsupported"));
	}
}
