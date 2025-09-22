---
title: Updated `glam`, `rand` and `getrand` versions with new failures when building for web
pull_requests: [18047]
---

We've upgraded `glam` and the other math crates (`encase`, `hexasphere`) that move in lockstep to the latest versions.
With newer versions of `glam` & `encase`, the updated versions don't seem to have introduced breakages, though as always, best to consult their docs [1](https://docs.rs/glam/latest/glam/) [2](https://docs.rs/encase/0.11.0/encase/) for any changes.

This has also upgraded the version of `rand` and `getrand` that Bevy relies on.
`rand` changes are more extensive, with changes such as `thread_rng()` -> `rng()`, `from_entropy()` -> `from_os_rng()`, and so forth. `RngCore` is now split into infallible `RngCore` and fallible `TryRngCore`, and the `distributions` module has been renamed to `distr`. Most of this affects only internals, and doesn't directly affect Bevy's APIs. For the full set of changes, see `rand` [migration notes](https://rust-random.github.io/book/update-0.9.html).

`getrandom` is also updated, and will require additional configuration when building Bevy for WASM/web browsers.
**This will affect you even if you are not using `rand` or `getrand` directly,**
as `glam` (and thus `bevy_math`) will pull it in.

You may encounter an error like:

```toml
error: the wasm*-unknown-unknown targets are not supported by default;
to enable support, add this to your `Cargo.toml`:

[dependencies]
getrandom = { version = "0.2", features = ["js"] }
```

This is due to a breaking change in how `getrandom` handles entropy generation.
For security reasons, this is no longer specified via feature flags,
as any crate in your dependency tree could quietly enable additional entropy sources.

Quoting from the `getrandom` docs on [WebAssembly support in `getrandom`](https://github.com/rust-random/getrandom?tab=readme-ov-file#opt-in-backends) [2](https://github.com/rust-random/getrandom?tab=readme-ov-file#webassembly-support):

> To enable getrandom's functionality on wasm32-unknown-unknown using the Web Crypto methods described above via wasm-bindgen, do both of the following:
>
> 1. Use the wasm_js feature flag, i.e. getrandom = { version = "0.3", features = ["wasm_js"] }. On its own, this only makes the backend available. (As a side effect this will make your Cargo.lock significantly larger if you are not already using wasm-bindgen, but otherwise enabling this feature is harmless.)
> 2. Set RUSTFLAGS='--cfg getrandom_backend="wasm_js"' (see above).

Note that if you were previously setting the [`RUSTFLAGS` environment variable](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
for any reason, this will override any previous settings: you need to add this to your existing list instead.

If you were using the community-provided [`bevy_cli`](https://github.com/TheBevyFlock/bevy_cli) to easily create builds of your game for different platforms (including web),
make sure to update to the latest version of this tool where these requirements are handled for you.
