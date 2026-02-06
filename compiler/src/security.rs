use crate::ir::{IrModule, IrOp};

pub fn harden(module: &mut IrModule) {
    for func in &mut module.functions {
        func.ops = harden_ops(&func.ops, &mut module.label_count);
    }
    module.constructor_ops = harden_ops(&module.constructor_ops, &mut module.label_count);
}

fn harden_ops(ops: &[IrOp], label_count: &mut usize) -> Vec<IrOp> {
    let mut out = Vec::with_capacity(ops.len() * 2);
    for op in ops {
        match op {
            IrOp::Add => emit_checked_add(&mut out, label_count),
            IrOp::Sub => emit_checked_sub(&mut out, label_count),
            IrOp::Mul => emit_checked_mul(&mut out, label_count),
            other => out.push(other.clone()),
        }
    }
    out
}

fn emit_checked_add(out: &mut Vec<IrOp>, label_count: &mut usize) {
    let ok_label = *label_count;
    *label_count += 1;
    out.push(IrOp::Dup(2));
    out.push(IrOp::Dup(2));
    out.push(IrOp::Add);
    out.push(IrOp::Dup(1));
    out.push(IrOp::Dup(3));
    out.push(IrOp::Lt);
    out.push(IrOp::IsZero);
    out.push(IrOp::JumpI(ok_label));
    out.push(IrOp::Push(vec![0]));
    out.push(IrOp::Push(vec![0]));
    out.push(IrOp::Revert);
    out.push(IrOp::JumpDest(ok_label));
    out.push(IrOp::Swap(2));
    out.push(IrOp::Pop);
    out.push(IrOp::Swap(1));
    out.push(IrOp::Pop);
}

fn emit_checked_sub(out: &mut Vec<IrOp>, label_count: &mut usize) {
    let ok_label = *label_count;
    *label_count += 1;
    out.push(IrOp::Dup(2));
    out.push(IrOp::Dup(2));
    out.push(IrOp::Lt);
    out.push(IrOp::IsZero);
    out.push(IrOp::JumpI(ok_label));
    out.push(IrOp::Push(vec![0]));
    out.push(IrOp::Push(vec![0]));
    out.push(IrOp::Revert);
    out.push(IrOp::JumpDest(ok_label));
    out.push(IrOp::Sub);
}

fn emit_checked_mul(out: &mut Vec<IrOp>, label_count: &mut usize) {
    let ok_label = *label_count;
    let zero_label = *label_count + 1;
    *label_count += 2;
    out.push(IrOp::Dup(2));
    out.push(IrOp::IsZero);
    out.push(IrOp::JumpI(zero_label));
    out.push(IrOp::Dup(2));
    out.push(IrOp::Dup(2));
    out.push(IrOp::Mul);
    out.push(IrOp::Dup(1));
    out.push(IrOp::Dup(3));
    out.push(IrOp::Div);
    out.push(IrOp::Dup(4));
    out.push(IrOp::Eq);
    out.push(IrOp::JumpI(ok_label));
    out.push(IrOp::Push(vec![0]));
    out.push(IrOp::Push(vec![0]));
    out.push(IrOp::Revert);
    out.push(IrOp::JumpDest(zero_label));
    out.push(IrOp::Pop);
    out.push(IrOp::Pop);
    out.push(IrOp::Push(vec![0]));
    out.push(IrOp::Jump(ok_label));
    out.push(IrOp::JumpDest(ok_label));
    out.push(IrOp::Swap(2));
    out.push(IrOp::Pop);
    out.push(IrOp::Swap(1));
    out.push(IrOp::Pop);
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
    fn harden_replaces_add() {
        let mut module = make_module(vec![
            IrOp::Push(vec![1]),
            IrOp::Push(vec![2]),
            IrOp::Add,
            IrOp::Return,
        ]);
        harden(&mut module);
        let ops = &module.functions[0].ops;
        assert!(ops.len() > 4);
        assert!(ops.iter().any(|op| matches!(op, IrOp::Revert)));
        assert!(ops.iter().any(|op| matches!(op, IrOp::JumpDest(_))));
    }

    #[test]
    fn harden_replaces_sub() {
        let mut module = make_module(vec![
            IrOp::Push(vec![5]),
            IrOp::Push(vec![3]),
            IrOp::Sub,
            IrOp::Return,
        ]);
        harden(&mut module);
        let ops = &module.functions[0].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Revert)));
        assert!(ops.iter().filter(|op| matches!(op, IrOp::Sub)).count() == 1);
    }

    #[test]
    fn harden_replaces_mul() {
        let mut module = make_module(vec![
            IrOp::Push(vec![3]),
            IrOp::Push(vec![4]),
            IrOp::Mul,
            IrOp::Return,
        ]);
        harden(&mut module);
        let ops = &module.functions[0].ops;
        assert!(ops.len() > 4);
        assert!(ops.iter().any(|op| matches!(op, IrOp::Revert)));
    }

    #[test]
    fn harden_leaves_sload_untouched() {
        let mut module = make_module(vec![
            IrOp::Push(vec![0]),
            IrOp::SLoad,
            IrOp::Return,
        ]);
        harden(&mut module);
        let ops = &module.functions[0].ops;
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn harden_constructor_too() {
        let mut module = IrModule {
            functions: vec![],
            constructor_ops: vec![
                IrOp::Push(vec![1]),
                IrOp::Push(vec![2]),
                IrOp::Add,
                IrOp::Stop,
            ],
            label_count: 0,
        };
        harden(&mut module);
        assert!(module.constructor_ops.len() > 4);
        assert!(module.constructor_ops.iter().any(|op| matches!(op, IrOp::Revert)));
    }

    #[test]
    fn harden_multiple_ops() {
        let mut module = make_module(vec![
            IrOp::Push(vec![1]),
            IrOp::Push(vec![2]),
            IrOp::Add,
            IrOp::Push(vec![3]),
            IrOp::Sub,
            IrOp::Push(vec![4]),
            IrOp::Mul,
            IrOp::Return,
        ]);
        harden(&mut module);
        let ops = &module.functions[0].ops;
        assert!(ops.iter().filter(|op| matches!(op, IrOp::Revert)).count() >= 3);
    }

    #[test]
    fn harden_unique_labels() {
        let mut module = make_module(vec![
            IrOp::Push(vec![1]),
            IrOp::Push(vec![2]),
            IrOp::Add,
            IrOp::Push(vec![3]),
            IrOp::Add,
            IrOp::Return,
        ]);
        harden(&mut module);
        let labels: Vec<usize> = module.functions[0]
            .ops
            .iter()
            .filter_map(|op| match op {
                IrOp::JumpDest(l) => Some(*l),
                _ => None,
            })
            .collect();
        let unique: std::collections::HashSet<usize> = labels.iter().copied().collect();
        assert_eq!(labels.len(), unique.len());
    }
}
