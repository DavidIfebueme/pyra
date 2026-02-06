use crate::storage::{StorageKind, StorageLayout};
use crate::{BinaryOp, Block, Expression, Function, Item, Program, Statement, UnaryOp};
use std::collections::HashMap;
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, Clone)]
pub enum IrOp {
    Push(Vec<u8>),
    Pop,
    Dup(u8),
    Swap(u8),
    Add,
    Sub,
    Mul,
    Div,
    SDiv,
    Mod,
    Exp,
    Lt,
    Gt,
    Eq,
    IsZero,
    And,
    Or,
    Not,
    Shr,
    MLoad,
    MStore,
    SLoad,
    SStore,
    Jump(usize),
    JumpI(usize),
    JumpDest(usize),
    Caller,
    CallValue,
    CallDataLoad,
    CallDataSize,
    Keccak256,
    Return,
    Revert,
    Log(u8),
    Stop,
    Invalid,
}

pub struct IrFunction {
    pub name: String,
    pub selector: [u8; 4],
    pub ops: Vec<IrOp>,
    pub label: usize,
}

pub struct IrModule {
    pub functions: Vec<IrFunction>,
    pub constructor_ops: Vec<IrOp>,
    pub label_count: usize,
}

struct LowerCtx {
    layout: StorageLayout,
    params: HashMap<String, usize>,
    locals: HashMap<String, usize>,
    next_mem: usize,
    label_count: usize,
}

impl LowerCtx {
    fn new(layout: StorageLayout) -> Self {
        Self {
            layout,
            params: HashMap::with_capacity(8),
            locals: HashMap::with_capacity(8),
            next_mem: 0x80,
            label_count: 0,
        }
    }

    fn fresh_label(&mut self) -> usize {
        let l = self.label_count;
        self.label_count += 1;
        l
    }

    fn alloc_local(&mut self, name: &str) -> usize {
        let off = self.next_mem;
        self.locals.insert(name.to_string(), off);
        self.next_mem += 32;
        off
    }

    fn reset_for_function(&mut self) {
        self.params.clear();
        self.locals.clear();
        self.next_mem = 0x80;
    }
}

pub fn lower_program(program: &Program) -> IrModule {
    let layout = StorageLayout::from_program(program);
    let mut ctx = LowerCtx::new(layout);
    let mut functions = Vec::new();
    let mut constructor_ops = Vec::new();

    for item in &program.items {
        if let Item::Const(c) = item {
            if let Some(slot) = ctx.layout.get(&c.name) {
                let slot_num = slot.slot;
                let mut ops = lower_expression(&mut ctx, &c.value);
                ops.push(IrOp::Push(u64_to_bytes(slot_num)));
                ops.push(IrOp::SStore);
                constructor_ops.extend(ops);
            }
        }
    }

    for item in &program.items {
        if let Item::Function(f) = item {
            ctx.reset_for_function();

            if f.name == "init" {
                for (i, p) in f.params.iter().enumerate() {
                    ctx.params.insert(p.name.clone(), 4 + 32 * i);
                }
                lower_block(&mut ctx, &f.body, &mut constructor_ops);
                continue;
            }

            let label = ctx.fresh_label();
            for (i, p) in f.params.iter().enumerate() {
                ctx.params.insert(p.name.clone(), 4 + 32 * i);
            }

            let mut ops = Vec::with_capacity(64);
            ops.push(IrOp::JumpDest(label));
            lower_block(&mut ctx, &f.body, &mut ops);

            if !ops.iter().any(|op| matches!(op, IrOp::Return | IrOp::Revert | IrOp::Stop)) {
                ops.push(IrOp::Stop);
            }

            let selector = compute_selector(f);
            functions.push(IrFunction {
                name: f.name.clone(),
                selector,
                ops,
                label,
            });
        }
    }

    let label_count = ctx.label_count;
    IrModule {
        functions,
        constructor_ops,
        label_count,
    }
}

