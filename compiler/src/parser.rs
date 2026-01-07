use crate::ast::*;
use crate::lexer::Token;
use chumsky::prelude::*;

pub type ParseError = Simple<Token>;

#[derive(Clone)]
enum PostfixOp {
    Member(String),
    Index(Expression),
    Call(Vec<Expression>),
}

#[derive(Clone)]
enum TargetOp {
    Member(String),
    Index(Expression),
}

fn fold_postfix(lhs: Expression, op: PostfixOp) -> Expression {
    match op {
        PostfixOp::Member(name) => Expression::Member(Box::new(lhs), name),
        PostfixOp::Index(idx) => Expression::Index(Box::new(lhs), Box::new(idx)),
        PostfixOp::Call(args) => Expression::Call(Box::new(lhs), args),
    }
}

fn fold_unary(op: UnaryOp, rhs: Expression) -> Expression {
    Expression::Unary(op, Box::new(rhs))
}

fn fold_binary(left: Expression, (op, right): (BinaryOp, Expression)) -> Expression {
    Expression::Binary(op, Box::new(left), Box::new(right))
}

fn fold_target(lhs: Expression, op: TargetOp) -> Expression {
    match op {
        TargetOp::Member(name) => Expression::Member(Box::new(lhs), name),
        TargetOp::Index(idx) => Expression::Index(Box::new(lhs), Box::new(idx)),
    }
}

fn fold_field_init((name, value): (String, Expression)) -> (String, Expression) {
    (name, value)
}

fn fold_struct_init((name, fields): (String, Vec<(String, Expression)>)) -> Expression {
    Expression::StructInit(name, fields)
}

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
        .ignore_then(
            choice((
                function_parser().map(Item::Function),
                struct_parser().map(Item::Struct),
                const_item_parser().map(Item::Const),
            ))
            .then_ignore(nl()),
        )
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
        just(Token::Uint8).to(Type::Uint8),
        just(Token::Uint256).to(Type::Uint256),
        just(Token::Int256).to(Type::Int256),
        just(Token::Bool).to(Type::Bool),
        just(Token::Address).to(Type::Address),
        just(Token::Bytes).to(Type::Bytes),
        just(Token::String).to(Type::String),
        identifier().map(Type::Custom),
    ))
}

fn generic_params_parser() -> impl Parser<Token, (), Error = ParseError> {
    let param = identifier()
        .then(just(Token::Colon).ignore_then(type_parser()).or_not())
        .ignored();

    just(Token::Less)
        .ignore_then(param.separated_by(just(Token::Comma)).allow_trailing())
        .then_ignore(just(Token::Greater))
        .ignored()
}

fn struct_parser() -> impl Parser<Token, StructDef, Error = ParseError> {
    let sep = choice((just(Token::Comma).ignore_then(nl()).ignored(), nl1()));
    just(Token::Struct)
        .ignore_then(identifier())
        .then_ignore(generic_params_parser().or_not())
        .then_ignore(nl())
        .then_ignore(just(Token::LBrace))
        .then_ignore(nl())
        .then_ignore(just(Token::Indent).or_not())
        .then_ignore(nl())
        .then(struct_field().separated_by(sep).allow_leading().allow_trailing())
        .then_ignore(nl())
        .then_ignore(just(Token::Dedent).or_not())
        .then_ignore(nl())
        .then_ignore(just(Token::RBrace))
        .map(|(name, fields)| StructDef {
            name,
            fields,
            span: Span { start: 0, end: 0 },
        })
}

fn struct_field() -> impl Parser<Token, StructField, Error = ParseError> {
    identifier()
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .map(|(name, type_)| StructField {
            name,
            type_,
            span: Span { start: 0, end: 0 },
        })
}

fn const_item_parser() -> impl Parser<Token, ConstDecl, Error = ParseError> {
    choice((just(Token::Const), just(Token::Let)))
        .ignore_then(identifier())
        .then(just(Token::Colon).ignore_then(type_parser()).or_not())
        .then_ignore(just(Token::Assign))
        .then(expression_parser())
        .map(|((name, type_), value)| ConstDecl {
            name,
            type_: type_.unwrap_or(Type::Uint256),
            value,
            span: Span { start: 0, end: 0 },
        })
}

