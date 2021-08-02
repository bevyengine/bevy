# Bevy Lint

## What is Bevy Lint?

This crates provides Lints for Bevy Code using [dylint](https://github.com/trailofbits/dylint).

## How to you run Lints

Add this to your Cargo.toml:

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/bevyengine/bevy", tag = "v0.6.0", pattern = "crates/bevy_dylint" },
]
```

Instead of a `tag`, you can also provide a `branch` or a `rev` (revision).

Afterwards you need to run these commans:

```sh
cargo install cargo-dylint dylint-link    # Only neccesary once
cargo dylint bevy_dylint
```

## Lint Creation

A Lint is created by implementing the [LateLintPass](https://doc.rust-lang.org/stable/nightly-rustc/rustc_lint/trait.LateLintPass.html) trait and adding to the `register_lints` function.

When creating a UI Test, add the Test as an Example to the [Cargo.toml](Cargo.toml).
Also make sure that your `.stderr` File uses `LF` Line-endings and not `CRLF`, as otherwise the Test will fail without any explanation.

For more Resources you can take a look at the [dylint resources](https://github.com/trailofbits/dylint#resources).