fn lower_block(ctx: &mut LowerCtx, block: &Block, ops: &mut Vec<IrOp>) {
    for stmt in &block.statements {
        lower_statement(ctx, stmt, ops);
    }
}

fn lower_statement(ctx: &mut LowerCtx, stmt: &Statement, ops: &mut Vec<IrOp>) {
    match stmt {
        Statement::Return(Some(e)) => {
            lower_expression_into(ctx, e, ops);
            ops.push(IrOp::Push(vec![0x40]));
            ops.push(IrOp::MStore);
            ops.push(IrOp::Push(vec![0x20]));
            ops.push(IrOp::Push(vec![0x40]));
            ops.push(IrOp::Return);
        }
        Statement::Return(None) => {
            ops.push(IrOp::Stop);
        }
        Statement::Require(e) => {
            let continue_label = ctx.fresh_label();
            lower_expression_into(ctx, e, ops);
            ops.push(IrOp::JumpI(continue_label));
            ops.push(IrOp::Push(vec![0x00]));
            ops.push(IrOp::Push(vec![0x00]));
            ops.push(IrOp::Revert);
            ops.push(IrOp::JumpDest(continue_label));
        }
        Statement::Let(l) => {
            let off = ctx.alloc_local(&l.name);
            if let Some(v) = &l.value {
                lower_expression_into(ctx, v, ops);
                ops.push(IrOp::Push(usize_to_bytes(off)));
                ops.push(IrOp::MStore);
            }
        }
        Statement::Assign(a) => {
            lower_assign(ctx, &a.target, &a.value, ops);
        }
        Statement::If(if_stmt) => {
            lower_if(ctx, if_stmt, ops);
        }
        Statement::For(_) => {
            ops.push(IrOp::Stop);
        }
        Statement::While(while_stmt) => {
            lower_while(ctx, while_stmt, ops);
        }
        Statement::Emit(em) => {
            lower_emit(ctx, em, ops);
        }
        Statement::Expression(e) => {
            lower_expression_into(ctx, e, ops);
            ops.push(IrOp::Pop);
        }
    }
}

fn lower_assign(ctx: &mut LowerCtx, target: &Expression, value: &Expression, ops: &mut Vec<IrOp>) {
    match target {
        Expression::Identifier(name) => {
            lower_expression_into(ctx, value, ops);
            if let Some(&off) = ctx.locals.get(name) {
                ops.push(IrOp::Push(usize_to_bytes(off)));
                ops.push(IrOp::MStore);
            } else if let Some(slot) = ctx.layout.get(name) {
                ops.push(IrOp::Push(u64_to_bytes(slot.slot)));
                ops.push(IrOp::SStore);
            }
        }
        Expression::Index(base, key) => {
            if let Expression::Identifier(name) = base.as_ref() {
                if let Some(slot) = ctx.layout.get(name) {
                    let slot_num = slot.slot;
                    lower_expression_into(ctx, value, ops);
                    lower_mapping_key(ctx, key, slot_num, ops);
                    ops.push(IrOp::SStore);
                }
            }
        }
        _ => {}
    }
}

fn lower_mapping_key(ctx: &mut LowerCtx, key: &Expression, slot: u64, ops: &mut Vec<IrOp>) {
    lower_expression_into(ctx, key, ops);
    ops.push(IrOp::Push(vec![0x00]));
    ops.push(IrOp::MStore);
    ops.push(IrOp::Push(u64_to_bytes(slot)));
    ops.push(IrOp::Push(vec![0x20]));
    ops.push(IrOp::MStore);
    ops.push(IrOp::Push(vec![0x40]));
    ops.push(IrOp::Push(vec![0x00]));
    ops.push(IrOp::Keccak256);
}