fn expression_parser() -> impl Parser<Token, Expression, Error = ParseError> {
    recursive(|expr| {
        let field_init = identifier()
            .then_ignore(just(Token::Colon))
            .then(expr.clone())
            .map(fold_field_init as fn((String, Expression)) -> (String, Expression));

        let sep = choice((just(Token::Comma).ignore_then(nl()).ignored(), nl1()));

        let struct_init = identifier()
            .then(
                just(Token::LBrace)
                    .ignore_then(nl())
                    .ignore_then(just(Token::Indent).or_not())
                    .ignore_then(nl())
                    .ignore_then(field_init.separated_by(sep).allow_leading().allow_trailing())
                    .then_ignore(nl())
                    .then_ignore(just(Token::Dedent).or_not())
                    .then_ignore(nl())
                    .then_ignore(just(Token::RBrace)),
            )
            .map(fold_struct_init as fn((String, Vec<(String, Expression)>)) -> Expression);

        let atom = choice((
            select! { Token::Number(n) => Expression::Number(n) },
            select! { Token::HexNumber(n) => Expression::HexNumber(n) },
            select! { Token::StringLiteral(s) => Expression::String(s) },
            select! { Token::BytesLiteral(b) => Expression::Bytes(b) },
            just(Token::True).to(Expression::Bool(true)),
            just(Token::False).to(Expression::Bool(false)),
            struct_init,
            identifier().map(Expression::Identifier),
            expr.clone().delimited_by(just(Token::LParen), just(Token::RParen)),
        ));

        let postfix_ops = choice((
            just(Token::Dot)
                .ignore_then(identifier())
                .map(PostfixOp::Member),
            just(Token::LBracket)
                .ignore_then(expr.clone())
                .then_ignore(just(Token::RBracket))
                .map(PostfixOp::Index),
            just(Token::LParen)
                .ignore_then(expr.clone().separated_by(just(Token::Comma)).allow_trailing())
                .then_ignore(just(Token::RParen))
                .map(PostfixOp::Call),
        ))
        .repeated();

        let postfix = atom
            .then(postfix_ops)
            .foldl(fold_postfix as fn(Expression, PostfixOp) -> Expression)
            .boxed();

        let unary = choice((
            just(Token::Not).to(UnaryOp::Not),
            just(Token::Minus).to(UnaryOp::Minus),
        ))
        .repeated()
        .then(postfix)
        .foldr(fold_unary as fn(UnaryOp, Expression) -> Expression)
        .boxed();

        let product = unary
            .clone()
            .then(
                choice((
                    just(Token::Multiply).to(BinaryOp::Mul),
                    just(Token::Divide).to(BinaryOp::Div),
                    just(Token::Modulo).to(BinaryOp::Mod),
                ))
                .then(unary.clone())
                .repeated(),
            )
            .foldl(fold_binary as fn(Expression, (BinaryOp, Expression)) -> Expression)
            .boxed();

        let sum = product
            .clone()
            .then(
                choice((just(Token::Plus).to(BinaryOp::Add), just(Token::Minus).to(BinaryOp::Sub)))
                    .then(product)
                    .repeated(),
            )
            .foldl(fold_binary as fn(Expression, (BinaryOp, Expression)) -> Expression)
            .boxed();

        let cmp = sum
            .clone()
            .then(
                choice((
                    just(Token::Equal).to(BinaryOp::Equal),
                    just(Token::NotEqual).to(BinaryOp::NotEqual),
                    just(Token::LessEqual).to(BinaryOp::LessEqual),
                    just(Token::GreaterEqual).to(BinaryOp::GreaterEqual),
                    just(Token::Less).to(BinaryOp::Less),
                    just(Token::Greater).to(BinaryOp::Greater),
                ))
                .then(sum)
                .repeated(),
            )
            .foldl(fold_binary as fn(Expression, (BinaryOp, Expression)) -> Expression)
            .boxed();

        let and_expr = cmp
            .clone()
            .then(just(Token::And).to(BinaryOp::And).then(cmp).repeated())
            .foldl(fold_binary as fn(Expression, (BinaryOp, Expression)) -> Expression)
            .boxed();

        and_expr
            .clone()
            .then(just(Token::Or).to(BinaryOp::Or).then(and_expr).repeated())
            .foldl(fold_binary as fn(Expression, (BinaryOp, Expression)) -> Expression)
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
    let target = assignment_target_parser();

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

fn assignment_target_parser() -> impl Parser<Token, Expression, Error = ParseError> {
    let base = identifier().map(Expression::Identifier).boxed();
    let ops = choice((
        just(Token::Dot)
            .ignore_then(identifier())
            .map(TargetOp::Member),
        just(Token::LBracket)
            .ignore_then(expression_parser())
            .then_ignore(just(Token::RBracket))
            .map(TargetOp::Index),
    ))
    .repeated();

    base.then(ops).foldl(fold_target as fn(Expression, TargetOp) -> Expression)
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

    #[test]
    fn parses_const_item() {
        let source = "const total_supply: uint256 = 100\n\ndef t() -> uint256: return total_supply\n";
        let program = parse_from_source(source).unwrap();
        assert_eq!(program.items.len(), 2);
        assert!(matches!(program.items[0], Item::Const(_)));
    }
}
