use crate::ast::*;
use crate::lexer::Token;
use chumsky::prelude::*;

pub type ParseError = Simple<Token>;

pub fn parse_program(tokens: Vec<Token>) -> Result<Program, Vec<ParseError>> {
    program_parser().parse(tokens)
}

pub fn parse_from_source(source: &str) -> Result<Program, Vec<ParseError>> {
    use crate::lexer::PyraLexer;

    let lexer = PyraLexer::new(source);
    let tokens: Vec<Token> = lexer.collect();

    let tokens: Vec<Token> = tokens
        .into_iter()
        .filter(|t| !matches!(t, Token::Newline | Token::Comment))
        .collect();

    parse_program(tokens)
}

fn program_parser() -> impl Parser<Token, Program, Error = ParseError> {
    function_parser()
        .map(Item::Function)
        .repeated()
        .map(|items| Program {
            items,
            span: Span { start: 0, end: 0 },
        })
        .then_ignore(end())
}

fn function_parser() -> impl Parser<Token, Function, Error = ParseError> {
    just(Token::Def)
        .ignore_then(identifier())
        .then_ignore(just(Token::LParen))
        .then(parameter_list())
        .then_ignore(just(Token::RParen))
        .then(return_type().or_not())
        .then_ignore(just(Token::Colon))
        .then(block_parser())
        .map(|(((name, params), return_type), body)| Function {
            name,
            params,
            return_type,
            body,
            span: Span { start: 0, end: 0 },
        })
}

fn parameter_list() -> impl Parser<Token, Vec<Parameter>, Error = ParseError> {
    parameter_parser()
        .separated_by(just(Token::Comma))
        .allow_trailing()
}

fn parameter_parser() -> impl Parser<Token, Parameter, Error = ParseError> {
    identifier()
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .map(|(name, type_)| Parameter {
            name,
            type_,
            span: Span { start: 0, end: 0 },
        })
}

fn return_type() -> impl Parser<Token, Type, Error = ParseError> {
    just(Token::Arrow).ignore_then(type_parser())
}

fn type_parser() -> impl Parser<Token, Type, Error = ParseError> {
    choice((
        just(Token::Uint256).to(Type::Uint256),
        just(Token::Int256).to(Type::Int256),
        just(Token::Bool).to(Type::Bool),
        just(Token::Address).to(Type::Address),
        just(Token::Bytes).to(Type::Bytes),
        just(Token::String).to(Type::String),
        identifier().map(Type::Custom),
    ))
}

fn expression_parser() -> impl Parser<Token, Expression, Error = ParseError> {
    recursive(|expr| {
        let atom1 = choice((
            select! { Token::Number(n) => Expression::Number(n) },
            select! { Token::HexNumber(n) => Expression::HexNumber(n) },
            select! { Token::StringLiteral(s) => Expression::String(s) },
            select! { Token::BytesLiteral(b) => Expression::Bytes(b) },
            just(Token::True).to(Expression::Bool(true)),
            just(Token::False).to(Expression::Bool(false)),
            identifier().map(Expression::Identifier),
            expr.clone()
                .delimited_by(just(Token::LParen), just(Token::RParen)),
        ));

        let atom2 = choice((
            select! { Token::Number(n) => Expression::Number(n) },
            select! { Token::HexNumber(n) => Expression::HexNumber(n) },
            select! { Token::StringLiteral(s) => Expression::String(s) },
            select! { Token::BytesLiteral(b) => Expression::Bytes(b) },
            just(Token::True).to(Expression::Bool(true)),
            just(Token::False).to(Expression::Bool(false)),
            identifier().map(Expression::Identifier),
            expr.delimited_by(just(Token::LParen), just(Token::RParen)),
        ));

        atom1
            .then(
                choice((
                    just(Token::Plus).to(BinaryOp::Add),
                    just(Token::Minus).to(BinaryOp::Sub),
                    just(Token::Multiply).to(BinaryOp::Mul),
                    just(Token::Divide).to(BinaryOp::Div),
                    just(Token::Modulo).to(BinaryOp::Mod),
                ))
                .then(atom2)
                .repeated(),
            )
            .foldl(|left, (op, right)| Expression::Binary(op, Box::new(left), Box::new(right)))
    })
}

fn return_statement() -> impl Parser<Token, Statement, Error = ParseError> {
    just(Token::Return)
        .ignore_then(expression_parser().or_not())
        .map(Statement::Return)
}

fn identifier() -> impl Parser<Token, String, Error = ParseError> {
    select! { Token::Identifier(name) => name }
}

fn let_statement() -> impl Parser<Token, Statement, Error = ParseError> {
    just(Token::Let)
        .ignore_then(identifier())
        .then_ignore(just(Token::Assign))
        .then(expression_parser())
        .map(|(name, value)| {
            Statement::Let(LetStatement {
                name,
                type_: None,
                value: Some(value),
                mutable: false,
                span: Span { start: 0, end: 0 },
            })
        })
}

fn statement_parser() -> impl Parser<Token, Statement, Error = ParseError> {
    choice((let_statement(), return_statement()))
}

fn block_parser() -> impl Parser<Token, Block, Error = ParseError> {
    statement_parser()
        .repeated()
        .at_least(1)
        .map(|statements| Block {
            statements,
            span: Span { start: 0, end: 0 },
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let source = "def transfer(to: address, amount: uint256) -> bool: return true";

        let result = parse_from_source(source);
        println!("Parse result: {:?}", result);

        assert!(result.is_ok(), "Parser should handle simple function");

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        if let Item::Function(func) = &program.items[0] {
            assert_eq!(func.name, "transfer");
            assert_eq!(func.params.len(), 2);
            assert_eq!(func.params[0].name, "to");
            assert!(matches!(func.params[0].type_, Type::Address));
        }
    }

    #[test]
    fn test_expression_parsing() {
        let source = "def test() -> uint256: return 42";

        let result = parse_from_source(source);
        assert!(result.is_ok(), "Should parse simple return statement");
    }
}
