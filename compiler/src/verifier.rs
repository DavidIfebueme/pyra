use crate::ir::{IrModule, IrOp};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum VerifyError {
    OrphanJump(usize),
    OrphanJumpI(usize),
    DuplicateLabel(usize),
    UnreachableCode,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::OrphanJump(l) => write!(f, "jump to undefined label {l}"),
            Self::OrphanJumpI(l) => write!(f, "conditional jump to undefined label {l}"),
            Self::DuplicateLabel(l) => write!(f, "duplicate label {l}"),
            Self::UnreachableCode => write!(f, "unreachable code after terminal instruction"),
        }
    }
}

pub fn verify_module(module: &IrModule) -> Vec<VerifyError> {
    let mut errors = Vec::new();
    for func in &module.functions {
        verify_ops(&func.ops, &mut errors);
    }
    verify_ops(&module.constructor_ops, &mut errors);
    errors
}

fn verify_ops(ops: &[IrOp], errors: &mut Vec<VerifyError>) {
    let mut defined_labels = HashSet::new();
    let mut referenced_labels = Vec::new();

    for op in ops {
        match op {
            IrOp::JumpDest(l) => {
                if !defined_labels.insert(*l) {
                    errors.push(VerifyError::DuplicateLabel(*l));
                }
            }
            IrOp::Jump(l) => referenced_labels.push((*l, false)),
            IrOp::JumpI(l) => referenced_labels.push((*l, true)),
            _ => {}
        }
    }

    for (label, conditional) in referenced_labels {
        if !defined_labels.contains(&label) {
            if conditional {
                errors.push(VerifyError::OrphanJumpI(label));
            } else {
                errors.push(VerifyError::OrphanJump(label));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{IrFunction, IrModule};

    fn make_module(ops: Vec<IrOp>) -> IrModule {
        IrModule {
            functions: vec![IrFunction {
                name: "test".into(),
                selector: [0; 4],
                ops,
                label: 0,
            }],
            constructor_ops: vec![],
            label_count: 1,
        }
    }

    #[test]
    fn valid_module_no_errors() {
        let module = make_module(vec![
            IrOp::Push(vec![42]),
            IrOp::JumpI(0),
            IrOp::JumpDest(0),
            IrOp::Return,
        ]);
        let errors = verify_module(&module);
        assert!(errors.is_empty());
    }

    #[test]
    fn orphan_jump() {
        let module = make_module(vec![
            IrOp::Jump(99),
            IrOp::Return,
        ]);
        let errors = verify_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], VerifyError::OrphanJump(99)));
    }

    #[test]
    fn orphan_jumpi() {
        let module = make_module(vec![
            IrOp::Push(vec![1]),
            IrOp::JumpI(50),
            IrOp::Return,
        ]);
        let errors = verify_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], VerifyError::OrphanJumpI(50)));
    }

    #[test]
    fn duplicate_label() {
        let module = make_module(vec![
            IrOp::JumpDest(0),
            IrOp::JumpDest(0),
            IrOp::Return,
        ]);
        let errors = verify_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], VerifyError::DuplicateLabel(0)));
    }

    #[test]
    fn verifies_constructor_too() {
        let module = IrModule {
            functions: vec![],
            constructor_ops: vec![IrOp::Jump(42), IrOp::Stop],
            label_count: 0,
        };
        let errors = verify_module(&module);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], VerifyError::OrphanJump(42)));
    }

    #[test]
    fn complex_valid_module() {
        let module = make_module(vec![
            IrOp::Push(vec![1]),
            IrOp::JumpI(0),
            IrOp::Push(vec![2]),
            IrOp::Jump(1),
            IrOp::JumpDest(0),
            IrOp::Push(vec![3]),
            IrOp::JumpDest(1),
            IrOp::Return,
        ]);
        let errors = verify_module(&module);
        assert!(errors.is_empty());
    }
}
