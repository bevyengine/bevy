---
title: Updated `glam` and `rand` versions.
pull_requests: [18047]
---

With newer versions of `glam` & `encase`, the updated versions don't seem to have introduced breakages, though as always, best to consult their docs [1](https://docs.rs/glam/latest/glam/) [2](https://docs.rs/encase/0.11.0/encase/) for any changes.

`rand` changes are more extensive, with changes such as `thread_rng()` -> `rng()`, `from_entropy()` -> `from_os_rng()`, and so forth. `RngCore` is now split into infallible `RngCore` and fallible `TryRngCore`, and the `distributions` module has been renamed to `distr`. Most of this affects only internals, and doesn't directly affect Bevy's APIs. For the full set of changes, see `rand` [migration notes](https://rust-random.github.io/book/update-0.9.html).

`getrandom` is also updated, and will require additional configuration when building Bevy for WASM/Web (if also using `rand`). The full details of how to do this is in the `getrandom` docs [1](https://github.com/rust-random/getrandom?tab=readme-ov-file#opt-in-backends) [2](https://github.com/rust-random/getrandom?tab=readme-ov-file#webassembly-support).
