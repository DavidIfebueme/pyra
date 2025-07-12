use logos::Logos;
use std::fmt;
use num_bigint::BigUint;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\f]+")]
pub enum Token {
    #[token("def")]
    Def,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("elif")]
    Elif,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("const")]
    Const,
    #[token("struct")]
    Struct,
    #[token("require")]
    Require,
    #[token("true")]
    True,
    #[token("false")]
    False,

    #[token("uint256")]
    Uint256,
    #[token("int256")]
    Int256,
    #[token("bool")]
    Bool,
    #[token("address")]
    Address,
    #[token("bytes")]
    Bytes,
    #[token("string")]
    String,

    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Multiply,
    #[token("/")]
    Divide,
    #[token("%")]
    Modulo,
    #[token("**")]
    Power,
    
    #[token("=")]
    Assign,
    #[token("+=")]
    PlusAssign,
    #[token("-=")]
    MinusAssign,
    #[token("*=")]
    MultiplyAssign,
    #[token("/=")]
    DivideAssign,

    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("<", priority = 2)]
    Less,
    #[token(">", priority = 2)]
    Greater,

    #[token("and")]
    And,
    #[token("or")]
    Or,
    #[token("not")]
    Not,

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(":", priority = 2)]
    Colon,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,

    #[token("<", priority = 1)]
    LAngle,
    #[token(">", priority = 1)]
    RAngle,

    #[regex(r"[0-9]+", |lex| {
        BigUint::parse_bytes(lex.slice().as_bytes(), 10)
    })]
    Number(BigUint),

    
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        Some(s[1..s.len()-1].to_string())
    })]
    StringLiteral(String),
    
    #[regex(r"0x[0-9a-fA-F]+", |lex| {
        BigUint::parse_bytes(&lex.slice().as_bytes()[2..], 16)
    })]
    HexNumber(BigUint),

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    #[regex(r"\n")]
    Newline,
    
    Indent,
    Dedent,
    
    Eof,

    #[regex(r"#[^\n]*", logos::skip)]
    Comment,

    Error,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(name) => write!(f, "Identifier({})", name),
            Token::Number(n) => write!(f, "Number({})", n),
            Token::StringLiteral(s) => write!(f, "String(\"{}\")", s),
            Token::HexNumber(n) => write!(f, "Hex(0x{:x})", n),
            _ => write!(f, "{:?}", self),
        }
    }
}

// indentation tracking
pub struct PyraLexer<'a> {
    inner: logos::Lexer<'a, Token>,
    indent_stack: Vec<usize>,
    pending_dedents: usize,
    pending_indent: bool,
    at_line_start: bool,
}

impl<'a> PyraLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
            indent_stack: vec![0],
            pending_dedents: 0,
            pending_indent: false,
            at_line_start: true,
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        if self.pending_indent {
            self.pending_indent = false;
            return Some(Token::Indent);
        }

        if self.pending_dedents > 0 {
            self.pending_dedents -= 1;
            return Some(Token::Dedent);
        }

        match self.inner.next()? {
            Ok(token) => {
                match token {
                    Token::Newline => {
                        self.at_line_start = true;
                        Some(Token::Newline)
                    }
                    _ => {
                        if self.at_line_start {
                            if self.has_indentation() {
                                self.handle_indentation();
                            }
                            self.at_line_start = false;
                            
                            if self.pending_indent || self.pending_dedents > 0 {
                                if self.pending_indent {
                                    self.pending_indent = false;
                                    return Some(Token::Indent);
                                } else if self.pending_dedents > 0 {
                                    self.pending_dedents -= 1;
                                    return Some(Token::Dedent);
                                }
                            }
                        }
                        Some(token)
                    }
                }
            }
            Err(_) => Some(Token::Error),
        }
    }

    fn has_indentation(&self) -> bool {
        let _remaining = self.inner.remainder();
        let current_pos = self.inner.span().start;
        
        let source = self.inner.source();
        
        let mut line_start = current_pos;
        while line_start > 0 {
            let prev_char = source.chars().nth(line_start - 1);
            if let Some('\n') = prev_char {
                break;
            }
            line_start -= 1;
        }
        
        let line_content = &source[line_start..current_pos];
        line_content.chars().any(|c| c == ' ' || c == '\t')
    }

    fn handle_indentation(&mut self) {
        let source = self.inner.source();
        let current_pos = self.inner.span().start;
        
        let mut line_start = current_pos;
        while line_start > 0 {
            let prev_char = source.chars().nth(line_start - 1);
            if let Some('\n') = prev_char {
                break;
            }
            line_start -= 1;
        }
        
        let mut indent = 0;
        for ch in source[line_start..current_pos].chars() {
            match ch {
                ' ' => indent += 1,
                '\t' => indent += 8,
                _ => break,
            }
        }

        let current_level = *self.indent_stack.last().unwrap();

        if indent > current_level {
            self.indent_stack.push(indent);
            self.pending_indent = true;
        } else if indent < current_level {
            while let Some(&level) = self.indent_stack.last() {
                if level <= indent {
                    break;
                }
                self.indent_stack.pop();
                self.pending_dedents += 1;
            }
        }
    }

    pub fn line_col(&self) -> (usize, usize) {
        let source = self.inner.source();
        let pos = self.inner.span().start;
        
        let mut line = 1;
        let mut col = 1;
        
        for (i, ch) in source.char_indices() {
            if i >= pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        
        (line, col)
    }

    pub fn span(&self) -> std::ops::Range<usize> {
        self.inner.span()
    }

    pub fn slice(&self) -> &'a str {
        self.inner.slice()
    }
}

