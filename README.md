# 🔥 Pyra (WIP) – A Pythonic Smart Contract Language for the EVM

> A compiled, statically typed, Python-inspired smart contract language designed for the EVM. Pyra brings together developer ergonomics, blazing-fast execution, and gas-optimized output through zero-cost abstractions and ahead-of-time compilation.

---

## 🧠 Philosophy

- **Pythonic Syntax** – clean, readable, indentation-based
- **Static Typing** – bug-catching, compiler-verified type safety
- **Ahead-of-Time Compilation** – no VM overhead, optimized bytecode
- **Gas Efficiency by Design** – no hidden costs, predictable gas usage
- **Security-Focused** – reentrancy-safe, overflow/underflow checked
- **Minimal Runtime** – zero-cost abstractions, no runtime bloat
- **EVM-first** – compiles directly to EVM via Yul and `solc`

---

## 📐 Language Design

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
- Immutable-by-default variables
- Explicit mutability with `mut`
- Static typing with built-in types
- No inheritance – favor modularity
- No runtime garbage collection
- Compile-time constants using `const`

### Built-in Types
- `uint256`, `int256`, `bool`, `address`, `bytes`
- Fixed-size arrays
- `struct` (custom types)
- No dynamic arrays or mappings (v1)

## 🧱 Compiler Architecture

```
Source Code (.pyra)
  ↓
Lexer / Parser (Lark)
  ↓
AST (Abstract Syntax Tree)
  ↓
Type Checking & Semantic Analysis
  ↓
Intermediate Representation (IR)
  ↓
Code Generation (Yul output)
  ↓
solc --strict-assembly
  ↓
EVM Bytecode
```
## 🧰 Compiler Stack

| Stage | Tooling/Tech | Why It's Used |
|-------|--------------|---------------|
| Lexer / Parser | Lark | EBNF support, fast iteration |
| AST | Custom Python classes | Flexible and introspective |
| Type Checker | Custom in Python | Full control over types & coercion |
| IR | Optional internal instruction list | Easier codegen split from AST |
| Code Generator | Yul generator (string-based) | Lean, gas-efficient, human-readable |
| Backend Compiler | solc | Yul to EVM compilation |
| Testing | Foundry or Hardhat | Contract-level testing |
| CLI | argparse or click | `pyra build contracts/MyToken.pyra` |

## 🔐 Security Features

- Reentrancy protection decorators or default behavior
- Overflow / underflow checks (unless explicitly unchecked)
- Explicit storage access and gas-metered operations
- Immutable by default to prevent unintentional side effects
- `assert` and `require` for guarantees

## 🎯 v1 Goals

- [ ] Lexer and Parser with Lark
- [ ] AST implementation
- [ ] Type Checker
- [ ] Yul Code Generator
- [ ] Integration with solc
- [ ] Minimal CLI (`pyra build ...`)
- [ ] Example contracts (ERC20, vault)
- [ ] Tests using Foundry or Hardhat

## 📁 Project Structure

```
pyra/
├── pyra/                  # Main compiler code
│   ├── parser.py          # Lark grammar + lexer/parser
│   ├── ast.py             # AST node definitions
│   ├── typer.py           # Static type checking
│   ├── ir.py              # (Optional) Intermediate representation
│   ├── codegen.py         # Yul code generator
│   ├── compiler.py        # Compile pipeline
│   └── stdlib/            # Optional standard library
├── contracts/             # Example .pyra contracts
├── tests/                 # Hardhat/Foundry tests
├── cli.py                 # Entry point: `pyra build ...`
├── README.md
└── requirements.txt
```
## 🛣️ Roadmap

| Week | Milestone |
|------|-----------|
| 1–2 | Parser + AST |
| 3 | Type Checker |
| 4 | Yul Code Generator |
| 5 | CLI Tool + solc Integration |
| 6 | Working Contracts + Deployment |

## 🧠 Future Features

- ZKVM-compatible backend
- Gas profiler CLI
- WASM output support
- ABIs & Contract Interface Generator
- Multi-chain codegen (EVM + Solana IR switch)
- Built-in safe primitives for DeFi, ZK voting, DAOs

## 🧪 Quick Start (once CLI is ready)

```bash
# Compile a Pyra contract
$ pyra build contracts/MyToken.pyra

# Outputs:
# - MyToken.yul
# - MyToken.bin (via solc)
```
## 👨‍💻 Author

Built by [DavidIfebueme]

## ⚠️ Disclaimer

This is an experimental project. Use at your own risk. Not production-ready until v1 is officially tagged and audited.

