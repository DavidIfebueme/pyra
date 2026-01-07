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

    let tokens: Vec<Token> = tokens.into_iter().filter(|t| !matches!(t, Token::Comment)).collect();

    parse_program(tokens)
}

fn program_parser() -> impl Parser<Token, Program, Error = ParseError> {
    nl()
        .ignore_then(function_parser().map(Item::Function))
        .then_ignore(nl())
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
        .then(suite_parser(statement_parser()))
        .map(|(((name, params), return_type), body)| Function {
            name,
            params,
            return_type,
            body,
            span: Span { start: 0, end: 0 },
        })
}

fn nl() -> impl Parser<Token, (), Error = ParseError> {
    just(Token::Newline).repeated().ignored()
}

fn nl1() -> impl Parser<Token, (), Error = ParseError> {
    just(Token::Newline).repeated().at_least(1).ignored()
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

fn require_statement() -> impl Parser<Token, Statement, Error = ParseError> {
    just(Token::Require)
        .ignore_then(expression_parser())
        .map(Statement::Require)
}

fn identifier() -> impl Parser<Token, String, Error = ParseError> {
    select! { Token::Identifier(name) => name }
}

fn let_statement() -> impl Parser<Token, Statement, Error = ParseError> {
    just(Token::Let)
        .ignore_then(just(Token::Mut).or_not())
        .then(identifier())
        .then(just(Token::Colon).ignore_then(type_parser()).or_not())
        .then(just(Token::Assign).ignore_then(expression_parser()).or_not())
        .map(|(((mutable, name), type_), value)| {
            Statement::Let(LetStatement {
                name,
                type_,
                value,
                mutable: mutable.is_some(),
                span: Span { start: 0, end: 0 },
            })
        })
}

fn assign_statement() -> impl Parser<Token, Statement, Error = ParseError> {
    let target = identifier().map(Expression::Identifier);

    let op = choice((
        just(Token::Assign).to(None),
        just(Token::PlusAssign).to(Some(BinaryOp::Add)),
        just(Token::MinusAssign).to(Some(BinaryOp::Sub)),
        just(Token::MultiplyAssign).to(Some(BinaryOp::Mul)),
        just(Token::DivideAssign).to(Some(BinaryOp::Div)),
    ));

    target
        .then(op)
        .then(expression_parser())
        .map(|((target, op), rhs)| {
            let value = match op {
                None => rhs,
                Some(bin_op) => Expression::Binary(bin_op, Box::new(target.clone()), Box::new(rhs)),
            };

            Statement::Assign(AssignStatement {
                target,
                value,
                span: Span { start: 0, end: 0 },
            })
        })
}

fn statement_parser() -> BoxedParser<'static, Token, Statement, ParseError> {
    recursive(|stmt| {
        let suite = suite_parser(stmt.clone().boxed());

        let if_stmt = just(Token::If)
            .ignore_then(expression_parser())
            .then_ignore(just(Token::Colon))
            .then(suite.clone())
            .then(
                nl1()
                    .ignore_then(
                        just(Token::Elif)
                            .ignore_then(expression_parser())
                            .then_ignore(just(Token::Colon))
                            .then(suite.clone()),
                    )
                    .repeated(),
            )
            .then(
                nl1()
                    .ignore_then(just(Token::Else).ignore_then(just(Token::Colon)).ignore_then(suite))
                    .or_not(),
            )
            .map(|(((cond, then_branch), elifs), else_branch)| {
                let mut else_acc = else_branch;
                for (elif_cond, elif_body) in elifs.into_iter().rev() {
                    let nested = IfStatement {
                        condition: elif_cond,
                        then_branch: elif_body,
                        else_branch: else_acc,
                        span: Span { start: 0, end: 0 },
                    };

                    else_acc = Some(Block {
                        statements: vec![Statement::If(nested)],
                        span: Span { start: 0, end: 0 },
                    });
                }

                Statement::If(IfStatement {
                    condition: cond,
                    then_branch,
                    else_branch: else_acc,
                    span: Span { start: 0, end: 0 },
                })
            });

        choice((
            if_stmt,
            require_statement(),
            let_statement(),
            return_statement(),
            assign_statement(),
        ))
        .boxed()
    })
    .boxed()
}

fn suite_parser<S>(stmt: S) -> BoxedParser<'static, Token, Block, ParseError>
where
    S: Parser<Token, Statement, Error = ParseError> + Clone + 'static,
{
    let single = stmt.clone().map(|st| Block {
        statements: vec![st],
        span: Span { start: 0, end: 0 },
    });

    let indented = nl1()
        .ignore_then(just(Token::Indent))
        .ignore_then(nl())
        .ignore_then(stmt.separated_by(nl1()).allow_leading().allow_trailing())
        .then_ignore(nl())
        .then_ignore(just(Token::Dedent))
        .map(|statements| Block {
            statements,
            span: Span { start: 0, end: 0 },
        });

    choice((indented, single)).boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_function() {
        let source = "def transfer(to: address, amount: uint256) -> bool: return true";

        let result = parse_from_source(source);
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

    #[test]
    fn parses_multiline_block_with_require() {
        let source = "def t() -> bool:\n    require true\n    return true\n";
        let program = parse_from_source(source).unwrap();
        assert_eq!(program.items.len(), 1);
        let Item::Function(f) = &program.items[0] else { panic!() };
        assert_eq!(f.body.statements.len(), 2);
        assert!(matches!(f.body.statements[0], Statement::Require(_)));
        assert!(matches!(f.body.statements[1], Statement::Return(_)));
    }

    #[test]
    fn parses_if_elif_else() {
        let source = "def t() -> uint256:\n    if true: return 1\n    elif false: return 2\n    else: return 3\n";
        let program = parse_from_source(source).unwrap();
        let Item::Function(f) = &program.items[0] else { panic!() };
        assert_eq!(f.body.statements.len(), 1);
        assert!(matches!(f.body.statements[0], Statement::If(_)));
    }

    #[test]
    fn parses_augmented_assignment() {
        let source = "def t() -> uint256:\n    let mut x = 1\n    x += 2\n    return x\n";
        let program = parse_from_source(source).unwrap();
        let Item::Function(f) = &program.items[0] else { panic!() };
        assert_eq!(f.body.statements.len(), 3);
        assert!(matches!(f.body.statements[1], Statement::Assign(_)));
    }
}
