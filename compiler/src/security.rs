use crate::ir::{IrModule, IrOp};

pub fn harden(module: &mut IrModule) {
    for func in &mut module.functions {
        func.ops = harden_ops(&func.ops, &mut module.label_count);
    }
    module.constructor_ops = harden_ops(&module.constructor_ops, &mut module.label_count);
}

pub fn add_reentrancy_guard(module: &mut IrModule, lock_slot: u64) {
    let slot_bytes = slot_to_bytes(lock_slot);
    for func in &mut module.functions {
        let body = std::mem::take(&mut func.ops);
        let mut guarded = Vec::with_capacity(body.len() + 16);
        let ok_label = module.label_count;
        module.label_count += 1;
        guarded.push(IrOp::Push(slot_bytes.clone()));
        guarded.push(IrOp::SLoad);
        guarded.push(IrOp::IsZero);
        guarded.push(IrOp::JumpI(ok_label));
        guarded.push(IrOp::Push(vec![0]));
        guarded.push(IrOp::Push(vec![0]));
        guarded.push(IrOp::Revert);
        guarded.push(IrOp::JumpDest(ok_label));
        guarded.push(IrOp::Push(vec![1]));
        guarded.push(IrOp::Push(slot_bytes.clone()));
        guarded.push(IrOp::SStore);
        for op in &body {
            match op {
                IrOp::Return | IrOp::Stop => {
                    guarded.push(IrOp::Push(vec![0]));
                    guarded.push(IrOp::Push(slot_bytes.clone()));
                    guarded.push(IrOp::SStore);
                    guarded.push(op.clone());
                }
                other => guarded.push(other.clone()),
            }
        }
        func.ops = guarded;
    }
}

fn slot_to_bytes(slot: u64) -> Vec<u8> {
    if slot == 0 {
        return vec![0];
    }
    let be = slot.to_be_bytes();
    let start = be.iter().position(|&b| b != 0).unwrap_or(7);
    be[start..].to_vec()
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

    #[test]
    fn reentrancy_guard_wraps_function() {
        let mut module = make_module(vec![
            IrOp::Push(vec![0]),
            IrOp::SLoad,
            IrOp::Return,
        ]);
        add_reentrancy_guard(&mut module, 5);
        let ops = &module.functions[0].ops;
        assert!(ops.len() > 3);
        assert!(ops.iter().any(|op| matches!(op, IrOp::Revert)));
        let sloads = ops.iter().filter(|op| matches!(op, IrOp::SLoad)).count();
        assert!(sloads >= 2);
        let sstores = ops.iter().filter(|op| matches!(op, IrOp::SStore)).count();
        assert!(sstores >= 2);
    }

    #[test]
    fn reentrancy_guard_clears_before_return() {
        let mut module = make_module(vec![
            IrOp::Push(vec![42]),
            IrOp::Return,
        ]);
        add_reentrancy_guard(&mut module, 0);
        let ops = &module.functions[0].ops;
        let return_idx = ops.iter().rposition(|op| matches!(op, IrOp::Return)).unwrap();
        let pre_return = &ops[return_idx - 3..return_idx];
        assert!(matches!(pre_return[0], IrOp::Push(ref v) if v == &[0]));
        assert!(matches!(pre_return[2], IrOp::SStore));
    }

    #[test]
    fn reentrancy_guard_uses_correct_slot() {
        let mut module = make_module(vec![IrOp::Stop]);
        add_reentrancy_guard(&mut module, 10);
        let ops = &module.functions[0].ops;
        assert!(ops.iter().any(|op| matches!(op, IrOp::Push(ref v) if v == &[10])));
    }

    #[test]
    fn reentrancy_skips_constructor() {
        let mut module = IrModule {
            functions: vec![],
            constructor_ops: vec![IrOp::Push(vec![1]), IrOp::Stop],
            label_count: 0,
        };
        let before = module.constructor_ops.len();
        add_reentrancy_guard(&mut module, 0);
        assert_eq!(module.constructor_ops.len(), before);
    }
}
