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
