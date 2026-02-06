use crate::ir::{lower_program, IrModule, IrOp};
use crate::Program;
use std::collections::HashMap;

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

struct Emitter {
    code: Vec<u8>,
    labels: HashMap<usize, usize>,
    patches: Vec<(usize, usize)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            code: Vec::with_capacity(4096),
            labels: HashMap::new(),
            patches: Vec::new(),
        }
    }

    fn byte(&mut self, b: u8) {
        self.code.push(b);
    }

    fn push_data(&mut self, data: &[u8]) {
        let n = data.len();
        debug_assert!(n > 0 && n <= 32);
        self.code.push(0x5f + n as u8);
        self.code.extend_from_slice(data);
    }

    fn label_ref(&mut self, label: usize) {
        self.code.push(0x61);
        let pos = self.code.len();
        self.code.push(0x00);
        self.code.push(0x00);
        self.patches.push((pos, label));
    }

    fn mark_label(&mut self, label: usize) {
        self.labels.insert(label, self.code.len());
        self.code.push(0x5b);
    }

    fn resolve(&mut self) {
        for &(pos, label) in &self.patches {
            if let Some(&offset) = self.labels.get(&label) {
                let bytes = (offset as u16).to_be_bytes();
                self.code[pos] = bytes[0];
                self.code[pos + 1] = bytes[1];
            }
        }
    }

    fn emit_op(&mut self, op: &IrOp) {
        match op {
            IrOp::Push(data) => self.push_data(data),
            IrOp::Pop => self.byte(0x50),
            IrOp::Dup(n) => self.byte(0x7f + n),
            IrOp::Swap(n) => self.byte(0x8f + n),
            IrOp::Add => self.byte(0x01),
            IrOp::Mul => self.byte(0x02),
            IrOp::Sub => self.byte(0x03),
            IrOp::Div => self.byte(0x04),
            IrOp::SDiv => self.byte(0x05),
            IrOp::Mod => self.byte(0x06),
            IrOp::Exp => self.byte(0x0a),
            IrOp::Lt => self.byte(0x10),
            IrOp::Gt => self.byte(0x11),
            IrOp::Eq => self.byte(0x14),
            IrOp::IsZero => self.byte(0x15),
            IrOp::And => self.byte(0x16),
            IrOp::Or => self.byte(0x17),
            IrOp::Not => self.byte(0x19),
            IrOp::Shr => self.byte(0x1c),
            IrOp::MLoad => self.byte(0x51),
            IrOp::MStore => self.byte(0x52),
            IrOp::SLoad => self.byte(0x54),
            IrOp::SStore => self.byte(0x55),
            IrOp::Jump(label) => {
                self.label_ref(*label);
                self.byte(0x56);
            }
            IrOp::JumpI(label) => {
                self.label_ref(*label);
                self.byte(0x57);
            }
            IrOp::JumpDest(label) => {
                self.mark_label(*label);
            }
            IrOp::Caller => self.byte(0x33),
            IrOp::CallValue => self.byte(0x34),
            IrOp::CallDataLoad => self.byte(0x35),
            IrOp::CallDataSize => self.byte(0x36),
            IrOp::Keccak256 => self.byte(0x20),
            IrOp::Return => self.byte(0xf3),
            IrOp::Revert => self.byte(0xfd),
            IrOp::Log(n) => self.byte(0xa0 + n),
            IrOp::Stop => self.byte(0x00),
            IrOp::Invalid => self.byte(0xfe),
        }
    }

    fn into_bytes(mut self) -> Vec<u8> {
        self.resolve();
        self.code
    }
}

pub fn program_to_runtime_bytecode(program: &Program) -> Result<Vec<u8>, CodegenError> {
    let module = lower_program(program);
    module_to_runtime(&module)
}

pub fn program_to_deploy_bytecode(program: &Program) -> Result<Vec<u8>, CodegenError> {
    let module = lower_program(program);

    let mut ctor_em = Emitter::new();
    for op in &module.constructor_ops {
        match op {
            IrOp::Return | IrOp::Stop => {}
            _ => ctor_em.emit_op(op),
        }
    }
    let ctor_bytes = ctor_em.into_bytes();

    let runtime = module_to_runtime(&module)?;
    Ok(build_deploy(&ctor_bytes, &runtime))
}

