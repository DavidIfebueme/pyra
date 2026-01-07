use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pyra_compiler::{parse_from_source, program_to_abi_json, program_to_deploy_bytecode};

fn compile_benchmark(c: &mut Criterion) {
    let source = include_str!("../../contracts/ERC20.pyra");

    c.bench_function("parse_only_erc20", |b| {
        b.iter(|| {
            let program = parse_from_source(black_box(source)).unwrap();
            black_box(program);
        })
    });

    c.bench_function("parse_and_abi_erc20", |b| {
        b.iter(|| {
            let program = parse_from_source(black_box(source)).unwrap();
            let abi = program_to_abi_json(&program).unwrap();
            black_box(abi);
        })
    });

    c.bench_function("parse_and_codegen_erc20", |b| {
        b.iter(|| {
            let program = parse_from_source(black_box(source)).unwrap();
            let bin = program_to_deploy_bytecode(&program).unwrap();
            black_box(bin);
        })
    });
}

criterion_group!(benches, compile_benchmark);
criterion_main!(benches);
