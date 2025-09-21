---
title: Hot Patching Systems in a Running App
authors: ["@mockersf"]
pull_requests: [19309]
---

Bevy now supports hot patching systems via [subsecond](https://crates.io/crates/subsecond) and the [`dx`](https://crates.io/crates/dioxus-cli) command line tool from the Dioxus project.

When the cargo feature `hotpatching` is enabled, every system can now be modified during execution, and the changes are immediately visible in your game.

`dx` is the Dioxus CLI. To install it run `cargo install dioxus-cli@0.7.0-alpha.1`
TODO: use the fixed version that will match the version of subsecond dependency used in Bevy at release time

Then run `BEVY_ASSET_ROOT="." dx serve --hot-patch --features "bevy/hotpatching"` to test it in your project. You can also try it out using Bevy's [`hotpatching_systems.rs`](https://github.com/bevyengine/bevy/blob/release-0.17.0/examples/ecs/hotpatching_systems.rs) example.

This is just the first step. There are known limitations:

- Only works on the binary crate. Dioxus has plans to expand support here.
- Not supported in Wasm. Dioxus supports this, but the Bevy side needs some work.
- If the system's parameters change, it will not be hot reloaded. This is something we need to work out on the Bevy side.
- It may be sensitive to rust/linker configuration. Dioxus is already pretty good about this though!

We have plans to further expand support, including making the upcoming [`bsn!` macro](https://github.com/bevyengine/bevy/pull/20158/) hot-reloadable (check out [this video](/news/bevys-fifth-birthday/#bevy-hot-reloading) of it in action!).
