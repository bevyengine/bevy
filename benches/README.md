# Bevy Benchmarks

This is a crate with a collection of benchmarks for Bevy, separate from the rest of the Bevy crates.

## Running the benchmarks

1. Setup everything you need for Bevy with the [setup guide](https://bevyengine.org/learn/book/getting-started/setup/).
2. Move into the `benches` directory (where this README is located).

    ```sh
    bevy $ cd benches
    ```

3. Run the benchmarks with cargo (This will take a while)

    ```sh
    bevy/benches $ cargo bench
    ```

    If you'd like to only compile the benchmarks (without running them), you can do that like this:

    ```sh
    bevy/benches $ cargo bench --no-run
    ```

## Criterion

Bevy's benchmarks use [Criterion](https://crates.io/crates/criterion). If you want to learn more about using Criterion for comparing performance against a baseline or generating detailed reports, you can read the [Criterion.rs documentation](https://bheisler.github.io/criterion.rs/book/criterion_rs.html).
