use pyra_compiler::lexer::{PyraLexer, Token};

fn main() {
    println!("🔥 Pyra Lexer Interactive Test");

    let test_cases = [
        "def transfer(to: address, amount: uint256):",
        "let balance: uint256 = 1000",
        "if amount > 0 and balance >= amount:",
        "balances[msg.sender] -= amount",
        r#"emit Transfer(from, to, "success")"#,
        "# This is a comment\nreturn true",
        "0xff + 123 * 0x10",
    ];

    for (i, source) in test_cases.iter().enumerate() {
        println!("\n--- Test Case {} ---", i + 1);
        println!("Source: {}", source);
        println!("Tokens:");

        let lexer = PyraLexer::new(source);
        let tokens: Vec<Token> = lexer.collect();

        for (j, token) in tokens.iter().enumerate() {
            println!("  {}: {}", j, token);
        }
    }
}
