# Bevy Benchmarks

This is a crate with a collection of benchmarks for Bevy.

## Running benchmarks

Benchmarks can be run through Cargo:

```sh
# Run all benchmarks. (This will take a while!)
cargo bench -p benches

# Just compile the benchmarks, do not run them.
cargo bench -p benches --no-run

# Run the benchmarks for a specific crate. (See `Cargo.toml` for a complete list of crates
# tracked.)
cargo bench -p benches --bench ecs

# Filter which benchmarks are run based on the name. This will only run benchmarks whose name
# contains "name_fragment".
cargo bench -p benches -- name_fragment

# List all available benchmarks.
cargo bench -p benches -- --list

# Save a baseline to be compared against later.
cargo bench -p benches -- --save-baseline before

# Compare the current benchmarks against a baseline to find performance gains and regressions.
cargo bench -p benches -- --baseline before
```

## Criterion

Bevy's benchmarks use [Criterion](https://crates.io/crates/criterion). If you want to learn more about using Criterion for comparing performance against a baseline or generating detailed reports, you can read the [Criterion.rs documentation](https://bheisler.github.io/criterion.rs/book/criterion_rs.html).

You can generate nicely formatted comparison tables of named benchmark runs (baselines) using the [critcmp](https://github.com/BurntSushi/critcmp) tool.

## WebAssembly

The benchmarks can also be run on `wasm32-wasip1`, which is useful for catching performance problems specific to 32-bit targets. Build them with `--no-run`, then run the resulting `.wasm` file with a WASI runtime such as [Wasmtime](https://wasmtime.dev/) or [Wasmer](https://wasmer.io/), giving it access to the current directory so Criterion can save its results:

```sh
rustup target add wasm32-wasip1

# Build the benchmarks, noting the path of the `.wasm` file that is printed. Only enable the
# features for the crate being benchmarked, as not every crate supports WASI.
cargo bench -p benches --bench ecs --no-default-features --features bench_ecs --target wasm32-wasip1 --no-run

# Run the benchmarks, passing arguments to Criterion after `--bench`.
wasmtime run --dir=. target/wasm32-wasip1/release/deps/ecs-<hash>.wasm --bench entity_hash
# Or with Wasmer, which needs a `--` before the arguments.
wasmer run --dir=. target/wasm32-wasip1/release/deps/ecs-<hash>.wasm -- --bench entity_hash
```