impl<'a> Iterator for PyraLexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let source = "def transfer(to: address, amount: uint256):";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Def,
            Token::Identifier("transfer".to_string()),
            Token::LParen,
            Token::Identifier("to".to_string()),
            Token::Colon,
            Token::Address,
            Token::Comma,
            Token::Identifier("amount".to_string()),
            Token::Colon,
            Token::Uint256,
            Token::RParen,
            Token::Colon,
        ]);
    }

    #[test]
    fn test_numbers() {
        
        let source = "123 0xff 0x1234";
        let lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Number(BigUint::from(123u64)),
            Token::HexNumber(BigUint::from(255u64)),
            Token::HexNumber(BigUint::from(0x1234u64)),
        ]);
    }

    #[test]
    fn test_large_numbers() {
        
        let source = "115792089237316195423570985008687907853269984665640564039457584007913129639935"; // 2^256 - 1
        let lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens.len(), 1);
        if let Token::Number(n) = &tokens[0] {
            assert!(n.bits() > 64);
        } else {
            panic!("Expected Number token");
        }
    }


    #[test]
    fn test_comparison_vs_generics() {
        let source = "a < b > c";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Identifier("a".to_string()),
            Token::Less,
            Token::Identifier("b".to_string()),
            Token::Greater,
            Token::Identifier("c".to_string()),
        ]);
    }

    #[test]
    fn test_strings() {
        let source = r#""hello world" "test\"quote""#;
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::StringLiteral("hello world".to_string()),
            Token::StringLiteral("test\\\"quote".to_string()),
        ]);
    }

    #[test]
    fn test_operators() {
        let source = "+ - * / == != <= >= and or not";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Plus,
            Token::Minus,
            Token::Multiply,
            Token::Divide,
            Token::Equal,
            Token::NotEqual,
            Token::LessEqual,
            Token::GreaterEqual,
            Token::And,
            Token::Or,
            Token::Not,
        ]);
    }

    #[test]
    fn test_assignment_operators() {
        let source = "= += -= *= /=";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Assign,
            Token::PlusAssign,
            Token::MinusAssign,
            Token::MultiplyAssign,
            Token::DivideAssign,
        ]);
    }

    #[test]
    fn test_comments_are_skipped() {
        let source = "def # this is a comment\ntransfer";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Def,
            Token::Newline,
            Token::Identifier("transfer".to_string()),
        ]);
    }

    #[test]
    fn test_complex_expression() {
        let source = "balances[msg.sender] += amount * 2";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Identifier("balances".to_string()),
            Token::LBracket,
            Token::Identifier("msg".to_string()),
            Token::Dot,
            Token::Identifier("sender".to_string()),
            Token::RBracket,
            Token::PlusAssign,
            Token::Identifier("amount".to_string()),
            Token::Multiply,
            Token::Number(BigUint::from(2u64)), // Changed this line
        ]);
    }

    #[test]
    fn test_keywords_vs_identifiers() {
        let source = "def definition if ifelse bool boolean";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Def,
            Token::Identifier("definition".to_string()),
            Token::If,
            Token::Identifier("ifelse".to_string()),
            Token::Bool,
            Token::Identifier("boolean".to_string()),
        ]);
    }

    #[test]
    fn test_error_handling() {
        let source = "def @ invalid";
        let mut lexer = PyraLexer::new(source);
        
        let tokens: Vec<Token> = lexer.collect();
        
        assert_eq!(tokens, vec![
            Token::Def,
            Token::Error,
            Token::Identifier("invalid".to_string()),
        ]);
    }
}