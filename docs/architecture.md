# Pyra Compiler Architecture

This document describes the internal architecture of the Pyra compiler.

## Compilation Pipeline

```
Source Code (.pyra) → Lexer → Parser → AST → ABI + Codegen → EVM Bytecode
```

## Modules

### Lexer (`lexer.rs`)
- Tokenizes source code using `logos`
- Handles Pythonic indentation and whitespace
- Produces token stream for parser

### Parser (`parser.rs`)  
- Uses `chumsky` parser combinators
- Converts token stream to Abstract Syntax Tree (AST)
- Handles Python-like, indentation-based syntax

### AST (`ast.rs`)
- Defines all AST node types
- Represents the structure of Pyra programs
- Includes type information and metadata

### Type Checker (`typer.rs`)
- Planned module; not currently wired into the compilation pipeline.

### Verifier (`verifier.rs`)
- Planned module; not currently wired into the compilation pipeline.

### Gas Estimator (`gas.rs`)
- Planned module; not currently wired into the compilation pipeline.

### Code Generator (`codegen.rs`)
- Generates deployable EVM bytecode for the supported subset.
- Direct bytecode generation (no Yul dependency).

### Security Analyzer (`security.rs`)
- Planned module; not currently wired into the compilation pipeline.