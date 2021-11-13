# UI tests for bevy_ecs

This crate is separate from `bevy_ecs` and not part of the Bevy workspace in order to not fail `crater` tests for Bevy. UI tests often fail for new Rust versions due to changed compiler output.

The `CI` workflow executes the UI tests on the stable rust toolchain (see [tools/ci](../../tools/ci/src/main.rs)).
