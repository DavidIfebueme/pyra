use std::collections::HashMap;
use crate::{
    BinaryOp, Block, Expression, Function, Item, Program, Statement, Type, UnaryOp,
};
use crate::storage::{StorageKind, StorageLayout};

#[derive(thiserror::Error, Debug, Clone)]
pub enum TypeError {
    #[error("undefined variable `{0}`")]
    Undefined(String),

    #[error("type mismatch: expected {expected}, got {got}")]
    Mismatch { expected: String, got: String },

    #[error("binary op `{op}` not supported for {left} and {right}")]
    BinaryOp { op: String, left: String, right: String },

    #[error("require condition must be bool, got {0}")]
    RequireBool(String),

    #[error("return type mismatch: expected {expected}, got {got}")]
    ReturnMismatch { expected: String, got: String },

    #[error("cannot index into non-mapping type {0}")]
    IndexNonMapping(String),

    #[error("duplicate definition `{0}`")]
    Duplicate(String),
}

struct Scope {
    vars: HashMap<String, Type>,
}

struct CheckCtx {
    globals: HashMap<String, Type>,
    scopes: Vec<Scope>,
    errors: Vec<TypeError>,
    current_return: Option<Type>,
}

impl CheckCtx {
    fn new() -> Self {
        Self {
            globals: HashMap::with_capacity(16),
            scopes: Vec::new(),
            errors: Vec::new(),
            current_return: None,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(Scope {
            vars: HashMap::with_capacity(8),
        });
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: &str, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.vars.insert(name.to_string(), ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.vars.get(name) {
                return Some(ty);
            }
        }
        self.globals.get(name)
    }

    fn err(&mut self, e: TypeError) {
        self.errors.push(e);
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(name, "msg" | "block" | "tx" | "self")
}

pub fn check_program(program: &Program) -> Vec<TypeError> {
    let mut ctx = CheckCtx::new();
    let layout = StorageLayout::from_program(program);

    for item in &program.items {
        if let Item::Const(c) = item {
            ctx.globals.insert(c.name.clone(), c.type_.clone());
        }
    }

    for (name, slot) in layout.iter() {
        if !ctx.globals.contains_key(name) {
            let ty = match slot.kind {
                StorageKind::Mapping => Type::Map(Box::new(Type::Uint256), Box::new(Type::Uint256)),
                StorageKind::Value => Type::Uint256,
            };
            ctx.globals.insert(name.clone(), ty);
        }
    }

    for item in &program.items {
        if let Item::Function(f) = item {
            check_function(&mut ctx, f);
        }
    }

    ctx.errors
}

fn check_function(ctx: &mut CheckCtx, func: &Function) {
    ctx.push_scope();
    ctx.current_return = func.return_type.clone();

    for p in &func.params {
        ctx.define(&p.name, p.type_.clone());
    }

    check_block(ctx, &func.body);

    ctx.current_return = None;
    ctx.pop_scope();
}

fn check_block(ctx: &mut CheckCtx, block: &Block) {
    for stmt in &block.statements {
        check_statement(ctx, stmt);
    }
}

fn check_statement(ctx: &mut CheckCtx, stmt: &Statement) {
    match stmt {
        Statement::Let(l) => {
            if let Some(val) = &l.value {
                let val_ty = infer_expression(ctx, val);
                if let (Some(declared), Some(inferred)) = (&l.type_, &val_ty) {
                    if !types_compatible(declared, inferred) {
                        ctx.err(TypeError::Mismatch {
                            expected: fmt_type(declared),
                            got: fmt_type(inferred),
                        });
                    }
                }
                let ty = l.type_.clone().or(val_ty).unwrap_or(Type::Uint256);
                ctx.define(&l.name, ty);
            } else {
                let ty = l.type_.clone().unwrap_or(Type::Uint256);
                ctx.define(&l.name, ty);
            }
        }
        Statement::Assign(a) => {
            let _target_ty = infer_expression(ctx, &a.target);
            let _val_ty = infer_expression(ctx, &a.value);
        }
        Statement::Return(Some(e)) => {
            let val_ty = infer_expression(ctx, e);
            if let (Some(expected), Some(got)) = (&ctx.current_return, &val_ty) {
                if !types_compatible(expected, got) {
                    ctx.err(TypeError::ReturnMismatch {
                        expected: fmt_type(expected),
                        got: fmt_type(got),
                    });
                }
            }
        }
        Statement::Return(None) => {}
        Statement::Require(e) => {
            let ty = infer_expression(ctx, e);
            if let Some(t) = &ty {
                if !matches!(t, Type::Bool) {
                    ctx.err(TypeError::RequireBool(fmt_type(t)));
                }
            }
        }
        Statement::If(if_stmt) => {
            let cond_ty = infer_expression(ctx, &if_stmt.condition);
            if let Some(t) = &cond_ty {
                if !matches!(t, Type::Bool) {
                    ctx.err(TypeError::Mismatch {
                        expected: "bool".into(),
                        got: fmt_type(t),
                    });
                }
            }
            check_block(ctx, &if_stmt.then_branch);
            if let Some(eb) = &if_stmt.else_branch {
                check_block(ctx, eb);
            }
        }
        Statement::For(for_stmt) => {
            ctx.push_scope();
            ctx.define(&for_stmt.var, Type::Uint256);
            check_block(ctx, &for_stmt.body);
            ctx.pop_scope();
        }
        Statement::While(while_stmt) => {
            let cond_ty = infer_expression(ctx, &while_stmt.condition);
            if let Some(t) = &cond_ty {
                if !matches!(t, Type::Bool) {
                    ctx.err(TypeError::Mismatch {
                        expected: "bool".into(),
                        got: fmt_type(t),
                    });
                }
            }
            check_block(ctx, &while_stmt.body);
        }
        Statement::Emit(em) => {
            for arg in &em.args {
                infer_expression(ctx, arg);
            }
        }
        Statement::Expression(e) => {
            infer_expression(ctx, e);
        }
    }
}

fn infer_expression(ctx: &mut CheckCtx, expr: &Expression) -> Option<Type> {
    match expr {
        Expression::Number(_) | Expression::HexNumber(_) => Some(Type::Uint256),
        Expression::Bool(_) => Some(Type::Bool),
        Expression::String(_) => Some(Type::String),
        Expression::Bytes(_) => Some(Type::Bytes),
        Expression::Identifier(name) => {
            if is_builtin(name) {
                None
            } else if let Some(ty) = ctx.lookup(name) {
                Some(ty.clone())
            } else {
                ctx.err(TypeError::Undefined(name.clone()));
                None
            }
        }
        Expression::Member(base, field) => {
            if let Expression::Identifier(name) = base.as_ref() {
                match (name.as_str(), field.as_str()) {
                    ("msg", "sender") => return Some(Type::Address),
                    ("msg", "value") => return Some(Type::Uint256),
                    ("block", "timestamp") => return Some(Type::Uint256),
                    ("block", "number") => return Some(Type::Uint256),
                    _ => {}
                }
            }
            infer_expression(ctx, base);
            None
        }
        Expression::Index(base, key) => {
            let base_ty = infer_expression(ctx, base);
            infer_expression(ctx, key);
            if let Some(Type::Map(_, v)) = base_ty {
                Some(*v)
            } else {
                None
            }
        }
        Expression::Binary(op, left, right) => {
            let lt = infer_expression(ctx, left);
            let rt = infer_expression(ctx, right);
            infer_binary_op(ctx, op, &lt, &rt)
        }
        Expression::Unary(op, operand) => {
            let t = infer_expression(ctx, operand);
            match op {
                UnaryOp::Not => Some(Type::Bool),
                UnaryOp::Minus => t,
            }
        }
        Expression::Call(callee, args) => {
            infer_expression(ctx, callee);
            for arg in args {
                infer_expression(ctx, arg);
            }
            None
        }
        Expression::StructInit(name, fields) => {
            for (_, val) in fields {
                infer_expression(ctx, val);
            }
            Some(Type::Custom(name.clone()))
        }
    }
}

fn infer_binary_op(
    ctx: &mut CheckCtx,
    op: &BinaryOp,
    left: &Option<Type>,
    right: &Option<Type>,
) -> Option<Type> {
    match op {
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
            if let (Some(l), Some(r)) = (left, right) {
                if is_numeric(l) && is_numeric(r) {
                    return Some(wider_numeric(l, r));
                }
                ctx.err(TypeError::BinaryOp {
                    op: format!("{:?}", op),
                    left: fmt_type(l),
                    right: fmt_type(r),
                });
            }
            Some(Type::Uint256)
        }
        BinaryOp::Equal | BinaryOp::NotEqual => Some(Type::Bool),
        BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
            Some(Type::Bool)
        }
        BinaryOp::And | BinaryOp::Or => {
            if let (Some(l), Some(r)) = (left, right) {
                if !matches!(l, Type::Bool) || !matches!(r, Type::Bool) {
                    ctx.err(TypeError::BinaryOp {
                        op: format!("{:?}", op),
                        left: fmt_type(l),
                        right: fmt_type(r),
                    });
                }
            }
            Some(Type::Bool)
        }
    }
}

