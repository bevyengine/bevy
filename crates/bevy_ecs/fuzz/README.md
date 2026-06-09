# bevy_ecs fuzz testing

Fuzz testing for `bevy_ecs`. The fuzzer feeds random byte sequences into test harnesses that exercise ECS operations, looking for crashes, panics, and logic bugs.

## How it works

Each fuzz target defines an enum of **operations** (spawn, despawn, insert, remove, query, etc.). The fuzzer generates random bytes, which the `arbitrary` crate decodes into a `Vec<Op>`. The harness then executes each operation in sequence against a real `World`.

**First level of verification**: no operation sequence should cause a panic, crash, or memory error (AddressSanitizer).

**Second level of verification**: where possible, the harness maintains a **shadow state**, a simple model of what the ECS state should look like. After operations, assertions compare the shadow against the real `World`.

## Running

Requires nightly Rust and `cargo-fuzz`:

```sh
cargo install cargo-fuzz

# Run a single target indefinitely (Ctrl+C to stop)
cargo +nightly fuzz run world_lifecycle

# Run for a fixed number of iterations
cargo +nightly fuzz run query_system -- -runs=10000
```
