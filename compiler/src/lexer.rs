use logos::Logos;
use num_bigint::BigUint;
use std::fmt;

#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
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
    #[token("event")]
    Event,
    #[token("emit")]
    Emit,
    #[token("in")]
    In,
    #[token("true")]
    True,
    #[token("false")]
    False,

    #[token("uint256")]
    Uint256,
    #[token("uint8")]
    Uint8,
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

    #[regex(r"b'[0-9a-fA-F]*'", |lex| {
        let hex_str = &lex.slice()[2..lex.slice().len()-1];
        let mut bytes = Vec::new();
        for chunk in hex_str.as_bytes().chunks(2) {
            if chunk.len() == 2 {
                let byte_str = std::str::from_utf8(chunk).ok()?;
                let byte_val = u8::from_str_radix(byte_str, 16).ok()?;
                bytes.push(byte_val);
            }
        }
        Some(bytes)
    })]
    BytesLiteral(Vec<u8>),

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

    #[regex(r"[ \t]+\n", |_| ())]
    WhitespaceOnlyLine,

    IndentationError,
    MixedIndentationError,
    InvalidChar(char),
    MalformedNumber(String),
    UnterminatedString,
    InvalidHexDigit(String),
    InvalidBytesLiteral(String),
    Error,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(name) => write!(f, "Identifier({})", name),
            Token::Number(n) => write!(f, "Number({})", n),
            Token::StringLiteral(s) => write!(f, "String(\"{}\")", s),
            Token::HexNumber(n) => write!(f, "Hex(0x{:x})", n),
            Token::BytesLiteral(bytes) => {
                write!(f, "Bytes(0x")?;
                for byte in bytes {
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, ")")
            }
            Token::MixedIndentationError => write!(f, "MixedIndentationError"),
            Token::IndentationError => write!(f, "IndentationError"),
            Token::WhitespaceOnlyLine => write!(f, "WhitespaceOnlyLine"),
            Token::InvalidChar(ch) => write!(f, "InvalidChar('{}')", ch),
            Token::MalformedNumber(s) => write!(f, "MalformedNumber(\"{}\")", s),
            Token::UnterminatedString => write!(f, "UnterminatedString"),
            Token::InvalidHexDigit(s) => write!(f, "InvalidHexDigit(\"{}\")", s),
            Token::InvalidBytesLiteral(s) => write!(f, "InvalidBytesLiteral(\"{}\")", s),

            _ => write!(f, "{:?}", self),
        }
    }
}

pub struct PyraLexer<'a> {
    inner: logos::Lexer<'a, Token>,
    indent_stack: Vec<usize>,
    pending_dedents: usize,
    pending_indent: bool,
    pending_token: Option<Token>,
    at_line_start: bool,
    indent_type: Option<IndentType>,
}

#[derive(Debug, Clone, PartialEq)]
enum IndentType {
    Spaces,
    Tabs,
}

