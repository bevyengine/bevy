# bevy_scene_macros fuzz testing

Fuzz testing for the `bsn!` and `bsn_list!` proc-macros. The fuzzer feeds random UTF-8 strings into the BSN parser and code generator, looking for panics.

## How it works

This crate re-includes `bsn/{types,parse,codegen}.rs` directly via `#[path]` mod declarations.

Each fuzz target:

1. Decodes the input as a UTF-8 `&str`.
2. Feed it into `proc_macro2::TokenStream`.
3. Parses it as `BsnRoot` or `BsnListRoot`. Parse errors are fine, panics are bugs.
4. On parse success, runs codegen. Codegen routes validation errors through `ctx.errors`, panics are bugs.

Targets:

- `bsn` - exercises the `bsn!` macro entry point (`BsnRoot`).
- `bsn_list` - exercises the `bsn_list!` macro entry point (`BsnListRoot`).

## Running

Requires nightly Rust and `cargo-fuzz`:

```sh
cargo install cargo-fuzz

# Run a single target indefinitely (Ctrl+C to stop).
cargo +nightly fuzz run bsn

# Run for a fixed number of iterations
cargo +nightly fuzz run bsn -- -runs=10000
```