fn lower_if(ctx: &mut LowerCtx, if_stmt: &crate::IfStatement, ops: &mut Vec<IrOp>) {
    let else_label = ctx.fresh_label();
    let end_label = ctx.fresh_label();

    lower_expression_into(ctx, &if_stmt.condition, ops);
    ops.push(IrOp::IsZero);
    ops.push(IrOp::JumpI(else_label));

    lower_block(ctx, &if_stmt.then_branch, ops);
    ops.push(IrOp::Jump(end_label));

    ops.push(IrOp::JumpDest(else_label));
    if let Some(eb) = &if_stmt.else_branch {
        lower_block(ctx, eb, ops);
    }

    ops.push(IrOp::JumpDest(end_label));
}

fn lower_while(ctx: &mut LowerCtx, while_stmt: &crate::WhileStatement, ops: &mut Vec<IrOp>) {
    let loop_label = ctx.fresh_label();
    let end_label = ctx.fresh_label();

    ops.push(IrOp::JumpDest(loop_label));
    lower_expression_into(ctx, &while_stmt.condition, ops);
    ops.push(IrOp::IsZero);
    ops.push(IrOp::JumpI(end_label));

    lower_block(ctx, &while_stmt.body, ops);
    ops.push(IrOp::Jump(loop_label));

    ops.push(IrOp::JumpDest(end_label));
}

fn lower_emit(ctx: &mut LowerCtx, em: &crate::EmitStatement, ops: &mut Vec<IrOp>) {
    if let Some(first_arg) = em.args.first() {
        lower_expression_into(ctx, first_arg, ops);
        ops.push(IrOp::Push(vec![0x00]));
        ops.push(IrOp::MStore);
    }
    let data_size = if em.args.is_empty() { 0u8 } else { 0x20 };
    ops.push(IrOp::Push(vec![data_size]));
    ops.push(IrOp::Push(vec![0x00]));
    ops.push(IrOp::Log(0));
}

fn lower_expression_into(ctx: &mut LowerCtx, expr: &Expression, ops: &mut Vec<IrOp>) {
    match expr {
        Expression::Number(n) => {
            ops.push(IrOp::Push(biguint_to_push_bytes(n)));
        }
        Expression::HexNumber(n) => {
            ops.push(IrOp::Push(biguint_to_push_bytes(n)));
        }
        Expression::Bool(b) => {
            ops.push(IrOp::Push(vec![u8::from(*b)]));
        }
        Expression::String(_) => {
            ops.push(IrOp::Push(vec![0]));
        }
        Expression::Bytes(b) => {
            if b.is_empty() {
                ops.push(IrOp::Push(vec![0]));
            } else {
                ops.push(IrOp::Push(b.clone()));
            }
        }
        Expression::Identifier(name) => {
            if let Some(&off) = ctx.params.get(name) {
                ops.push(IrOp::Push(usize_to_bytes(off)));
                ops.push(IrOp::CallDataLoad);
            } else if let Some(&off) = ctx.locals.get(name) {
                ops.push(IrOp::Push(usize_to_bytes(off)));
                ops.push(IrOp::MLoad);
            } else if let Some(slot) = ctx.layout.get(name) {
                if slot.kind == StorageKind::Value {
                    ops.push(IrOp::Push(u64_to_bytes(slot.slot)));
                    ops.push(IrOp::SLoad);
                }
            }
        }
        Expression::Member(base, field) => {
            if let Expression::Identifier(name) = base.as_ref() {
                match (name.as_str(), field.as_str()) {
                    ("msg", "sender") => ops.push(IrOp::Caller),
                    ("msg", "value") => ops.push(IrOp::CallValue),
                    _ => ops.push(IrOp::Push(vec![0])),
                }
            } else {
                ops.push(IrOp::Push(vec![0]));
            }
        }
        Expression::Index(base, key) => {
            if let Expression::Identifier(name) = base.as_ref() {
                if let Some(slot) = ctx.layout.get(name) {
                    lower_mapping_key(ctx, key, slot.slot, ops);
                    ops.push(IrOp::SLoad);
                }
            }
        }
        Expression::Binary(op, left, right) => {
            lower_expression_into(ctx, left, ops);
            lower_expression_into(ctx, right, ops);
            match op {
                BinaryOp::Add => ops.push(IrOp::Add),
                BinaryOp::Sub => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Sub);
                }
                BinaryOp::Mul => ops.push(IrOp::Mul),
                BinaryOp::Div => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Div);
                }
                BinaryOp::Mod => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Mod);
                }
                BinaryOp::Pow => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Exp);
                }
                BinaryOp::Equal => ops.push(IrOp::Eq),
                BinaryOp::NotEqual => {
                    ops.push(IrOp::Eq);
                    ops.push(IrOp::IsZero);
                }
                BinaryOp::Less => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Lt);
                }
                BinaryOp::Greater => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Gt);
                }
                BinaryOp::LessEqual => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Gt);
                    ops.push(IrOp::IsZero);
                }
                BinaryOp::GreaterEqual => {
                    ops.push(IrOp::Swap(1));
                    ops.push(IrOp::Lt);
                    ops.push(IrOp::IsZero);
                }
                BinaryOp::And => ops.push(IrOp::And),
                BinaryOp::Or => ops.push(IrOp::Or),
            }
        }
        Expression::Unary(op, operand) => {
            lower_expression_into(ctx, operand, ops);
            match op {
                UnaryOp::Not => ops.push(IrOp::IsZero),
                UnaryOp::Minus => {
                    ops.push(IrOp::Push(vec![0]));
                    ops.push(IrOp::Sub);
                }
            }
        }
        Expression::Call(callee, args) => {
            lower_expression_into(ctx, callee, ops);
            for arg in args {
                lower_expression_into(ctx, arg, ops);
            }
        }
        Expression::StructInit(_, _) => {
            ops.push(IrOp::Push(vec![0]));
        }
    }
}

