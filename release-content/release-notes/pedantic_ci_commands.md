---
title: Pedantic CI commands
authors: ["@hukasu"]
pull_requests: [todo]
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

### `bevy_app`

Runs clippy on `bevy_app` with multiple feature permutations.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 bevy_app`.

### `bevy_ecs`

Runs clippy on `bevy_ecs` with multiple feature permutations.

If you are on the workspace, run `cargo run -p ci -- --build-jobs 4 bevy_ecs`.