# Pyra Compiler Architecture

This document describes the internal architecture of the Pyra compiler.

## Compilation Pipeline

```
Source Code (.pyra) → Lexer → Parser → AST → Type Checker → Verifier → Code Generator → EVM Bytecode
```

## Modules

### Lexer (`lexer.rs`)
- Tokenizes source code using `nom` parser combinator library
- Handles Pythonic indentation and whitespace
- Produces token stream for parser

### Parser (`parser.rs`)  
- Uses `lalrpop` for parser generation
- Converts token stream to Abstract Syntax Tree (AST)
- Handles Python-like syntax with static typing

### AST (`ast.rs`)
- Defines all AST node types
- Represents the structure of Pyra programs
- Includes type information and metadata

### Type Checker (`typer.rs`)
- Performs static type checking
- Handles generic type resolution
- Ensures type safety across the program

### Verifier (`verifier.rs`)
- Formal verification using Z3 SMT solver
- Checks for security vulnerabilities
- Verifies contract correctness properties

### Gas Estimator (`gas.rs`)
- Calculates gas costs at compile time
- Provides accurate gas estimates for operations
- Optimizes for minimal gas usage

### Code Generator (`codegen.rs`)
- Generates optimized EVM bytecode
- Direct bytecode generation (no Yul dependency)
- Applies optimizations for gas efficiency

### Security Analyzer (`security.rs`)
- Automatic reentrancy protection
- Overflow/underflow detection
- Security pattern enforcement