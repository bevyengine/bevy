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

## [super-linter](https://github.com/github/super-linter)

`super-linter` provides easy access to many different Linters.

### [markdownlint](https://github.com/DavidAnson/markdownlint)

`markdownlint` is provided by `super-linter` and is responsible for `.md` Files. It's configuration is saved in the [.markdown-lint.yml](../.github/linters/markdown-lint.yml) File.

The provided Rules are documented [here](https://github.com/DavidAnson/markdownlint/blob/main/doc/Rules.md) and Information about setting the Config can be seen [here](https://github.com/DavidAnson/markdownlint#optionsconfig).

#### Using [VSCode markdownlint](https://marketplace.visualstudio.com/items?itemName=DavidAnson.vscode-markdownlint)

(Currently not need as the Extension already disables the Rule `MD013` by default.)

If you want to use the VSCode Extension with the Rules defined in [.markdown-lint.yml](../.github/linters/markdown-lint.yml), than you need to create a local config File in the Root of the Project, with this base configuration:

```json
{
  "extends": "./.github/linters/.markdown-lint.yml"
}
```

### Other Linters provided by [super-linter](https://github.com/github/super-linter)

All other Linters not mentioned in the this file are not activated and can be seen [here](https://github.com/github/super-linter#supported-linters).
