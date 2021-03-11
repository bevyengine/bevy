# Linters in this Repository

## [rustfmt](https://github.com/rust-lang/rustfmt)

Can be automatically be run with the [CI Script](../tools/ci) together with `Clippy` or manually with this command:

```bash
cargo +nightly fmt --all
```

## [Clippy](https://github.com/rust-lang/rust-clippy)

Can be automatically be run with the [CI Script](../tools/ci) together with `rustfmt` or manually with this command:

```bash
cargo clippy --all-targets --all-features -- -D warnings -A clippy::type_complexity -A clippy::manual-strip
```

* `-D warnings`: No warnings are allowed in the codebase.
* `-A clippy::type_complexity`: type complexity must be ignored because we use huge templates for queries.

## [super-linter](https://github.com/github/super-linter)

`super-linter` provides easy access to many different Linters.

### [markdownlint](https://github.com/DavidAnson/markdownlint)

`markdownlint` is provided by `super-linter` and is responsible for `.md` files.
Its configuration is saved in the [.markdown-lint.yml](../.github/linters/markdown-lint.yml) file.

The provided rules are documented [here](https://github.com/DavidAnson/markdownlint/blob/main/doc/Rules.md) and information about setting the config can be seen [here](https://github.com/DavidAnson/markdownlint#optionsconfig).

#### Using [VS Code markdownlint](https://marketplace.visualstudio.com/items?itemName=DavidAnson.vscode-markdownlint)

If you want to use the VS Code Extension with the rules defined in [.markdown-lint.yml](../.github/linters/markdown-lint.yml), then you need to create a local config file in the root of the project with the configuration below.
Currently, this is not needed as the extension already disables the rule `MD013` by default.

```json
{
  "extends": ".github/linters/.markdown-lint.yml"
}
```

### Other Linters provided by [super-linter](https://github.com/github/super-linter)

All other linters not mentioned in the this file are not activated and can be seen [here](https://github.com/github/super-linter#supported-linters).
