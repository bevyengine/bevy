---
title: rand, glam & uuid updated to latest versions
pull_requests: [22928]
---

`glam` & `uuid` don't impose any new migration notes other than support for the latest `rand` version.

For `rand`/`rand_core`, the `RngCore` trait is now `Rng` and `Rng` trait is now `RngExt`, as such this will need updated wherever the traits are being used to resolve any compilation errors. For the full extent of the changes to `rand` v0.10, consult the [`rand` book here](https://rust-random.github.io/book/update-0.10.html).

`getrandom` v0.4 does nothing new for Web WASM support, so toggling the `wasm_js` feature in `wasm32-unknown-unknown` builds will be enough to enable it to compile.
