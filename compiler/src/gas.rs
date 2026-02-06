use crate::ir::{IrModule, IrOp};

#[derive(Debug, Clone)]
pub struct FunctionGas {
    pub name: String,
    pub selector: [u8; 4],
    pub estimated_gas: u64,
}

#[derive(Debug, Clone)]
pub struct GasReport {
    pub functions: Vec<FunctionGas>,
    pub constructor_gas: u64,
    pub dispatch_overhead: u64,
}

impl GasReport {
    pub fn from_module(module: &IrModule) -> Self {
        let dispatch_overhead = module.functions.len() as u64 * DISPATCH_PER_BRANCH;

        let functions: Vec<FunctionGas> = module
            .functions
            .iter()
            .map(|f| FunctionGas {
                name: f.name.clone(),
                selector: f.selector,
                estimated_gas: estimate_ops(&f.ops) + dispatch_overhead,
            })
            .collect();

        let constructor_gas = estimate_ops(&module.constructor_ops) + DEPLOY_BASE;

        Self {
            functions,
            constructor_gas,
            dispatch_overhead,
        }
    }
}

const DEPLOY_BASE: u64 = 32000;
const DISPATCH_PER_BRANCH: u64 = 22;

fn estimate_ops(ops: &[IrOp]) -> u64 {
    let mut total: u64 = 0;
    for op in ops {
        total += op_gas(op);
    }
    total
}

fn op_gas(op: &IrOp) -> u64 {
    match op {
        IrOp::Push(_) => 3,
        IrOp::Pop => 2,
        IrOp::Dup(_) => 3,
        IrOp::Swap(_) => 3,
        IrOp::Add | IrOp::Sub => 3,
        IrOp::Mul | IrOp::Div | IrOp::SDiv | IrOp::Mod => 5,
        IrOp::Exp => 10,
        IrOp::Lt | IrOp::Gt | IrOp::Eq => 3,
        IrOp::IsZero => 3,
        IrOp::And | IrOp::Or | IrOp::Not => 3,
        IrOp::Shr => 3,
        IrOp::MLoad | IrOp::MStore => 3,
        IrOp::SLoad => 2100,
        IrOp::SStore => 5000,
        IrOp::Jump(_) => 8,
        IrOp::JumpI(_) => 10,
        IrOp::JumpDest(_) => 1,
        IrOp::Caller => 2,
        IrOp::CallValue => 2,
        IrOp::CallDataLoad => 3,
        IrOp::CallDataSize => 2,
        IrOp::Keccak256 => 30,
        IrOp::Return => 0,
        IrOp::Revert => 0,
        IrOp::Log(n) => 375 + (*n as u64) * 375,
        IrOp::Stop => 0,
        IrOp::Invalid => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::IrFunction;

    fn make_module(ops: Vec<IrOp>, constructor_ops: Vec<IrOp>) -> IrModule {
        IrModule {
            functions: vec![IrFunction {
                name: "transfer".into(),
                selector: [0xa9, 0x05, 0x9c, 0xbb],
                ops,
                label: 0,
            }],
            constructor_ops,
            label_count: 1,
        }
    }

    #[test]
    fn gas_simple_return() {
        let module = make_module(
            vec![IrOp::Push(vec![42]), IrOp::Return],
            vec![],
        );
        let report = GasReport::from_module(&module);
        assert_eq!(report.functions.len(), 1);
        assert_eq!(report.functions[0].estimated_gas, 3 + 0 + DISPATCH_PER_BRANCH);
    }

    #[test]
    fn gas_sload_is_2100() {
        let module = make_module(
            vec![IrOp::Push(vec![0]), IrOp::SLoad, IrOp::Return],
            vec![],
        );
        let report = GasReport::from_module(&module);
        assert_eq!(report.functions[0].estimated_gas, 3 + 2100 + 0 + DISPATCH_PER_BRANCH);
    }

    #[test]
    fn gas_sstore_is_5000() {
        let module = make_module(
            vec![IrOp::Push(vec![1]), IrOp::Push(vec![0]), IrOp::SStore, IrOp::Stop],
            vec![],
        );
        let report = GasReport::from_module(&module);
        assert_eq!(report.functions[0].estimated_gas, 3 + 3 + 5000 + 0 + DISPATCH_PER_BRANCH);
    }

    #[test]
    fn gas_constructor_includes_deploy_base() {
        let module = make_module(
            vec![IrOp::Stop],
            vec![IrOp::Push(vec![0]), IrOp::Push(vec![0]), IrOp::SStore],
        );
        let report = GasReport::from_module(&module);
        assert_eq!(report.constructor_gas, 3 + 3 + 5000 + DEPLOY_BASE);
    }

    #[test]
    fn gas_log1_is_750() {
        let module = make_module(
            vec![IrOp::Log(1), IrOp::Stop],
            vec![],
        );
        let report = GasReport::from_module(&module);
        assert_eq!(report.functions[0].estimated_gas, 750 + 0 + DISPATCH_PER_BRANCH);
    }

    #[test]
    fn gas_keccak_is_30() {
        let module = make_module(
            vec![IrOp::Keccak256, IrOp::Return],
            vec![],
        );
        let report = GasReport::from_module(&module);
        assert_eq!(report.functions[0].estimated_gas, 30 + 0 + DISPATCH_PER_BRANCH);
    }

    #[test]
    fn gas_dispatch_scales_with_functions() {
        let module = IrModule {
            functions: vec![
                IrFunction { name: "a".into(), selector: [0; 4], ops: vec![IrOp::Stop], label: 0 },
                IrFunction { name: "b".into(), selector: [1; 4], ops: vec![IrOp::Stop], label: 1 },
                IrFunction { name: "c".into(), selector: [2; 4], ops: vec![IrOp::Stop], label: 2 },
            ],
            constructor_ops: vec![],
            label_count: 3,
        };
        let report = GasReport::from_module(&module);
        assert_eq!(report.dispatch_overhead, 3 * DISPATCH_PER_BRANCH);
        for f in &report.functions {
            assert_eq!(f.estimated_gas, 0 + 3 * DISPATCH_PER_BRANCH);
        }
    }

    #[test]
    fn gas_arithmetic_costs() {
        let module = make_module(
            vec![
                IrOp::Push(vec![1]),
                IrOp::Push(vec![2]),
                IrOp::Add,
                IrOp::Push(vec![3]),
                IrOp::Mul,
                IrOp::Push(vec![4]),
                IrOp::Exp,
                IrOp::Return,
            ],
            vec![],
        );
        let report = GasReport::from_module(&module);
        assert_eq!(
            report.functions[0].estimated_gas,
            3 + 3 + 3 + 3 + 5 + 3 + 10 + 0 + DISPATCH_PER_BRANCH
        );
    }
}
