# Compile fail tests for Bevy macros

This crate is not part of the Bevy workspace in order to not fail `crater` tests for Bevy.
The tests assert on the exact compiler errors and can easily fail for new Rust versions due to updated compiler errors (e.g. changes in spans).

The `CI` workflow executes these tests on the stable rust toolchain (see [tools/ci](../../tools/ci/src/main.rs)).