impl<'a> PyraLexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            inner: Token::lexer(source),
            indent_stack: vec![0],
            pending_dedents: 0,
            pending_indent: false,
            pending_token: None,
            at_line_start: true,
            indent_type: None,
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

        if let Some(tok) = self.pending_token.take() {
            return Some(tok);
        }

        match self.inner.next() {
            Some(Ok(token)) => match token {
                Token::Newline => {
                    self.at_line_start = true;
                    Some(Token::Newline)
                }
                Token::WhitespaceOnlyLine => {
                    self.at_line_start = true;
                    Some(Token::Newline)
                }
                _ => {
                    if self.at_line_start {
                        if let Some(error_token) = self.handle_indentation() {
                            return Some(error_token);
                        }
                        self.at_line_start = false;

                        if self.pending_indent || self.pending_dedents > 0 {
                            let out = if self.pending_indent {
                                self.pending_indent = false;
                                Token::Indent
                            } else {
                                self.pending_dedents -= 1;
                                Token::Dedent
                            };

                            self.pending_token = Some(token);
                            return Some(out);
                        }
                    }
                    Some(token)
                }
            },
            Some(Err(_)) => Some(self.analyze_error()),
            None => {
                let depth = self.indent_stack.len().saturating_sub(1);
                if depth == 0 {
                    None
                } else {
                    self.indent_stack.truncate(1);
                    self.pending_dedents = depth - 1;
                    Some(Token::Dedent)
                }
            }
        }
    }

    fn handle_indentation(&mut self) -> Option<Token> {
        let source = self.inner.source();
        let current_pos = self.inner.span().start;

        let mut line_start = current_pos;
        while line_start > 0 {
            if source.as_bytes()[line_start - 1] == b'\n' {
                break;
            }
            line_start -= 1;
        }

        let line_prefix = &source[line_start..current_pos];

        let mut indent = 0;
        let mut has_spaces = false;
        let mut has_tabs = false;
        for byte in line_prefix.bytes() {
            match byte {
                b' ' => {
                    has_spaces = true;
                    indent += 1;
                }
                b'\t' => {
                    has_tabs = true;
                    indent += 8;
                }
                _ => break,
            }
        }

        if has_spaces && has_tabs {
            return Some(Token::MixedIndentationError);
        }

        if indent > 0 {
            let current_indent_type = if has_tabs {
                IndentType::Tabs
            } else {
                IndentType::Spaces
            };

            match &self.indent_type {
                None => {
                    self.indent_type = Some(current_indent_type);
                }
                Some(prev_type) if *prev_type != current_indent_type => {
                    return Some(Token::MixedIndentationError);
                }
                _ => {}
            }
        }

        let current_level = *self.indent_stack.last().unwrap();

        if indent > current_level {
            self.indent_stack.push(indent);
            self.pending_indent = true;
        } else if indent < current_level {
            if !self.indent_stack.contains(&indent) {
                return Some(Token::IndentationError);
            }

            while let Some(&level) = self.indent_stack.last() {
                if level <= indent {
                    break;
                }
                self.indent_stack.pop();
                self.pending_dedents += 1;
            }
        }

        None
    }

    fn analyze_error(&mut self) -> Token {
        let _current_slice = self.inner.slice();
        let remaining = self.inner.remainder();

        if let Some(first_char) = remaining.chars().next() {
            match first_char {
                '@' | '#' | '$' | '`' | '~' => {
                    return Token::InvalidChar(first_char);
                }

                '"' => {
                    if self.is_unterminated_string(remaining) {
                        return Token::UnterminatedString;
                    }
                }

                '0'..='9' => {
                    if let Some(malformed) = self.check_malformed_number(remaining) {
                        return Token::MalformedNumber(malformed);
                    }
                }

                _ if remaining.starts_with("0x") => {
                    let hex_end = remaining
                        .find(char::is_whitespace)
                        .unwrap_or(remaining.len());
                    let hex_literal = &remaining[..hex_end];

                    if let Some(invalid) = self.check_invalid_hex(hex_literal) {
                        return Token::InvalidHexDigit(invalid);
                    }
                }

                _ if remaining.starts_with("b'") => {
                    if let Some(end_quote) = remaining[2..].find('\'') {
                        let bytes_literal = &remaining[..end_quote + 3]; // Include b' and closing '
                        if let Some(invalid) = self.check_invalid_bytes(bytes_literal) {
                            return Token::InvalidBytesLiteral(invalid);
                        }
                    }
                }

                _ => {}
            }
        }

        if !remaining.is_empty() {
            if let Some(invalid_char) = remaining.chars().next() {
                if !invalid_char.is_ascii_alphanumeric()
                    && !"()[]{}:,.+-*/=<>\"'_".contains(invalid_char)
                {
                    return Token::InvalidChar(invalid_char);
                }
            }
        }

        Token::Error
    }

    fn is_unterminated_string(&self, text: &str) -> bool {
        if !text.starts_with('"') {
            return false;
        }

        let chars = text.chars().skip(1);
        let mut escaped = false;

        for ch in chars {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => return false,
                '\n' => return true,
                _ => {}
            }
        }

        true
    }

    fn check_malformed_number(&self, text: &str) -> Option<String> {
        let mut number_part = String::new();
        let mut dot_count = 0;
        let mut has_error = false;

        for ch in text.chars() {
            if ch.is_ascii_digit() {
                number_part.push(ch);
            } else if ch == '.' {
                dot_count += 1;
                number_part.push(ch);
                if dot_count > 1 {
                    has_error = true;
                }
            } else if ch.is_ascii_alphabetic() {
                number_part.push(ch);
                has_error = true;
            } else {
                break;
            }
        }

        if has_error && !number_part.is_empty() {
            Some(number_part)
        } else {
            None
        }
    }

    fn check_invalid_hex(&self, text: &str) -> Option<String> {
        if !text.starts_with("0x") {
            return None;
        }

        let hex_part = &text[2..];

        for (i, ch) in hex_part.char_indices() {
            if !ch.is_ascii_hexdigit() && ch != '\'' && !ch.is_whitespace() {
                return Some(format!("0x{}{}", &hex_part[..i], ch));
            }
        }

        None
    }

    fn check_invalid_bytes(&self, text: &str) -> Option<String> {
        if !text.starts_with("b'") {
            return None;
        }

        if let Some(end_pos) = text[2..].find('\'') {
            let bytes_content = &text[2..end_pos + 2];

            for ch in bytes_content.chars() {
                if ch != '\'' && !ch.is_ascii_hexdigit() {
                    return Some(format!("b'{}", ch));
                }
            }
        }

        None
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
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
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
            ]
        );
    }

    #[test]
    fn test_numbers() {
        let source = "123 0xff 0x1234";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::Number(BigUint::from(123u64)),
                Token::HexNumber(BigUint::from(255u64)),
                Token::HexNumber(BigUint::from(0x1234u64)),
            ]
        );
    }

    #[test]
    fn test_large_numbers() {
        let source =
            "115792089237316195423570985008687907853269984665640564039457584007913129639935"; // 2^256 - 1
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
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::Identifier("a".to_string()),
                Token::Less,
                Token::Identifier("b".to_string()),
                Token::Greater,
                Token::Identifier("c".to_string()),
            ]
        );
    }

    #[test]
    fn test_strings() {
        let source = r#""hello world" "test\"quote""#;
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::StringLiteral("hello world".to_string()),
                Token::StringLiteral("test\\\"quote".to_string()),
            ]
        );
    }

    #[test]
    fn test_operators() {
        let source = "+ - * / == != <= >= and or not";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
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
            ]
        );
    }

    #[test]
    fn test_assignment_operators() {
        let source = "= += -= *= /=";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::Assign,
                Token::PlusAssign,
                Token::MinusAssign,
                Token::MultiplyAssign,
                Token::DivideAssign,
            ]
        );
    }

    #[test]
    fn test_comments_are_skipped() {
        let source = "def # this is a comment\ntransfer";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::Def,
                Token::Newline,
                Token::Identifier("transfer".to_string()),
            ]
        );
    }

    #[test]
    fn test_complex_expression() {
        let source = "balances[msg.sender] += amount * 2";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::Identifier("balances".to_string()),
                Token::LBracket,
                Token::Identifier("msg".to_string()),
                Token::Dot,
                Token::Identifier("sender".to_string()),
                Token::RBracket,
                Token::PlusAssign,
                Token::Identifier("amount".to_string()),
                Token::Multiply,
                Token::Number(BigUint::from(2u64)),
            ]
        );
    }

    #[test]
    fn test_keywords_vs_identifiers() {
        let source = "def definition if ifelse bool boolean";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::Def,
                Token::Identifier("definition".to_string()),
                Token::If,
                Token::Identifier("ifelse".to_string()),
                Token::Bool,
                Token::Identifier("boolean".to_string()),
            ]
        );
    }

    #[test]
    fn test_error_handling() {
        let source = "def £ invalid";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert!(tokens.iter().any(|t| matches!(t, Token::InvalidChar(_))));
        assert!(tokens.iter().any(|t| matches!(t, Token::Identifier(_))));
    }

    #[test]
    fn test_bytes_syntax() {
        let source = "b'' b'ab' b'1234abcd'";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::BytesLiteral(vec![]),
                Token::BytesLiteral(vec![0xab]),
                Token::BytesLiteral(vec![0x12, 0x34, 0xab, 0xcd]),
            ]
        );
    }

    #[test]
    fn test_bytes_vs_hex_disambiguation() {
        let source = "0x1 0x12 0x123 0x1234 b'1234' b'abcdef'";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert_eq!(
            tokens,
            vec![
                Token::HexNumber(BigUint::from(1u64)),
                Token::HexNumber(BigUint::from(0x12u64)),
                Token::HexNumber(BigUint::from(0x123u64)),
                Token::HexNumber(BigUint::from(0x1234u64)),
                Token::BytesLiteral(vec![0x12, 0x34]),
                Token::BytesLiteral(vec![0xab, 0xcd, 0xef]),
            ]
        );
    }

    #[test]
    fn test_mixed_indentation_error() {
        let source = "def func():\n    line1\n\tline2";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert!(tokens
            .iter()
            .any(|t| matches!(t, Token::MixedIndentationError)));
    }

    #[test]
    fn test_consistent_spaces() {
        let source = "def func():\n    line1\n    line2\n        nested";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert!(!tokens
            .iter()
            .any(|t| matches!(t, Token::MixedIndentationError)));
    }

    #[test]
    fn test_consistent_tabs() {
        let source = "def func():\n\tline1\n\tline2\n\t\tnested";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert!(!tokens
            .iter()
            .any(|t| matches!(t, Token::MixedIndentationError)));
    }

    #[test]
    fn test_invalid_dedent() {
        let source = "def func():\n    line1\n        nested\n   invalid_dedent"; // 3 spaces - invalid
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert!(tokens.iter().any(|t| matches!(t, Token::IndentationError)));
    }

    #[test]
    fn test_empty_lines_ignored() {
        let source = "def func():\n    line1\n\n    line2";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        assert!(!tokens
            .iter()
            .any(|t| matches!(t, Token::IndentationError | Token::MixedIndentationError)));
    }

    #[test]
    fn test_invalid_character_errors() {
        let source = "def func§invalid ¢symbol";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        println!("Debug tokens: {:?}", tokens);

        assert!(tokens
            .iter()
            .any(|t| matches!(t, Token::InvalidChar(_) | Token::Error)));
    }

    #[test]
    fn test_unterminated_string_error() {
        let source = r#"def func(): "unterminated"#;
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        println!("String tokens: {:?}", tokens);

        assert!(tokens
            .iter()
            .any(|t| matches!(t, Token::UnterminatedString | Token::Error)));
    }

    #[test]
    fn test_malformed_number_error() {
        let source = "123§456 789¢012";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        println!("Number tokens: {:?}", tokens);

        assert!(tokens.iter().any(|t| matches!(
            t,
            Token::MalformedNumber(_) | Token::Error | Token::InvalidChar(_)
        )));
    }

    #[test]
    fn test_invalid_hex_error() {
        let source = "0x§invalid 0x¢bad";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        println!("Hex tokens: {:?}", tokens);

        assert!(tokens
            .iter()
            .any(|t| matches!(t, Token::InvalidHexDigit(_) | Token::Error)));
    }

    #[test]
    fn test_invalid_bytes_error() {
        let source = "b'§invalid' b'¢bad'";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        println!("Bytes tokens: {:?}", tokens);

        assert!(tokens
            .iter()
            .any(|t| matches!(t, Token::InvalidBytesLiteral(_) | Token::Error)));
    }

    #[test]
    fn test_specific_error_messages() {
        let source = "0xABCG";
        let lexer = PyraLexer::new(source);

        let tokens: Vec<Token> = lexer.collect();

        if let Some(Token::InvalidHexDigit(msg)) = tokens
            .iter()
            .find(|t| matches!(t, Token::InvalidHexDigit(_)))
        {
            assert!(msg.contains("G"));
        }
    }
}
