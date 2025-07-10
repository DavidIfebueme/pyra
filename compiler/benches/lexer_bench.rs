use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pyra_compiler::lexer::PyraLexer;

fn lexer_benchmark(c: &mut Criterion) {
    let large_contract = r#"
        def transfer(to: address, amount: uint256):
            require amount > 0
            require balances[msg.sender] >= amount
            
            balances[msg.sender] -= amount
            balances[to] += amount
            
            return true
        
        def approve(spender: address, amount: uint256):
            allowances[msg.sender][spender] = amount
            return true
    "#.repeat(100); // Large input
    
    c.bench_function("lexer_large_contract", |b| {
        b.iter(|| {
            let mut lexer = PyraLexer::new(black_box(&large_contract));
            let _tokens: Vec<_> = lexer.collect();
        })
    });
}

criterion_group!(benches, lexer_benchmark);
criterion_main!(benches);