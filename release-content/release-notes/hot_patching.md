---
title: Hot Patching Systems in a Running App
authors: ["@mockersf"]
pull_requests: [19309]
---

Bevy now supports hot patching systems through subsecond from the Dixous project.

Enabled with the feature `hotpatching`, every system can now be modified during execution, and the change directly visible in your game.

Run `BEVY_ASSET_ROOT="." dx serve --hot-patch --example hotpatching_systems --features hotpatching` to test it.

`dx` is the Dioxus CLI, to install it run `cargo install dioxus-cli@0.7.0-alpha.1`
TODO: use the fixed version that will match the version of subsecond dependency used in Bevy at release time

Known limitations:

- Only works on the binary crate (todo: plan to support it in Dioxus)
- Not supported in Wasm (todo: supported in Dioxus but not yet implemented in Bevy)
- No system signature change support (todo: add that in Bevy)
- May be sensitive to rust/linker configuration (todo: better support in Dioxus)
