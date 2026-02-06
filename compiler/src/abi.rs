use crate::{Block, EventDef, Function, Item, Parameter, Program, Statement, Type};

#[derive(thiserror::Error, Debug)]
pub enum AbiError {
    #[error("unsupported type: {0}")]
    UnsupportedType(String),
}

pub fn program_to_abi_json(program: &Program) -> Result<String, AbiError> {
    let mut out = String::with_capacity(1024);
    out.push('[');
    let mut first = true;

    for item in &program.items {
        match item {
            Item::Function(func) => {
                if !first { out.push(','); }
                first = false;
                if func.name == "init" {
                    emit_constructor(&mut out, func)?;
                } else {
                    emit_function(&mut out, func)?;
                }
            }
            Item::Event(event) => {
                if !first { out.push(','); }
                first = false;
                emit_event(&mut out, event)?;
            }
            _ => {}
        }
    }

    out.push(']');
    Ok(out)
}

fn emit_function(out: &mut String, func: &Function) -> Result<(), AbiError> {
    out.push('{');
    out.push_str("\"type\":\"function\"");
    out.push_str(",\"name\":\"");
    push_escaped(out, &func.name);
    out.push('"');
    out.push_str(",\"stateMutability\":\"");
    out.push_str(detect_mutability(func));
    out.push('"');
    emit_inputs(out, &func.params)?;
    emit_outputs(out, &func.return_type)?;
    out.push('}');
    Ok(())
}

fn emit_constructor(out: &mut String, func: &Function) -> Result<(), AbiError> {
    out.push('{');
    out.push_str("\"type\":\"constructor\"");
    out.push_str(",\"stateMutability\":\"nonpayable\"");
    emit_inputs(out, &func.params)?;
    out.push('}');
    Ok(())
}

fn emit_event(out: &mut String, event: &EventDef) -> Result<(), AbiError> {
    out.push('{');
    out.push_str("\"type\":\"event\"");
    out.push_str(",\"name\":\"");
    push_escaped(out, &event.name);
    out.push('"');
    out.push_str(",\"inputs\":[");
    let mut first = true;
    for field in &event.fields {
        if !first { out.push(','); }
        first = false;
        out.push('{');
        out.push_str("\"name\":\"");
        push_escaped(out, &field.name);
        out.push('"');
        out.push_str(",\"type\":\"");
        out.push_str(&abi_type(&field.type_)?);
        out.push('"');
        out.push_str(",\"indexed\":false");
        out.push('}');
    }
    out.push(']');
    out.push('}');
    Ok(())
}

fn emit_inputs(out: &mut String, params: &[Parameter]) -> Result<(), AbiError> {
    out.push_str(",\"inputs\":[");
    let mut first = true;
    for p in params {
        if !first { out.push(','); }
        first = false;
        out.push('{');
        out.push_str("\"name\":\"");
        push_escaped(out, &p.name);
        out.push('"');
        out.push_str(",\"type\":\"");
        out.push_str(&abi_type(&p.type_)?);
        out.push('"');
        out.push('}');
    }
    out.push(']');
    Ok(())
}

fn emit_outputs(out: &mut String, ret: &Option<Type>) -> Result<(), AbiError> {
    out.push_str(",\"outputs\":[");
    if let Some(ty) = ret {
        out.push('{');
        out.push_str("\"name\":\"\"");
        out.push_str(",\"type\":\"");
        out.push_str(&abi_output_type(ty)?);
        out.push('"');
        out.push('}');
    }
    out.push(']');
    Ok(())
}

fn abi_type(ty: &Type) -> Result<String, AbiError> {
    match ty {
        Type::Uint8 => Ok("uint8".to_string()),
        Type::Uint256 => Ok("uint256".to_string()),
        Type::Int256 => Ok("int256".to_string()),
        Type::Bool => Ok("bool".to_string()),
        Type::Address => Ok("address".to_string()),
        Type::Bytes => Ok("bytes".to_string()),
        Type::String => Ok("string".to_string()),
        Type::Custom(name) => Err(AbiError::UnsupportedType(name.clone())),
        Type::Vec(_) => Err(AbiError::UnsupportedType("Vec".to_string())),
        Type::Map(_, _) => Err(AbiError::UnsupportedType("Map".to_string())),
        Type::Generic(name, _) => Err(AbiError::UnsupportedType(name.clone())),
    }
}

fn abi_output_type(ty: &Type) -> Result<String, AbiError> {
    match ty {
        Type::Custom(_) => Ok("bytes".to_string()),
        _ => abi_type(ty),
    }
}

fn detect_mutability(func: &Function) -> &'static str {
    if body_has_writes(&func.body) {
        "nonpayable"
    } else {
        "view"
    }
}

fn body_has_writes(block: &Block) -> bool {
    block.statements.iter().any(|s| match s {
        Statement::Assign(_) | Statement::Emit(_) => true,
        Statement::If(if_stmt) => {
            body_has_writes(&if_stmt.then_branch)
                || if_stmt.else_branch.as_ref().map_or(false, body_has_writes)
        }
        Statement::For(for_stmt) => body_has_writes(&for_stmt.body),
        Statement::While(while_stmt) => body_has_writes(&while_stmt.body),
        _ => false,
    })
}

fn push_escaped(dst: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => dst.push_str("\\\""),
            '\\' => dst.push_str("\\\\"),
            '\n' => dst.push_str("\\n"),
            '\r' => dst.push_str("\\r"),
            '\t' => dst.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write;
                let _ = write!(dst, "\\u{:04x}", c as u32);
            }
            _ => dst.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_from_source;

    #[test]
    fn abi_json_for_view_function() {
        let program = parse_from_source("def t(a: uint256) -> bool: return true").unwrap();
        let abi = program_to_abi_json(&program).unwrap();
        assert_eq!(abi, "[{\"type\":\"function\",\"name\":\"t\",\"stateMutability\":\"view\",\"inputs\":[{\"name\":\"a\",\"type\":\"uint256\"}],\"outputs\":[{\"name\":\"\",\"type\":\"bool\"}]}]");
    }

    #[test]
    fn abi_json_for_nonpayable_function() {
        let program = parse_from_source("def t():\n    x = 1\n").unwrap();
        let abi = program_to_abi_json(&program).unwrap();
        assert!(abi.contains("\"stateMutability\":\"nonpayable\""));
    }

    #[test]
    fn abi_json_for_constructor() {
        let program = parse_from_source("def init(supply: uint256) -> bool: return true").unwrap();
        let abi = program_to_abi_json(&program).unwrap();
        assert!(abi.contains("\"type\":\"constructor\""));
        assert!(!abi.contains("\"name\":\"init\""));
    }

    #[test]
    fn abi_json_for_event() {
        let source = "event Transfer(from: address, to: address, amount: uint256)\n\ndef t() -> bool: return true\n";
        let program = parse_from_source(source).unwrap();
        let abi = program_to_abi_json(&program).unwrap();
        assert!(abi.contains("\"type\":\"event\""));
        assert!(abi.contains("\"name\":\"Transfer\""));
        assert!(abi.contains("\"indexed\":false"));
    }

    #[test]
    fn abi_rejects_unknown_type() {
        let program = parse_from_source("def t(a: Foo) -> bool: return true").unwrap();
        let err = program_to_abi_json(&program).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unsupported type"));
    }
}
