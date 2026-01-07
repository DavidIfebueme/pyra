use pyra_compiler::lexer::PyraLexer;
use pyra_compiler::parser::parse_from_source;
use pyra_compiler::Token;

#[test]
fn lexes_basic_tokens() {
    let source = "def transfer(to: address, amount: uint256):";
    let tokens: Vec<Token> = PyraLexer::new(source).collect();
    assert!(!tokens.is_empty());
    assert!(matches!(tokens[0], Token::Def));
}

#[test]
fn parses_minimal_function() {
    let source = "def t() -> bool: return true";
    let program = parse_from_source(source).unwrap();
    assert_eq!(program.items.len(), 1);
}
