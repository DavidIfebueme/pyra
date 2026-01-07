use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pyra_compiler::lexer::PyraLexer;

fn lexer_benchmark(c: &mut Criterion) {
    let large_contract = r#"
let total_supply: uint256 = 115792089237316195423570985008687907853269984665640564039457584007913129639935

struct Token {
    name: string,
    symbol: string,
    decimals: uint8,
    total_supply: uint256
}

def init() -> Token:
    return Token {
        name: "PyraToken",
        symbol: "PYRA",
        decimals: 18,
        total_supply: total_supply
    }

def transfer(to: address, amount: uint256) -> bool:
    require amount > 0
    require balances[msg.sender] >= amount
    balances[msg.sender] -= amount
    balances[to] += amount
    return true

def balance_of(owner: address) -> uint256:
    return balances[owner]
"#.repeat(50);

    c.bench_function("lexer_comprehensive_stress_test", |b| {
        b.iter(|| {
            let lexer = PyraLexer::new(black_box(&large_contract));
            let _tokens: Vec<_> = lexer.collect();
        })
    });

    let error_test_contract = r#"
def t() -> uint256:
    x: uint256 = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    if x > 0:
            return 1
        return 0
"#
    .repeat(300);

    c.bench_function("lexer_error_handling_stress", |b| {
        b.iter(|| {
            let lexer = PyraLexer::new(black_box(&error_test_contract));
            let _tokens: Vec<_> = lexer.collect();
        })
    });
}

criterion_group!(benches, lexer_benchmark);
criterion_main!(benches);
