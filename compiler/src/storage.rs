use std::collections::HashMap;
use crate::{Expression, Item, Program, Statement, Type};

#[derive(Debug, Clone, PartialEq)]
pub enum StorageKind {
    Value,
    Mapping,
}

#[derive(Debug, Clone)]
pub struct StorageSlot {
    pub slot: u64,
    pub kind: StorageKind,
}

#[derive(Debug, Clone)]
pub struct StorageLayout {
    slots: HashMap<String, StorageSlot>,
    next_slot: u64,
}

impl StorageLayout {
    pub fn from_program(program: &Program) -> Self {
        let mut layout = Self {
            slots: HashMap::with_capacity(16),
            next_slot: 0,
        };

        for item in &program.items {
            if let Item::Const(c) = item {
                let kind = match &c.type_ {
                    Type::Map(_, _) => StorageKind::Mapping,
                    _ => StorageKind::Value,
                };
                layout.alloc(&c.name, kind);
            }
        }

        for item in &program.items {
            if let Item::Struct(s) = item {
                for field in &s.fields {
                    let kind = match &field.type_ {
                        Type::Map(_, _) => StorageKind::Mapping,
                        _ => StorageKind::Value,
                    };
                    layout.alloc(&field.name, kind);
                }
            }
        }

        for item in &program.items {
            if let Item::Function(f) = item {
                let mut locals: Vec<&str> = f.params.iter().map(|p| p.name.as_str()).collect();
                discover_state(&f.body.statements, &mut locals, &mut layout);
            }
        }

        layout
    }

    fn alloc(&mut self, name: &str, kind: StorageKind) {
        if !self.slots.contains_key(name) {
            self.slots.insert(name.to_string(), StorageSlot {
                slot: self.next_slot,
                kind,
            });
            self.next_slot += 1;
        }
    }

    pub fn get(&self, name: &str) -> Option<&StorageSlot> {
        self.slots.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &StorageSlot)> {
        self.slots.iter()
    }

    pub fn slot_count(&self) -> u64 {
        self.next_slot
    }
}

fn discover_state<'a>(
    stmts: &'a [Statement],
    locals: &mut Vec<&'a str>,
    layout: &mut StorageLayout,
) {
    for stmt in stmts {
        match stmt {
            Statement::Let(l) => {
                if let Some(v) = &l.value {
                    discover_expr_mappings(v, locals, layout);
                }
                locals.push(&l.name);
            }
            Statement::Assign(a) => {
                discover_target(&a.target, locals, layout);
                discover_expr_mappings(&a.value, locals, layout);
            }
            Statement::Return(Some(e)) | Statement::Require(e) | Statement::Expression(e) => {
                discover_expr_mappings(e, locals, layout);
            }
            Statement::Emit(em) => {
                for arg in &em.args {
                    discover_expr_mappings(arg, locals, layout);
                }
            }
            Statement::If(if_stmt) => {
                discover_expr_mappings(&if_stmt.condition, locals, layout);
                discover_state(&if_stmt.then_branch.statements, locals, layout);
                if let Some(eb) = &if_stmt.else_branch {
                    discover_state(&eb.statements, locals, layout);
                }
            }
            Statement::For(for_stmt) => {
                discover_expr_mappings(&for_stmt.iterable, locals, layout);
                let mut inner = locals.clone();
                inner.push(&for_stmt.var);
                discover_state(&for_stmt.body.statements, &mut inner, layout);
            }
            Statement::While(while_stmt) => {
                discover_expr_mappings(&while_stmt.condition, locals, layout);
                discover_state(&while_stmt.body.statements, locals, layout);
            }
            Statement::Return(None) => {}
        }
    }
}

fn discover_target(expr: &Expression, locals: &[&str], layout: &mut StorageLayout) {
    match expr {
        Expression::Identifier(name) => {
            if !locals.contains(&name.as_str()) && !is_builtin(name) {
                layout.alloc(name, StorageKind::Value);
            }
        }
        Expression::Index(base, _) => {
            if let Expression::Identifier(name) = base.as_ref() {
                if !locals.contains(&name.as_str()) && !is_builtin(name) {
                    layout.alloc(name, StorageKind::Mapping);
                }
            }
        }
        Expression::Member(base, _) => {
            discover_target(base, locals, layout);
        }
        _ => {}
    }
}

fn discover_expr_mappings(expr: &Expression, locals: &[&str], layout: &mut StorageLayout) {
    match expr {
        Expression::Index(base, idx) => {
            if let Expression::Identifier(name) = base.as_ref() {
                if !locals.contains(&name.as_str()) && !is_builtin(name) {
                    layout.alloc(name, StorageKind::Mapping);
                }
            }
            discover_expr_mappings(idx, locals, layout);
        }
        Expression::Binary(_, l, r) => {
            discover_expr_mappings(l, locals, layout);
            discover_expr_mappings(r, locals, layout);
        }
        Expression::Unary(_, e) => {
            discover_expr_mappings(e, locals, layout);
        }
        Expression::Call(callee, args) => {
            discover_expr_mappings(callee, locals, layout);
            for arg in args {
                discover_expr_mappings(arg, locals, layout);
            }
        }
        Expression::Member(base, _) => {
            discover_expr_mappings(base, locals, layout);
        }
        _ => {}
    }
}

fn is_builtin(name: &str) -> bool {
    matches!(name, "msg" | "block" | "tx" | "self")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_from_source;

    #[test]
    fn layout_from_const() {
        let src = "const supply: uint256 = 100\n\ndef t() -> uint256: return supply\n";
        let program = parse_from_source(src).unwrap();
        let layout = StorageLayout::from_program(&program);
        let slot = layout.get("supply").unwrap();
        assert_eq!(slot.slot, 0);
        assert_eq!(slot.kind, StorageKind::Value);
    }

    #[test]
    fn layout_discovers_mapping_from_assign() {
        let src = "def t():\n    balances[msg.sender] = 100\n";
        let program = parse_from_source(src).unwrap();
        let layout = StorageLayout::from_program(&program);
        let slot = layout.get("balances").unwrap();
        assert_eq!(slot.kind, StorageKind::Mapping);
    }

    #[test]
    fn layout_discovers_mapping_from_read() {
        let src = "def t(owner: address) -> uint256: return balances[owner]\n";
        let program = parse_from_source(src).unwrap();
        let layout = StorageLayout::from_program(&program);
        let slot = layout.get("balances").unwrap();
        assert_eq!(slot.kind, StorageKind::Mapping);
    }

    #[test]
    fn layout_skips_locals_and_params() {
        let src = "def t(a: uint256):\n    let x = 1\n    x = 2\n    a = 3\n";
        let program = parse_from_source(src).unwrap();
        let layout = StorageLayout::from_program(&program);
        assert!(layout.get("a").is_none());
        assert!(layout.get("x").is_none());
    }

    #[test]
    fn layout_skips_builtins() {
        let src = "def t():\n    msg.sender = 1\n";
        let program = parse_from_source(src).unwrap();
        let layout = StorageLayout::from_program(&program);
        assert!(layout.get("msg").is_none());
    }

    #[test]
    fn layout_sequential_slots() {
        let src = "const a: uint256 = 1\nconst b: uint256 = 2\n\ndef t():\n    c = 3\n";
        let program = parse_from_source(src).unwrap();
        let layout = StorageLayout::from_program(&program);
        assert_eq!(layout.get("a").unwrap().slot, 0);
        assert_eq!(layout.get("b").unwrap().slot, 1);
        assert_eq!(layout.get("c").unwrap().slot, 2);
        assert_eq!(layout.slot_count(), 3);
    }
}
