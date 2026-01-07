# Installing Pyra (tester quickstart)

This repo ships the `pyra` CLI as the `pyra-compiler` Cargo package.

## Option A: Install from GitHub (recommended for testers)

Requires: Rust toolchain installed.

```bash
cargo install --git https://github.com/DavidIfebueme/pyra --package pyra-compiler
```

Verify:

```bash
pyra --help
pyra build contracts/ERC20.pyra
pyra build contracts/Vault.pyra
```

## Option B: Install from a local checkout (contributors)

From the repo root:

```bash
cargo install --path compiler
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
