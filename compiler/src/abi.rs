use crate::{Item, Program, Type};

#[derive(thiserror::Error, Debug)]
pub enum AbiError {
    #[error("unsupported type: {0}")]
    UnsupportedType(String),
}

pub fn program_to_abi_json(program: &Program) -> Result<String, AbiError> {
    let mut out = String::from("[");
    let mut first_fn = true;

    for item in &program.items {
        let func = match item {
            Item::Function(f) => f,
            _ => continue,
        };

        if !first_fn {
            out.push(',');
        }
        first_fn = false;

        out.push('{');
        out.push_str("\"type\":\"function\"");
        out.push_str(",\"name\":\"");
        push_escaped_json_str(&mut out, &func.name);
        out.push('"');

        out.push_str(",\"stateMutability\":\"nonpayable\"");

        out.push_str(",\"inputs\":[");
        let mut first_in = true;
        for p in &func.params {
            if !first_in {
                out.push(',');
            }
            first_in = false;
            out.push('{');
            out.push_str("\"name\":\"");
            push_escaped_json_str(&mut out, &p.name);
            out.push('"');
            out.push_str(",\"type\":\"");
            out.push_str(&abi_type(&p.type_)?);
            out.push('"');
            out.push('}');
        }
        out.push(']');

        out.push_str(",\"outputs\":[");
        if let Some(ret) = &func.return_type {
            out.push('{');
            out.push_str("\"name\":\"\"");
            out.push_str(",\"type\":\"");
            out.push_str(&abi_output_type(ret)?);
            out.push('"');
            out.push('}');
        }
        out.push(']');

        out.push('}');
    }

    out.push(']');
    Ok(out)
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

fn push_escaped_json_str(dst: &mut String, s: &str) {
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
    fn abi_json_for_simple_function() {
        let program = parse_from_source("def t(a: uint256) -> bool: return true").unwrap();
        let abi = program_to_abi_json(&program).unwrap();
        assert_eq!(abi, "[{\"type\":\"function\",\"name\":\"t\",\"stateMutability\":\"nonpayable\",\"inputs\":[{\"name\":\"a\",\"type\":\"uint256\"}],\"outputs\":[{\"name\":\"\",\"type\":\"bool\"}]}]");
    }

    #[test]
    fn abi_rejects_unknown_type() {
        let program = parse_from_source("def t(a: Foo) -> bool: return true").unwrap();
        let err = program_to_abi_json(&program).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unsupported type"));
    }
}
