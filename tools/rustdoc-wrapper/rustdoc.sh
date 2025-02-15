#!/bin/env sh

# Use a different target directory because the workspace-level one will be locked.
cargo build --release --target-dir tools/rustdoc-wrapper/target
# Pass on all arguments to our rustdoc wrapper.
tools/rustdoc-wrapper/target/release/rustdoc-wrapper "$@"
