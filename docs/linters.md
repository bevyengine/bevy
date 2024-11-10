# Linters in this Repository

## Code Format Linting with [rustfmt](https://github.com/rust-lang/rustfmt)

Can be automatically validated with [`cargo run -p ci`](../tools/ci) (which also runs other checks). Running this command will actually format the code:

```bash
cargo fmt --all
```

## Code Linting with [Clippy](https://github.com/rust-lang/rust-clippy)

Can be automatically run with [`cargo run -p ci`](../tools/ci) (which also runs other checks) or manually with this command:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Explanation:

- `-D warnings`: No warnings are allowed in the codebase.

## [super-linter](https://github.com/github/super-linter)

`super-linter` provides easy access to many different Linters.

### [markdownlint](https://github.com/DavidAnson/markdownlint)

`markdownlint` is provided by `super-linter` and is responsible for `.md` files.
Its configuration is saved in [.markdownlint.jsonc](../.markdownlint.jsonc) with various folders extending it to create localized exceptions.

The provided rules are documented [here](https://github.com/DavidAnson/markdownlint/blob/main/doc/Rules.md) and information about setting the config can be seen [here](https://github.com/DavidAnson/markdownlint#optionsconfig).

### Other Linters provided by [super-linter](https://github.com/github/super-linter)

All other linters not mentioned in this file are not activated and can be seen [here](https://github.com/github/super-linter#supported-linters).
