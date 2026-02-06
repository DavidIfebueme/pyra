# Pyra: a smart contract language for the EVM

A compiled, statically typed language with Python-like syntax. It compiles directly to EVM bytecode. The focus is clarity, safety, and predictable gas use.

## Philosophy

- Python-like, indentation-based syntax
- Static typing
- Ahead-of-time compilation
- Predictable gas usage
- Security-focused: reentrancy checks and arithmetic checks
- Minimal runtime
- EVM-first: compiles directly to EVM bytecode (no Yul dependency)

## Language design

### Syntax

Inspired by Python, but statically typed and compiled:

```py
let total_supply: uint256 = 10000

def transfer(to: address, amount: uint256):
    if amount > 0:
        call_contract(to, amount)
```

### Features

- Indentation-based blocks
- Immutable by default; use `mut` for mutable state
- Static typing with built-in types
- Abstractions compile with no runtime overhead
- Generics/templates are resolved at compile time
- Reentrancy protection is enabled by default
- Compile-time gas estimation
- Built-in hooks for formal verification
- No inheritance; prefer modular composition
- No runtime garbage collection
- Compile-time constants using `const`

### Built-in types

- `uint256`, `int256`, `bool`, `address`, `bytes`
- Fixed-size arrays
- `struct` (custom types)
- Generic containers such as `Vec<T>` and `Map<K, V>` (compile-time optimized)
- No dynamic arrays or mappings in v1

## Compiler architecture

```
Source Code (.pyra)
  ↓
Lexer (logos) → Token stream with Indent/Dedent
  ↓
Parser (chumsky) → AST
  ↓
Type Checker → scoped symbol table + storage-aware globals
  ↓
Storage Layout → auto-discovered slot allocation
  ↓
IR Lowering → intermediate representation with 40+ opcodes
  ↓
Safety Hardening → checked add/sub/mul + reentrancy guards
  ↓
Bytecode Verification → jump safety + label validation
  ↓
Code Generation → EVM bytecode with ABI function dispatch
  ↓
Deploy Bytecode (.bin) + ABI JSON (.abi)
```

### Compiler notes

- Implemented in Rust
- Single-pass compilation: parse, type-check, and verify in one pass
- Direct bytecode generation: no Yul or solc dependency
- Parallel and incremental compilation
- Zero-copy parsing to reduce allocations

## Compiler stack

| Stage | Tooling/Tech | Notes |
|-------|--------------|-------|
| Lexer/Parser | logos + chumsky (Rust) | Zero-copy parsing with indentation tracking |
| AST + Type Checker | Custom Rust structs | Scoped symbol table with storage-aware globals |
| Storage Layout | Auto-discovery engine | Sequential slot allocation from AST analysis |
| IR Lowering | Custom IR with 40+ opcodes | AST → IR with selector computation |
| Safety Hardening | Overflow/underflow/reentrancy | Automatic checked math and mutex guards |
| Gas Estimator | Static analysis engine | Per-function gas cost prediction |
| Code Generator | IR → EVM bytecode (Rust) | ABI dispatch + label resolution |
| Bytecode Verifier | Jump/label validation | Orphan jump and duplicate label detection |
| CLI | clap (Rust) | `pyra build contracts/MyToken.pyra --gas-report` |

## How it differs from Vyper

### 1. Abstractions without runtime cost

Write high-level code that compiles to the same bytecode as hand-written versions:

```py
# This generic function...
def safe_add<T: Numeric>(a: T, b: T) -> T:
    return a + b

# ...generates identical bytecode to:
def add_uint256(a: uint256, b: uint256) -> uint256:
    return a + b
```

### 2. Compile-time gas estimation

```py
# Compiler provides gas costs:
def transfer(to: address, amount: uint256):  # Gas: 21,000 + 5,000 SSTORE
    balances[msg.sender] -= amount           # Gas: 5,000 SSTORE
    balances[to] += amount                   # Gas: 20,000 SSTORE
```

### 3. Built-in formal verification

```py
def withdraw(amount: uint256):
    require amount <= balances[msg.sender]
    # @verify: balance_sum_invariant
    # @verify: no_overflow_underflow
    # @verify: reentrancy_safe
    balances[msg.sender] -= amount
    msg.sender.transfer(amount)
```