fn lower_expression(ctx: &mut LowerCtx, expr: &Expression) -> Vec<IrOp> {
    let mut ops = Vec::with_capacity(8);
    lower_expression_into(ctx, expr, &mut ops);
    ops
}

pub fn compute_selector(func: &Function) -> [u8; 4] {
    let mut sig = func.name.clone();
    sig.push('(');
    for (i, p) in func.params.iter().enumerate() {
        if i > 0 {
            sig.push(',');
        }
        sig.push_str(&type_to_abi_string(&p.type_));
    }
    sig.push(')');

    let mut hasher = Keccak::v256();
    let mut output = [0u8; 32];
    hasher.update(sig.as_bytes());
    hasher.finalize(&mut output);

    [output[0], output[1], output[2], output[3]]
}

fn type_to_abi_string(ty: &crate::Type) -> String {
    match ty {
        crate::Type::Uint8 => "uint8".into(),
        crate::Type::Uint256 => "uint256".into(),
        crate::Type::Int256 => "int256".into(),
        crate::Type::Bool => "bool".into(),
        crate::Type::Address => "address".into(),
        crate::Type::Bytes => "bytes".into(),
        crate::Type::String => "string".into(),
        _ => "bytes".into(),
    }
}

fn biguint_to_push_bytes(n: &num_bigint::BigUint) -> Vec<u8> {
    let bytes = n.to_bytes_be();
    if bytes.is_empty() || (bytes.len() == 1 && bytes[0] == 0) {
        return vec![0];
    }
    bytes
}

fn u64_to_bytes(n: u64) -> Vec<u8> {
    if n == 0 {
        return vec![0];
    }
    let bytes = n.to_be_bytes();
    let start = bytes.iter().position(|&b| b != 0).unwrap_or(7);
    bytes[start..].to_vec()
}

