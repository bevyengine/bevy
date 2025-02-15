#!/bin/env sh

# Use a different target directory because the workspace-level one will be locked.
cargo build --package rustdoc-wrapper --release --target-dir tools/rustdoc-wrapper/target --color always
# Pass on all arguments to our rustdoc wrapper.
tools/rustdoc-wrapper/target/release/rustdoc-wrapper "$@"