fn is_numeric(ty: &Type) -> bool {
    matches!(ty, Type::Uint256 | Type::Uint8 | Type::Int256)
}

fn wider_numeric(a: &Type, b: &Type) -> Type {
    match (a, b) {
        (Type::Uint256, _) | (_, Type::Uint256) => Type::Uint256,
        (Type::Int256, _) | (_, Type::Int256) => Type::Int256,
        _ => a.clone(),
    }
}

fn types_compatible(expected: &Type, got: &Type) -> bool {
    if expected == got {
        return true;
    }
    if is_numeric(expected) && is_numeric(got) {
        return true;
    }
    false
}

fn fmt_type(ty: &Type) -> String {
    match ty {
        Type::Uint8 => "uint8".into(),
        Type::Uint256 => "uint256".into(),
        Type::Int256 => "int256".into(),
        Type::Bool => "bool".into(),
        Type::Address => "address".into(),
        Type::Bytes => "bytes".into(),
        Type::String => "string".into(),
        Type::Vec(inner) => format!("Vec<{}>", fmt_type(inner)),
        Type::Map(k, v) => format!("Map<{},{}>", fmt_type(k), fmt_type(v)),
        Type::Custom(name) => name.clone(),
        Type::Generic(name, args) => {
            let args_str: Vec<String> = args.iter().map(|a| fmt_type(a)).collect();
            format!("{}<{}>", name, args_str.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_from_source;

    #[test]
    fn accepts_valid_function() {
        let src = "def t(a: uint256) -> uint256: return a";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(errors.is_empty());
    }

    #[test]
    fn catches_return_type_mismatch() {
        let src = "def t(a: uint256) -> bool: return a";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(!errors.is_empty());
        assert!(errors[0].to_string().contains("return type mismatch"));
    }

    #[test]
    fn catches_require_non_bool() {
        let src = "def t():\n    require 42\n";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(!errors.is_empty());
        assert!(errors[0].to_string().contains("require"));
    }

    #[test]
    fn catches_undefined_variable() {
        let src = "def t() -> uint256: return x";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(!errors.is_empty());
        assert!(errors[0].to_string().contains("undefined"));
    }

    #[test]
    fn accepts_params_and_locals() {
        let src = "def t(a: uint256) -> uint256:\n    let b: uint256 = a\n    return b\n";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(errors.is_empty());
    }

    #[test]
    fn accepts_bool_comparison() {
        let src = "def t(a: uint256, b: uint256) -> bool: return a > b";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(errors.is_empty());
    }

    #[test]
    fn accepts_bool_and_or() {
        let src = "def t(a: bool, b: bool) -> bool: return a and b";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(errors.is_empty());
    }

    #[test]
    fn catches_and_non_bool() {
        let src = "def t(a: uint256, b: uint256) -> bool: return a and b";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(!errors.is_empty());
    }

    #[test]
    fn accepts_msg_sender() {
        let src = "def t() -> address: return msg.sender";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(errors.is_empty());
    }

    #[test]
    fn accepts_global_const() {
        let src = "const supply: uint256 = 100\n\ndef t() -> uint256: return supply\n";
        let program = parse_from_source(src).unwrap();
        let errors = check_program(&program);
        assert!(errors.is_empty());
    }
}