### 4. Generics and templates

Type-safe code reuse without runtime cost:

```py
# Generic data structures
struct Vault<T: Token> {
    token: T,
    balance: uint256
}

# Generic functions with constraints
def swap<A: ERC20, B: ERC20>(token_a: A, token_b: B, amount: uint256)
```

### 5. Automatic reentrancy protection

Built into the language:

```py
def withdraw(amount: uint256):
    # Automatically generates a reentrancy guard
    balances[msg.sender] -= amount
    msg.sender.transfer(amount)  # Guarded by default
```

## Security features

- Automatic reentrancy protection on external calls
- Overflow and underflow checks unless explicitly marked `unchecked`
- Formal verification integration
- Compile-time bounds checking for array access
- Immutable by default to prevent accidental state changes
- Gas limit analysis
- Compile-time integer overflow detection

## v1 goals

- [x] Rust-based lexer and parser with logos and chumsky
- [x] Single-pass AST and type checker
- [x] Direct EVM bytecode generator
- [x] Compile-time gas estimation
- [ ] Basic formal verification (Z3 integration)
- [x] Automatic reentrancy protection
- [ ] Generics with no runtime overhead
- [x] CLI tool (`pyra build ...`)
- [x] Example contracts (ERC20, vault)

## Project structure

```
pyra/
├── compiler/              # Rust compiler codebase
│   ├── src/
│   │   ├── lexer.rs       # logos-based lexer with indentation tracking
│   │   ├── parser.rs      # chumsky grammar (functions, structs, events, control flow)
│   │   ├── ast.rs         # AST definitions
│   │   ├── typer.rs       # Static type checker with scoped symbol table
│   │   ├── storage.rs     # Storage layout engine with auto-discovery
│   │   ├── ir.rs          # Intermediate representation (AST → IR lowering)
│   │   ├── codegen.rs     # IR → EVM bytecode with ABI dispatch
│   │   ├── gas.rs         # Per-function gas estimation engine
│   │   ├── security.rs    # Overflow/underflow checks + reentrancy guards
│   │   ├── verifier.rs    # Bytecode verification (jump safety, label checks)
│   │   ├── abi.rs         # ABI JSON generation (functions, constructors, events)
│   │   ├── evm.rs         # Low-level EVM helpers
│   │   ├── compiler.rs    # Compilation driver
│   │   └── bin/pyra.rs    # CLI entry point
│   ├── benches/           # Criterion benchmarks
│   ├── tests/             # Integration tests
│   └── Cargo.toml
├── contracts/             # Example .pyra contracts
├── stdlib/                # Standard library (Pyra code)
├── docs/                  # Architecture and language reference
└── README.md
```

## Roadmap

| Week | Milestone |
|------|-----------|
| 1-2 | Rust lexer and parser plus basic AST |
| 3 | Type checker and generics |
| 4 | Direct EVM code generation and gas estimation |
| 5 | Formal verification and security analysis |
| 6 | CLI tool and working contracts |

## Future features

- ZKVM-compatible backend (Cairo, Risc0)
- Gas profiler CLI with optimization suggestions
- WASM output for off-chain simulation
- Contract interface generator (ABIs)
- Multi-chain support (EVM, Solana, Move)
- Built-in DeFi primitives (AMM, lending, governance)
- IDE integration (VS Code, Neovim)

## Quick start

```bash
# Install (crates.io)
cargo install --locked pyra-compiler

# Compile example contracts
pyra build contracts/ERC20.pyra
pyra build contracts/Vault.pyra

# Compile with gas report
pyra build contracts/ERC20.pyra --gas-report
```

Fallback (GitHub):

```bash
cargo install --locked --git https://github.com/DavidIfebueme/pyra pyra-compiler
```

Outputs:

- `<Contract>.abi`
- `<Contract>.bin`

## Performance

The compiler includes Criterion benchmarks under `compiler/benches/` to track lexer/parse/codegen performance.

## Author

Built by DavidIfebueme

## Disclaimer

This is an experimental project. Use at your own risk. Not production-ready until v1 is officially tagged and audited.