fn usize_to_bytes(n: usize) -> Vec<u8> {
    u64_to_bytes(n as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_from_source;

    #[test]
    fn lower_return_constant() {
        let program = parse_from_source("def t() -> uint256: return 42").unwrap();
        let module = lower_program(&program);
        assert_eq!(module.functions.len(), 1);
        let ops = &module.functions[0].ops;
        assert!(matches!(ops[0], IrOp::JumpDest(0)));
        assert!(matches!(&ops[1], IrOp::Push(v) if v == &[42]));
        assert!(matches!(ops.last().unwrap(), IrOp::Return));
    }

    #[test]
    fn lower_binary_add() {
        let program = parse_from_source("def t() -> uint256: return 1 + 2").unwrap();
        let module = lower_program(&program);
        let ops = &module.functions[0].ops;
        let has_add = ops.iter().any(|op| matches!(op, IrOp::Add));
        assert!(has_add);
    }

    #[test]
    fn lower_param_access() {
        let program = parse_from_source("def t(x: uint256) -> uint256: return x").unwrap();
        let module = lower_program(&program);
        let ops = &module.functions[0].ops;
        let has_calldataload = ops.iter().any(|op| matches!(op, IrOp::CallDataLoad));
        assert!(has_calldataload);
    }

    #[test]
    fn lower_require() {
        let program = parse_from_source("def t():\n    require true\n").unwrap();
        let module = lower_program(&program);
        let ops = &module.functions[0].ops;
        let has_jumpi = ops.iter().any(|op| matches!(op, IrOp::JumpI(_)));
        let has_revert = ops.iter().any(|op| matches!(op, IrOp::Revert));
        assert!(has_jumpi);
        assert!(has_revert);
    }

    #[test]
    fn lower_state_write() {
        let program = parse_from_source("def t():\n    x = 42\n").unwrap();
        let module = lower_program(&program);
        let ops = &module.functions[0].ops;
        let has_sstore = ops.iter().any(|op| matches!(op, IrOp::SStore));
        assert!(has_sstore);
    }

    #[test]
    fn lower_mapping_access() {
        let program =
            parse_from_source("def t():\n    balances[msg.sender] = 100\n").unwrap();
        let module = lower_program(&program);
        let ops = &module.functions[0].ops;
        let has_keccak = ops.iter().any(|op| matches!(op, IrOp::Keccak256));
        let has_sstore = ops.iter().any(|op| matches!(op, IrOp::SStore));
        assert!(has_keccak);
        assert!(has_sstore);
    }

    #[test]
    fn lower_msg_sender() {
        let program = parse_from_source("def t():\n    balances[msg.sender] = 1\n").unwrap();
        let module = lower_program(&program);
        let ops = &module.functions[0].ops;
        let has_caller = ops.iter().any(|op| matches!(op, IrOp::Caller));
        assert!(has_caller);
    }

    #[test]
    fn selector_transfer() {
        let program =
            parse_from_source("def transfer(to: address, amount: uint256) -> bool: return true")
                .unwrap();
        let module = lower_program(&program);
        assert_eq!(module.functions[0].selector, [0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn lower_constructor_const() {
        let src = "const supply: uint256 = 100\n\ndef t() -> uint256: return supply\n";
        let program = parse_from_source(src).unwrap();
        let module = lower_program(&program);
        let has_sstore = module
            .constructor_ops
            .iter()
            .any(|op| matches!(op, IrOp::SStore));
        assert!(has_sstore);
    }

    #[test]
    fn lower_if_branch() {
        let src = "def t() -> uint256:\n    if true: return 1\n    else: return 2\n";
        let program = parse_from_source(src).unwrap();
        let module = lower_program(&program);
        let ops = &module.functions[0].ops;
        let jumpi_count = ops.iter().filter(|op| matches!(op, IrOp::JumpI(_))).count();
        let jumpdest_count = ops
            .iter()
            .filter(|op| matches!(op, IrOp::JumpDest(_)))
            .count();
        assert!(jumpi_count >= 1);
        assert!(jumpdest_count >= 2);
    }
}