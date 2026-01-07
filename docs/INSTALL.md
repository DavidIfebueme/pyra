# Installing Pyra (tester quickstart)

This repo ships the `pyra` CLI as the `pyra-compiler` Cargo package.

## Option A: Install from crates.io (recommended)

Requires: Rust toolchain installed.

```bash
cargo install --locked pyra-compiler
```

Verify:

```bash
pyra --help
pyra build contracts/ERC20.pyra
pyra build contracts/Vault.pyra
```

## Option B: Install from GitHub (fallback)

```bash
cargo install --locked --git https://github.com/DavidIfebueme/pyra pyra-compiler
```

## Option C: Install from a local checkout (contributors)

From the repo root:

```bash
cargo install --locked --path compiler
```

Verify:

```bash
pyra --help
pyra build contracts/ERC20.pyra
pyra build contracts/Vault.pyra
```

## Outputs

`pyra build path/to/Contract.pyra` writes:

- `Contract.abi`
- `Contract.bin`

By default these are written next to the input file unless `--out-dir` is provided.