fn module_to_runtime(module: &IrModule) -> Result<Vec<u8>, CodegenError> {
    let mut em = Emitter::new();

    if !module.functions.is_empty() {
        em.push_data(&[0x00]);
        em.byte(0x35);
        em.push_data(&[0xe0]);
        em.byte(0x1c);

        for func in &module.functions {
            em.byte(0x80);
            em.push_data(&func.selector);
            em.byte(0x14);
            em.label_ref(func.label);
            em.byte(0x57);
        }
    }

    em.push_data(&[0x00]);
    em.push_data(&[0x00]);
    em.byte(0xfd);

    for func in &module.functions {
        for (i, op) in func.ops.iter().enumerate() {
            em.emit_op(op);
            if i == 0 && matches!(op, IrOp::JumpDest(_)) {
                em.byte(0x50);
            }
        }
    }

    Ok(em.into_bytes())
}

fn build_deploy(constructor: &[u8], runtime: &[u8]) -> Vec<u8> {
    let mut cr_len = 0usize;
    for _ in 0..8 {
        let total_prefix = constructor.len() + cr_len;
        let mut cr = Vec::new();
        cr.extend(push_usize(runtime.len()));
        cr.extend(push_usize(total_prefix));
        cr.extend(push_usize(0));
        cr.push(0x39);
        cr.extend(push_usize(runtime.len()));
        cr.extend(push_usize(0));
        cr.push(0xf3);

        if cr.len() == cr_len {
            let mut out =
                Vec::with_capacity(constructor.len() + cr.len() + runtime.len());
            out.extend_from_slice(constructor);
            out.extend(cr);
            out.extend_from_slice(runtime);
            return out;
        }
        cr_len = cr.len();
    }

    let total_prefix = constructor.len() + cr_len;
    let mut out = Vec::from(constructor);
    out.extend(push_usize(runtime.len()));
    out.extend(push_usize(total_prefix));
    out.extend(push_usize(0));
    out.push(0x39);
    out.extend(push_usize(runtime.len()));
    out.extend(push_usize(0));
    out.push(0xf3);
    out.extend_from_slice(runtime);
    out
}

fn push_usize(value: usize) -> Vec<u8> {
    if value == 0 {
        return vec![0x60, 0x00];
    }
    let mut buf = [0u8; 32];
    let mut v = value;
    let mut i = 32;
    while v > 0 {
        i -= 1;
        buf[i] = (v & 0xff) as u8;
        v >>= 8;
    }
    let n = 32 - i;
    let mut out = Vec::with_capacity(1 + n);
    out.push(0x5f + (n as u8));
    out.extend_from_slice(&buf[i..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_from_source;

    #[test]
    fn runtime_starts_with_dispatcher() {
        let program = parse_from_source("def t() -> uint256: return 1").unwrap();
        let code = program_to_runtime_bytecode(&program).unwrap();
        assert!(!code.is_empty());
        assert_eq!(code[0], 0x60);
        assert_eq!(code[1], 0x00);
        assert_eq!(code[2], 0x35);
        assert_eq!(code[3], 0x60);
        assert_eq!(code[4], 0xe0);
        assert_eq!(code[5], 0x1c);
    }

    #[test]
    fn deploy_ends_with_runtime() {
        let program = parse_from_source("def t() -> uint256: return 1").unwrap();
        let runtime = program_to_runtime_bytecode(&program).unwrap();
        let deploy = program_to_deploy_bytecode(&program).unwrap();
        assert!(deploy.ends_with(&runtime));
        assert!(deploy.len() > runtime.len());
    }

    #[test]
    fn deploy_has_codecopy() {
        let program = parse_from_source("def t() -> uint256: return 1").unwrap();
        let deploy = program_to_deploy_bytecode(&program).unwrap();
        assert!(deploy.contains(&0x39));
    }

    #[test]
    fn runtime_contains_push_42() {
        let program = parse_from_source("def t() -> uint256: return 42").unwrap();
        let code = program_to_runtime_bytecode(&program).unwrap();
        let found = code.windows(2).any(|w| w[0] == 0x60 && w[1] == 0x2a);
        assert!(found);
    }

    #[test]
    fn runtime_has_jumpdest() {
        let program = parse_from_source("def t() -> uint256: return 1").unwrap();
        let code = program_to_runtime_bytecode(&program).unwrap();
        assert!(code.contains(&0x5b));
    }

    #[test]
    fn constructor_stores_constant() {
        let src = "const supply: uint256 = 100\n\ndef t() -> uint256: return supply\n";
        let program = parse_from_source(src).unwrap();
        let deploy = program_to_deploy_bytecode(&program).unwrap();
        let runtime = program_to_runtime_bytecode(&program).unwrap();
        let ctor_region = &deploy[..deploy.len() - runtime.len()];
        assert!(ctor_region.contains(&0x55));
    }

    #[test]
    fn runtime_reads_state_variable() {
        let src = "const supply: uint256 = 100\n\ndef t() -> uint256: return supply\n";
        let program = parse_from_source(src).unwrap();
        let code = program_to_runtime_bytecode(&program).unwrap();
        assert!(code.contains(&0x54));
    }
}
