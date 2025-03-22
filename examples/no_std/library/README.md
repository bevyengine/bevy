# Bevy `no_std` Compatible Library

This example demonstrates how to create a `no_std`-compatible library crate for use with Bevy.
For the sake of demonstration, this library adds a way for a component to be added to an entity after a certain delay has elapsed.
Check the [Cargo.toml](Cargo.toml) and [lib.rs](src/lib.rs) for details around how this is implemented, and how we're able to make a library compatible for all users in the Bevy community.

## Testing `no_std` Compatibility

To check if your library is `no_std` compatible, it's not enough to just compile with your `std` feature disabled.
The problem is dependencies can still include `std` even if the top-most crate is declared as `#![no_std]`.
Instead, you need to compile your library without the standard library at all.

The simplest way to compile Rust code while ensuring `std` isn't linked is to simply use a target without the standard library.
Targets with [Tier 2](https://doc.rust-lang.org/beta/rustc/platform-support.html#tier-2-without-host-tools) or [Tier 3](https://doc.rust-lang.org/beta/rustc/platform-support.html#tier-3) support often do not have access to `std`, and therefore can _only_ compile if `no_std` compatible.

Some recommended targets you can check against are:

* [`x86_64-unknown-none`](https://doc.rust-lang.org/beta/rustc/platform-support/x86_64-unknown-none.html)
  * Representative of desktop architectures.
  * Should be the most similar to typical `std` targets so it's a good starting point when porting existing libraries.
* [`wasm32v1-none`](https://doc.rust-lang.org/beta/rustc/platform-support/wasm32v1-none.html)
  * Newer WebAssembly target with the bare minimum functionality for broad compatibility.
  * Similar to `wasm32-unknown-unknown`, which is typically used for web builds.
* [`thumbv6m-none-eabi`](https://doc.rust-lang.org/beta/rustc/platform-support/thumbv6m-none-eabi.html)
  * Representative of embedded platforms.
  * Has only partial support for atomics, making this target a good indicator for atomic incompatibility in your code.

Note that the first time you attempt to compile for a new target, you will need to install the supporting components via `rustup`:

```sh
rustup target add x86_64-unknown-none
```

Once installed, you can check your library by specifying the appropriate features and target:

```sh
cargo check --no-default-features --features libm,critical-section --target x86_64-unknown-none
```

### CI

Checking `no_std` compatibility can be tedious and easy to forget if you're not actively using it yourself.
To avoid accidentally breaking that compatibility, we recommend adding these checks to your CI pipeline.
For example, here is a [GitHub Action](https://github.com/features/actions) you could use as a starting point:

```yml
jobs:
  check-compiles-no-std:
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        target:
          - "x86_64-unknown-none"
          - "wasm32v1-none"
          - "thumbv6m-none-eabi"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - name: Check Compile
        run: cargo check --no-default-features --features libm,critical-section --target ${{ matrix.target }}
```
