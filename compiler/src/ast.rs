use num_bigint::BigUint;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Function(Function),
    Struct(StructDef),
    Const(ConstDecl),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub type_: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Uint8,
    Uint256,
    Int256,
    Bool,
    Address,
    Bytes,
    String,

    Vec(Box<Type>),
    Map(Box<Type>, Box<Type>),

    Custom(String),

    Generic(String, Vec<Type>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let(LetStatement),
    Assign(AssignStatement),
    Expression(Expression),
    If(IfStatement),
    For(ForStatement),
    While(WhileStatement),
    Return(Option<Expression>),
    Require(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LetStatement {
    pub name: String,
    pub type_: Option<Type>,
    pub value: Option<Expression>,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Number(BigUint),
    HexNumber(BigUint),
    String(String),
    Bool(bool),
    Bytes(Vec<u8>),

    StructInit(String, Vec<(String, Expression)>),

    Identifier(String),

    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    Unary(UnaryOp, Box<Expression>),

    Call(Box<Expression>, Vec<Expression>),

    Member(Box<Expression>, String),
    Index(Box<Expression>, Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Minus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub type_: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub name: String,
    pub type_: Type,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignStatement {
    pub target: Expression,
    pub value: Expression,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_branch: Block,
    pub else_branch: Option<Block>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStatement {
    pub var: String,
    pub iterable: Expression,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: Block,
    pub span: Span,
}
