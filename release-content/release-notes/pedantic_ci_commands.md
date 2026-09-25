---
title: Pedantic CI commands
authors: ["@hukasu"]
pull_requests: [21145]
---

New commands were added to CI tool to allow running clippy with multiple permutations of
features. For a crate it is run with:

* `--no-default-features`
* Multiple `--no-default-features --features="..."`
* Default features
* Multiple `--all-features --features="..."`

There are crates that are run with multiple `--all-features` due to them depending
another crate that has a switchable features. Examples are `bevy_image/zstd_c` or
`bevy_image/zstd_rust`, or `bevy_reflect/auto_register_inventory` or `bevy_reflect/auto_register_static`.

## Commands

### `clippys`

This is a meta commands that runs the other clippy permutation commands.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 clippys`.

### `clippy_android`

Runs clippy on crates for Android targets. Requires an Android
target.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 --target aarch64-linux-android clippy_android`.

### `clippy_dlss`

Runs clippy on crates and features that require the Dlss SDK.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 clippy_dlss`.

### `bevy_a11y`

Runs clippy on `bevy_a11y` with multiple feature permutations.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 bevy_a11y`.

### `bevy_android`

Runs clippy on `bevy_android` with multiple feature permutations. Requires an Android
target.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 --target aarch64-linux-android bevy_android`.

### `bevy_animation`

Runs clippy on `bevy_animation` with multiple feature permutations.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 bevy_animation`.

### `bevy_anti_alias`

Runs clippy on `bevy_anti_alias` with multiple feature permutations.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 bevy_anti_alias`.

### `bevy_app`

Runs clippy on `bevy_app` with multiple feature permutations.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 bevy_app`.

### `bevy_ecs`

Runs clippy on `bevy_ecs` with multiple feature permutations.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 bevy_ecs`.
